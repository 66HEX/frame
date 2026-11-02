# Future work

Verified against Parley 0.11.1, upstream `main`, and PR #766 on 2026-09-15.

## Vertical alignment

GPUI handles `baseline`, `middle`, `top`, `bottom`, and their line metrics itself.

- Parley 0.11.1 supports bottom-to-baseline alignment only.
- [PR #639](https://github.com/linebender/parley/pull/639) added custom box baselines and matching line-height calculation to `main`, but not the other modes.
- [PR #766](https://github.com/linebender/parley/pull/766) implements and tests all four modes for spans and inline boxes, including line-height calculation. It is open and unmerged.

Keep `align_inline_boxes` until #766 ships and passes GPUI's inline-layout tests, especially `middle` alignment and line metrics.

## Whitespace at wrapped line starts

Extend GPUI's existing whitespace behavior once Parley implements the necessary support tracked in [issue #619](https://github.com/linebender/parley/issues/619).

## Out-of-flow boxes

[`InlineBoxKind::OutOfFlow`](https://docs.rs/parley/0.11.1/parley/enum.InlineBoxKind.html#variant.OutOfFlow) already exists in 0.11.1. It can provide an absolute inline child's static position without affecting text flow. Taffy must still handle sizing, insets, and final placement.
