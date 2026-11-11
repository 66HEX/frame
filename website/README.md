# Frame website

Landing page and documentation for [Frame](https://github.com/66HEX/frame), a native media converter for macOS, Windows, and Linux. The site lives in `website/` within the Frame monorepo and owns its JavaScript dependencies and lockfile. The desktop application's Cargo workspace remains at the repository root.

The website uses the SvelteKit documentation architecture from [Spektral](https://github.com/kaltwrk/spektral), with shadcn-svelte components and Frame's Graphite theme. The landing-page section layouts draw on [Tailark Quartz Libre](https://tailark.com/pages/quartz/libre).

## Run locally

Use Node 24 and pnpm 10.24.0. `website/.nvmrc` selects Node 24. From the repository root:

```bash
cd website
nvm use
corepack enable
pnpm install --frozen-lockfile
pnpm dev
```

Open the local URL printed by Vite. The landing page is at `/` and documentation starts at `/docs`. Run the commands below from `website/`; from the repository root, use `pnpm --dir website <command>`.

## Check and build

```bash
pnpm check
pnpm test
pnpm build
```

The content tests check documentation discovery, internal links and anchors, search results, and missing-page handling. The production build prerenders documentation and social images and emits a Cloudflare Worker with static assets under `.svelte-kit/cloudflare`.

```bash
pnpm preview
```

## Edit the site

| Area | Source |
| --- | --- |
| Brand, canonical URL, metadata, links | `src/lib/site/site.ts` |
| Colors, typography, geometry | `src/lib/site/theme.css` |
| Landing sections | `src/lib/features/landing/` |
| Landing route | `src/routes/+page.svelte` |
| Native app screenshots | `static/media/frame-dark.webp`, `static/media/frame-light.webp`; original PNG captures alongside them |
| Documentation screenshots and capture provenance | `static/media/docs/`, [archived screenshot manifest](https://github.com/66HEX/frame/blob/497c40c383d705f58d5deebd4062948e23ffcc9b/docs/screenshot-manifest.json) |
| Documentation content | `src/lib/site/content/docs/` |
| Documentation UI and preferences | `src/lib/site/content-ui.ts` |
| Sidebar, search, table of contents, pagination | `src/lib/features/docs/`, `src/lib/features/shell/` |
| shadcn-svelte components | `src/lib/components/ui/` |
| Nucleo UI Outline icons and source manifest | `src/lib/icons/` |

The landing page uses real captures of the native Frame application in light and dark themes, exported at 2400 × 1600. `FramePreview.svelte` selects the lossless WebP that matches the website theme. The original `frame-dark.png` and `frame-light.png` captures are retained in the same directory; the WebP files decode to identical pixels. Replace both formats when the app interface changes.

Documentation includes 15 native app views in both themes (30 lossless WebP files, each 2400 × 1600). `DocsScreenshot.svelte` loads thumbnails lazily and opens a full-size viewer. The capture manifest records the app revision, scenes, and integrity hashes. `scripts/capture-frame-docs.rs` retains the native capture helper; `python3 scripts/prepare-docs-media.py` regenerates its sample media in the system temporary directory using FFmpeg. These fixtures contain the attributed sample photograph, a generated reference tone, subtitles, and an intentionally unreadable file for the troubleshooting example.

Capture manifests and design references from the standalone website are preserved in [the pre-monorepo archive](https://github.com/66HEX/frame/tree/497c40c383d705f58d5deebd4062948e23ffcc9b/docs). Working notes belong in the repository's ignored `docs/website/` directory. Published documentation stays in `website/src/lib/site/content/docs/` and is tracked with the application code.

See [the content authoring guide](src/lib/site/README.md) for page metadata and routing. SvelteKit configuration lives in `svelte-options.ts` and is passed to the Vite SvelteKit plugin.

Documentation describes the current Frame app, using its source and README as the reference. Recheck feature and installation details when updating the app. Release buttons use GitHub's latest-release endpoints rather than pinning an old release.

## Deployment

The site uses the existing Cloudflare Worker `frame-landing-page`, serving `framegui.app` and `www.framegui.app`. Its name and routes are retained in `wrangler.jsonc`.

Cloudflare's Git connection and build settings are configured separately from Wrangler. To deploy from this monorepo, connect the existing Worker to the following settings in **Settings > Builds**:

| Setting | Value |
| --- | --- |
| Git repository | `66HEX/frame` |
| Production branch | `master` |
| Root directory | `website` |
| Build command | `pnpm build` |
| Deploy command | `pnpm exec wrangler deploy` |
| Included build watch paths | `website/**` |

Use Node 24 and pnpm 10.24.0 in the build environment. Keep the existing account, Worker, domains, build token, and runtime bindings when changing the Git connection. Copying the source into this repository does not change that connection. Cloudflare documents the [monorepo setup](https://developers.cloudflare.com/workers/ci-cd/builds/advanced-setups/) and [build watch paths](https://developers.cloudflare.com/workers/ci-cd/builds/build-watch-paths/).

Local development and validation do not publish changes. To deploy manually with access to the configured Cloudflare account, run from `website/`:

```bash
pnpm deploy
```

Review `wrangler.jsonc` and `src/lib/site/site.ts` before deploying a fork; their domains and canonical URLs belong to Frame.

## License and sources

The retained Spektral website template is MIT licensed. Its original copyright notice is preserved in [LICENSE](LICENSE). Fonts, icons, photography, and product artwork retain their own terms; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

The Frame desktop application in this monorepo is licensed under [GPL-3.0](../LICENSE). The website template's MIT license does not change the application's license.
