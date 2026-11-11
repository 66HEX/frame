# Website content and identity

This directory contains Frame's website identity, theme, interface copy, and documentation. It uses the file-based content system retained from Spektral.

Run the commands in this guide from the monorepo's `website/` directory. Published pages stay in `website/src/lib/site/content/docs/`; the ignored root `docs/website/` directory is for working notes.

## Add or edit a documentation page

Create a Markdown file under `content/docs/` with a `.svx`, `.mdx`, or `.md` extension. All three use mdsvex and accept Svelte syntax for embedded components.

```markdown
---
title: Example guide
description: A short description for the page header, search, and social preview.
order: 20
sidebar:
  label: Example guide
---

## First step

Write the guide here.
```

The shell renders the page title as H1, so start the body at H2. H2 and H3 headings populate the table of contents. Code fences receive highlighting and copy controls. Language names must be included in the Shiki configuration in `svelte-options.ts`.

Every page needs a useful title and description. Use actual Frame interface labels and verify behavior against the app before documenting it. Avoid promising a codec, platform feature, or output format without checking its current availability.

## URLs and navigation

| Source | URL |
| --- | --- |
| `content/docs/index.svx` | `/docs` |
| `content/docs/installation.svx` | `/docs/installation` |
| `content/docs/(output)/video.svx` | `/docs/video` |
| `content/docs/advanced/example.svx` | `/docs/advanced/example` |

Parenthesized directories create sidebar groups without adding a URL segment. A group's `_meta.svx` contains frontmatter only:

```yaml
---
group:
  label: Output settings
  order: 200
---
```

The group label and order can also be defined in a group's index page. Define them in one place. Pages sort by `order`, then filename; child groups follow the directory's pages. Sidebar order also determines previous and next links.

The root index defines the documentation view:

```yaml
view:
  label: Documentation
  icon: BookOpenIcon
  order: 0
```

Reusable components belong outside `content/`. Every supported file there becomes a page except `_meta` files. The registry rejects duplicate routes, invalid metadata, missing view indexes, and reserved slugs.

## Add a native app screenshot

Use paired captures from the current Frame application. Place 2400 × 1600 WebP images at `static/media/docs/<scene>-light.webp` and `static/media/docs/<scene>-dark.webp`.

Import the screenshot component once after the page's frontmatter:

```svelte
<script>
  import DocsScreenshot from '$lib/features/docs/DocsScreenshot.svelte';
</script>
```

Place the screenshot next to the instructions it illustrates:

```svelte
<DocsScreenshot
  scene="presets"
  alt="Frame Presets inspector showing saved configurations and the custom preset field."
  caption="Save and reuse an output configuration in the Presets tab."
/>
```

The component follows the website theme and loads article images lazily. Its lightbox fits the image to the viewport over a dim overlay, with a bare close icon in the screen corner. A FLIP animation moves the image between its thumbnail and fitted view while the overlay fades; reduced-motion preferences skip that movement. Clicking the overlay or pressing Escape closes the image and restores focus. Captions stay in the article, and the lightbox title and description are available to screen readers.

Capture provenance and dimensions are recorded in the [archived screenshot manifest](https://github.com/66HEX/frame/blob/497c40c383d705f58d5deebd4062948e23ffcc9b/docs/screenshot-manifest.json).

Use one to three relevant views in a guide. Describe the visible state in the caption and the useful visual information in the alt text. Check both theme files before referencing a scene. Imports and component markup are excluded from the search text by the existing Markdown parser.

## Search, sharing, and navigation

The search index includes visible Markdown page titles, headings, and body text. It loads when search opens. `sidebar.hidden: true` removes a page from navigation, search, and pagination while keeping its direct URL.

The original page source is served at `/docs/raw/<slug>`; the overview uses `/docs/raw/index`. For SVX pages with screenshots, this source includes the Svelte import and `DocsScreenshot` tags. Copy Markdown preserves that source. Search excludes scripts and component markup from its text index. The page menu can copy Markdown, open the raw source, or open the Frame repository. There are no framework, package-manager, or AI-provider selectors.

The shell preserves the Spektral sidebar and responsive table of contents. Keyboard shortcuts use Command on macOS or Control on other platforms: K opens search, B toggles the sidebar, and Shift+L changes the theme. Theme preference is stored under `frame-theme`.

## Identity and assets

- `site.ts`: product name, canonical URL, descriptions, social links, icons, and browser theme colors.
- `theme.css`: Graphite colors, Inter fonts, shadcn geometry, and shared surfaces.
- `content-ui.ts`: documentation labels, search limits, navigation, page actions, and theme defaults.
- `labels.ts`: shared control labels and copy feedback.
- `assets/`: Frame logo and app icons. See its `NOTICE.md` for provenance.

The landing page lives in `lib/features/landing/` and the root route. Edit its composition there without changing the documentation registry.

## Generated endpoints

`/sitemap.xml`, `/llms.txt`, and the documentation routes discover content from the registry. `/og` and `/docs/og/<slug>` generate social images from site metadata and the shared theme. The overview uses `/docs/og/index`.

Run the normal checks after content changes:

```bash
pnpm check
pnpm test
pnpm build
```

The invariant tests catch broken internal documentation links and anchors, missing search entries, stale section registration, and unknown-page handling. The build also validates frontmatter and compiles embedded Svelte content.
