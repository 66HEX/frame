# Landing design references

The current landing follows the [Linear homepage analysis](./linear-reference.md). The Tailark notes below record the earlier design iteration.

The landing uses the public [Tailark Quartz / Libre reference](https://tailark.com/pages/quartz/libre), inspected at its [landing preview](https://pro-pages.tailark.com/libre/landing-one) on 10 November 2026. Spektral supplies the SvelteKit documentation structure and shadcn-svelte primitives. Frame supplies the identity, product content, palette and control treatment.

## Layout observations

The public HTML has a fixed header, a centered hero, a large product composition, feature sections and a card-colored footer. The main hero container uses `max-w-6xl`; several sections use `max-w-5xl`. Horizontal padding is 24px, increasing to 48px in larger hero layouts. Section padding alternates between 64px and 96px.

The hero scales from 48px to 60px with balanced lines. Section headings scale from 36px to 48px. The page combines two-column text/media compositions with smaller feature grids. Container queries increase some grids from two to three columns. Rounded media surfaces, quiet separators and broad whitespace establish the rhythm.

These measurements come from the served HTML classes. Browser inspection complements them; static classes alone do not establish final pixel positions at every viewport. The Frame page adapts the section rhythm and composition to a media converter.

## Hero typography refinement

The final hero typography follows [Tailark Quartz Hero Section 1](https://tailark.com/blocks/quartz/hero-section), inspected in its public preview at `https://tailark.com/view/hero-section-1?theme=dark` on 10 November 2026. The rendered reference heading uses `text-4xl font-semibold tracking-tight md:text-5xl`, a 576px maximum width, and a 32px top margin. At the desktop breakpoint, measured styles were 48px type, 48px line height, weight 600, and −1.2px tracking. Its supporting paragraph uses an 18px / 28px scale, the same 576px measure, and a 16px top margin.

Frame adopts that restrained heading scale (36px mobile, 48px desktop), weight, tracking, width, and paragraph spacing. The paragraph stays at 16px on mobile for the longer product description. The centered composition, native screenshot, actions, and lower format strip remain in place. This refinement was measured through browser DOM inspection; it is separate from the earlier REA analysis below.

## Frame styling

`src/lib/site/theme.css` carries the native application's Graphite tokens from `frame-app/assets/themes/theme.css`. Light and dark palettes retain all source values. Radii follow the native additive scale: 6, 10, 14, 18, 22, 26 and 30px. Layered shadows come from `frame-app/docs/ui-shadows/scale.css`; text fields use recessed shadows.

Do not combine a shadow with a border on the same surface: the shadow stack already creates the edge. Section separators and keyboard focus indicators are independent of this surface treatment.

Interface icons use Nucleo UI 1.9.0 Outline (`outline`, 18px grid), following the current website preference. The supplied archive is the same collection used by the native application; the website selects its Outline variant. Original geometry, stroke widths, caps, joins, and opacity are preserved, with black fills and strokes normalized to `currentColor`. Language, framework, and brand logos retain their own artwork. Source paths and integrity hashes are recorded in `src/lib/icons/manifest.json`.

The regular and medium Inter WOFF2 files are lossless conversions of Frame's bundled Inter 4.1 fonts at optical size 14. Additional Inter variable files supply web heading weights. Buttons retain the native 10px control radius, neutral foreground/background pair, layered depth, 2px focus ring and 1px press displacement. Website controls use larger text and touch targets where needed.

## Reverse engineering evidence

[Reverse Engineer Anything](https://github.com/morluto/rea) 6.8.0 was run through its public CLI with Node 24.22.2. The input consists of public HTML and its directly referenced JavaScript and CSS assets. The completed analysis selects three UI-bearing JavaScript chunks; all relevant captured files remain inventoried. This is static analysis of shipped assets. It does not recover the original component repository or establish runtime behavior.

The successful REA run parsed 113,344 bytes across three files with no syntax failures. It produced 154 application graph nodes and 168 edges. Its Evidence ID is `ev_65e0ac84a00b071ae77ebf09878d1da5bc99789902c71701bfe62ef2a766e444`. Semantic retention was limited to 600 nodes, so whole-application and semantic coverage remain partial. The largest UI/demo chunk reached the tool's timeout; both partial results were preserved before narrowing the selection.

Separate literal inspection found Radix context-menu, navigation-menu and accordion variables together with `data-slot` conventions. These support a shadcn-style component fingerprint; they do not establish an exact dependency version. No runtime calls or original source-map recovery were claimed.

The full local capture, SHA-256 manifest, REA evidence and summary are retained in `/tmp/frame-tailark-evidence/`. `design-reference.json` records the portable provenance and result summary. The downloaded reference source is kept outside this repository.
