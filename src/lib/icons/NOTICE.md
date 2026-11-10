# Nucleo Icons notice

Copyright © Nucleo

The Nucleo icon components are covered by the project's Nucleo license. They are
restricted to use within this project and may not be redistributed as a standalone
icon collection. Brand and framework logos retain their separate sources below.

- Source: https://nucleoapp.com/
- License details: https://nucleoapp.com/license
- Original archive copyright notice: [nucleo-copyright-notice.html](nucleo-copyright-notice.html)
- Copyright notice online: https://nucleoapp.com/copyright-notice
- Social Media Icons: https://nucleoapp.com/social-media-icons

## UI Outline 18 sources

All 39 interface glyphs use Nucleo UI 1.9.0 **Outline** (`outline`) on
the original 18 × 18 grid. They were read from the user-provided archive
`nucleo_ui_svg_v1.9.0.zip`, whose SHA-256 is:

`332166b787195f04e5b9792fd4afc8f22fcaba51d73512f57c40240dba7093b1`

This is the same archive identified by the native Frame application's icon
manifest. [manifest.json](manifest.json) records every selected SVG path, its
original SHA-256, and its normalized SVG SHA-256.

The website selects the Outline variant by user preference. The native app and
its screenshots keep their existing artwork.

Only black fills and strokes are changed to `currentColor`. Path geometry,
element order, transforms, stroke widths, line caps, line joins, fill rules,
opacity, `fill-opacity`, and `data-color` attributes remain as supplied.
`IconRoot` preserves the 18 × 18 viewBox and permits the rendered size to follow
the surrounding control without adding a global stroke. The original Nucleo
stroke widths are retained. Optional `strokeWidth` API usage remains available
for the preserved React and Vue framework logos.

`CopyIcon` uses the `copy` symbol from the Outline set, replacing the template's
old `tasks-2` artwork. `LanguageBashIcon` is the `square-terminal` UI glyph,
not a Bash brand logo, and also uses Outline. `CpuIcon` is `microchip`;
`BoltIcon` remains the separate Bolt.new brand logo.

| Component | Original SVG in the archive |
| --- | --- |
| `ArrowLeftIcon` | `outline/arrows/18px_arrow-left.svg` |
| `ArrowRightIcon` | `outline/arrows/18px_arrow-right.svg` |
| `ArrowRotateClockwiseIcon` | `outline/arrows/18px_arrow-rotate-clockwise.svg` |
| `ArrowTurnRightIcon` | `outline/arrows/18px_arrow-turn-right-2.svg` |
| `BookOpenIcon` | `outline/school-education/18px_book-open.svg` |
| `CheckIcon` | `outline/ui-layout/18px_check.svg` |
| `ChevronDownIcon` | `outline/arrows/18px_chevron-down.svg` |
| `ChevronExpandYIcon` | `outline/arrows/18px_chevron-expand-y.svg` |
| `ChevronRightIcon` | `outline/arrows/18px_chevron-right.svg` |
| `ChevronUpIcon` | `outline/arrows/18px_chevron-up.svg` |
| `CircleQuestionIcon` | `outline/ui-layout/18px_circle-question.svg` |
| `CircleWarningIcon` | `outline/ui-layout/18px_circle-warning.svg` |
| `CodeIcon` | `outline/design-development/18px_code.svg` |
| `CopyIcon` | `outline/ui-layout/18px_copy.svg` |
| `CpuIcon` | `outline/technology-devices/18px_microchip.svg` |
| `CubeAxisIcon` | `outline/ar-vr/18px_cube-axis.svg` |
| `CubeIcon` | `outline/ar-vr/18px_cube.svg` |
| `CubesIcon` | `outline/ar-vr/18px_cubes.svg` |
| `DatabaseIcon` | `outline/design-development/18px_database.svg` |
| `DotsIcon` | `outline/ui-layout/18px_dots.svg` |
| `FileIcon` | `outline/files/18px_file.svg` |
| `Grid2x2Icon` | `outline/ui-layout/18px_grid-2x2.svg` |
| `HashtagIcon` | `outline/shopping/18px_hashtag.svg` |
| `LanguageBashIcon` | `outline/design-development/18px_square-terminal.svg` |
| `LayersIcon` | `outline/design-development/18px_layers.svg` |
| `LinkIcon` | `outline/editing/18px_link.svg` |
| `LoaderIcon` | `outline/ui-layout/18px_loader.svg` |
| `MagnifierIcon` | `outline/filtering-sorting/18px_magnifier.svg` |
| `Menu2Icon` | `outline/ui-layout/18px_menu-2.svg` |
| `MinusIcon` | `outline/ui-layout/18px_minus.svg` |
| `SidebarLeftClosedIcon` | `outline/ui-layout/18px_layout-left.svg` |
| `SidebarLeftIcon` | `outline/ui-layout/18px_sidebar-left.svg` |
| `TableOfContentsIcon` | `outline/ui-layout/18px_table-rows.svg` |
| `TerminalIcon` | `outline/design-development/18px_terminal.svg` |
| `ThemeIcon` | `outline/photography-video/18px_dark-light.svg` |
| `TouchClickIcon` | `outline/design-development/18px_touch-click.svg` |
| `TriangleWarningIcon` | `outline/ui-layout/18px_triangle-warning.svg` |
| `WorkflowIcon` | `outline/arrows/18px_workflow.svg` |
| `XmarkIcon` | `outline/ui-layout/18px_xmark.svg` |

## Preserved brand, language, and framework logos

These brand, language, and framework logos keep their existing artwork and viewBoxes.
Their component files are unchanged by the Outline migration; SHA-256 hashes
and original source comments are recorded in `manifest.json`.

| Component | Original name | Source |
| --- | --- | --- |
| `AnthropicIcon` | Anthropic (anthropic.svg) | [Simple Icons](https://github.com/simple-icons/simple-icons/blob/develop/icons/anthropic.svg) |
| `BoltIcon` | BoltIcon (Bolt.new logo) | [Preserved repository artwork](https://github.com/kaltwrk/spektral/blob/e7a3551f/apps/web/src/lib/icons/BoltIcon.svelte) |
| `CursorIcon` | CursorIcon | [Preserved repository artwork](https://github.com/kaltwrk/spektral/blob/e7a3551f/apps/web/src/lib/icons/CursorIcon.svelte) |
| `FrameworkReactIcon` | ReactIcon | [Hugeicons core-free-icons](https://www.npmjs.com/package/@hugeicons/core-free-icons) |
| `FrameworkSvelteIcon` | AppFrameworkSvelteIcon | [Preserved repository artwork](https://github.com/kaltwrk/spektral/blob/01498e0d/apps/web/src/lib/components/app-icons/AppFrameworkSvelteIcon.svelte) |
| `FrameworkVueIcon` | AppFrameworkVueIcon | [Preserved repository artwork](https://github.com/kaltwrk/spektral/blob/01498e0d/apps/web/src/lib/components/app-icons/AppFrameworkVueIcon.svelte) |
| `GitHubIcon` | GitHub | [Nucleo Social Media Icons](https://nucleoapp.com/social-media-icons) |
| `GrokIcon` | Grok.Mono (Grok/components/Mono.tsx) | [Lobe Icons](https://github.com/lobehub/lobe-icons/blob/master/src/Grok/components/Mono.tsx) |
| `LanguageCssIcon` | CSS (css.svg) | [Simple Icons](https://github.com/simple-icons/simple-icons/blob/develop/icons/css.svg) |
| `LanguageDotenvIcon` | .ENV (dotenv.svg) | [Simple Icons](https://github.com/simple-icons/simple-icons/blob/develop/icons/dotenv.svg) |
| `LanguageHtmlIcon` | HTML5 (html5.svg) | [Simple Icons](https://github.com/simple-icons/simple-icons/blob/develop/icons/html5.svg) |
| `LanguageJavaScriptIcon` | JavaScript (javascript.svg) | [Simple Icons](https://github.com/simple-icons/simple-icons/blob/develop/icons/javascript.svg) |
| `LanguageJsonIcon` | JSON (json.svg) | [Simple Icons](https://github.com/simple-icons/simple-icons/blob/develop/icons/json.svg) |
| `LanguageMarkdownIcon` | Markdown (markdown.svg) | [Simple Icons](https://github.com/simple-icons/simple-icons/blob/develop/icons/markdown.svg) |
| `LanguageTypeScriptIcon` | TypeScript (typescript.svg) | [Simple Icons](https://github.com/simple-icons/simple-icons/blob/develop/icons/typescript.svg) |
| `LanguageWebGlIcon` | WebGL (webgl.svg) | [Simple Icons](https://github.com/simple-icons/simple-icons/blob/develop/icons/webgl.svg) |
| `LanguageWebGpuIcon` | WebGPU (webgpu.svg) | [Simple Icons](https://github.com/simple-icons/simple-icons/blob/develop/icons/webgpu.svg) |
| `LovableIcon` | Lovable (lovable.svg) | [SVGL](https://github.com/pheralb/svgl/blob/main/static/library/lovable.svg) |
| `MetaAIIcon` | MetaAIIcon | [Preserved repository artwork](https://github.com/kaltwrk/spektral/blob/e7a3551f/apps/web/src/lib/icons/MetaAIIcon.svelte) |
| `OpenAIIcon` | OpenAI (openai.svg) | [Simple Icons](https://github.com/simple-icons/simple-icons/blob/14.15.0/icons/openai.svg) |
| `PerplexityIcon` | Perplexity.Mono (Perplexity/components/Mono.tsx) | [Lobe Icons](https://github.com/lobehub/lobe-icons/blob/master/src/Perplexity/components/Mono.tsx) |
| `ZedIcon` | zed_logo.svg | [Zed repository](https://github.com/zed-industries/zed/blob/main/assets/images/zed_logo.svg) |

The existing Bolt.new, Cursor, Meta AI, Svelte, and Vue files did not record an upstream
asset name or supplier. Their comments link to the earliest available repository
version and identify the original local component name. These entries document
known provenance; they do not claim a verified external source.

### Simple Icons

The Simple Icons adaptations use its CC0-1.0 licensed SVG data. Their original
brand colors are intentionally replaced with `currentColor`.

- Source: https://simpleicons.org/
- License: https://github.com/simple-icons/simple-icons/blob/develop/LICENSE.md

### Other retained artwork

- Lobe Icons: https://github.com/lobehub/lobe-icons/blob/master/LICENSE
- Hugeicons: https://github.com/hugeicons/hugeicons-react/blob/main/LICENSE
- SVGL: https://github.com/pheralb/svgl/blob/main/LICENSE
- Zed branding: https://zed.dev/brand

The Frame logo in `../site/assets/logo.svg`, the generated `/favicon.svg`, and the
PNG/ICO app icons are product branding, not interface glyphs. Their sources
are documented in `../site/assets/NOTICE.md`. The generated SVG
mask in `TableOfContents.svelte` is a layout decoration, not an icon.
