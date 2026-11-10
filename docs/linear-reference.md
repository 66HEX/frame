# Linear homepage: reconstruction audit

Reference: https://linear.app/ — captured 2026-11-10T 19:36:57.091606+00:00.

This record covers the public homepage, its layout, responsive rules, and observed interactions. Sources are retained under `/tmp/frame-linear-evidence`; the portable findings are in `linear-reference.json`. The Frame documentation UI and content were excluded from the implementation.

## What creates the visual character

The page uses a near-black continuous canvas, left-aligned large typography, very broad product imagery, and generous chapter spacing. It has few standalone marketing cards. Fine separators, restrained gray levels and repeated alignment edges provide structure. Three illustrated values establish the product story before four detailed product chapters. Each chapter reuses the same title/copy/illustration/features arrangement.

## Homepage sequence

1. **Fixed navigation:** Brand at leading edge; primary links, account actions and responsive menu. Transparent initially, background/border strengthen when scrolled.
2. **Hero:** Left-aligned large headline, short muted description and one understated announcement/action; substantial empty space above; large app presentation below.
3. **Customer proof:** Concise trust caption plus customer marks, followed by an oversized product-positioning statement.
4. **Three value illustrations:** Three equal open columns divided by fine vertical lines on desktop; figure labels above, illustration occupying most height, short title and paragraph below. Horizontal cards on narrower screens.
5. **Four product chapters:** Intake/integrations; planning/monitoring; AI/automations; build/review/shipping. Each reuses the same two-column title/copy header, broad app illustration and compact feature links.
6. **Supporting proof and updates:** Changelog plus customer quotes/proof area. Not an additional grid of generic feature cards. Some customer presentation is hidden below laptop breakpoint.
7. **Closing CTA:** Centered short headline with a restrained action row and generous isolation.
8. **Footer:** Fine top separator and structured link columns, collapsing from six to four to two; status, legal and theme utilities.

## Measured and declared values

| Element | Desktop | Narrow screen | Evidence |
|---|---|---|---|
| Canvas | `#08090a` | Same inspected dark theme | Global CSS and browser measurement |
| Hero | 64px / 64px, weight 510, tracking −1.408px; x 40/y 272 at 1280 | 38px / 41.8px, weight 510; x 23/y 196 at 375 | Browser measurement; CSS confirms 38px override |
| Chapter h 2 | 48px / 48px, weight 510 | 24px / 31.92px at 375 | Browser measurement |
| Intro statement | 40px / 44px | Adapts to narrow layout | Browser measurement |
| Navigation | Fixed, 72px high | 64px at ≤640 | CSS and browser measurement |
| Chapter spacing | 128px above/below; 96px under two-column header | Header collapses at ≤1024; mobile image-first at ≤640 | `vfNSRYu7.css` |
| Paragraph | Regular token 15px /24px; large 17px /27.2px | Smaller measure, same restrained density | `N0NSUQdS.css` |
| Value presentation | Open columns, 360px high at 1280; fine separators | Horizontal 328×440px cards below 1025 | `BlGFcVAC.css`, homepage JSX |
| Closing CTA | Centered; 224px vertical margins, 40px internal gap | Margins 128px below 769, 96px below 641 | `CDNJI-Em.css` |

Browser measurements were taken at 1280px and 375px. Source CSS declarations and runtime values are kept distinct where responsive rendering changes the emitted text classes.

## Layout and responsive behavior

The homepage width is `1344px + 2 × outer gutter`. Outer gutter is 46px normally, 10px at ≤1280, 28px at ≤1024 and 16px at ≤640. Text receives another 32px inset, reduced to 8px at ≤1024. Large text has an optical −2px leading adjustment (−1px on phones). Thus the desktop 1280 headline sits at 10+32−2=40px and mobile 375 at 16+8−1=23px. This is a coordinated nested-gutter system, not a single universal padding value.

Chapter headers use equal columns, title left and a description of at most 38ch right. At ≤1024 they become one column with 32px between heading and description. At ≤640 the entire chapter is reversed so its illustration comes first; the copy block adds 40px above and 48px below, with 24px between heading and description. Keep this intentional reading order when adapting.

The hero preview is oversized and clipped on mobile in the browser inspection; reducing the entire app to phone width would lose the reference's legibility. The three value panels switch to horizontal scrolling rather than shrinking into tiny columns. Footer columns reduce from six to four at 1024 and two at 640.

## Typography, surfaces and interaction

Global CSS declares Inter Variable and weights 400/510/590/680. Display tracking is −.022em. Dark text levels are primary `#f7f8f8`, secondary `#d0d6e0`, tertiary `#8a8f98`, quaternary `#62666d`; separators are `#23252a` or translucent `#ffffff14`. Most page sections share the base canvas. Cards and framed application imagery carry subtle edges; decoration is concentrated around product illustrations.

The shipped homepage module exports Hero, Benefits, Pillar, Plan, AIAutomations, ReviewsFrame and GlobalSidePanel. Feature-plus buttons slugify the feature label and set the active panel identifier. Hover/focus preloads panel content; the global panel clears the identifier on close. Clicking Customer Requests opened a Tech Specs dialog with a blueprint/table, then closed it with Escape. Frame should link these small feature items to its existing documentation instead of recreating another modal system.

Declared animation includes a 0.4s headline start delay plus 0.033s per line, restrained 0.16s feature-link hover brightness and .97 active scale. Homepage code uses IntersectionObserver for near-viewport loading/video handling. These are static code observations, not a claim that every animation was executed or timed in the browser.

## Practical Frame translation

- Keep Frame's existing native Graphite tokens, Inter font, Nucleo icons and authentic screenshots. Use Linear's composition and hierarchy as the reference.
- Lead with a left-aligned statement and one broad native app view. Let media and typography carry the hero.
- Use three quiet video/audio/image illustrations, followed by substantial chapters for queue/presets, output controls and local conversion. Avoid extra badges, nested panels and decorative settings controls.
- Keep product chapter headings/copy paired on desktop; show the media first on phones. Make screenshots intentionally readable instead of indiscriminately shrinking them.
- Use real documentation links, actual download packages and verifiable project facts. Do not borrow customer proof or AI capabilities.
- Change only the landing page. This audit does not authorize changes to docs or shared density.

## REA provenance and limits

REA version 6.8.0 ran through its local CLI. The relevant homepage source was selected through a traceable static chain: SSR RSC imports name Hero/Benefits/Pillar; the directly loaded module map in `CkHShNvM.js` maps those IDs to `DeL57MwE.js`; that chunk is explicitly preloaded in the captured HTML. Its 182, 540 bytes have SHA-256 `a74a40a37a6169dcf66ce383afb977d0e3520aba4017cdeeed307020cba54f76`.

Homepage evidence: `ev_7acd8c2b89ab530e17eb0646b01eaeb0cac9c09900431fd05b03b4a00824b527` in `rea-homepage-evidence.json`. Summary: `rea-homepage-summary.json`. REA parsed 38, 232 AST nodes with zero parse failures and 209 retained findings. The call completed. It used a 45s per-source deadline, 512MiB worker heap, and 1200 semantic node/unknown caps. Application and semantic coverage are **partial**; semantic caps were reached. The summary's “complete-within-view” describes its own exhausted projection only.

A preliminary bootstrap-focused call is also retained as `rea-ui-evidence.json` with ID `ev_b14a3257cc28b4adc3fd913dfac04f8154c7a3e655d921f2a59a71c39ad36736`. It is supplementary, not a substitute for the homepage-specific call.

All 46 directly linked stylesheet files, three script-src entry assets, two explicitly preloaded UI modules, the HTML and inline styles are retained. Total external asset bytes: 1, 122, 439. The HTML declares 260 JavaScript URLs; only five were downloaded, and only the selected homepage module and preliminary bootstrap file received bounded REA analysis. No target JavaScript was executed during static analysis. No source maps, authenticated content, images/fonts/video bytes, or unlisted dynamic assets were fetched. CSS import scan found no `@import` declarations.

## Evidence files

- `capture-manifest.json`: origin URL, HTTP result, size and SHA-256 for HTML and each downloaded asset; all declared JS URLs and limits.
- `capture/index.html`: exact received public HTML; SHA-256 `e206ecb77a8ff4a50ad885b37e984b9e237f0d069c446a143619d5ba2abcb151`.
- `css-rule-inventory.json`: 3600 extracted rules from 46 CSS files plus inline styles; `css-focus.json` narrows the key homepage selectors.
- `dom-headings.json`, `dom-sections.json`, `rsc-decoded.txt`: observed structure and inert decoded server-component data.
- `media-inventory.json`, `font-inventory.json`: public media declarations only.
- `root-browser-observations.json`: desktop/mobile/interaction observations with attribution.
- `rea-homepage-evidence.json`, `rea-homepage-summary.json`: complete exported REA record and compact projection.
- `screenshots/linear-desktop-full.jpg`: full-page browser reference captured during visual inspection.
- `audit.json`: portable structured findings, source paths, runtime attribution and coverage limitations.

Primary source URLs: [global tokens](https://static.linear.app/web/_next/static/css/N0NSUQdS.css), [chapter layout](https://static.linear.app/web/_next/static/css/vfNSRYu7.css), [hero and value styling](https://static.linear.app/web/_next/static/css/BlGFcVAC.css), [navigation](https://static.linear.app/web/_next/static/css/CX5ehBfa.css), [footer](https://static.linear.app/web/_next/static/css/DEeMOhYy.css), [closing CTA](https://static.linear.app/web/_next/static/css/CDNJI-Em.css), [homepage UI chunk](https://static.linear.app/web/_next/static/chunks/DeL57MwE.js).

Parent also saved the full desktop browser screenshot at `screenshots/linear-desktop-full.jpg` in the evidence directory. This is direct runtime visual evidence, separate from the static captures.

## Implemented in Frame

The landing now uses a locally scoped dark Graphite canvas, a left-aligned hero, open value columns, four product chapters, quiet separators and a multi-column footer. Frame retains its own font, colors and control radii. Website interface icons use Nucleo UI Outline following the current user preference; native application screenshots retain their original Fill Duo artwork. The shared theme, documentation content and documentation shell remain unchanged.

The 1280px container has 40px desktop gutters and 24px phone gutters. Chapters have 128px desktop/80px mobile vertical padding. Heading scale is 64px desktop and 36–40px mobile; product chapter headings are 48px/32px. These are deliberate Frame values, informed by the measurements above. They are not a claim of pixel-for-pixel reproduction.

New native screenshots and exact crops are listed in `landing-capture-manifest.json`: four full views and eight detail crops, each in dark/light variants. The page uses the dark assets; every image starts from the real Frame renderer. The preview and workflow compositions position these captures, with CSS clipping and fades around their edges. The video/audio/image and local-processing figures are original HTML illustrations. The supplied Frame app icon remains in the local-processing figure.

Frame-specific choices: feature links open existing documentation; an accessible tab group switches audio, metadata and subtitle details. On phones the chapter descriptions stay before the media and the three value figures stack vertically. The hero and queue are deliberately cropped at a readable scale. Marketing presentation is dark without changing the saved documentation theme. No customer endorsements, usage numbers, or AI claims were borrowed.

Validation: responsive review at 320, 375, 768, 1024 and 1280px; no horizontal document overflow; keyboard tab switching; mobile menu/Escape/focus restoration; FAQ expansion; existing download URLs; and navigation into the unchanged documentation.
