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

**Frame** è un'utility di conversione multimediale ad alte prestazioni costruita sul framework Tauri v2. Fornisce un'interfaccia nativa per le operazioni FFmpeg, consentendo un controllo granulare sui parametri di conversione di video, audio e immagini. L'applicazione sfrutta un backend basato su Rust per la gestione concorrente dei task e l'esecuzione dei processi, abbinato a un frontend Svelte 5 per la configurazione e il monitoraggio dello stato.

<br />
<div align="center">
  <img src="../preview.png" alt="Frame Application Preview" width="800" />
</div>
<br />

> [!ATTENZIONE]
> **Avviso di candidatura non firmato**
> Poiché l'applicazione è attualmente non firmata, il sistema operativo la segnalerà:
>
> - **Il sistema contrassegnerà l'applicazione e i suoi binari secondari con un attributo di quarantena. Per eseguire l'applicazione, rimuovere manualmente l'attributo:
>   ``bash
>   xattr -dr com.apple.quarantine /Applicazioni/Frame.app
>   ```
> - **Windows:** Windows SmartScreen potrebbe impedire l'avvio dell'applicazione. Fare clic su **"Ulteriori informazioni "** e quindi su **"Esegui comunque "** per procedere.

## Sponsor GitHub

Se Frame vi aiuta, considerate il supporto del progetto su GitHub Sponsors:

[**Telaio sponsor**](https://github.com/sponsors/66HEX)

Obiettivi di finanziamento attuali:

- **Apple Developer Program:** `$99/anno` per firmare e autenticare le build di macOS.
- **Certificato di firma del codice di Microsoft: ** stimato a 300-700 dollari l'anno per firmare le build di Windows e ridurre l'attrito di SmartScreen.

I contributi degli sponsor vengono utilizzati in primo luogo per questi costi di firma della liberatoria.

Vedere [GitHub Sponsors](https://github.com/sponsors/66HEX) per tutti i dettagli sulla sponsorizzazione, i suggerimenti sui livelli e una lista di controllo per il lancio.

## Caratteristiche

### Nucleo di conversione dei media

- **Tipi di media:** Video, audio, immagini.
- **Formati di output supportati:**
  - **Video:** `mp4`, `mkv`, `webm`, `mov`, `gif`
  - **Audio:** `mp3`, `m4a`, `wav`, `flac`
  - **Immagine:** `png`, `jpg`, `webp`, `bmp`, `tiff`
- **Codificatori video:**
  - `libx264` (H.264 / AVC)
  - `libx265` (H.265 / HEVC)
  - `vp9` (Google VP9)
  - `prores` (Apple ProRes)
  - `libsvtav1` (Tecnologia video scalabile AV1)
  - **Accelerazione hardware:** `h264_videotoolbox` (Apple Silicon), `hevc_videotoolbox` (Apple Silicon), `h264_nvenc` (NVIDIA), `hevc_nvenc` (NVIDIA), `av1_nvenc` (NVIDIA).
- **Codificatori di immagini:** `png`, `mjpeg` (JPEG), `libwebp` (WebP), `bmp`, `tiff`.
- **Codificatori audio:** `aac`, `ac3` (Dolby Digital), `libopus`, `mp3`, `alac` (Apple Lossless), `flac` (Free Lossless Audio Codec), `pcm_s16le` (WAV).
- **Controllo bitrate: ** Fattore di velocità costante (CRF) o bitrate target (kbps).
- **Bicubica, Lanczos, Bilineare, Vicino più vicino.
- **Sondaggio dei metadati:** Estrazione automatica dei dettagli del flusso (codec, durata, bitrate, disposizione dei canali) tramite `ffprobe`.
- **AI Upscaling:** `Real-ESRGAN` integrato per l'upscaling di alta qualità di video e immagini (x2, x4).

### Architettura e flusso di lavoro

- **Elaborazione concorrente:** Il gestore asincrono delle code di attività implementato in Rust (`tokio::mpsc`) limita i processi FFmpeg concorrenti (default: 2).
- **Telemetria in tempo reale: ** Analizza il flusso di `stderr` di FFmpeg per un monitoraggio accurato dei progressi e dei log.
- **Gestione dei preset:** persistenza della configurazione per i profili di conversione riutilizzabili.

## Stack tecnico

### Backend (Rust / Tauri)

- **Core:** Tauri v2 (Rust Edition 2024).
- **Runtime:** `tokio` (Async I/O).
- **Serializzazione:** `serde`, `serde_json`.
- **Gestione dei processi:** `tauri-plugin-shell` per l'esecuzione di sidecar (FFmpeg/FFprobe).
- **Integrazione del sistema:** `tauri-plugin-dialog`, `tauri-plugin-fs`.

### Frontend (SvelteKit)

- **Framework:** Svelte 5 (Runes API).
- **Sistema di costruzione:** Vite.
- **Styling:** Tailwind CSS v4, `clsx`, `tailwind-merge`.
- **Gestione dello stato:** Svelte 5 `$state` / `$props`.
- **Internazionalizzazione:** Interfaccia multilingue con rilevamento automatico della lingua del sistema.
- **Tipografia: ** Loskeley Mono (incorporato).

## Installazione

### Scarica i binari precostruiti

Il modo più semplice per iniziare è scaricare l'ultima versione per la propria piattaforma (macOS, Windows o Linux) direttamente da GitHub.

[**Scaricare l'ultima versione**](https://github.com/66HEX/frame/releases)

> **Nota: ** Poiché l'applicazione non è ancora firmata da un codice, potrebbe essere necessario approvarla manualmente nelle impostazioni di sistema (vedere l'avviso all'inizio di questo file).

### WinGet (Windows)

Frame è disponibile nel repository ufficiale di WinGet con l'identificatore `66HEX.Frame`.

```powershell
winget install --id 66HEX.Frame -e
```

Per aggiornare:

```powershell
winget upgrade --id 66HEX.Frame -e
```

### Homebrew (macOS)

Per gli utenti di macOS, è possibile installare e aggiornare Frame facilmente utilizzando il nostro Homebrew Tap personalizzato:

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### Requisiti di sistema di Linux

Anche quando si usa l'**AppImage**, Frame si affida alle librerie **WebKitGTK** e **GStreamer** del sistema per il rendering dell'interfaccia utente e la gestione della riproduzione dei file multimediali. Le finestre di dialogo native su Linux richiedono anche l'integrazione con **XDG Desktop Portal** (oltre a un backend specifico per il desktop) e `zenity` come ripiego. Se l'applicazione si blocca dopo l'aggiunta di una fonte, l'anteprima del video rimane vuota o le finestre di dialogo dei file non si aprono/tematizzano correttamente, installare i pacchetti seguenti.

- **Ubuntu / Debian:**

  ```bash
  sudo apt update
  sudo apt install libwebkit2gtk-4.1-0 gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

- **Arch Linux:**

  ```bash
  sudo pacman -S --needed webkit2gtk-4.1 gst-plugins-base gst-plugins-good gst-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

- **Fedora:**
  ```bash
  sudo dnf install webkit2gtk4.1 gstreamer1-plugins-base gstreamer1-plugins-good gstreamer1-libav xdg-desktop-portal xdg-desktop-portal-gtk zenity
  ```

> **Utenti di KDE:** installate `xdg-desktop-portal-kde` (invece di `xdg-desktop-portal-gtk`) per ottenere finestre di dialogo a tema Plasma-nativo.

### Costruire dalla sorgente

Se preferite costruire l'applicazione da soli o volete contribuire, seguite questi passaggi.

**1. Prerequisiti**

- **Rust:** [Installare Rust](https://www.rust-lang.org/tools/install)
- **Bun (o Node.js):** [Installare Bun](https://bun.sh/)
- **Dipendenze del sistema operativo:** Seguire i [prerequisiti di Tauri](https://v2.tauri.app/start/prerequisites/) per il proprio sistema operativo.

**2. Impostare il progetto**

Clonare il repository e installare le dipendenze:

```bash
git clone https://github.com/66HEX/frame.git
cd frame
bun install
```

**3. Impostare i binari**

Frame richiede i binari sidecar FFmpeg/FFprobe e gli asset sidecar Real-ESRGAN per l'upscaling dell'AI. Forniamo script per recuperare automaticamente le versioni corrette per la vostra piattaforma:

```bash
bun run setup:ffmpeg
bun run setup:upscaler
```

**4. Costruire o eseguire**

- **Sviluppo:**

  ```bash
  bun tauri dev
  ```

- **Costruzione di produzione:**
  ```bash
  bun tauri build
  ```

## Utilizzo

1.  **Input:** Usare la finestra di dialogo del sistema per selezionare i file.
2.  **Configurazione:**
    - **Fonte:** Visualizza i metadati dei file rilevati.
    - **Selezionare il formato del contenitore e il nome del file di output.
    - **Video:** Configurare codec, bitrate/CRF, risoluzione e framerate.
    - **Immagini:** Configurare la risoluzione/scalatura delle immagini, il formato dei pixel e l'upscaling AI opzionale.
    - **Audio:** Selezionare codec, bitrate, canali e tracce specifiche.
    - **Salva e carica profili di conversione riutilizzabili.
3.  **Esecuzione:** Avvia il processo di conversione tramite il backend Rust.
4.  **Monitoraggio:** Visualizzazione dei registri in tempo reale e dei contatori percentuali nell'interfaccia utente.

## Storia delle stelle

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline&theme=dark" />
  <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
  <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
</picture>

## Riconoscimenti e codice di terze parti

- **Real-ESRGAN**: Copyright (c) 2021, Xintao Wang. Licenza [BSD 3-Clause](https://github.com/xinntao/Real-ESRGAN/blob/master/LICENSE).
- **FFmpeg**: licenza [GPLv3](https://www.ffmpeg.org/legal.html).

## Licenza

Licenza GPLv3. Vedere [LICENSE](../LICENSE) per i dettagli.
