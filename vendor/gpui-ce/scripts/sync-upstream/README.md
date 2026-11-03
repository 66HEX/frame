# Upstream sync

Automated, local-only tooling to pull GPUI changes from the upstream Zed monorepo
(`zed-industries/zed`) into this standalone fork. Conflicts and resulting build
breakage are resolved with `claude -p`. **Nothing is ever pushed.**

## TL;DR

```sh
just sync-upstream-bootstrap   # ONE TIME — records the baseline to sync from
just sync-upstream             # pull upstream changes onto a fresh sync/ branch
just sync-upstream-status      # how far behind upstream are we?
```

`just sync-upstream` leaves a `sync/zed-<date>-<sha>` branch with the upstream delta
merged, conflicts resolved, and the workspace compiling (or a clear report of what's
left). Review it, then fast-forward `main` onto it or open a PR — by hand.

## How it works

Upstream keeps the GPUI crates **in-tree** in the monorepo under `crates/gpui*`. This
fork keeps the same crates at the **same relative paths**, so upstream changes apply at
identical paths here. The script uses a **vendor-branch 3-way merge** (a generalized
`git subtree` merge):

1. A local branch `vendor/zed-gpui` holds a *filtered replay* of upstream's actual
   gpui-touching commits — each keeps its original author/date/message (plus a
   `zed-upstream: <sha>` trailer) but carries only the tracked `crates/gpui*` trees,
   built via a throwaway git index (no working-tree churn). Non-gpui and merge commits
   are dropped.
2. Each sync extends that chain with the new commits and `git merge`s its tip. Git's
   merge base is the previous tip, so the merge replays exactly the upstream delta since
   the last sync — anything this fork already cherry-picked upstream produces **no
   conflict** — and every upstream commit is preserved in the merge's **second-parent
   history** (so `git log` shows both histories; `git log --first-parent` shows just the
   -ce line).
3. The merge is committed as **two commits** for reviewability:
   - **Commit 1 — raw merge:** git's auto-merges applied, conflict markers committed in
     as-is (deterministic add/delete conflicts settled by policy: gpui-ce's deletions
     kept). This captures exactly what git could *not* resolve.
   - **Commit 2 — resolution:** `claude -p` (`resolve-conflicts.prompt.md`, looped up to
     `--retries`) edits out the markers. Because it's a separate commit, its diff shows
     *exactly* what was chosen — auditable in isolation, distinct from git's auto-merge.
   (A conflict-free sync is just one clean merge commit.)
4. The **verify-fix loop** runs: the build gate (`just check`,
   which also treats compile **warnings** as fixable so the branch stays CI-clean), then
   the test gate (`just test`). (There are no `zed-industries/zed` git deps left to bump — PR #91
   vendored everything in-tree, so the old dep-bump step is now a no-op.) Any failure — compile
   errors, warnings, or test failures —
   is handed to `claude -p` (`fix-build.prompt.md`), looped up to `--retries` times, and
   committed as a **third** commit. Disable tests with `SYNC_RUN_TESTS=0`, or warning
   enforcement with `SYNC_FAIL_ON_WARNINGS=0`.

### Tracked crates (fork dir ← upstream dir; dirs, not package names)

Synced 1:1 (same dir): `gpui`, `gpui_linux`, `gpui_macos`, `gpui_macros`, `gpui_platform`,
`gpui_shared_string`, `gpui_tokio`, `gpui_web`, `gpui_wgpu`, `gpui_windows`.

Synced with **path remapping** — vendored + renamed by the fork (PR #91 removed the git sources):
`gpui_collections`←`collections`, `gpui_sum_tree`←`sum_tree`, `gpui_refineable`←`refineable`,
`gpui_derive_refineable`←`refineable/derive_refineable`, `gpui_scheduler`←`scheduler`,
`gpui_media`←`media`, `gpui_zed_util`←`util`, `gpui_ce_util`←`gpui_util`, `gpui_path`←`path`.
Packaging is orthogonal to directories: every fork package is `gpui_ce_*` (main crate `gpui-ce`
with a hyphen), each with `[lib] name` preserving the upstream crate name and a
`[workspace.dependencies]` alias mapping the upstream name back (e.g.
`collections = { package = "gpui_ce_collections" }`, `gpui = { package = "gpui-ce" }`).
The merge preserves each
crate's gpui-ce adaptations (package rename + lib-name override, path deps, `ztracing`→`tracing`,
`zlog` removal) via
conflict resolution while taking upstream's real changes — so upstream API additions land through the
merge instead of being hand-ported during the build-fix pass.

Left untouched: `crates/gpui_elements` (fork-only, package `gpui_ce_elements`),
`tooling/perf` (fork-only);
`util_macros`
is no longer used by the fork. The mapping lives in `TRACKED_CRATES` in `sync_upstream.py`.

### Cross-crate file moves

Upstream relocates code between crates (e.g. #61029 split `util/src/rel_path.rs` out into the new
`crates/path`). Naively that reads as "upstream deleted a file the fork had modified", and the
delete/modify policy would resurrect a stale duplicate of code that now lives elsewhere.

Because **both sides of a vendor-history diff are already remapped to fork paths**, git's rename
detection reports such a move directly in gpui-ce terms — `detect_moves()` runs
`git diff -M<similarity> --diff-filter=R <prev vendor tip> <new tip>` and keeps the cross-directory
hits (`crates/gpui_zed_util/src/rel_path.rs` → `crates/gpui_path/src/rel_path.rs`, detected at 61%
similarity for #61029). Those moves then:

- settle the resulting `UD` conflict as **accept the deletion** (the content arrived at the new path
  in the same merge) instead of keeping the fork's now-orphaned copy, and
- get handed to the resolution prompt (rule 8), which re-applies the fork's adaptations at the **new**
  location and repoints `use`/`mod` references.

Tune with `SYNC_MOVE_SIMILARITY` (default `40%`). A move that falls below the threshold simply
degrades to the old behaviour — the fork's copy is kept and flagged for review, never lost.

## One-time bootstrap

The script needs to know which upstream commit this fork currently corresponds to, to use as the
merge base. Historically this was auto-read from the `zed-industries/zed` rev pinned in `Cargo.toml`,
but PR #91 removed those git sources, so there is no longer a pinned rev to default to — **pass the
baseline explicitly**:

```sh
just sync-upstream-bootstrap <upstream-sha>   # e.g. 876ec5a8a074 (the last rev pinned before #91)
```

Bootstrap adds the `zed` remote, builds the baseline vendor snapshot (all tracked crates remapped to
their fork paths at that rev), records it in the current branch's history with a **no-op** `-s ours`
merge (no files change), and writes `state.json`. Run it once. A too-old baseline just means the
first real sync has more to merge; correctness is unaffected. After the first successful sync the
recorded baseline advances automatically, so bootstrap is not needed again.

## Files

| File | Purpose |
|------|---------|
| `sync_upstream.py` | orchestrator (git plumbing + `claude -p` loops); stdlib-only, fully typed |
| `resolve-conflicts.prompt.md` | rules for the conflict-resolution `claude -p` pass |
| `fix-build.prompt.md` | rules for the build-fix `claude -p` pass |
| `state.json` | committed: last synced upstream sha + vendor tip |

## Config / env overrides

Every default near the top of `sync_upstream.py` is overridable via a `SYNC_*` env var:

```sh
SYNC_MODEL=sonnet just sync-upstream          # cheaper model
SYNC_RETRIES=5    just sync-upstream           # more claude passes
SYNC_VERIFY_CMD="just ci-test" just sync-upstream
just sync-upstream --ref some-tag --no-bump --dry-run
```

## Caveats

- The compile gate is host-only (`just check` / `cargo check --workspace`). macOS- and
  Windows-specific changes can't be fully verified on a Linux host — verify those on the
  platform or in CI. The build-fix prompt asks claude to flag such changes.
- Requires Python 3 (stdlib only — no pip installs), the `claude` CLI on `PATH`, and a
  working `just` + Rust toolchain. The `just` recipes shell out to `sync_upstream.py`.
- If conflict resolution exhausts its retries, the branch is left mid-merge for you to
  finish. If the build can't be fixed in time, the merge is committed and the branch is
  left with the remaining errors plus a clear report.
- `--allowedTools` is passed as a single space-separated string; if your `claude` CLI
  version expects a different format, adjust `SYNC_CLAUDE_ALLOWED_TOOLS`.
