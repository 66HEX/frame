# Noto Color Emoji subset

`NotoColorEmoji.subset.ttf` comes from Noto Color Emoji 2.051 at
[`e92753bfa55fd449e427d4d325f9c8c40408c74e`](https://github.com/googlefonts/noto-emoji/commit/e92753bfa55fd449e427d4d325f9c8c40408c74e).
The source font has SHA-256
`72a635cb3d2f3524c51620cdde406b217204e8a6a06c6a096ff8ed4b5fd6e27b`.
The generated subset has SHA-256
`6859dbc2eafe747cd19617650ddaa3e6cdcea01fad60254322949ace4f325231`.

Install FontTools, then run these commands from the repository root:

```sh
curl -L https://raw.githubusercontent.com/googlefonts/noto-emoji/e92753bfa55fd449e427d4d325f9c8c40408c74e/fonts/NotoColorEmoji.ttf \
  -o /tmp/NotoColorEmoji.2.051.ttf
pyftsubset /tmp/NotoColorEmoji.2.051.ttf \
  --output-file=assets/fonts/noto-color-emoji/NotoColorEmoji.subset.ttf \
  --unicodes=U+0020,U+0031,U+200D,U+20E3,U+2728,U+FE0F,U+1F1E7,U+1F1EC,U+1F389,U+1F3FD,U+1F466-1F469,U+1F4A1,U+1F4BB,U+1F525,U+1F600,U+1F680 \
  --layout-features='*'
```

The Unicode list covers the text-system sample and the joined, modified,
regional-indicator, and keycap sequences used by rendering fixtures.
