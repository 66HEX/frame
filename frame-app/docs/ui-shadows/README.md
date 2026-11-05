# Mira shadows

[scale.css](scale.css) records the Tailwind v4 shadow scale used by the pinned Mira components. Native `theme/shadows.rs` preserves layer order, opacity, offset and spread; CSS blur is converted to GPUI Gaussian sigma by dividing by two. Tests compare every layer against the CSS.

| Surface | Shadow | Edge |
| --- | --- | --- |
| Button, Input, Checkbox, Slider, Tabs | none | Component border/ring |
| Card | none | foreground/10 ring, 1 px |
| Popover and floating toolbar | md | foreground/10 ring, 1 px |
| Tooltip | none | none |
| Dialog | none | foreground/10 ring, 1 px |
| Sheet | lg | border-left, 1 px |

Borders, elevation, and focus rings use separate primitives. Mira controls have no inset shadows or embossed separators.

Component assignments follow [the Mira component snapshot](../design-system/reference).
