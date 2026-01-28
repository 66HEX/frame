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

**Frame** es una utilidad de conversión de medios de alto rendimiento construida sobre el framework Tauri v2. Proporciona una interfaz nativa para operaciones FFmpeg, permitiendo un control granular sobre los parámetros de transcodificación de video y audio. La aplicación utiliza un backend basado en Rust para la gestión de tareas concurrentes y ejecución de procesos, junto con un frontend Svelte 5 para configuración y monitoreo de estado.

<br />
<div align="center">
  <img src="../preview.png" alt="Vista previa de Frame" width="800" />
</div>
<br />

> [!WARNING]
> **Aviso de aplicación sin firmar**
> Como la aplicación actualmente no está firmada, tu sistema operativo mostrará advertencias:
>
> - **macOS:** El sistema marcará la app y sus binarios con un atributo de cuarentena. Para ejecutar la app, elimina el atributo manualmente:
>   ```bash
>   xattr -dr com.apple.quarantine /Applications/Frame.app
>   ```
> - **Windows:** SmartScreen puede impedir que la aplicación se inicie. Haz clic en **"Más información"** y luego en **"Ejecutar de todos modos"**.

## Características

### Núcleo de conversión

- **Contenedores soportados:** `mp4`, `mkv`, `webm`, `mov`, `mp3`, `m4a`, `wav`, `flac`.
- **Codificadores de video:**
  - `libx264` (H.264 / AVC)
  - `libx265` (H.265 / HEVC)
  - `vp9` (Google VP9)
  - `prores` (Apple ProRes)
  - `libsvtav1` (SVT-AV1)
  - **Aceleración por hardware:** `h264_videotoolbox` (Apple Silicon), `h264_nvenc` (NVIDIA)
- **Codificadores de audio:** `aac`, `ac3` (Dolby Digital), `libopus`, `mp3`
- **Control de bitrate:** Factor de calidad constante (CRF) o bitrate objetivo (kbps)
- **Escalado:** Bicúbico, Lanczos, Bilineal, Vecino más cercano
- **Análisis de metadatos:** Extracción automática de información de streams vía `ffprobe`

### Arquitectura y flujo de trabajo

- **Procesamiento concurrente:** Gestor de cola de tareas asíncrono en Rust (`tokio::mpsc`), limitando procesos FFmpeg simultáneos (por defecto: 2)
- **Telemetría en tiempo real:** Análisis del stream `stderr` de FFmpeg para seguimiento preciso del progreso
- **Gestión de presets:** Persistencia de configuraciones para perfiles de conversión reutilizables

## Stack técnico

### Backend (Rust / Tauri)

- **Núcleo:** Tauri v2 (Rust Edition 2024)
- **Runtime:** `tokio` (I/O asíncrono)
- **Serialización:** `serde`, `serde_json`
- **Gestión de procesos:** `tauri-plugin-shell` para ejecución sidecar (FFmpeg/FFprobe)
- **Integración del sistema:** `tauri-plugin-dialog`, `tauri-plugin-fs`, `window-vibrancy`

### Frontend (SvelteKit)

- **Framework:** Svelte 5 (Runes API)
- **Sistema de build:** Vite
- **Estilos:** Tailwind CSS v4, `clsx`, `tailwind-merge`
- **Gestión de estado:** Svelte 5 `$state` / `$props`
- **Internacionalización:** Interfaz multilingüe con detección automática del idioma del sistema
- **Tipografía:** Geist Mono (embebida)

### Instalación

#### Vía Homebrew (macOS)

La forma más fácil de instalar y mantener Frame actualizado en macOS es mediante nuestro Homebrew Tap:

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### Usar versiones precompiladas

La forma más fácil de ejecutar Frame es descargar un paquete precompilado desde [GitHub Releases](https://github.com/66HEX/frame/releases). Cada versión incluye builds para macOS (Intel/Apple Silicon), Windows y Linux (AppImage/Deb). Los binarios no están firmados, por lo que tu SO puede mostrar advertencias.

### Requisitos previos

- Runtime Node.js (o Bun)
- Toolchain de Rust (`cargo`)
- Binarios **FFmpeg** y **FFprobe** en el directorio `src-tauri/binaries/`
  - Convención de nombres: `ffmpeg-<target-triple>` (ej: `ffmpeg-aarch64-apple-darwin`)

> Consejo: Ejecuta `bun run setup:binaries` para descargar automáticamente los binarios correctos. Usa `--force` para actualizar descargas existentes.

### Instrucciones de compilación

1.  **Instalar dependencias:**

    ```bash
    bun install
    ```

2.  **Iniciar servidor de desarrollo:**

    ```bash
    bun run tauri dev
    ```

3.  **Compilar para producción:**
    ```bash
    bun run tauri build
    ```

## Uso

1.  **Entrada:** Usa el diálogo del sistema para seleccionar archivos
2.  **Configuración:**
    - **Fuente:** Ver metadatos del archivo detectado
    - **Salida:** Seleccionar formato de contenedor y nombre de archivo
    - **Vídeo:** Configurar códec, bitrate/CRF, resolución y framerate
    - **Audio:** Seleccionar códec, bitrate, canales y pistas específicas
    - **Ajustes:** Guardar y cargar perfiles de conversión reutilizables
3.  **Ejecución:** Inicia el proceso de conversión vía backend Rust
4.  **Monitoreo:** Ver logs en tiempo real y porcentaje de progreso en la UI

## Licencia

Licencia GPLv3. Ver [LICENSE](../LICENSE) para más detalles.
