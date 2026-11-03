## Repository context

`gpui-ce` is a standalone fork of Zed's GPUI. A 3-way merge of upstream Zed GPUI changes was just
committed. (There are no `zed-industries/zed` git deps left to bump — PR #91 vendored everything
in-tree.) The result has a problem the sync introduced — a compile error, a compile warning,
or a test failure (see the output above). Fix it so the gate passes.

## Rules

1. **Fix only what the merge/sync caused.** Address the issues in the output: items moved/renamed
   upstream, changed function signatures or trait bounds, added/removed enum variants, and any
   fallout in gpui-ce's own patches. NOTE: the util crates (`collections`, `util`, `gpui_util`,
   `sum_tree`, `refineable`, `derive_refineable`, `scheduler`, `media`, `path`) are now
   **vendored in-tree** as dirs `crates/gpui_collections`, `crates/gpui_zed_util`,
   `crates/gpui_ce_util`, `crates/gpui_sum_tree`, `crates/gpui_refineable`,
   `crates/gpui_derive_refineable`, `crates/gpui_scheduler`, `crates/gpui_media`,
   `crates/gpui_path` (packages `gpui_ce_*`, `[lib] name` = upstream name so `use` sites are
   unchanged) and are **synced by this same tool** — so if gpui needs a new API from one of them,
   it should already be present from the merge. Prefer using that API; only hand-add to a vendored
   crate if the merge genuinely didn't bring it (and say so in your summary).

2. **Compile warnings.** Fix every compile warning the merge introduced (unused imports/variables,
   unreachable code, deprecated APIs, etc.) by addressing the **root cause** — the synced branch
   must be warning-clean to pass CI. Do **not** silence warnings with `#[allow(...)]`, `_`-prefixes,
   or `#[allow(dead_code)]` unless that is genuinely the correct fix.

3. **Test failures.** Fix the underlying cause. Do **not** delete tests, add `#[ignore]`, weaken or
   delete assertions, or otherwise change a test just to make it pass. If an upstream change
   legitimately changes behavior, update the test to match upstream's intent — and call that out in
   your summary. Note that some tests may fail for environmental reasons (e.g. no display); flag
   those rather than "fixing" them.

4. **Prefer minimal, idiomatic changes** consistent with how upstream intends the new API to be
   used, and matching the surrounding gpui-ce code style. Preserve gpui-ce's existing features
   (blur, kinetic scrolling, wgpu device-loss API, etc.); if an upstream API change requires
   updating a gpui-ce patch, update the patch correctly.

5. **Do not** edit `tooling/perf` or `crates/gpui_elements` unless one of them is the actual source
   of an issue. Do not run `git commit`, `git merge`, or `git push` (the surrounding script commits
   and re-runs the gate). You may run `cargo check` / `cargo build` / `cargo test` to verify. If you
   need scratch space, use `/tmp` — never write scratch files into the working tree (they would be
   committed).

6. If an issue stems from the **root `Cargo.toml`** (a workspace dependency that must be added or
   updated to match upstream's new requirements — the sync merges crate trees but not the root
   manifest, so new `[workspace.dependencies]` entries upstream added often need adding here), fix it
   there using gpui-ce's sourcing convention: **path deps via workspace aliases** (e.g.
   `collections = { path = "crates/gpui_collections", package = "gpui_ce_collections" }`,
   `gpui = { path = "crates/gpui", package = "gpui-ce" }`) for the vendored/in-tree crates,
   `zed-font-kit` for font-kit, and crates.io versions otherwise. There are no longer any
   `zed-industries/zed` git deps.

7. **Newly vendored crates need fork packaging applied.** When upstream adds a crate that this tool
   tracks, it arrives as a *clean add* — no conflict, so no resolution pass adapted it, and it still
   carries upstream's packaging verbatim. If a tracked crate directory is new in this merge, bring it
   in line with its siblings before anything else: set `name` to the fork's `gpui_ce_*` name
   (`gpui-ce` for the main crate; see `crates/gpui_path` / `crates/gpui_zed_util` for the current
   template, keeping `[lib] name` as the upstream crate name so `use` sites are unchanged), match
   the siblings' `version`/`edition`/`publish`/`description`/`repository`, convert workspace/git deps to gpui-ce's
   sourcing convention (rule 6), add the crate to the root `Cargo.toml` members, and **set
   `license = "Apache-2.0"`** — gpui-ce is Apache-only, and an upstream manifest may declare another
   license (or contradict its own bundled license file). Also copy a sibling's `LICENSE-APACHE` file
   into the new crate dir; upstream ships that as a symlink to a root file gpui-ce doesn't have, so
   the link would dangle. Call out in your summary anything whose license looks non-Apache, and do
   NOT silently vendor code that genuinely is — flag it for a human instead.

When finished, briefly summarize the fixes and anything a human should double-check (especially
changes to macOS/Windows-only code that this host can't fully compile, and any tests you judged to
be failing for environmental rather than correctness reasons).
