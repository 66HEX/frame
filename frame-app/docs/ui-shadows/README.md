# Graphite shadows

[scale.css](scale.css) records the supplied Sebastiano Guerriero reference scale,
from `2xs` to `2xl`. Each level combines depth with its own painted edge and
white inset highlights. The dark layers use `#030302`; the alpha bytes match
the reference.

The native adapter in [theme/shadows.rs](../../src/theme/shadows.rs) preserves
color, alpha, offset and spread. It reverses the CSS list for GPUI's paint order
and divides CSS blur by two to obtain Gaussian sigma. The existing adapter tests
compare every layer with the CSS, including the inset flag. All shadow dimensions
are logical pixels and do not change layout or control spacing.

| Surface | Treatment |
| --- | --- |
| Card | `sm` |
| Raised button and selected tab | `xs` |
| Select trigger, selected list item and checked checkbox | `2xs` |
| Slider thumb | `xs`; the rail stays flat |
| Popover and floating toolbar | `md` |
| Dialog | `xl` |
| Settings sheet | `lg` |
| Text input, unchecked checkbox and tabs track | Recessed inset shadows, adjusted for light/dark |
| Pressed raised button | Recessed inset shadows |
| Badge, unselected ghost button and quiet list content | Flat fill, no depth shadow |

`surface_fill` adds a small amount of white at the top of static raised surfaces.
Controls whose backgrounds animate retain solid fills because GPUI currently
interpolates only solid backgrounds. Their shadows provide the relief.

Decorative borders retain their existing width but become transparent. This keeps
the content box and spacing unchanged while the shadow supplies the visible edge.
Focus rings and invalid-state outlines remain separate semantic cues. Structural
separators keep their existing one-pixel geometry and use the theme border color.

Closely nested controls use the parent's radius minus its existing inset. The
toolbar and tabs radii follow this relationship without changing their padding.
