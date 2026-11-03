## Repository context

`gpui-ce` is a standalone community fork of Zed's GPUI. It vendors crates from the upstream Zed
monorepo (`zed-industries/zed`). Two groups:

**Same relative path** (upstream dir == fork dir; dirs, not package names — packages are
`gpui-ce` / `gpui_ce_*`): `crates/gpui`, `crates/gpui_linux`,
`crates/gpui_macos`, `crates/gpui_macros`, `crates/gpui_platform`, `crates/gpui_shared_string`,
`crates/gpui_tokio`, `crates/gpui_web`, `crates/gpui_wgpu`, `crates/gpui_windows`.

**Vendored + renamed** (upstream dir → fork dir; dirs, not package names), formerly pulled as
`zed-industries/zed` git deps but now vendored in-tree by the fork:
`collections`→`gpui_collections`, `sum_tree`→`gpui_sum_tree`, `refineable`→`gpui_refineable`,
`refineable/derive_refineable`→`gpui_derive_refineable`, `scheduler`→`gpui_scheduler`,
`media`→`gpui_media`, `util`→`gpui_zed_util`, `gpui_util`→`gpui_ce_util`, `path`→`gpui_path`.
(Packaging is orthogonal: every fork package is `gpui_ce_*` — main crate `gpui-ce` with a hyphen —
with `[lib] name` preserving the upstream crate name so `use` sites are unchanged.) The sync remaps upstream's
content into these fork dirs, so a conflict here is upstream's version of the crate vs. the fork's
vendored+adapted version. Preserve the fork's adaptations (see rule 4) while taking upstream's real
changes. (`util_macros` is no longer used by the fork; `gpui_elements` and `tooling/perf` are
fork-only and never synced.)

A 3-way `git merge` of the upstream delta produced conflicts, and the raw merge — with conflict
markers committed in — is already its own commit. Your job is to resolve **every** marker in the
listed files so the result is correct gpui-ce code that incorporates the upstream changes. Your
edits land as a **separate, reviewable resolution commit** diffed against that raw merge.

## Rules

1. **Resolve every conflict marker** (`<<<<<<<`, `=======`, `>>>>>>>`, `|||||||`) in the listed
   files. Leave no markers behind. Do not touch files that aren't conflicted.

2. **gpui-ce keeps its own patches.** gpui-ce carries features/fixes not yet upstream (e.g. blur
   filters, kinetic scrolling on Wayland, the wgpu device-loss API). When a conflict pits an
   upstream change against a gpui-ce patch, **keep both behaviours** — integrate the upstream
   change around gpui-ce's additions rather than dropping either. Only drop a gpui-ce line if the
   upstream change genuinely supersedes it.

3. **Already-present (cherry-picked) changes.** gpui-ce frequently contributes to and cherry-picks
   from upstream, so an upstream commit may already be present here under a different hash. If a
   conflict exists *only* because the change is **already applied** in gpui-ce (semantically
   equivalent, even if worded differently), keep gpui-ce's version and do **not** duplicate the code.

4. **Vendored crate adaptations — preserve them.** The vendored+renamed crates carry mechanical
   gpui-ce adaptations on top of upstream. When a conflict pits upstream against one of these, KEEP
   the adaptation and take upstream's real code change around it:
   - **Package rename:** the package name is the fork's `gpui_ce_*` name (e.g.
     `gpui_ce_collections`, not `collections`; the main crate is `gpui-ce` with a hyphen), with
     `[lib] name` preserving the upstream crate name (e.g. `collections`, `gpui`) so `use` sites
     are unchanged, plus the fork's `publish`/version/workspace metadata. Do **not** revert to
     upstream's package name or drop the `[lib] name` override.
   - **Path deps:** intra-fork deps go through `[workspace.dependencies]` aliases — e.g.
     `collections = { path = "crates/gpui_collections", package = "gpui_ce_collections" }`,
     `gpui = { path = "crates/gpui", package = "gpui-ce" }` — not git or crates.io deps.
     Upstream referring to a sibling as `collections`/`util`/`sum_tree`/etc. maps to the fork's
     aliased workspace dep. Do not convert fork path deps back to git/registry deps.
   - **Stripped zed-internal crates:** the fork replaces zed-only infra with std/community crates —
     e.g. `ztracing`→`tracing`, and `zlog`/`zlog::init_test()` test-logger blocks are removed. Keep
     these substitutions; don't reintroduce `ztracing`/`zlog`.

5. **`Cargo.toml` (per-crate and root):**
   - KEEP gpui-ce packaging (names, `publish`, `edition`, workspace metadata) and the fork's dep
     *sources* (path deps for the vendored crates; `zed-font-kit` for font-kit).
   - **ADOPT upstream's real changes: newly added/removed dependencies, new features, new
     `[target.'cfg(...)']` blocks, and — importantly — dependency VERSION BUMPS.** If upstream bumped
     a crate (e.g. `resvg`/`usvg` 0.45→0.46, `taffy`, `accesskit`), take the new version; a bumped
     dep is often paired with a regression test that only passes on the new version. Wire any newly
     required workspace dependency through the fork's convention.

6. **Removed Zed-app / AGPL code.** gpui-ce stripped Zed-application-specific and non-Apache code.
   If an upstream change references a crate or module that doesn't exist in gpui-ce (e.g.
   `http_client`, `reqwest_client`, `util_macros`), drop that reference rather than reintroducing the
   removed code.

7. **Add/delete conflicts are handled for you** — the script settles `modify/delete` cases
   (files gpui-ce deleted that upstream changed are kept deleted) before calling you, so you
   only ever see content conflicts. Don't recreate a deleted file.

8. **Relocated files.** When upstream MOVES a file to another crate, the prompt lists it above as
   `old path → new path`. The script has already deleted the old path and the merge has already
   brought in the new one, so the code is not lost — but any **gpui-ce adaptation that lived in the
   old file is**. For each listed move: diff what the fork had at the old path against what arrived
   at the new one, re-apply the fork's adaptations (rule 4) at the NEW location, and update every
   `use`/`mod`/path reference to point there. Never leave the old copy behind alongside the new one —
   a duplicated type is worse than either version alone. Do **not** re-create the old file.

9. **Do not** run `git commit`, `git merge`, `git rebase`, or `git push`. Only edit files to
   resolve the conflicts — the surrounding script stages and commits. Do not change anything
   unrelated to the conflicts.

10. **No scratch files in the repo.** If you need to save a base/upstream/fork copy of a file to
   diff it, write it under `/tmp`, never inside the working tree (a stray `.merge_tmp/` or similar
   would be committed). Resolve strictly by editing the conflicted files in place.

When finished, briefly summarize what you resolved and any decisions worth a human's attention.
