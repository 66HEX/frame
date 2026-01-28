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

**Frame** — высокопроизводительная утилита для конвертации медиа, построенная на фреймворке Tauri v2. Предоставляет нативный интерфейс для операций FFmpeg с детальным контролем параметров транскодирования видео и аудио. Приложение использует бэкенд на Rust для управления параллельными задачами и выполнения процессов, а также фронтенд на Svelte 5 для настройки и мониторинга состояния.

<br />
<div align="center">
  <img src="../preview.png" alt="Превью приложения Frame" width="800" />
</div>
<br />

> [!WARNING]
> **Уведомление о неподписанном приложении**
> Поскольку приложение не подписано, ОС выдаст предупреждение:
>
> - **macOS:** Система пометит приложение атрибутом карантина. Для запуска удалите атрибут вручную:
>   ```bash
>   xattr -dr com.apple.quarantine /Applications/Frame.app
>   ```
> - **Windows:** SmartScreen может заблокировать запуск. Нажмите **«Подробнее»**, затем **«Выполнить в любом случае»**.

## Возможности

### Ядро конвертации

- **Поддержка контейнеров:** `mp4`, `mkv`, `webm`, `mov`, `mp3`, `m4a`, `wav`, `flac`.
- **Видеокодеры:**
  - `libx264` (H.264 / AVC)
  - `libx265` (H.265 / HEVC)
  - `vp9` (Google VP9)
  - `prores` (Apple ProRes)
  - `libsvtav1` (SVT-AV1)
  - **Аппаратное ускорение:** `h264_videotoolbox` (Apple Silicon), `h264_nvenc` (NVIDIA)
- **Аудиокодеры:** `aac`, `ac3` (Dolby Digital), `libopus`, `mp3`
- **Контроль битрейта:** Постоянное качество (CRF) или целевой битрейт (kbps)
- **Масштабирование:** Бикубическое, Ланцош, Билинейное, Ближайший сосед
- **Анализ метаданных:** Автоматическое извлечение информации о потоках через `ffprobe`

### Архитектура и рабочий процесс

- **Параллельная обработка:** Асинхронный менеджер очереди задач на Rust (`tokio::mpsc`), ограничение параллельных процессов FFmpeg (по умолчанию: 2)
- **Телеметрия в реальном времени:** Парсинг потока `stderr` FFmpeg для точного отслеживания прогресса
- **Управление пресетами:** Сохранение конфигураций для повторного использования

## Технологический стек

### Бэкенд (Rust / Tauri)

- **Ядро:** Tauri v2 (Rust Edition 2024)
- **Рантайм:** `tokio` (асинхронный I/O)
- **Сериализация:** `serde`, `serde_json`
- **Управление процессами:** `tauri-plugin-shell` для sidecar-выполнения (FFmpeg/FFprobe)
- **Системная интеграция:** `tauri-plugin-dialog`, `tauri-plugin-fs`, `window-vibrancy`

### Фронтенд (SvelteKit)

- **Фреймворк:** Svelte 5 (Runes API)
- **Система сборки:** Vite
- **Стили:** Tailwind CSS v4, `clsx`, `tailwind-merge`
- **Управление состоянием:** Svelte 5 `$state` / `$props`
- **Интернационализация:** Многоязычный интерфейс с автоматическим определением языка системы
- **Типографика:** Geist Mono (встроенный)

### Установка

#### Через Homebrew (macOS)

Самый простой способ установить и обновлять Frame на macOS — использовать наш Homebrew Tap:

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### Готовые сборки

Самый простой способ запустить Frame — скачать готовый пакет со страницы [GitHub Releases](https://github.com/66HEX/frame/releases). Каждый релиз содержит сборки для macOS (Intel/Apple Silicon), Windows и Linux (AppImage/Deb). Бинарные файлы не подписаны, поэтому ОС может показать предупреждение.

### Требования

- Среда выполнения Node.js (или Bun)
- Инструментарий Rust (`cargo`)
- Бинарные файлы **FFmpeg** и **FFprobe** в директории `src-tauri/binaries/`
  - Соглашение об именах: `ffmpeg-<target-triple>` (например, `ffmpeg-aarch64-apple-darwin`)

> Совет: Выполните `bun run setup:binaries` для автоматической загрузки нужных бинарных файлов. Используйте `--force` для обновления.

### Инструкции по сборке

1.  **Установка зависимостей:**

    ```bash
    bun install
    ```

2.  **Запуск сервера разработки:**

    ```bash
    bun run tauri dev
    ```

3.  **Сборка для продакшена:**
    ```bash
    bun run tauri build
    ```

## Использование

1.  **Ввод:** Выберите файлы через системный диалог
2.  **Настройка:**
    - **Источник:** Просмотр метаданных файла
    - **Выход:** Выбор формата контейнера и имени файла
    - **Видео:** Настройка кодека, битрейта/CRF, разрешения и частоты кадров
    - **Аудио:** Выбор кодека, битрейта, каналов и дорожек
    - **Пресеты:** Сохранение и загрузка профилей конвертации
3.  **Выполнение:** Запуск процесса конвертации через бэкенд Rust
4.  **Мониторинг:** Просмотр логов и прогресса в реальном времени

## Лицензия

Лицензия GPLv3. Подробности в [LICENSE](../LICENSE).
