# Frame colors

The active design system is [shadcn Mira Neutral](../design-system/README.md). Edit `../../assets/themes/theme.css` by pasting shadcn `:root` and `.dark` blocks. Cargo compiles these tokens automatically without 8-bit rounding; no JSON export is required.

`ThemePalette::token` exposes original shadcn names. Native aliases in `src/theme/mod.rs` serve existing domain code. Alpha modifiers multiply the original alpha. The secondary hover mix is evaluated in OKLCH.
