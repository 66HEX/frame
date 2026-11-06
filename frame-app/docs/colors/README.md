# Frame colors

The active theme is [Frame Graphite](../design-system/README.md), with Guerriero-inspired surfaces on the existing Mira component geometry. Edit `../../assets/themes/theme.css` by pasting shadcn `:root` and `.dark` blocks. Cargo compiles these tokens automatically without 8-bit rounding; no JSON export is required.

`ThemePalette::token` exposes original shadcn names. Native aliases in `src/theme/mod.rs` serve existing domain code. Alpha modifiers multiply the original alpha. The secondary hover mix is evaluated in OKLCH.

The optional `--success` token supplies the green Applied label on preset rows, with separate light and dark values. Themes without this extension fall back to `--foreground`.
