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

**Frame** est un utilitaire de conversion multimédia haute performance construit sur le framework Tauri v2. Il fournit une interface native pour les opérations FFmpeg, permettant un contrôle précis des paramètres de transcodage vidéo et audio. L'application utilise un backend Rust pour la gestion des tâches concurrentes et l'exécution des processus, couplé à un frontend Svelte 5 pour la configuration et le monitoring.

<br />
<div align="center">
  <img src="../preview.png" alt="Aperçu de Frame" width="800" />
</div>
<br />

> [!WARNING]
> **Application non signée**
> L'application n'étant pas signée, votre système d'exploitation affichera des avertissements :
>
> - **macOS :** Le système marquera l'app avec un attribut de quarantaine. Pour l'exécuter, supprimez l'attribut manuellement :
>   ```bash
>   xattr -dr com.apple.quarantine /Applications/Frame.app
>   ```
> - **Windows :** SmartScreen peut bloquer le lancement. Cliquez sur **« Plus d'infos »** puis **« Exécuter quand même »**.

## Fonctionnalités

### Cœur de conversion

- **Conteneurs supportés :** `mp4`, `mkv`, `webm`, `mov`, `mp3`, `m4a`, `wav`, `flac`.
- **Encodeurs vidéo :**
  - `libx264` (H.264 / AVC)
  - `libx265` (H.265 / HEVC)
  - `vp9` (Google VP9)
  - `prores` (Apple ProRes)
  - `libsvtav1` (SVT-AV1)
  - **Accélération matérielle :** `h264_videotoolbox` (Apple Silicon), `h264_nvenc` (NVIDIA)
- **Encodeurs audio :** `aac`, `ac3` (Dolby Digital), `libopus`, `mp3`
- **Contrôle du débit :** Qualité constante (CRF) ou débit cible (kbps)
- **Mise à l'échelle :** Bicubique, Lanczos, Bilinéaire, Plus proche voisin
- **Analyse des métadonnées :** Extraction automatique des informations de flux via `ffprobe`

### Architecture et workflow

- **Traitement concurrent :** Gestionnaire de file d'attente asynchrone en Rust (`tokio::mpsc`), limitation des processus FFmpeg simultanés (défaut : 2)
- **Télémétrie temps réel :** Analyse du flux `stderr` FFmpeg pour un suivi précis de la progression
- **Gestion des préréglages :** Persistance des configurations pour des profils réutilisables

## Stack technique

### Backend (Rust / Tauri)

- **Cœur :** Tauri v2 (Rust Edition 2024)
- **Runtime :** `tokio` (I/O asynchrone)
- **Sérialisation :** `serde`, `serde_json`
- **Gestion des processus :** `tauri-plugin-shell` pour l'exécution sidecar (FFmpeg/FFprobe)
- **Intégration système :** `tauri-plugin-dialog`, `tauri-plugin-fs`, `window-vibrancy`

### Frontend (SvelteKit)

- **Framework :** Svelte 5 (Runes API)
- **Système de build :** Vite
- **Styles :** Tailwind CSS v4, `clsx`, `tailwind-merge`
- **Gestion d'état :** Svelte 5 `$state` / `$props`
- **Internationalisation :** Interface multilingue avec détection automatique de la langue système
- **Typographie :** Geist Mono (intégrée)

### Installation

#### Via Homebrew (macOS)

Le moyen le plus simple d'installer et maintenir Frame à jour sur macOS est via notre Homebrew Tap :

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### Versions précompilées

Le moyen le plus simple d'exécuter Frame est de télécharger un package depuis [GitHub Releases](https://github.com/66HEX/frame/releases). Chaque version inclut des builds pour macOS (Intel/Apple Silicon), Windows et Linux (AppImage/Deb). Les binaires ne sont pas signés, votre OS peut afficher des avertissements.

### Prérequis

- Runtime Node.js (ou Bun)
- Toolchain Rust (`cargo`)
- Binaires **FFmpeg** et **FFprobe** dans le répertoire `src-tauri/binaries/`
  - Convention de nommage : `ffmpeg-<target-triple>` (ex : `ffmpeg-aarch64-apple-darwin`)

> Astuce : Exécutez `bun run setup:binaries` pour télécharger automatiquement les binaires appropriés. Utilisez `--force` pour rafraîchir.

### Instructions de compilation

1.  **Installer les dépendances :**

    ```bash
    bun install
    ```

2.  **Démarrer le serveur de développement :**

    ```bash
    bun run tauri dev
    ```

3.  **Compiler pour la production :**
    ```bash
    bun run tauri build
    ```

## Utilisation

1.  **Entrée :** Utilisez le dialogue système pour sélectionner des fichiers
2.  **Configuration :**
    - **Source :** Voir les métadonnées du fichier détecté
    - **Sortie :** Sélectionner le format de conteneur et le nom de fichier
    - **Vidéo :** Configurer codec, débit/CRF, résolution et framerate
    - **Audio :** Sélectionner codec, débit, canaux et pistes spécifiques
    - **Préréglages :** Sauvegarder et charger des profils de conversion
3.  **Exécution :** Lancer le processus de conversion via le backend Rust
4.  **Monitoring :** Voir les logs en temps réel et le pourcentage de progression

## Licence

Licence GPLv3. Voir [LICENSE](../LICENSE) pour les détails.
