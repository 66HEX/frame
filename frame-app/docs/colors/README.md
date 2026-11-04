# Frame colors

Frame uses exact Geist color values, arranged into semantic roles for its native
GPUI interface. The hierarchy follows shadcn's neutral theme; the recessed
controls, neumorphic separators, shadow scale and graphite primary gradient are Frame
choices. There is no additional 10% dark-mode dimming.

## Sources and scope

- [Geist colors](https://vercel.com/geist/colors), captured 2026-11-03.
- [Source snapshot](../../assets/themes/geist.json): original CSS expressions,
  resolved sRGB channels, available P3 expressions, source URL and SHA-256.
- [CSS reference](geist-source.css): the imported colors in light and dark modes.
- [Native source adapter](../../src/theme/geist.rs) and
  [semantic mapping](../../src/theme/mod.rs).

The snapshot includes 167 color variables per mode: all 92 modern scale entries
(two backgrounds and ten steps each of gray, gray-alpha, blue, red, amber, green,
teal, purple and pink), supporting tokens, legacy aliases and supplementary
colors from the official foundation stylesheet. Its 82 direct P3 definitions per
mode are preserved. GPUI currently consumes the official **sRGB fallback**, not
the P3 expressions. Fractional channels such as 4% are kept as floats; the hex
values below are rounded display references, not replacement source values.

Geist generally assigns steps 100–300 to component states, 400–600 to edges,
700–800 to strong fills and 900–1000 to text/icons. Solid gray is the default;
alpha gray is for intentional blending. Our dark cards occupy gray-100, so the
three neutral control states move one step up to 200/300/400. This is a deliberate
Frame role mapping, not a modification of the imported colors.

## What shadcn actually defaults to

Reference: [neutral theme](https://github.com/shadcn-ui/ui/blob/4aaea19d9cff3e0908e12a8c1d504b8c5ec34610/apps/v4/registry/themes.ts)
and [theme token documentation](https://ui.shadcn.com/docs/theming), inspected
2026-11-03. The current documentation uses the neutral base with the Nova style.
Other bases and older component styles can differ.

| shadcn neutral role | Light, approximate sRGB | Dark, approximate sRGB | Relationship |
| --- | --- | --- | --- |
| Background | `#ffffff` | `#0a0a0a` | Page foundation. |
| Card / popover | `#ffffff` | `#171717` | Same as page in light; lighter in dark. |
| Secondary / muted / accent | `#f5f5f5` | `#262626` | Darker than card in light; lighter in dark. |
| Primary | `#171717` | `#e5e5e5` | Inverts the interface's overall contrast. |
| Primary foreground | `#fafafa` | `#171717` | Paired with the action fill. |
| Ordinary foreground | `#0a0a0a` | `#fafafa` | Highest text emphasis. |
| Muted foreground | `#737373` | `#a3a3a3` | Secondary text, separate from disabled state. |
| Border / input | `#e5e5e5` | White at 10% / 15% | Edge tokens, not universal control backgrounds. |

Dark elevation therefore tends toward lighter surfaces. Light elevation does
not require each surface to get darker: a white card and white popover can share
the page color and still have different depth. Popover and card are equal in the
reference theme. Their separation comes from the component's edge and shadow.
The `accent` role in shadcn is a neutral interaction fill, not a blue brand color.

### Component behavior matters as much as the token table

The [Nova style](https://github.com/shadcn-ui/ui/blob/4aaea19d9cff3e0908e12a8c1d504b8c5ec34610/apps/v4/registry/styles/style-nova.css)
uses primary with an inverted foreground. Secondary hover mixes a little
foreground into secondary, so it gets darker in light and lighter in dark.
Ghost buttons acquire a neutral fill. Menus and popovers use the popover pair.
Inputs/selects are transparent in light and use a translucent input treatment in
dark. A checked checkbox switches to the primary pair. Slider rails use muted.

In particular, dark `input` is 15% white, while a control's `bg-input/30` applies
30% of that opacity: about 4.5% white over its actual parent. This is neither a
solid gray nor 30% white. It renders differently on the page and on a card.

The older [New York input](https://github.com/shadcn-ui/ui/blob/4aaea19d9cff3e0908e12a8c1d504b8c5ec34610/apps/v4/registry/new-york-v4/ui/input.tsx)
also starts transparent, uses the dark input overlay and adds its own shadow.
Its [button](https://github.com/shadcn-ui/ui/blob/4aaea19d9cff3e0908e12a8c1d504b8c5ec34610/apps/v4/registry/new-york-v4/ui/button.tsx)
uses opacity for primary and secondary hover; those rules differ from Nova's
secondary color mix. We adopt the tonal relationships, not every CSS recipe.

The [tab implementation](https://github.com/shadcn-ui/ui/blob/4aaea19d9cff3e0908e12a8c1d504b8c5ec34610/apps/v4/registry/new-york-v4/ui/tabs.tsx)
also demonstrates why selection and primary actions need separate roles: an
active tab has its own surface treatment rather than necessarily becoming a
high-contrast primary button.

## Frame surface hierarchy

Names below are native `ThemePalette` fields. Geist source names omit `--ds-`.

| Frame role | Light Geist token / approximate hex | Dark Geist token / approximate hex | Consumers |
| --- | --- | --- | --- |
| `background` | background-100 / `#ffffff` | background-100 / `#0a0a0a` | Window and workspace root. |
| `canvas` | background-200 / `#fafafa` | background-100 / `#0a0a0a` | Recessed input, checkbox, selection dot, slider/timeline/progress rail, Workspace/Logs navigation container, infinite canvas. |
| `surface` | background-100 / `#ffffff` | gray-100 / `#1a1a1a` | Main cards, settings and update dialogs. |
| `surface_elevated` | background-100 / `#ffffff` | gray-100 / `#1a1a1a` | Menus, color picker, floating preview toolbars, log scroll button. |
| `fill_subtle` | gray-100 / `#f2f2f2` | gray-200 / `#1f1f1f` | Secondary buttons, select triggers, subtle row/ghost hover. |
| `fill_selected` | gray-200 / `#ebebeb` | gray-300 / `#292929` | Secondary hover and ghost pressed state. |
| `fill_strong` | gray-300 / `#e6e6e6` | gray-400 / `#2e2e2e` | Selected tabs/choices, value badges, secondary pressed state. |

In dark: **background < card/popover < control < hover < selected** in brightness.
In light: **background = card/popover > control > hover > selected**.
Recessed controls sit below their parent card in brightness in both modes.

The input family keeps one opaque background in every state, including checked
checkboxes. Check/dash/dot marks and slider/progress fills use `text_primary` to communicate state.
This preserves the requested inset treatment instead of importing shadcn's
translucent input and inverted checked-checkbox fills. Inputs, checkboxes and
sliders still share the inset `xs` shadow. Infinite canvas keeps it too.

Main cards stay fully opaque. Popovers share their fill but use a stronger shadow.
The [shadow scale](../ui-shadows/README.md) supplies each surface's edge and
highlight; do not add shadcn's decorative border/ring on top. Neumorphic
separators retain their two-tone structure and 1 logical px edges on every OS.

### Native reference views

[Dark workspace](workspace-dark.png) · [Light workspace](workspace-light.png)

Captured with the GPUI Metal renderer at 100% UI scale. These fixtures omit media
decoding so the preview shows the canvas surface itself.

## Text, icons and semantic colors

| Role | Geist source in both themes | Rule |
| --- | --- | --- |
| `text_primary` | gray-1000 | Labels, values, titles and active icons. |
| `text_muted` | gray-900 | Descriptions, placeholders, inactive icons, version and log line numbers. |
| `text_disabled` | gray-700 | Disabled button labels/icons; never substitute for ordinary descriptions. |
| `control_muted` | gray-700 | Non-text control marks. |
| `accent` | blue-900 | Informational status and link emphasis; distinct from shadcn's neutral accent. |
| `danger` / `danger_surface` | red-900 / red-100 | Destructive icons, errors and error panel fill. |
| `warning` | amber-900 | Warning text and status. |
| `success` | green-900 | Available for success status; existing state meanings stay unchanged. |
| `focus_ring` | ds-focus-color | Blue-700 light / blue-900 dark, fully opaque keyboard focus. |
| `text_selection` | blue-200 | Input selection with normal foreground text. |
| `tooltip_surface` / `tooltip_foreground` | gray-1000 / background-100 | Inverted tooltip pair. |
| `border_subtle` | gray-400 | Functional edges/separator shading, never body text. |
| `highlight_subtle` | gray-alpha-100 | The separator's blending layer. |
| `overlay_backdrop` | ds-overlay-backdrop-color | Modal/import scrim, with existing animated coverage opacity. |

Use modern green for success. The imported legacy `geist-success` alias is blue
and is not Frame's success role. Teal, purple and pink remain available in the
source palette; introducing one requires a named semantic purpose.

## Primary gradient

Only `ButtonVariant::Primary` uses this gradient. Selected tabs, choices and
toolbar toggles stay neutral, with ordinary text. The shared button adapter
renders a 180-degree GPUI gradient, equivalent to top-to-bottom CSS, and animates
both stops on hover. Pressing keeps the gradient instead of replacing it with a
flat fill. Normal, hover and pressed states all have a brighter top.

| State | Light top → bottom | Dark top → bottom |
| --- | --- | --- |
| Normal (`primary` → `primary_to`) | accents-6 → gray-1000 (`#444444` → `#171717`) | gray-500 → gray-400 (`#454545` → `#2e2e2e`) |
| Hover | accents-5 → accents-6 (`#666666` → `#444444`) | accents-4 → gray-500 (`#666666` → `#454545`) |
| Pressed | accents-7 → gray-1000 (`#333333` → `#171717`) | gray-400 → gray-200 (`#2e2e2e` → `#1f1f1f`) |

The foreground is `ds-contrast-fg`: white in both modes. This graphite treatment
follows the supplied button reference and deliberately replaces shadcn's inverted
white primary in dark mode. A darker fill keeps the existing `CONTROL` / `xs`
white inset edge and top highlight visible. The same eight shadow layers provide
the outer depth, 1 px edge and highlight; no extra border or highlight is added.
All primary builders (titlebar, welcome, update dialog and compact Apply buttons)
use this shared gradient and shadow treatment.

Disabled primary buttons use the ordinary flat disabled treatment, without a
gradient or hover/pressed color changes. All stops remain exact Geist source
tokens. The gradient is a Frame addition; shadcn's default primary is flat.

## Rules for changes

1. Keep the official snapshot unchanged when adjusting hierarchy. Change the
   source-token assignment in `ThemePalette::resolve` instead.
2. Components consume semantic fields, never raw hex, source lookup strings or
   arbitrary alpha variants of a color. Add a semantic field if the role is new.
3. Pair foreground and background roles. A primary/tooltip fill must bring its
   matching foreground; ordinary `text_primary` is not suitable for both.
4. Keep source colors separate from effects. Hover interpolation, disabled-group
   opacity, animated scrims and the approved translucent shadow stacks are
   intentional effects. They do not redefine the palette.
5. Media pixels, user-selected subtitle colors, HSV picker values and crop
   geometry markers represent content rather than interface palette choices.
   The imported shadow ink `#030302` also stays as supplied in the shadow scale.

## Validation

Theme tests check the hierarchy in both modes, opaque neutral surfaces, readable
text/status/tooltip/selection pairs, focus/control contrast and all primary
stops. Button tests verify gradient direction through hover and press and keep
selection/disabled treatments separate. Source tests require every scale step,
valid channels and the preserved P3 definitions. Native previews cover both modes.

```sh
cargo test --locked -p frame-app
cargo test --locked -p xtask theme_color_audit_accepts_current_source_inventory
cargo clippy --locked -p frame-app --all-targets -- -D warnings
cargo fmt --all -- --check
```

Validated 2026-11-03: 793 application tests passed (8 ignored); the color audit,
Clippy and formatting passed. Native Metal previews covered the video workspace,
open crop select, app settings with an open theme select, and update dialog in
both modes. Windows and Linux were not run natively.
