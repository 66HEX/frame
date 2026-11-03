# Direction and bidirectional text

GPUI supports horizontal LTR and RTL layout through element styles. Independent
roots default to LTR.

```rust
div()
    .rtl()
    .text_start()
    .child("مرحبا")
    .child(div().ltr().child("English content"))
```

`LayoutDirection::Inherit` uses the logical parent's direction. `ltr()` and `rtl()`
create HTML-style directional boundaries. `LayoutDirection::Auto` uses the first
strong character in eligible descendant text, excluding subtrees with their own
direction, and falls back to LTR. Editable text examines its value, not its
placeholder.

## Alignment

`text_start()` and `text_end()` follow the line direction. `text_left()`,
`text_center()`, and `text_right()` remain physical. Taffy applies direction to
flex, block, and grid layout without changing source, focus, or accessibility
order.

`items_start()`, `items_end()`, `content_start()`, and `content_end()` remain
flex-relative. `justify_start()` and `justify_end()` are logical; use
`justify_flex_start()` and `justify_flex_end()` for flex-relative placement.

## Bidirectional scopes

`unicode_bidi()` supports `Normal`, `Embed`, `Isolate`, `BidiOverride`,
`IsolateOverride`, and `Plaintext`, and does not inherit. Explicit direction and
`Auto` default to isolation unless overridden. Caret, selection, and hit-test
positions retain their original UTF-8 offsets.

Custom measured elements can read `Window::resolved_direction()`. Elements that
own source text should call `Window::set_layout_direction_text()` after
requesting their layout ID.

Tests follow Web Platform Test coverage for
[`dir=auto`](https://github.com/web-platform-tests/wpt/blob/master/html/dom/elements/global-attributes/dir-assorted.window.js),
[`text-align: start`](https://github.com/web-platform-tests/wpt/blob/master/css/css-text/text-align/text-align-start-009.html),
[flex alignment](https://github.com/web-platform-tests/wpt/blob/master/css/css-flexbox/flexbox_justifycontent-start-rtl.html),
and [`plaintext` scrolling](https://github.com/web-platform-tests/wpt/blob/master/css/css-overflow/unicode-bidi-plaintext-scroll-direction.html).

## Limits

- Horizontal writing only; vertical writing modes are unsupported.
- Logical margin, padding, border, and inset properties are unavailable.
- GPUI has no HTML shadow-tree, table, or specialized form-control direction rules.
- Content without source text does not affect automatic direction.

Run `cargo run -p gpui-ce --example direction` for an interactive demo.
