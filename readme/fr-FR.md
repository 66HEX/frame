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

**Frame** est un utilitaire de conversion média haute performance construit sur le framework Tauri v2. Il fournit une interface native pour les opérations FFmpeg, permettant un contrôle granulaire des paramètres de conversion vidéo, audio et image. L'application s'appuie sur un backend basé sur Rust pour la gestion des tâches concurrentes et l'exécution des processus, couplé à un frontend Svelte 5 pour la configuration et la surveillance de l'état.

<br />
<div align="center">
  <img src="../preview.png" alt="Frame Application Preview" width="800" />
</div>
<br />

> [!WARNING]
> **Avis de candidature non signé
> Comme l'application n'est pas signée, votre système d'exploitation la signalera :
>
> - **macOS:** Le système marquera l'application et ses binaires sidecar avec un attribut de quarantaine. Pour exécuter l'application, supprimez l'attribut manuellement :
>   ``bash
>   xattr -dr com.apple.quarantine /Applications/Frame.app
>   ```
> - **Cliquez sur **"Plus d'infos "** puis **"Exécuter quand même "** pour continuer.

## Sponsors de GitHub

Si Frame vous aide, pensez à soutenir le projet sur GitHub Sponsors :

[**Cadre de parrainage**] (https://github.com/sponsors/66HEX)

Objectifs de financement actuels :

- **Apple Developer Program:** `99$/an` pour signer et notarier les builds de macOS.
- **Certificat de signature de code Microsoft:** estimé à 300-700 $/an pour signer les versions de Windows et réduire les frictions avec SmartScreen.

Les contributions des sponsors sont utilisées en premier lieu pour ces coûts de signature de la licence.

Voir [GitHub Sponsors] (https://github.com/sponsors/66HEX) pour plus de détails sur le parrainage, des suggestions de paliers et une liste de contrôle pour le lancement.

## Caractéristiques

### Conversion des médias (Core)

- **Types de médias:** Vidéo, Audio, Image.
- **Formats de sortie pris en charge:**
  - **Vidéo:** `mp4`, `mkv`, `webm`, `mov`, `gif`
  - **Audio:** `mp3`, `m4a`, `wav`, `flac`
  - **Image:** `png`, `jpg`, `webp`, `bmp`, `tiff`
- **Encodeurs vidéo:**
  - `libx264` (H.264 / AVC)
  - `libx265` (H.265 / HEVC)
  - `vp9` (Google VP9)
  - `prores` (Apple ProRes)
  - `libsvtav1` (Scalable Video Technology AV1)
  - **Accélération matérielle:** `h264_videotoolbox` (Apple Silicon), `hevc_videotoolbox` (Apple Silicon), `h264_nvenc` (NVIDIA), `hevc_nvenc` (NVIDIA), `av1_nvenc` (NVIDIA).
- **Encodeurs d'images:** `png`, `mjpeg` (JPEG), `libwebp` (WebP), `bmp`, `tiff`.
- **Encodeurs audio:** `aac`, `ac3` (Dolby Digital), `libopus`, `mp3`, `alac` (Apple Lossless), `flac` (Free Lossless Audio Codec), `pcm_s16le` (WAV).
- **Contrôle du débit : *Facteur de débit constant (CRF) ou débit cible (kbps).
- **Mise à l'échelle:** Bicubique, Lanczos, Bilinéaire, Voisin le plus proche.
- **Metadata Probing:** Extraction automatisée des détails du flux (codec, durée, débit, disposition des canaux) via `ffprobe`.
- **AI Upscaling:** Integrated `Real-ESRGAN` for high-quality video and image upscaling (x2, x4).

### Architecture et flux de travail

- **Traitement simultané:** Gestionnaire de file d'attente de tâches asynchrones implémenté en Rust (`tokio::mpsc`) limitant les processus FFmpeg simultanés (par défaut : 2).
- **Télémétrie en temps réel:** Analyse du flux de FFmpeg `stderr` pour un suivi précis de la progression et de la sortie du journal.
- **Preset Management:** Persistance de la configuration pour les profils de conversion réutilisables.

## Pile technique

### Backend (Rust / Tauri)

- **Core:** Tauri v2 (Rust Edition 2024).
- **Runtime:** `tokio` (Async I/O).
- **Serialisation:** `serde`, `serde_json`.
- **Gestion des processus:** `tauri-plugin-shell` pour l'exécution de sidecar (FFmpeg/FFprobe).
- **Intégration système:** `tauri-plugin-dialog`, `tauri-plugin-fs`.

### Frontend (SvelteKit)

- **Framework:** Svelte 5 (Runes API).
- **Système de construction:** Vite.
- **Styling:** Tailwind CSS v4, `clsx`, `tailwind-merge`.
- **Gestion de l'état:** Svelte 5 `$state` / `$props`.
- **Internationalisation:** Interface multilingue avec détection automatique de la langue du système.
- **Typographie:** Loskeley Mono (intégré).

## Installation

### Télécharger les binaires préconstruits

La façon la plus simple de commencer est de télécharger la dernière version pour votre plateforme (macOS, Windows ou Linux) directement depuis GitHub.

[**Télécharger la dernière version**] (https://github.com/66HEX/frame/releases)

> **Note:** Comme l'application n'est pas encore signée par le code, il se peut que vous deviez l'approuver manuellement dans les paramètres de votre système (voir l'avertissement au début de ce fichier).

### WinGet (Windows)

Frame est disponible dans le dépôt officiel de WinGet sous l'identifiant `66HEX.Frame`.

```powershell
winget install --id 66HEX.Frame -e
```

A mettre à jour :

```powershell
winget upgrade --id 66HEX.Frame -e
```

### Homebrew (macOS)

Pour les utilisateurs de macOS, vous pouvez installer et mettre à jour Frame facilement en utilisant notre Homebrew Tap personnalisé :

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### Configuration requise pour Linux

Même en utilisant **AppImage**, Frame s'appuie sur les bibliothèques **WebKitGTK** et **GStreamer** du système pour le rendu de l'interface utilisateur et la gestion de la lecture des médias. Les dialogues natifs sur Linux nécessitent également l'intégration de **XDG Desktop Portal** (plus un backend spécifique au bureau) et `zenity` comme solution de repli. Si l'application plante lors de l'ajout d'une source, si l'aperçu vidéo reste vide ou si les dialogues de fichiers ne parviennent pas à s'ouvrir ou à s'agencer correctement, installez les paquets ci-dessous.

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

> **Utilisateurs de KDE : ** installez `xdg-desktop-portal-kde` (au lieu de `xdg-desktop-portal-gtk`) pour obtenir des dialogues à thème natifs de Plasma.

### Construire à partir de la source

Si vous préférez créer l'application vous-même ou si vous souhaitez y contribuer, suivez les étapes suivantes.

**1. conditions préalables**

- **Rust:** [Install Rust](https://www.rust-lang.org/tools/install)
- **Bun (ou Node.js):** [Installer Bun](https://bun.sh/)
- **Dépendances du système d'exploitation:** Suivez les [prérequis Tauri] (https://v2.tauri.app/start/prerequisites/) pour votre système d'exploitation.

**2. mettre en place le projet**

Cloner le dépôt et installer les dépendances :

```bash
git clone https://github.com/66HEX/frame.git
cd frame
bun install
```

**3. installer les binaires**

Frame nécessite des binaires FFmpeg/FFprobe sidecar et des actifs Real-ESRGAN sidecar pour l'upscaling AI. Nous fournissons des scripts pour récupérer automatiquement les versions correctes pour votre plateforme :

```bash
bun run setup:ffmpeg
bun run setup:upscaler
```

**4. construire ou courir**

- **Développement:**

  ```bash
  bun tauri dev
  ```

- **Production Build:**
  ```bash
  bun tauri build
  ```

## Utilisation

1.  **Entrée:** Utiliser la boîte de dialogue du système pour sélectionner les fichiers.
2.  **Configuration:**
    - **Source:** Afficher les métadonnées des fichiers détectés.
    - **Sortie:** Sélectionner le format du conteneur et le nom du fichier de sortie.
    - **Vidéo:** Configurer le codec, le débit binaire/CRF, la résolution et le taux de rafraîchissement.
    - **Images:** Configurer la résolution et la mise à l'échelle de l'image, le format des pixels et la mise à l'échelle de l'IA en option.
    - **Audio:** Sélectionnez le codec, le débit, les canaux et les pistes spécifiques.
    - **Presets:** Enregistrez et chargez des profils de conversion réutilisables.
3.  **Exécution:** Initie le processus de conversion via le backend Rust.
4.  **Surveillance:** Affichez les journaux en temps réel et les compteurs de pourcentage dans l'interface utilisateur.

## Histoire des étoiles

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline&theme=dark" />
  <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
  <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
</picture>

## Remerciements et code des tiers

- **Real-ESRGAN** : Copyright (c) 2021, Xintao Wang, sous licence [BSD 3-Clause] (https://github.com/xinntao/Real-ESRGAN/blob/master/LICENSE).
- **FFmpeg** : sous licence [GPLv3] (https://www.ffmpeg.org/legal.html).

## Licence

Licence GPLv3, voir [LICENSE](../LICENSE) pour plus de détails.
