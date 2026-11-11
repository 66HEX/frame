# Third-party sources

## Spektral website template

The documentation runtime, application shell, Svelte components, content registry, search, Markdown rendering, and SEO helpers were adapted from `apps/web` in [kaltwrk/spektral](https://github.com/kaltwrk/spektral), revision `4acb4c50005e953fa60d1dff3edc90e68f5e9564`.

Spektral's original MIT license and copyright notice are preserved verbatim in [LICENSE](LICENSE): Copyright (c) 2026 Motion Core contributors.

The GPU library, interactive playground, demo code, and Spektral brand assets are not part of this website. Package dependencies retain their own licenses.

## shadcn-svelte and Bits UI

The retained component source under `src/lib/components/ui/` comes from the Spektral template's shadcn-svelte implementation. Its accessible component behavior uses Bits UI.

- [shadcn-svelte source and license](https://github.com/huntabyte/shadcn-svelte)
- [Bits UI source and license](https://github.com/huntabyte/bits-ui)

## Interface icons

Website interface icons use Nucleo UI 1.9.0 Outline (`outline`, 18px grid), from the supplied `nucleo_ui_svg_v1.9.0.zip`. Original geometry, stroke widths, caps, joins, and opacity are preserved; black fills and strokes are normalized to `currentColor`. Native application screenshots retain their original Fill Duo artwork. The Nucleo icons are covered by the project's Nucleo license and are not an independently redistributable icon collection. Brand, framework, and programming-language logos have separate sources.

The source names, archive version, integrity hashes, and license links are preserved in [the icon notice](src/lib/icons/NOTICE.md) and its accompanying manifest. Current Frame brand asset provenance is documented separately below.

## Inter

Inter is copyright 2016 The Inter Project Authors and is distributed under the SIL Open Font License 1.1. The full license is retained at [src/lib/assets/fonts/inter/LICENSE.txt](src/lib/assets/fonts/inter/LICENSE.txt).

The Regular and Medium WOFF2 files were converted from Frame's bundled Inter TTF files without changing their glyph outlines. The Latin and Latin Extended variable WOFF2 files came from the installed `@fontsource-variable/inter` package in the local Frame Mira reference project. The website uses the same typeface as the native application.

## Frame branding and application

The Frame logomark comes from `frame-app/assets/icons/frame.svg` in [66HEX/frame](https://github.com/66HEX/frame). PNG app icons were retained from the previous Frame landing page. See [the artwork notice](src/lib/site/assets/NOTICE.md).

The Frame desktop application is licensed under [GPL-3.0](../LICENSE). Its license is separate from the retained MIT website template. Product artwork and branding are not relicensed by the template notice.

## Native application screenshots

`static/media/frame-dark.png` and `static/media/frame-light.png` are original 2400 × 1600 captures of the native Frame application. The website displays `frame-dark.webp` and `frame-light.webp`, lossless copies in the same directory that decode to identical pixels, in the matching theme. They show Frame's native interface.

`static/media/docs/` contains 15 additional native views in light and dark themes, captured at 2400 × 1600 and stored as lossless WebP. The screenshots were rendered by the real application's GPUI window renderer without recreating or compositing interface elements. Capture details and hashes are recorded in the [archived screenshot manifest](https://github.com/66HEX/frame/blob/497c40c383d705f58d5deebd4062948e23ffcc9b/docs/screenshot-manifest.json). Their sample video uses the photograph credited below, a generated reference tone, and original sample subtitles. The audio fixture is a generated reference tone. The troubleshooting fixture is deliberately unreadable media. The fixture generation recipe is retained in [scripts/prepare-docs-media.py](scripts/prepare-docs-media.py).

## Sample photograph

`static/media/alpine-lake.jpg` is a photograph by Philip Ho, sourced from [Unsplash](https://unsplash.com/photos/2HFe1VFPlyQ), under the [Unsplash License](https://unsplash.com/license). It is retained as sample media for the Frame preview.

## Layout reference

The landing-page section composition was informed by [Tailark Quartz Libre](https://tailark.com/pages/quartz/libre). The hero's type scale and spacing follow the public preview of [Quartz Hero Section 1](https://tailark.com/blocks/quartz/hero-section). Frame's product content, native application screenshots, theme, and Svelte implementation are maintained in this repository. The layout references do not change the licenses of the assets listed above.
