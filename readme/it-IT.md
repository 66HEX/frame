<div align="center">
  <img src="../icon.png" width="128" height="128" alt="Frame Icon" />
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
</div>

**Frame** è un'utility di conversione multimediale ad alte prestazioni costruita sul framework Tauri v2. Fornisce un'interfaccia nativa per le operazioni FFmpeg, consentendo un controllo granulare sui parametri di transcodifica video e audio. L'applicazione utilizza un backend Rust per la gestione delle attività concorrenti e l'esecuzione dei processi, abbinato a un frontend Svelte 5 per la configurazione e il monitoraggio dello stato.

<br />
<div align="center">
  <img src="../preview.png" alt="Anteprima di Frame" width="800" />
</div>
<br />

> [!WARNING]
> **Avviso applicazione non firmata**
> Poiché l'applicazione non è attualmente firmata, il sistema operativo mostrerà avvisi:
>
> - **macOS:** Il sistema contrassegnerà l'app con un attributo di quarantena. Per eseguirla, rimuovi l'attributo manualmente:
>   ```bash
>   xattr -dr com.apple.quarantine /Applications/Frame.app
>   ```
> - **Windows:** SmartScreen potrebbe bloccare l'avvio. Clicca su **"Ulteriori informazioni"** e poi **"Esegui comunque"**.

## Funzionalità

### Core di conversione

- **Container supportati:** `mp4`, `mkv`, `webm`, `mov`, `mp3`, `m4a`, `wav`, `flac`.
- **Encoder video:**
  - `libx264` (H.264 / AVC)
  - `libx265` (H.265 / HEVC)
  - `vp9` (Google VP9)
  - `prores` (Apple ProRes)
  - `libsvtav1` (SVT-AV1)
  - **Accelerazione hardware:** `h264_videotoolbox` (Apple Silicon), `h264_nvenc` (NVIDIA)
- **Encoder audio:** `aac`, `ac3` (Dolby Digital), `libopus`, `mp3`
- **Controllo bitrate:** Qualità costante (CRF) o bitrate target (kbps)
- **Ridimensionamento:** Bicubico, Lanczos, Bilineare, Vicino più prossimo
- **Analisi metadati:** Estrazione automatica delle informazioni sui flussi tramite `ffprobe`

### Architettura e workflow

- **Elaborazione concorrente:** Gestore di coda asincrono in Rust (`tokio::mpsc`), limitazione dei processi FFmpeg simultanei (default: 2)
- **Telemetria in tempo reale:** Parsing dello stream `stderr` di FFmpeg per un tracciamento preciso del progresso
- **Gestione preset:** Persistenza delle configurazioni per profili di conversione riutilizzabili

## Stack tecnologico

### Backend (Rust / Tauri)

- **Core:** Tauri v2 (Rust Edition 2024)
- **Runtime:** `tokio` (I/O asincrono)
- **Serializzazione:** `serde`, `serde_json`
- **Gestione processi:** `tauri-plugin-shell` per esecuzione sidecar (FFmpeg/FFprobe)
- **Integrazione sistema:** `tauri-plugin-dialog`, `tauri-plugin-fs`, `window-vibrancy`

### Frontend (SvelteKit)

- **Framework:** Svelte 5 (Runes API)
- **Sistema di build:** Vite
- **Stili:** Tailwind CSS v4, `clsx`, `tailwind-merge`
- **Gestione stato:** Svelte 5 `$state` / `$props`
- **Internazionalizzazione:** Interfaccia multilingue con rilevamento automatico della lingua di sistema
- **Tipografia:** Geist Mono (incorporato)

### Installazione

#### Via Homebrew (macOS)

Il modo più semplice per installare e mantenere aggiornato Frame su macOS è tramite il nostro Homebrew Tap:

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### Release precompilate

Il modo più semplice per eseguire Frame è scaricare un pacchetto precompilato da [GitHub Releases](https://github.com/66HEX/frame/releases). Ogni release include build per macOS (Intel/Apple Silicon), Windows e Linux (AppImage/Deb). I binari non sono firmati, quindi il sistema operativo potrebbe mostrare avvisi.

### Prerequisiti

- Runtime Node.js (o Bun)
- Toolchain Rust (`cargo`)
- Binari **FFmpeg** e **FFprobe** nella directory `src-tauri/binaries/`
  - Convenzione di denominazione: `ffmpeg-<target-triple>` (es: `ffmpeg-aarch64-apple-darwin`)

> Suggerimento: Esegui `bun run setup:binaries` per scaricare automaticamente i binari corretti. Usa `--force` per aggiornare.

### Istruzioni di build

1.  **Installare le dipendenze:**

    ```bash
    bun install
    ```

2.  **Avviare il server di sviluppo:**

    ```bash
    bun run tauri dev
    ```

3.  **Compilare per la produzione:**
    ```bash
    bun run tauri build
    ```

## Utilizzo

1.  **Input:** Usa la finestra di dialogo di sistema per selezionare i file
2.  **Configurazione:**
    - **Sorgente:** Visualizza i metadati del file rilevato
    - **Output:** Seleziona formato container e nome file di output
    - **Video:** Configura codec, bitrate/CRF, risoluzione e framerate
    - **Audio:** Seleziona codec, bitrate, canali e tracce specifiche
    - **Preset:** Salva e carica profili di conversione riutilizzabili
3.  **Esecuzione:** Avvia il processo di conversione tramite backend Rust
4.  **Monitoraggio:** Visualizza log in tempo reale e percentuale di avanzamento

## Licenza

Licenza GPLv3. Vedi [LICENSE](../LICENSE) per i dettagli.
