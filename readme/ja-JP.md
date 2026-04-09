<div align="center">
  <img src="../icon.png" width="256" height="256" alt="Frame Icon" />
  <h1>Frame</h1>
</div>

<div align="center">

[English](../README.md) | [简体中文](./zh-CN.md) | [日本語](./ja-JP.md) | [한국어](./ko-KR.md) | [Español](./es-ES.md) | [Русский](./ru-RU.md) | [Français](./fr-FR.md) | [Deutsch](./de-DE.md) | [Italiano](./it-IT.md)

</div>

<div align="center">
	<img src="https://img.shields.io/badge/Tauri-v2-orange?style=flat-square&logo=tauri" alt="Tauri" />
	<img src="https://img.shields.io/badge/Svelte-v5-red?style=flat-square&logo=svelte" alt="Svelte" />
	<img src="https://img.shields.io/badge/Rust-Edition_2024-black?style=flat-square&logo=rust" alt="Rust" />
	<img src="https://img.shields.io/badge/TypeScript-5.9.3-blue?style=flat-square&logo=typescript" alt="TypeScript" />
	<img src="https://img.shields.io/badge/Tailwind_CSS-v4-38bdf8?style=flat-square&logo=tailwindcss" alt="Tailwind" />
	<img src="https://img.shields.io/badge/license-GPL--3.0-green?style=flat-square" alt="License" />
	<a href="https://github.com/sponsors/66HEX">
		<img src="https://img.shields.io/badge/Sponsor-GitHub-pink?style=flat-square&logo=githubsponsors" alt="GitHub Sponsors" />
	</a>
</div>

**Frame**は、Tauri v2フレームワーク上に構築された高性能なメディア変換ユーティリティです。 FFmpeg操作のためのネイティブ・インターフェースを提供し、ビデオ、オーディオ、画像変換パラメーターのきめ細かな制御を可能にします。 このアプリケーションは、同時タスク管理とプロセス実行のためのRustベースのバックエンドと、設定と状態監視のためのSvelte 5フロントエンドを活用しています。

<br />
<div align="center">
  <img src="../preview.png" alt="Frame Application Preview" width="800" />
</div>
<br />

> 警告
> **無記名出願のお知らせ
> 現在、アプリケーションは署名されていないため、オペレーティング・システムはそのアプリケーションにフラグを立てます：
>
> - **アプリを実行するには、この属性を手動で削除してください：
>   バッシュ
>   xattr -dr com.apple.quarantine /Applications/Frame.app
>   ```
> - **Windows:**Windowsスマートスクリーンがアプリケーションの起動を妨げることがあります。 詳細情報 "**"をクリックし、**"とにかく実行 "**してください。

## GitHub スポンサー

もしFrameがあなたのお役に立てるなら、GitHub Sponsorsでプロジェクトをサポートすることをご検討ください：

[スポンサー枠**](https://github.com/sponsors/66HEX)

現在の資金調達目標

- **Apple Developer Program:** `$99/年`でmacOSビルドに署名し、公証します。
- **マイクロソフトのコード署名証明書：** Windowsのビルドに署名し、SmartScreenの摩擦を減らすために、年間300ドルから700ドルと推定される。

スポンサーからの寄付金は、まずこれらのリリース契約費用に充てられる。

スポンサーシップの詳細、ティアの提案、立ち上げのチェックリストは[GitHub Sponsors](https://github.com/sponsors/66HEX)をご覧ください。

## 特徴

### メディア変換コア

- **メディアの種類：**ビデオ、オーディオ、イメージ。
- *サポートされる出力フォーマット：***。
  - **ビデオ：** `mp4`, `mkv`, `webm`, `mov`, `gif`
  - **オーディオ:** `mp3`, `m4a`, `wav`, `flac`
  - **画像:** `png`、`jpg`、`webp`、`bmp`、`tiff`。
- **ビデオ・エンコーダー
  - libx264` (H.264 / AVC)
  - libx265` (H.265 / HEVC)
  - vp9` (Google VP9)
  - Prores` (Apple ProRes)
  - libsvtav1` (スケーラブル・ビデオ・テクノロジー AV1)
  - **ハードウェアアクセラレーション:** `h264_videotoolbox` (Apple Silicon), `hevc_videotoolbox` (Apple Silicon), `h264_nvenc` (NVIDIA), `hevc_nvenc` (NVIDIA), `av1_nvenc` (NVIDIA).
- **画像エンコーダー: ** `png`、`mjpeg` (JPEG)、`libwebp` (WebP)、`bmp`、`tiff`。
- **オーディオエンコーダー: ** `aac`、`ac3` (Dolby Digital)、`libopus`、`mp3`、`alac` (Apple Lossless)、`flac` (Free Lossless Audio Codec)、`pcm_s16le` (WAV).
- **ビットレート・コントロール： **コンスタント・レート・ファクター（CRF）またはターゲット・ビットレート（kbps）。
- **スケーリング：**バイキュービック、ランチョス、バイリニア、最近傍。
- **メタデータのプロービング:** `ffprobe` によるストリームの詳細（コーデック、時間、ビットレート、チャンネルレイアウト）の自動抽出。
- **AIアップスケーリング:** 高品質のビデオと画像のアップスケーリング（x2、x4）のための統合された`Real-ESRGAN`。

### アーキテクチャとワークフロー

- **Concurrent Processing:** Rust で実装された非同期タスクキューマネージャ (`tokio::mpsc`) は、FFmpeg の同時処理を制限しています (デフォルト: 2)。
- **リアルタイムテレメトリー：** FFmpeg `stderr` のストリーム解析による正確な進捗追跡とログ出力。
- **プリセット管理:** 再利用可能な変換プロファイルのための構成永続化。

## テクニカル・スタック

### バックエンド（Rust / Tauri）

- **コア:** Tauri v2 (Rust Edition 2024)。
- **Runtime:** `tokio` (非同期 I/O).
- **シリアライズ:** `serde`, `serde_json`.
- **プロセス管理:** サイドカー実行のための `tauri-plugin-shell` (FFmpeg/FFprobe).
- **システム統合:** `tauri-plugin-dialog`, `tauri-plugin-fs`.

### フロントエンド（SvelteKit）

- **フレームワーク:** Svelte 5 (Runes API)。
- **ビルドシステム:** Vite.
- **Styling:** Tailwind CSS v4、`clsx`、`tailwind-merge`。
- **状態管理:** Svelte 5 `$state` / `$props`.
- **国際化:** 自動システム言語検出による多言語インターフェイス。
- **タイポグラフィ：** Loskeley Mono (embedded).

## インストール

### ビルド済みバイナリのダウンロード

GitHubからあなたのプラットフォーム（macOS、Windows、Linux）の最新リリースを直接ダウンロードするのが一番簡単な方法だ。

[最新リリース**のダウンロード](https://github.com/66HEX/frame/releases)

> **注意:** アプリケーションはまだコード署名されていないため、システム設定で手動で承認する必要があるかもしれません（このファイルの一番上の警告を参照してください）。

### ウィンゲット（Windows）

Frameは公式のWinGetリポジトリで`66HEX.Frame`という識別子で入手できます。

```powershell
winget install --id 66HEX.Frame -e
```

更新する：

```powershell
winget upgrade --id 66HEX.Frame -e
```

### ホームブリュー（macOS）

macOSユーザーの方は、カスタムHomebrew Tapを使って簡単にFrameをインストール、アップデートすることができます：

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### Linuxシステム要件

AppImage** を使用している場合でも、Frame は UI のレンダリングとメディア再生の処理にシステムの **WebKitGTK** および **GStreamer** ライブラリを使用しています。 Linux でのネイティブ ダイアログには、**XDG Desktop Portal** 統合 (デスクトップ固有のバックエンドを含む) とフォールバックとして `zenity` も必要です。 ソースを追加するとアプリケーションがクラッシュする、ビデオ プレビューが空白のまま、ファイル ダイアログが正しく開かない/テーマが設定されない場合は、以下のパッケージをインストールしてください。

- **Ubuntu/Debian:**。

  ```bash
  sudo apt update
  sudo apt install libwebkit2gtk-4.1-0 gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

- **アーチ・リナックス

  ```bash
  sudo pacman -S --needed webkit2gtk-4.1 gst-plugins-base gst-plugins-good gst-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

- **フェドラ:**
  ```bash
  sudo dnf install webkit2gtk4.1 gstreamer1-plugins-base gstreamer1-plugins-good gstreamer1-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

> **KDE ユーザー:** Plasma ネイティブテーマのダイアログを表示するには、(`xdg-desktop-portal-gtk` の代わりに) `xdg-desktop-portal-kde` をインストールしてください。

### ソースからのビルド

アプリケーションを自分で作りたい場合、あるいは貢献したい場合は、以下の手順に従ってください。

**1.前提条件

- **錆:** [錆のインストール](https://www.rust-lang.org/tools/install)
- **Bun (または Node.js):** [Bun のインストール](https://bun.sh/)
- **OSの依存関係：**お使いのOSの[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)に従ってください。

**プロジェクト**の設定

リポジトリをクローンし、依存関係をインストールする：

```bash
git clone https://github.com/66HEX/frame.git
cd frame
bun install
```

**3.バイナリのセットアップ

Frameは、AIのアップスケーリングにFFmpeg/FFprobeのサイドカーバイナリとReal-ESRGANのサイドカーアセットを必要とします。 私たちは、あなたのプラットフォームに適したバージョンを自動的に取得するスクリプトを提供します：

```bash
bun run setup:ffmpeg
bun run setup:upscaler
```

**ビルド・オア・ラン

- **開発:**

  ```bash
  bun tauri dev
  ```

- ***Production Build:**
  ```bash
  bun tauri build
  ```

## 使用方法

1.  **システムダイアログを使ってファイルを選択する。
2.  **コンフィギュレーション
    - **検出されたファイルのメタデータを表示します。
    - **出力：** コンテナ形式と出力ファイル名を選択。
    - **ビデオ:** コーデック、ビットレート/CRF、解像度、フレームレートを設定する。
    - **画像：** 画像の解像度/スケーリング、ピクセルフォーマット、およびオプションのAIアップスケーリングを設定します。
    - **オーディオ：**コーデック、ビットレート、チャンネル、特定のトラックを選択します。
    - **プリセット：**再利用可能な変換プロファイルの保存と読み込み。
3.  **実行:** Rustバックエンド経由で変換プロセスを開始する。
4.  **モニタリング：** リアルタイムのログとパーセンテージ・カウンターをUIで表示します。

## スターヒストリー

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline&theme=dark" />
  <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
  <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
</picture>

## 謝辞とサードパーティ・コード

- **Real-ESRGAN**: Copyright (c) 2021, Xintao Wang. ライセンスは[BSD 3-Clause](https://github.com/xinntao/Real-ESRGAN/blob/master/LICENSE)です。
- **FFmpeg**：[GPLv3](https://www.ffmpeg.org/legal.html)の下でライセンスされています。

## ライセンス

GPLv3ライセンス。 詳細は[LICENSE](../LICENSE)を参照。
