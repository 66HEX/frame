# Frame website design rules

- Follow the native Frame Graphite tokens in `src/lib/site/theme.css`.
- Use shadcn component variants and sizes without overriding their dimensions, padding, typography, or built-in surface styling. Put page layout spacing on surrounding containers.
- Use semantic shadcn color tokens such as `background`, `foreground`, `muted-foreground`, and `border`, rather than literal colors. The landing page follows the shared theme preference, including native screenshot variants.
- Prefer the standard Tailwind spacing, type, container, radius, and breakpoint scales. Use grid tracks and fractions for composition. Reserve custom values for measured source-image crop geometry or actual illustration data, and document their purpose rather than moving magic numbers into CSS variables.
- When a surface uses a shadow, do not add a border to that same surface. The layered shadow already supplies its edge. Keep independent section separators and accessible focus indicators where needed.
- Show the real Frame application with screenshots rather than recreating its interface in HTML/CSS.
- Use Nucleo UI Outline (`outline`, 18px grid) for interface icons, following the current website preference. Preserve the supplied geometry, stroke widths, caps, joins, and opacity; normalize black fills and strokes to `currentColor`. Language, framework, and brand logos keep their own artwork. Do not use brand logos as generic interface symbols.
- Keep the footer credit “Built by Marek Jóźwiak” linked to `https://madebyhex.com/`.
