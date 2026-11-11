# Landing page

The root route renders `LandingPage.svelte`. Change section order in that file.

- `layout/` owns the header, footer, brand, and page surface. `LandingLayout.svelte` assembles the shared homepage and error-page shell; `LandingSurface.svelte` scopes its theme, focus styles, and reduced-motion behavior.
- `shared/` contains the container, section headings, text links, feature links, and screenshot renderer used across sections.
- `sections/` groups each section with its screenshots or illustrations. Edit the section entry below for its copy and composition.
- `data/content.ts` contains navigation, release links, platform downloads, and FAQ answers.

| Section | Entry file |
| --- | --- |
| Hero | `sections/hero/HeroSection.svelte` |
| Media features | `sections/features/FeaturesSection.svelte` |
| Preview | `sections/preview/PreviewSection.svelte` |
| Batch workflow | `sections/workflow/WorkflowSection.svelte` |
| Output details | `sections/details/DetailsSection.svelte` |
| Local processing | `sections/local/LocalSection.svelte` |
| Downloads | `sections/download/DownloadSection.svelte` |
| FAQ | `sections/faq/FaqSection.svelte` |

Layout, spacing, typography, and responsive rules use Tailwind utilities. Shadcn controls keep their built-in variants and sizes; place layout spacing on surrounding containers instead of overriding control padding or dimensions. Use semantic theme colors for surfaces, text, and separators.

Use standard Tailwind scales for spacing, typography, containers, radii, and breakpoints. Compose layouts with grid tracks, aspect ratios, and fractions instead of one-off pixel values or custom properties. Arbitrary selector variants in the page surface and brand wrapper scope behavior; they do not introduce custom design values.

The header reuses the shared `ThemeToggle`. The landing page and error page inherit the root theme preference, just like the documentation. Pass both `src` (dark) and `lightSrc` to `AppScreenshot` for native captures; CSS selects the matching image before hydration.

Two small CSS rules remain:

- `layout/LandingSurface.svelte` disables transitions and animations for reduced motion, including descendants and pseudo-elements.
- `sections/features/illustrations/TimeRuler.svelte` draws the repeating one-pixel ruler marks at sixteenth intervals.

`shared/AppScreenshot.svelte` calculates aspect ratio, radius, image width, and translation from source-pixel crop data. Set crop bounds inside the native panel edge to remove its baked border and shadow. A separate `shadow-lg` layer above the bitmap supplies the edge, highlight, and drop shadow; do not add a border. The padded mask fades the image and shadow together while preserving the composition's dimensions. Pass standard layout utilities through its `class` prop, such as `max-h-72` in `OutputDetails.svelte`. The hero uses `framed={false}` because it owns its window shadow and mask. The audio illustration also uses a dynamic height for each waveform sample. Crop coordinates, waveform samples, and ruler marks are the intentional exceptions to the standard scales.

Keep landing-specific changes here. Shared UI components, site theme tokens, and application documentation have separate owners.
