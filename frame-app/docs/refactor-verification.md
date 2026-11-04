# Structural refactor verification — 2026-11-04

Reference commit: `4af1f0076790fd2b8045aafca1996fd28e81f053`.
The refactor changes source organization, imports and internal visibility. It
retains the public crate entry points, configuration fields, defaults, assets,
theme tokens, element identifiers and runtime logic.

## Native rendering

All **100 before/after PNG pairs are byte-identical**. The matrix covers every
one of the 25 named visual fixtures in light and dark themes at 100% and 200%
interface scale. Captures were made on the same macOS machine using the native
GPUI Metal renderer, with a 1200 × 800 logical-pixel window and 2400 × 1600 PNGs.

The original application was built from an archived copy of the reference commit.
Both versions used the same `render-snapshot` harness, dependencies, fonts, fixed
pointer position, reduced-motion setting and three synchronous draw passes.
The harness uses `FrameRoot::new()` and does not persist settings or advance
asynchronous media tasks during capture.

An initial comparison exposed pointer-dependent row-action visibility. Fixing
the simulated pointer in the harness and recapturing both versions removed that
environmental difference; application hover behavior was not changed.

[Snapshot SHA-256 manifest](refactor-snapshots.json) records the common hash of
each matching pair. [Capture instructions](../../CONTRIBUTING_VISUAL_FIXTURES.md)
describe how to repeat an individual scene.

## Source contracts

A Rust AST comparison accounted for the original 3392 function/method bodies,
constants, statics, structs and enums. Declaration order within structs, enum
variants and constant expressions was retained. The comparison also checked
2950 signatures, semantic attributes, type aliases and item macros; differences
in import paths and formatter-inserted trailing commas were normalized.

The main render method is the only production body split into new methods:
preview focus registration, subtitle focus registration and overlay attachment.
Inlining those three extracted blocks reconstructs its original statement order.
The extraction adds four private structs carrying existing focus handles and
borrowed render inputs. Other body differences are module qualification and
formatting. No conversion or preview scheduling logic was rewritten.

The largest Rust source file decreased from 4459 to 613 lines. `app/mod.rs`
decreased from 932 to 59 lines, and root rendering is organized under
`app/render/`. The [architecture guide](architecture.md) records the ownership
rules and old-to-new paths.

## Checks and limits

- Application tests: 801 passed, 9 intentionally ignored, matching the reference.
- Four real FFmpeg preview-session tests passed when explicitly enabled.
- Theme color audit: all four audit tests passed after updating the approved
  crop-overlay source path.
- `cargo clippy --locked -p frame-app --all-targets -- -D warnings` passed.
- Formatting and whitespace checks passed.

Byte identity here means the rendered PNG output for the tested states. It does
not mean identical executable files: module paths and build metadata can change
binary bytes. Static snapshots do not prove every interactive state or every
platform; existing interaction, accessibility, geometry, persistence and media
tests provide complementary coverage. Native Windows and Linux builds were not
run in this macOS environment; platform-specific imports and unchanged rendering
bodies were reviewed separately.
