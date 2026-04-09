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

**Frame** - это высокопроизводительная утилита для преобразования мультимедиа, построенная на фреймворке Tauri v2. Она предоставляет нативный интерфейс для операций FFmpeg, позволяя осуществлять детальный контроль над параметрами преобразования видео, аудио и изображений. Приложение использует бэкенд на основе Rust для одновременного управления задачами и выполнения процессов, в сочетании с фронтендом Svelte 5 для конфигурации и мониторинга состояния.

<br />
<div align="center">
  <img src="../preview.png" alt="Frame Application Preview" width="800" />
</div>
<br />

> [!WARNING]
> **Уведомление о неподписанном заявлении**
> Поскольку приложение в настоящее время не подписано, операционная система отметит это:
>
> - **macOS:** Система пометит приложение и его двоичные файлы с побочными файлами атрибутом карантина. Чтобы запустить приложение, снимите атрибут вручную:
>   ``bash
>   xattr -dr com.apple.quarantine /Applications/Frame.app
>   ```
> - **Windows:** Windows SmartScreen может помешать запуску приложения. Нажмите **"Дополнительная информация "**, а затем **"Запустить в любом случае "**, чтобы продолжить.

## Спонсоры GitHub

Если Frame поможет вам, поддержите проект на GitHub Sponsors:

[**Спонсорская рамка**](https://github.com/sponsors/66HEX)

Текущие цели финансирования:

- **Apple Developer Program:** `99 долларов США в год` для подписания и нотариального заверения сборок macOS.
- **Сертификат подписи кода от Microsoft:** по оценкам, 300-700 долларов США в год для подписи сборок Windows и снижения трения SmartScreen.

Спонсорские взносы используются в первую очередь для покрытия расходов на подписание релизов.

Полную информацию о спонсорстве, предложения по уровню и контрольный список запуска смотрите на [GitHub Sponsors](https://github.com/sponsors/66HEX).

## Характеристики

### Медиаконверсия Core

- **Типы медиафайлов:** Видео, Аудио, Изображение.
- ** Поддерживаемые форматы вывода:**
  - **Видео:** `mp4`, `mkv`, `webm`, `mov`, `gif`
  - **Аудио:** `mp3`, `m4a`, `wav`, `flac`
  - **Изображение:** `png`, `jpg`, `webp`, `bmp`, `tiff`
- **Видеокодеры:**
  - `libx264` (H.264 / AVC)
  - `libx265` (H.265 / HEVC)
  - `vp9` (Google VP9)
  - `prores` (Apple ProRes)
  - `libsvtav1` (масштабируемая видеотехнология AV1)
  - ** Аппаратное ускорение:** `h264_videotoolbox` (Apple Silicon), `hevc_videotoolbox` (Apple Silicon), `h264_nvenc` (NVIDIA), `hevc_nvenc` (NVIDIA), `av1_nvenc` (NVIDIA).
- **Кодировщики изображений:** `png`, `mjpeg` (JPEG), `libwebp` (WebP), `bmp`, `tiff`.
- **Аудиокодеры:** `aac`, `ac3` (Dolby Digital), `libopus`, `mp3`, `alac` (Apple Lossless), `flac` (Free Lossless Audio Codec), `pcm_s16le` (WAV).
- **Контроль битрейта:** Постоянный коэффициент скорости (CRF) или целевой битрейт (кбит/с).
- **Масштабирование:** бикубическое, Ланцоша, билинейное, ближайший сосед.
- **Исследование метаданных:** Автоматическое извлечение деталей потока (кодек, продолжительность, битрейт, расположение каналов) с помощью `ffprobe`.
- **AI Upscaling:** Встроенный `Real-ESRGAN` для высококачественного масштабирования видео и изображений (x2, x4).

### Архитектура и рабочий процесс

- **Современная обработка:** Асинхронный менеджер очередей задач, реализованный на Rust (`tokio::mpsc`), ограничивающий одновременные процессы FFmpeg (по умолчанию: 2).
- **Телеметрия в реальном времени:** Потоковый разбор FFmpeg `stderr` для точного отслеживания прогресса и вывода логов.
- **Управление пресетами:** Сохранение конфигурации для многократно используемых профилей преобразования.

## Технический стек

### Бэкэнд (Rust / Tauri)

- **Ядро:** Tauri v2 (Rust Edition 2024).
- **Runtime:** `tokio` (Async I/O).
- **Сериализация:** `serde`, `serde_json`.
- **Управление процессами:** `tauri-plugin-shell` для выполнения sidecar (FFmpeg/FFprobe).
- **Системная интеграция:** `tauri-plugin-dialog`, `tauri-plugin-fs`.

### Фронтенд (SvelteKit)

- **Framework:** Svelte 5 (Runes API).
- **Система сборки:** Vite.
- **Стайлинг:** Tailwind CSS v4, `clsx`, `tailwind-merge`.
- **Управление государством:** Svelte 5 `$state` / `$props`.
- **Интернационализация:** Многоязычный интерфейс с автоматическим определением языка системы.
- **Типография:** Loskeley Mono (встроенная).

## Установка

### Скачать готовые двоичные файлы

Самый простой способ начать работу - загрузить последнюю версию для вашей платформы (macOS, Windows или Linux) прямо с GitHub.

[**Скачать последнюю версию**](https://github.com/66HEX/frame/releases)

> **Примечание:** Поскольку приложение еще не подписано кодом, вам может потребоваться вручную одобрить его в настройках системы (см. предупреждение в верхней части этого файла).

### WinGet (Windows)

Frame доступен в официальном репозитории WinGet под идентификатором `66HEX.Frame`.

```powershell
winget install --id 66HEX.Frame -e
```

Обновить:

```powershell
winget upgrade --id 66HEX.Frame -e
```

### Homebrew (macOS)

Пользователи macOS могут легко установить и обновить Frame с помощью нашего собственного Homebrew Tap:

```bash
brew tap 66HEX/frame
brew install --cask frame
```

### Системные требования Linux

Даже при использовании **AppImage** Frame полагается на системные библиотеки **WebKitGTK** и **GStreamer** для рендеринга пользовательского интерфейса и обработки воспроизведения медиа. Для нативных диалогов в Linux также требуется интеграция **XDG Desktop Portal** (плюс бэкенд, специфичный для настольных систем) и `zenity` в качестве резервной копии. Если приложение падает при добавлении источника, предварительный просмотр видео остается пустым, или диалоги файлов не открываются/не отображаются корректно, установите пакеты ниже.

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

> **Пользователи KDE:** установите `xdg-desktop-portal-kde` (вместо `xdg-desktop-portal-gtk`), чтобы получить тематические диалоги в стиле Plasma-native.

### Сборка из источника

Если вы предпочитаете создавать приложение самостоятельно или хотите внести свой вклад, выполните следующие шаги.

**1. Необходимые условия**

- **Rust:** [Установить Rust](https://www.rust-lang.org/tools/install)
- **Bun (или Node.js):** [Установите Bun](https://bun.sh/)
- **Зависимости от ОС:** Следуйте указаниям [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) для вашей операционной системы.

**2. Настройка проекта**

Клонируйте репозиторий и установите зависимости:

```bash
git clone https://github.com/66HEX/frame.git
cd frame
bun install
```

**3. Установка двоичных файлов**

Для апскейлинга AI Frame требуются двоичные файлы FFmpeg/FFprobe и двоичные файлы Real-ESRGAN. Мы предоставляем скрипты для автоматического поиска правильных версий для вашей платформы:

```bash
bun run setup:ffmpeg
bun run setup:upscaler
```

**4. Строить или бежать*

- **Развитие:**

  ```bash
  bun tauri dev
  ```

- **Производственная сборка:**
  ```bash
  bun tauri build
  ```

## Использование

1.  **Ввод:** Используйте системный диалог для выбора файлов.
2.  **Конфигурация:**
    - **Источник:** Просмотр обнаруженных метаданных файла.
    - **Вывод:** Выберите формат контейнера и имя выходного файла.
    - **Видео:** Настройте кодек, битрейт/CRF, разрешение и частоту кадров.
    - **Изображения:** Настройте разрешение/масштабирование изображения, формат пикселей и дополнительное увеличение AI.
    - **Аудио:** Выберите кодек, битрейт, каналы и конкретные дорожки.
    - **Пресеты:** Сохраняйте и загружайте многократно используемые профили преобразования.
3.  **Исполнение:** Запускает процесс преобразования через бэкэнд Rust.
4.  **Мониторинг:** Просмотр журналов в реальном времени и процентных счетчиков в пользовательском интерфейсе.

## Звездная история

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline&theme=dark" />
  <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
  <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=66HEX/frame&type=timeline" />
</picture>

## Благодарности и код третьей стороны

- **Real-ESRGAN**: Copyright (c) 2021, Xintao Wang. Лицензия [BSD 3-Clause](https://github.com/xinntao/Real-ESRGAN/blob/master/LICENSE).
- **FFmpeg**: Лицензия [GPLv3](https://www.ffmpeg.org/legal.html).

## Лицензия

Лицензия GPLv3. Подробности см. в [LICENSE](../LICENSE).
