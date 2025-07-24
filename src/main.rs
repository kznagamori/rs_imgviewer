#![windows_subsystem = "windows"]
//! rs_imgviewer: Rust 製高速画像ビューアー
//!
//! JPEG/PNG/WEBP に対応し、ディレクトリ指定時はソートされた画像リストを順次表示します。

use clap::Parser;
use fern::Dispatch;
use log::{error, info};
use serde::Deserialize;
use show_image::{create_window, event, WindowOptions, ImageView, ImageInfo};
use show_image::event::WindowEvent;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use image::{GenericImageView, DynamicImage};

#[cfg(feature = "native-win")]
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

/// コマンドライン引数
#[derive(Parser)]
#[command(name = "rs_imgviewer", about = "高速画像ビューアー")]
struct Args {
    /// 表示するファイルまたはディレクトリのパス
    path: PathBuf,
}

/// 設定ファイル (`rs_imgviewer.toml`) の内容
#[derive(Debug, Deserialize)]
struct Config {
    /// 最小ウィンドウ幅（ピクセル）
    min_window_width: u32,
    /// 最小ウィンドウ高（ピクセル）
    min_window_height: u32,
    /// ソートアルゴリズム
    sort_algorithm: SortAlgorithm,
}

/// 画像ファイルのソートアルゴリズム
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
enum SortAlgorithm {
    FileName,
    CreatedTime,
    ModifiedTime,
}

/// エントリポイント（GUI コンテキストを初期化）
#[show_image::main]
fn main() -> Result<(), Box<dyn Error>> {
    // ロガー初期化
    init_logger()?;

    // 引数パース
    let args = Args::parse();

    // 設定ファイル読み込み（存在しない場合はデフォルトを生成）
    let exe_path = std::env::current_exe()?;
    let exe_dir = exe_path.parent().unwrap().to_path_buf();
    let config_path = exe_dir.join("rs_imgviewer.toml");
    if !config_path.exists() {
        let default = r#"# rs_imgviewer.toml
# 最小ウィンドウサイズ（ピクセル）
min_window_width  = 800
min_window_height = 600

# ソートアルゴリズム: "FileName" | "CreatedTime" | "ModifiedTime"
sort_algorithm = "FileName"
"#;
        fs::write(&config_path, default)?;
        info!("rs_imgviewer.toml が見つからなかったため、デフォルトファイルを生成: {}", config_path.display());
    }
    let config: Config = toml::from_str(&fs::read_to_string(&config_path)?)?;
    info!("Loaded config: {:?}", config);

    // 画像ファイル一覧取得
    let (dir, mut idx) = if args.path.is_file() {
        let dir = args.path.parent().unwrap().to_path_buf();
        let files = collect_image_paths(&dir, &config.sort_algorithm);
        let i = find_initial_index(&files, &args.path);
        (dir, i)
    } else {
        (args.path.clone(), 0)
    };
    let files = collect_image_paths(&dir, &config.sort_algorithm);
    if files.is_empty() {
        error!("対象の画像ファイルが見つかりません: {:?}", dir);
        return Ok(());
    }
    info!("{} ファイルを {} でロード", files.len(), dir.display());

    // 最初の画像をロードして表示
    let dyn_img = image::open(&files[idx])?;
    let rgba = dyn_img.to_rgba8();
    let (iw, ih) = rgba.dimensions();
    let raw = rgba.into_raw();
    let (disp_w, disp_h) = compute_display_size(&dyn_img, &config);
    let window = create_window(
        "rs_imgviewer",
        WindowOptions {
            size: Some([disp_w, disp_h]),
            ..Default::default()
        },
    )?;
    window.set_image("img-0", ImageView::new(ImageInfo::rgba8(iw, ih), &raw))?;
    let event_rx = window.event_channel()?;

    // イベントループ
    'outer: loop {
        if let Ok(ev) = event_rx.recv() {
            if let WindowEvent::KeyboardInput(k) = ev {
                if k.input.state == event::ElementState::Pressed {
                    use event::VirtualKeyCode;
                    match (k.input.key_code, k.input.modifiers.alt()) {
                        (Some(VirtualKeyCode::Return), _)
                        | (Some(VirtualKeyCode::Escape), _)
                        | (Some(VirtualKeyCode::F4), true) => break 'outer,
                        (Some(VirtualKeyCode::Right), _)
                        | (Some(VirtualKeyCode::X), _) => idx = (idx + 1) % files.len(),
                        (Some(VirtualKeyCode::Left), _)
                        | (Some(VirtualKeyCode::Z), _) => idx = (idx + files.len() - 1) % files.len(),
                        _ => continue,
                    }
                    // 画像更新
                    let dyn_img = image::open(&files[idx])?;
                    let rgba = dyn_img.to_rgba8();
                    let (iw, ih) = rgba.dimensions();
                    let raw = rgba.into_raw();
                    let (_disp_w, _disp_h) = compute_display_size(&dyn_img, &config);
                    window.set_image("img-0", ImageView::new(ImageInfo::rgba8(iw, ih), &raw))?;
                }
            }
        }
    }

    Ok(())
}

/// ロガーを初期化する
fn init_logger() -> Result<(), fern::InitError> {
    Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] {}",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S"),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

/// 指定ディレクトリから画像ファイルを収集し、ソートして返す
fn collect_image_paths(dir: &Path, alg: &SortAlgorithm) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let p = entry.path();
            if p.is_file() && p.extension().and_then(|e| e.to_str())
                .map(|e| matches!(e.to_lowercase().as_str(), "jpg"|"jpeg"|"png"|"webp"))
                .unwrap_or(false)
            {
                files.push(p);
            }
        }
    }
    match alg {
        SortAlgorithm::FileName => {
            files.sort_by(|a, b| {
                let na = a.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                let nb = b.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                if let (Ok(x), Ok(y)) = (na.parse::<u64>(), nb.parse::<u64>()) {
                    x.cmp(&y)
                } else {
                    na.to_lowercase().cmp(&nb.to_lowercase())
                }
            });
        }
        SortAlgorithm::CreatedTime => files.sort_by_key(|p| {fs::metadata(p).and_then(|m| m.created()).unwrap_or(SystemTime::UNIX_EPOCH)}),
        SortAlgorithm::ModifiedTime => files.sort_by_key(|p| {fs::metadata(p).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH)}),
    }
    files
}

/// 初期インデックス取得
fn find_initial_index(list: &[PathBuf], target: &Path) -> usize {
    list.iter().position(|p| p == target).unwrap_or(0)
}

/// 画像表示用サイズを計算
fn compute_display_size(img: &DynamicImage, cfg: &Config) -> (u32, u32) {
    let (iw, ih) = img.dimensions();
    let (sw, sh) = get_screen_size();
    let max_scale = (sw as f64 / iw as f64).min(sh as f64 / ih as f64).min(1.0);
    let min_scale = (cfg.min_window_width as f64 / iw as f64)
        .max(cfg.min_window_height as f64 / ih as f64)
        .max(1.0);
    let scale = if iw < cfg.min_window_width && ih < cfg.min_window_height {
        min_scale
    } else {
        max_scale
    };
    ((iw as f64 * scale).round() as u32, (ih as f64 * scale).round() as u32)
}

/// 画面解像度取得 (Windows)
#[cfg(feature = "native-win")]
fn get_screen_size() -> (u32, u32) {
    unsafe { (GetSystemMetrics(SM_CXSCREEN) as u32, GetSystemMetrics(SM_CYSCREEN) as u32) }
}

/// 画面解像度取得 (その他)
#[cfg(not(feature = "native-win"))]
fn get_screen_size() -> (u32, u32) {
    (800, 600)
}
