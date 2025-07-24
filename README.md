# rs_imgviewer

Rust 製の高速画像ビューアーです。JPEG、PNG、WEBP に対応し、ディレクトリ指定時にはソートされた画像リストを順次閲覧できます。

## 特徴

* **対応フォーマット**: JPEG / PNG / WEBP
* **高速起動**: ネイティブ Rust 実装で高速に画像を表示
* **ソート機能**: ファイル名、作成日時、更新日時でソート可能
* **キー操作**: 次へ・前へ・終了のキー操作をサポート
* **自動生成設定**: `config.toml` がない場合、デフォルト設定ファイルを自動生成

## インストール

```bash
# リポジトリをクローン
git clone https://github.com/kznagamori/rs_imgviewer
cd rs_imgviewer

# ビルド（デバッグ版）
cargo build

# または、Cargo Install でインストール
cargo install --git https://github.com/kznagamori/rs_imgviewer --branch main
```

## 使い方

```bash
# ディレクトリ指定
rs_imgviewer C:\Users\<ユーザ>\Pictures

# 単一ファイル指定
rs_imgviewer C:\Users\<ユーザ>\Pictures\sample.webp
```

* **次へ**: 右矢印キー / `X`
* **前へ**: 左矢印キー / `Z`
* **終了**: Enter / Esc / Alt+F4

## 設定ファイル (`config.toml`)

実行ファイルと同じディレクトリに `config.toml` を配置します。存在しない場合は自動生成されます。

```toml
# config.toml
# 最小ウィンドウサイズ（ピクセル）
min_window_width  = 800
min_window_height = 600

# ソートアルゴリズム: "FileName" | "CreatedTime" | "ModifiedTime"
sort_algorithm = "FileName"
```

### パラメータ

* `min_window_width`, `min_window_height`: 画像がそれ以下の場合に拡大表示する最小サイズ
* `sort_algorithm`: 画像ファイル一覧のソート方法

## 開発・ビルド要件

* Rust 1.60 以上
* Windows 11（x86\_64-pc-windows-gnu toolchain）

## ライセンス

MIT License

---

© 2025 kznagamori


