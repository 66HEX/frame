#!/usr/bin/env python3
"""sync_upstream.py — bring upstream Zed GPUI changes into the standalone gpui-ce fork.

Strategy: a vendor-branch 3-way (subtree-style) merge that preserves upstream history.
A local branch (vendor/zed-gpui) holds a *filtered replay* of upstream's actual
gpui-touching commits — each preserving its author/date/message (plus a `zed-upstream:`
trailer) but carrying only the tracked crates/gpui* trees. Merging its tip replays exactly
the upstream delta since the last sync (already-cherry-picked changes no-op) while every
upstream commit stays in the merge's second-parent history. A conflicted merge is committed
as TWO commits for reviewability: the raw merge with conflict markers committed in, then
claude's resolution as a separate, auditable diff. Build fixes land in a third commit.

Local-only: this never pushes, and never lets claude push.

Usage:
  sync_upstream.py bootstrap [BASELINE_SHA]   # one-time setup (see README)
  sync_upstream.py sync [REF] [options]       # default subcommand
  sync_upstream.py status

Options (sync):
  --ref REF        upstream ref to sync to (default: main / SYNC_ZED_REF)
  --model NAME     claude model (default: opus / SYNC_MODEL)
  --retries N      max claude passes per phase (default: 3 / SYNC_RETRIES)
  --no-bump        don't bump pinned zed git-dep revs (no-op since PR #91 removed them)
  --dry-run        show what would be synced, then stop
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import NoReturn, cast

# locate repo
SCRIPT_DIR: Path = Path(__file__).resolve().parent
REPO_ROOT: Path = Path(
    subprocess.run(
        ["git", "-C", str(SCRIPT_DIR), "rev-parse", "--show-toplevel"],
        capture_output=True, text=True, check=True,
    ).stdout.strip()
)


# static config (env-overridable)
def _env(name: str, default: str) -> str:
    return os.environ.get(name, default)


# NOTE: `zed` is a SEPARATE remote from `upstream` (which points at gpui-ce/gpui-ce).
ZED_REMOTE_NAME: str = _env("SYNC_ZED_REMOTE_NAME", "zed")
ZED_REMOTE_URL: str = _env("SYNC_ZED_REMOTE_URL", "https://github.com/zed-industries/zed.git")

# Fork crate DIR (under crates/) -> upstream crate path (under crates/). Keys are directory
# names, not package names. The in-tree gpui* dirs map to themselves; the rest were vendored
# from the Zed monorepo by PR #91 ("removed all of the git sources") and are tracked via
# path remapping. Packaging is orthogonal to directories: every fork package is now `gpui_ce_*`
# — except the main crate, which is `gpui-ce` (hyphen) — each with `[lib] name` preserving the
# upstream crate name so `use` sites are unchanged, plus a `[workspace.dependencies]` alias
# mapping the upstream name back (e.g. `collections = { package = "gpui_ce_collections" }`,
# `gpui = { package = "gpui-ce" }`). Their gpui-ce adaptations (package rename + lib-name
# override, path deps, ztracing->tracing, zlog removal, etc.) are preserved through the 3-way
# merge's conflict resolution — never re-applied by hand.
# Left untouched: gpui_elements (fork-only, package gpui_ce_elements), tooling/perf (fork-only).
# util_macros is no longer used by the fork.
TRACKED_CRATES: dict[str, str] = {
    # in-tree gpui crates (identity mapping)
    "gpui": "gpui",
    "gpui_linux": "gpui_linux",
    "gpui_macos": "gpui_macos",
    "gpui_macros": "gpui_macros",
    "gpui_platform": "gpui_platform",
    "gpui_shared_string": "gpui_shared_string",
    "gpui_tokio": "gpui_tokio",
    "gpui_web": "gpui_web",
    "gpui_wgpu": "gpui_wgpu",
    "gpui_windows": "gpui_windows",
    # vendored + renamed from upstream (fork dir -> upstream path)
    "gpui_collections": "collections",
    "gpui_sum_tree": "sum_tree",
    "gpui_refineable": "refineable",
    "gpui_derive_refineable": "refineable/derive_refineable",  # nested upstream
    "gpui_scheduler": "scheduler",
    "gpui_media": "media",
    "gpui_zed_util": "util",
    "gpui_ce_util": "gpui_util",
    # Created mid-range by upstream's "Split out `RelPath` into a separate crate" (#61029),
    # which MOVED crates/util/src/rel_path.rs into it. Tracked so the split arrives through the
    # merge (see MOVE detection below) instead of leaving a stale duplicate in gpui_zed_util.
    "gpui_path": "path",
}

VENDOR_BRANCH: str = _env("SYNC_VENDOR_BRANCH", "vendor/zed-gpui")

CLAUDE_BIN: str = _env("SYNC_CLAUDE_BIN", "claude")
# Per-invocation wall-clock cap (seconds); 0 disables. The real safety bound — hitting it
# is non-fatal (the loop re-checks progress and retries).
CLAUDE_TIMEOUT: int = int(_env("SYNC_CLAUDE_TIMEOUT", "1800"))
# Optional cap on claude's agentic turns per invocation (0 = no cap, the default).
CLAUDE_MAX_TURNS: int = int(_env("SYNC_CLAUDE_MAX_TURNS", "0"))
# Edits/writes are auto-accepted via --permission-mode acceptEdits; the rest are
# read-only/build helpers. Git staging/commits are done here, never by claude; push is
# never allowed.
_DEFAULT_ALLOWED_TOOLS: str = (
    "Read Edit Write Grep Glob Bash(cargo check:*) Bash(cargo build:*) "
    "Bash(cargo metadata:*) Bash(git status:*) Bash(git diff:*) Bash(git log:*) "
    "Bash(rg:*) Bash(grep:*) Bash(ls:*) Bash(cat:*) Bash(sed:*) Bash(find:*)"
)
CLAUDE_ALLOWED_TOOLS: str = _env("SYNC_CLAUDE_ALLOWED_TOOLS", _DEFAULT_ALLOWED_TOOLS)

# Post-merge gates (host-buildable crates only; macOS/Windows changes need their own
# platform or CI). The verify-fix loop runs the build gate, then the test gate.
VERIFY_CMD: str = _env("SYNC_VERIFY_CMD", "just check")
TEST_CMD: str = _env("SYNC_TEST_CMD", "just test")
RUN_TESTS: bool = _env("SYNC_RUN_TESTS", "1") == "1"
# Treat compile warnings as a fixable condition (the synced branch must be warning-clean to
# pass CI, which denies warnings). Set 0 if pre-existing warnings cause churn.
FAIL_ON_WARNINGS: bool = _env("SYNC_FAIL_ON_WARNINGS", "1") == "1"

STATE_FILE: Path = REPO_ROOT / "scripts" / "sync-upstream" / "state.json"


# runtime config (env defaults; overridable by CLI flags / during a run)
@dataclass
class Runtime:
    zed_ref: str
    model: str
    retries: int
    bump_zed_deps: bool
    dry_run: bool
    build_log: str = ""


RT = Runtime(
    zed_ref=_env("SYNC_ZED_REF", "main"),
    model=_env("SYNC_MODEL", "opus"),
    retries=int(_env("SYNC_RETRIES", "3")),
    bump_zed_deps=_env("SYNC_BUMP_ZED_DEPS", "1") == "1",
    dry_run=False,
)


# output
_COLOR: bool = sys.stdout.isatty()


def _c(code: str) -> str:
    return code if _COLOR else ""


_B, _G, _Y, _R, _D, _Z = (
    _c("\033[34m"), _c("\033[32m"), _c("\033[33m"), _c("\033[31m"), _c("\033[2m"), _c("\033[0m"),
)


def log(msg: str) -> None:
    print(f"{_B}▶{_Z} {msg}", flush=True)


def ok(msg: str) -> None:
    print(f"{_G}✓{_Z} {msg}", flush=True)


def warn(msg: str) -> None:
    print(f"{_Y}⚠{_Z} {msg}", file=sys.stderr, flush=True)


class SyncError(Exception):
    """Fatal, user-facing error; caught in main() and printed without a traceback."""


def die(msg: str) -> NoReturn:
    raise SyncError(msg)


# git helpers
def git(
    *args: str,
    check: bool = True,
    capture: bool = True,
    env: dict[str, str] | None = None,
    input_text: str | None = None,
) -> subprocess.CompletedProcess[str]:
    full_env: dict[str, str] = dict(os.environ)
    if env:
        full_env.update(env)
    result = subprocess.run(
        ["git", *args],
        cwd=REPO_ROOT,
        env=full_env,
        input=input_text,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
    )
    if check and result.returncode != 0:
        detail = (result.stderr or "").strip()
        die(f"`git {' '.join(args)}` failed (exit {result.returncode})"
            + (f":\n{detail}" if detail else ""))
    return result


def run_git(
    *args: str,
    check: bool = True,
    capture: bool = True,
    env: dict[str, str] | None = None,
    input_text: str | None = None,
) -> None:
    """git() for side-effect-only calls (result intentionally discarded)."""
    _ = git(*args, check=check, capture=capture, env=env, input_text=input_text)


def gout(*args: str, env: dict[str, str] | None = None) -> str:
    return git(*args, env=env).stdout.strip()


def gok(*args: str) -> bool:
    return git(*args, check=False).returncode == 0


# vendor history (filtered replay of upstream commits)
def tracked_pathspec() -> list[str]:
    """Upstream pathspecs for the tracked crates (used to filter upstream rev-lists)."""
    return [f"crates/{up}" for up in TRACKED_CRATES.values()]


def filtered_tree(sha: str) -> str | None:
    """Tree object holding ONLY the tracked crates from <sha>, each placed at its FORK path
    (remapping upstream dirs -> gpui-ce's renamed dirs). None if none are present.

    Uses a throwaway index, so there's no working-tree churn.
    """
    tmp = tempfile.mkdtemp()
    env = {"GIT_INDEX_FILE": os.path.join(tmp, "index")}
    try:
        run_git("read-tree", "--empty", env=env)
        added = 0
        for fork_path, up_path in TRACKED_CRATES.items():
            if gok("cat-file", "-e", f"{sha}:crates/{up_path}"):
                run_git("read-tree", f"--prefix=crates/{fork_path}/", f"{sha}:crates/{up_path}", env=env)
                added += 1
        if added == 0:
            return None
        # A tracked upstream path can nest inside another (refineable/derive_refineable lives
        # under refineable). The parent read pulled the nested subtree into the parent's fork
        # prefix; drop it so each crate holds only its own files.
        for fork_path, up_path in TRACKED_CRATES.items():
            for other_up in TRACKED_CRATES.values():
                if other_up != up_path and other_up.startswith(up_path + "/"):
                    rel = other_up[len(up_path) + 1:]
                    run_git("rm", "-r", "-q", "--cached", "--ignore-unmatch", "--",
                            f"crates/{fork_path}/{rel}", env=env, check=False)
        return gout("write-tree", env=env)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def build_vendor_snapshot(sha: str, parent: str | None = None) -> str:
    """Bootstrap baseline: a single filtered snapshot commit. Echoes the commit sha."""
    tree = filtered_tree(sha)
    if tree is None:
        die(f"no tracked crates found at {sha[:12]}")
    parent_args = ["-p", parent] if parent else []
    msg = f"vendor: zed gpui baseline @ {sha[:12]}\n"
    return git("commit-tree", tree, *parent_args, input_text=msg).stdout.strip()


def replay_commit(tree: str, parent: str, original: str) -> str:
    """Filtered commit preserving <original>'s author/committer/dates/message + trailer."""
    message = gout("show", "-s", "--format=%B", original)
    env = {
        "GIT_AUTHOR_NAME": gout("show", "-s", "--format=%an", original),
        "GIT_AUTHOR_EMAIL": gout("show", "-s", "--format=%ae", original),
        "GIT_AUTHOR_DATE": gout("show", "-s", "--format=%aI", original),
        "GIT_COMMITTER_NAME": gout("show", "-s", "--format=%cn", original),
        "GIT_COMMITTER_EMAIL": gout("show", "-s", "--format=%ce", original),
        "GIT_COMMITTER_DATE": gout("show", "-s", "--format=%cI", original),
    }
    body = f"{message}\n\nzed-upstream: {original}\n"
    return git("commit-tree", tree, "-p", parent, env=env, input_text=body).stdout.strip()


def build_vendor_history(parent: str, frm: str, to: str) -> str:
    """Replay upstream gpui-touching commits frm..to as a filtered chain onto <parent>.

    Merge commits and commits with no net change inside the tracked crates are dropped.
    Echoes the new tip sha.
    """
    prev = parent
    replayed = 0
    listing = gout("rev-list", "--reverse", "--topo-order", "--no-merges",
                   f"{frm}..{to}", "--", *tracked_pathspec())
    for commit in listing.split():
        tree = filtered_tree(commit)
        if tree is None:
            continue
        if tree == gout("rev-parse", f"{prev}^{{tree}}"):
            continue  # no net change inside the tracked crates
        prev = replay_commit(tree, prev, commit)
        replayed += 1
    log(f"replayed {replayed} upstream gpui commit(s) into vendor history")
    return prev


# cross-crate file moves
# Upstream regularly relocates code between crates (e.g. #61029 split crates/util/src/rel_path.rs
# out into the new crates/path). Because BOTH sides of a vendor-history diff are already remapped
# to this fork's paths, git's rename detection reports such a move in fork terms
# (crates/gpui_zed_util/src/rel_path.rs -> crates/gpui_path/src/rel_path.rs) — which is exactly
# what's needed to settle the resulting delete/modify conflict correctly.
MOVE_SIMILARITY: str = _env("SYNC_MOVE_SIMILARITY", "40%")


def detect_moves(base: str, tip: str) -> dict[str, str]:
    """Map old fork path -> new fork path for files upstream relocated across tracked crates.

    Only cross-directory moves are reported; a pure in-place rewrite isn't a move.
    """
    out = gout("diff", f"-M{MOVE_SIMILARITY}", "--name-status", "--diff-filter=R", base, tip)
    moves: dict[str, str] = {}
    for line in out.splitlines():
        parts = line.split("\t")
        if len(parts) != 3:
            continue
        _, old, new = parts
        if os.path.dirname(old) != os.path.dirname(new):
            moves[old] = new
    for old, new in moves.items():
        log(f"  upstream moved {old} → {new}")
    if moves:
        ok(f"detected {len(moves)} cross-crate file move(s) in this range")
    return moves


# claude invocation
def render_prompt(kind: str, files: list[str], issue: str = "",
                  moves: dict[str, str] | None = None) -> str:
    if kind == "resolve":
        head = (
            "You are resolving git merge conflicts from syncing upstream Zed's GPUI crates "
            "into the standalone `gpui-ce` fork. A merge commit with the conflict markers "
            "committed in already exists; edit the working-tree files to remove every marker. "
            "Your edits become a SEPARATE resolution commit that will be reviewed in "
            "isolation, so resolve faithfully."
        )
        listing = "\n".join(f"  - {f}" for f in files)
        moves_block = ""
        if moves:
            relocated = "\n".join(f"  - {old} → {new}" for old, new in moves.items())
            moves_block = (
                "\n\nUpstream RELOCATED these files across crates in this range (the old path has\n"
                "already been removed for you; the new one arrived via the merge). Carry any gpui-ce\n"
                "adaptation that lived in the old file over to the new location, and make sure nothing\n"
                "still refers to the old path:\n" + relocated
            )
        body = (SCRIPT_DIR / "resolve-conflicts.prompt.md").read_text()
        return f"{head}\n\nConflicted / unresolved files:\n{listing}{moves_block}\n\n{body}"
    head = (
        f"You are fixing {issue or 'build issues'} that the upstream Zed GPUI merge introduced "
        "in `gpui-ce`. The merge is already committed; fix only what the merge/sync caused."
    )
    cmd_used = TEST_CMD if issue == "test failures" else VERIFY_CMD
    tail = "\n".join(RT.build_log.splitlines()[-300:])
    body = (SCRIPT_DIR / "fix-build.prompt.md").read_text()
    block = f"```\n{tail}\n```"
    return f"{head}\n\nCommand: `{cmd_used}`\nRecent output (tail):\n{block}\n\n{body}"


def run_claude(prompt: str) -> None:
    """Invoke claude -p. Never fatal: max-turns/timeout exit non-zero AFTER useful work, so
    the surrounding loop re-checks real progress and retries up to RT.retries."""
    cmd = [
        CLAUDE_BIN, "-p", prompt,
        "--model", RT.model,
        "--permission-mode", "acceptEdits",
        "--allowedTools", CLAUDE_ALLOWED_TOOLS,
    ]
    if CLAUDE_MAX_TURNS > 0:
        cmd += ["--max-turns", str(CLAUDE_MAX_TURNS)]
    try:
        result = subprocess.run(cmd, cwd=REPO_ROOT, timeout=CLAUDE_TIMEOUT or None)
        if result.returncode != 0:
            warn(f"claude exited non-zero ({result.returncode}; likely --max-turns) — re-checking progress, may retry")
    except subprocess.TimeoutExpired:
        warn(f"claude hit the {CLAUDE_TIMEOUT}s timeout — re-checking progress, may retry")


# conflict detection / resolution
_MARKER_RE = r"^(<{7}|>{7}|\|{7})"


def _unmerged() -> list[str]:
    out = gout("diff", "--name-only", "--diff-filter=U")
    return out.split() if out else []


def _marker_files() -> list[str]:
    result = git("grep", "-lE", _MARKER_RE, "--", "crates/", check=False)
    out = result.stdout.strip()
    return out.split() if (result.returncode == 0 and out) else []


def conflicted_files() -> list[str]:
    return sorted(set(_unmerged()) | set(_marker_files()))


def has_unresolved() -> bool:
    return bool(_unmerged()) or bool(_marker_files())


def auto_resolve_add_delete(moves: dict[str, str] | None = None) -> None:
    """Settle add/delete conflicts claude can't express via edits:
    DU = deleted by us (gpui-ce), modified by them -> keep gpui-ce's deletion;
    UD = modified by us, deleted by them            -> keep gpui-ce's version (flag),
         UNLESS upstream merely RELOCATED the file to another tracked crate, in which case
         the deletion is accepted (the content arrived at the new path via this same merge)
         and claude is told to carry the fork's adaptations across.
    """
    moves = moves or {}
    handled = False
    for line in gout("status", "--porcelain").splitlines():
        if len(line) < 4:
            continue
        code, path = line[:2], line[3:]
        if code == "DU":
            log(f"  modify/delete — keeping gpui-ce's deletion of {path}")
            run_git("rm", "-q", "--force", "--", path, check=False)
            handled = True
        elif code == "UD":
            dest = moves.get(path)
            if dest and (REPO_ROOT / dest).exists():
                log(f"  delete/modify — upstream MOVED {path} → {dest}; accepting the deletion")
                run_git("rm", "-q", "--force", "--", path, check=False)
            else:
                warn(f"  delete/modify — upstream deleted {path}; keeping gpui-ce's version (review)")
                run_git("add", "--", path, check=False)
            handled = True
    if handled:
        ok("auto-resolved add/delete conflicts")


def resolve_conflicts_loop(branch: str, moves: dict[str, str] | None = None) -> None:
    attempt = 0
    while has_unresolved():
        attempt += 1
        if attempt > RT.retries:
            warn(f"still unresolved after {RT.retries} attempt(s):")
            for path in conflicted_files():
                print(f"    {path}", file=sys.stderr)
            die(f"conflict resolution failed; branch '{branch}' left for manual finishing")
        files = conflicted_files()
        before = len(files)
        log(f"claude conflict-resolution pass {attempt}/{RT.retries} ({before} file(s) remaining)")
        for path in files:
            print(f"    {_D}conflict:{_Z} {path}")
        run_claude(render_prompt("resolve", files, moves=moves))
        # Stage only tracked updates (resolves the conflicted files). NOT `-A`: conflict
        # resolution edits existing files, so any new untracked file is claude's scratch
        # (e.g. saved base/upstream copies for diffing) and must not be swept into the commit.
        run_git("add", "-u")
        after = len(conflicted_files())
        if after > 0 and after >= before:
            warn(f"no progress this pass ({before} → {after} files) — claude may be stuck")


# dependency bump (legacy no-op: PR #91 removed all zed git deps, so there is currently
# nothing to bump; kept in case a git dep ever returns)
def bump_zed_deps(target: str) -> None:
    path = REPO_ROOT / "Cargo.toml"
    lines = path.read_text().splitlines(keepends=True)
    bumped = 0
    for i, line in enumerate(lines):
        if 'github.com/zed-industries/zed"' in line:
            new_line, count = re.subn(r'rev = "[0-9a-fA-F]+"', f'rev = "{target}"', line)
            if count:
                lines[i] = new_line
                bumped += 1
    if bumped == 0:
        log("no zed-industries/zed git deps to bump (expected since PR #91 vendored everything)")
        return
    _ = path.write_text("".join(lines))
    ok(f"rewrote {bumped} zed dep rev(s) in Cargo.toml")


# build / test verification + fixing
# A compile warning line: `warning: ...` or `warning[...]` at the start of a line.
_WARNING_RE = r"(?m)^warning(:|\[)"


def _run_gate(cmd: str) -> tuple[int, str]:
    result = subprocess.run(
        cmd, cwd=REPO_ROOT, shell=True, text=True,
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
    )
    out = result.stdout or ""
    print(out, end="")
    return result.returncode, out


def _build_issue() -> str:
    """Run the compile gate; '' if clean, else a short issue label (errors or warnings)."""
    log(f"verifying build: {VERIFY_CMD}")
    rc, out = _run_gate(VERIFY_CMD)
    RT.build_log = out
    if rc != 0:
        return "compile errors"
    if FAIL_ON_WARNINGS and re.search(_WARNING_RE, out) is not None:
        return "compile warnings"
    return ""


def _test_issue() -> str:
    """Run the test gate; '' if it passes, else 'test failures'."""
    log(f"running tests: {TEST_CMD}")
    rc, out = _run_gate(TEST_CMD)
    RT.build_log = out
    return "test failures" if rc != 0 else ""


def verify_fix_loop() -> bool:
    """Loop the build (compile errors + warnings) and test gates, fixing failures with claude.

    The build gate runs first; only once it's clean do we run tests. Any failing gate's
    output is handed to claude, bounded by RT.retries total fix passes.
    """
    attempt = 0
    while True:
        issue = _build_issue()
        if not issue and RUN_TESTS:
            issue = _test_issue()
        if not issue:
            ok("verification passed (build + tests)" if RUN_TESTS else "verification passed (build)")
            return True
        attempt += 1
        if attempt > RT.retries:
            warn(f"{issue} remain after {RT.retries} fix attempt(s); leaving branch for manual finishing")
            return False
        log(f"claude fix pass {attempt}/{RT.retries} ({issue})")
        run_claude(render_prompt("verify", [], issue))


# state
def state_get(key: str) -> str:
    if not STATE_FILE.exists():
        return ""
    try:
        data = cast("dict[str, object]", json.loads(STATE_FILE.read_text()))
    except (json.JSONDecodeError, OSError):
        return ""
    value = data.get(key)
    return value if isinstance(value, str) else ""


def write_state(last_synced: str, vendor_tip: str) -> None:
    payload: dict[str, str] = {
        "last_synced_sha": last_synced,
        "vendor_tip": vendor_tip,
        "last_synced_date": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "upstream_url": ZED_REMOTE_URL,
        "upstream_ref": RT.zed_ref,
    }
    _ = STATE_FILE.write_text(json.dumps(payload, indent=2) + "\n")


# upstream remote
def require_clean_tree() -> None:
    if gout("status", "--porcelain"):
        die("working tree is not clean; commit or stash changes first")


def ensure_remote() -> None:
    if not gok("remote", "get-url", ZED_REMOTE_NAME):
        log(f"adding remote '{ZED_REMOTE_NAME}' → {ZED_REMOTE_URL}")
        run_git("remote", "add", ZED_REMOTE_NAME, ZED_REMOTE_URL)


def fetch_ref(ref: str) -> None:
    log(f"fetching {ZED_REMOTE_NAME}/{ref} (blobless partial fetch)…")
    run_git("fetch", "--filter=blob:none", ZED_REMOTE_NAME, ref, capture=False)


def fetch_object(sha: str) -> None:
    if gok("cat-file", "-e", f"{sha}^{{commit}}"):
        return
    log(f"fetching object {sha[:12]} from {ZED_REMOTE_NAME}…")
    run_git("fetch", "--filter=blob:none", ZED_REMOTE_NAME, sha, check=False, capture=False)
    if not gok("cat-file", "-e", f"{sha}^{{commit}}"):
        die(f"could not fetch {sha} from upstream")


def default_baseline() -> str:
    """Legacy auto-baseline: the zed rev pinned in the root Cargo.toml.

    Always empty since PR #91 removed all zed git deps — bootstrap now requires an
    explicit SHA.
    """
    for line in (REPO_ROOT / "Cargo.toml").read_text().splitlines():
        if 'github.com/zed-industries/zed"' in line:
            match = re.search(r'rev = "([0-9a-fA-F]{7,40})"', line)
            if match:
                return match.group(1)
    return ""


# subcommands
def cmd_bootstrap(baseline: str | None) -> None:
    require_clean_tree()
    ensure_remote()
    base = baseline or default_baseline()
    if not base:
        die("no baseline sha given and none pinned in Cargo.toml; pass one explicitly")
    log(f"bootstrapping sync baseline at {base[:12]}")
    fetch_object(base)
    base = gout("rev-parse", f"{base}^{{commit}}")

    v0 = build_vendor_snapshot(base)
    run_git("update-ref", f"refs/heads/{VENDOR_BRANCH}", v0)
    ok(f"vendor branch '{VENDOR_BRANCH}' → {v0[:12]}")

    if gok("merge-base", "--is-ancestor", v0, "HEAD"):
        warn("baseline already recorded in this branch's history; skipping ours-merge")
    else:
        msg = (f"chore(sync): record zed gpui baseline {base[:12]}\n\n"
               "Establishes the merge base for automated upstream syncs. No file changes.")
        run_git("merge", "-s", "ours", "--allow-unrelated-histories", "--no-edit", "-m", msg, v0,
                capture=False)
        ok("recorded baseline in history (no file changes)")

    write_state(base, v0)
    run_git("add", str(STATE_FILE))
    run_git("commit", "-m", f"chore(sync): initialize upstream sync state at {base[:12]}")
    ok("bootstrap complete — run 'just sync-upstream' to pull upstream changes")


def cmd_sync(ref: str | None) -> None:
    require_clean_tree()
    ensure_remote()
    if ref:
        RT.zed_ref = ref

    last = state_get("last_synced_sha")
    vendor_tip = state_get("vendor_tip")
    if not last:
        die("no sync baseline recorded — run: just sync-upstream-bootstrap [SHA]")
    if not vendor_tip:
        die("state is missing vendor_tip — re-run bootstrap")

    fetch_ref(RT.zed_ref)
    target = gout("rev-parse", "FETCH_HEAD^{commit}")
    log(f"last synced {last[:12]}  →  target {target[:12]} ({RT.zed_ref})")
    if target == last:
        ok(f"already up to date with {RT.zed_ref}")
        return

    log("upstream commits touching tracked crates:")
    listing = git("log", "--oneline", f"{last}..{target}", "--", *tracked_pathspec(), check=False)
    for line in listing.stdout.splitlines():
        print(f"    {line}")

    if RT.dry_run:
        ok("dry run — no changes made")
        return

    if not gok("cat-file", "-e", f"{vendor_tip}^{{commit}}"):
        die(f"vendor_tip {vendor_tip[:12]} missing locally — re-run bootstrap")
    if not gok("merge-base", "--is-ancestor", vendor_tip, "HEAD"):
        die("vendor_tip not in current branch history — run sync from the branch holding the last sync (usually main)")

    vnew = build_vendor_history(vendor_tip, last, target)
    run_git("update-ref", f"refs/heads/{VENDOR_BRANCH}", vnew)
    if vnew == vendor_tip:
        warn("no gpui-touching upstream commits in range — only deps will be updated")

    # Both sides are already remapped to fork paths, so this reports upstream's cross-crate
    # relocations in gpui-ce terms — used to settle delete/modify conflicts as moves.
    moves = detect_moves(vendor_tip, vnew)

    branch = f"sync/zed-{datetime.now(timezone.utc):%Y%m%d}-{target[:7]}"
    run_git("switch", "-C", branch, capture=False)
    ok(f"working on branch '{branch}'")

    upstream_note = (
        "The individual upstream commits are preserved in this merge's second-parent\n"
        "history (filtered to the tracked crates)."
    )
    merge_clean_msg = (
        f"merge: sync zed gpui {last[:12]}..{target[:12]}\n\n"
        f"Synced tracked GPUI crates from zed-industries/zed ({RT.zed_ref}); no conflicts.\n"
        f"{upstream_note}\nUpstream range: {last}..{target}"
    )
    merge_raw_msg = (
        f"merge: sync zed gpui {last[:12]}..{target[:12]} (raw, conflict markers)\n\n"
        "git's automatic 3-way merge of the filtered upstream history. Files git could\n"
        "NOT auto-merge are committed here WITH their conflict markers intact; the very\n"
        "next commit contains the resolution, so it can be reviewed as an isolated diff\n"
        "against this raw state. Deterministic add/delete conflicts were settled by\n"
        f"policy (gpui-ce's deletions kept).\n{upstream_note}\nUpstream range: {last}..{target}"
    )
    resolution_msg = (
        f"resolve conflicts from zed gpui sync {last[:12]}..{target[:12]}\n\n"
        "Resolution of the conflict markers left by the preceding merge commit, performed\n"
        "by claude -p. Review THIS diff in isolation to audit the resolution — it shows\n"
        "exactly which side/lines were chosen, distinct from what git auto-merged."
    )

    log("merging upstream delta…")
    # 3-way merge but DON'T auto-commit, so we can split raw conflicts from the resolution.
    merged = git("merge", "--no-ff", "--no-commit", vnew, check=False, capture=False)
    if merged.returncode == 0:
        run_git("commit", "--no-edit", "-m", merge_clean_msg)
        ok("merged cleanly (no conflicts)")
    else:
        if not gok("rev-parse", "-q", "--verify", "MERGE_HEAD"):
            die("git merge failed without conflicts to resolve; aborting")
        # Settle deterministic add/delete conflicts (kept out of the LLM step).
        auto_resolve_add_delete(moves)
        nconf = len(conflicted_files())
        # Commit 1: the RAW merge — auto-merges applied, conflict markers committed in.
        run_git("add", "-A")
        run_git("commit", "--no-edit", "-m", merge_raw_msg)
        ok(f"committed raw merge with conflict markers ({nconf} file(s) to resolve)")
        if nconf > 0:
            # Commit 2: claude's resolution of the markers — reviewable as an isolated diff.
            warn("resolving conflict markers with claude…")
            resolve_conflicts_loop(branch, moves)
            run_git("commit", "-m", resolution_msg)
            ok("committed conflict resolution (separate, reviewable commit)")
        else:
            ok("no content conflicts (only add/delete, settled in the merge commit)")

    if RT.bump_zed_deps:
        bump_zed_deps(target)

    verify_ok = verify_fix_loop()

    if gout("status", "--porcelain"):
        run_git("add", "-A")
        run_git("commit", "-m",
                "chore(sync): post-merge fixes (zed deps bump + build/test/warning fixes)",
                check=False)
        ok("committed post-merge fixes")

    write_state(target, vnew)
    run_git("add", str(STATE_FILE))
    run_git("commit", "-m", f"chore(sync): advance sync state to {target[:12]}")
    summary(branch, last, target, verify_ok)


def cmd_status() -> None:
    ensure_remote()
    last = state_get("last_synced_sha")
    print(f"last synced : {last or '<none — run bootstrap>'}")
    print(f"vendor tip  : {state_get('vendor_tip')}")
    print(f"synced date : {state_get('last_synced_date')}")
    run_git("fetch", "--filter=blob:none", ZED_REMOTE_NAME, RT.zed_ref, check=False)
    target = gout("rev-parse", "FETCH_HEAD^{commit}") if gok("rev-parse", "FETCH_HEAD^{commit}") else ""
    if target:
        print(f"upstream {RT.zed_ref} : {target[:12]}")
    if last and target and last != target:
        count = gout("rev-list", "--count", f"{last}..{target}") if gok(
            "rev-list", "--count", f"{last}..{target}") else "?"
        print(f"\nbehind by {count} upstream commit(s) on {RT.zed_ref}")
    elif last and target and last == target:
        ok(f"up to date with {RT.zed_ref}")


def summary(branch: str, last: str, target: str, verify_ok: bool) -> None:
    gates = f"{VERIFY_CMD} + {TEST_CMD}" if RUN_TESTS else VERIFY_CMD
    print()
    if verify_ok:
        ok("sync complete — build + tests pass" if RUN_TESTS else "sync complete — build passes")
    else:
        warn("sync complete — VERIFICATION FAILED, finish manually")
    print(f"  branch         : {branch}")
    print(f"  upstream range : {last[:12]}..{target[:12]}")
    print(f"  verify ({gates}) : {'OK' if verify_ok else 'FAILED'}")
    print(f"\nReview : git log --oneline --stat main..{branch}")
    print(f"Merge  : git switch main && git merge --ff-only {branch}   (or open a PR)")
    print(f"{_D}Nothing was pushed.{_Z}")


# entrypoint
def main() -> int:
    parser = argparse.ArgumentParser(
        prog="sync_upstream.py",
        description="Sync upstream Zed GPUI changes into gpui-ce (local-only, never pushes).",
    )
    _ = parser.add_argument("command", nargs="?", default="sync",
                            choices=["sync", "bootstrap", "status"])
    _ = parser.add_argument("arg", nargs="?", default=None,
                            help="bootstrap: baseline sha; sync: upstream ref")
    _ = parser.add_argument("--ref", default=None, help="upstream ref to sync to")
    _ = parser.add_argument("--model", default=None, help="claude model")
    _ = parser.add_argument("--retries", type=int, default=None, help="max claude passes per phase")
    _ = parser.add_argument("--no-bump", action="store_true", help="don't bump pinned zed git-dep revs (no-op since PR #91 removed them)")
    _ = parser.add_argument("--dry-run", action="store_true", help="show what would be synced, then stop")
    ns = parser.parse_args()

    command = cast(str, ns.command)
    positional = cast("str | None", ns.arg)
    model = cast("str | None", ns.model)
    retries = cast("int | None", ns.retries)
    ref = cast("str | None", ns.ref)

    if model is not None:
        RT.model = model
    if retries is not None:
        RT.retries = retries
    if cast(bool, ns.no_bump):
        RT.bump_zed_deps = False
    RT.dry_run = cast(bool, ns.dry_run)

    try:
        if command == "bootstrap":
            cmd_bootstrap(positional)
        elif command == "status":
            cmd_status()
        else:
            cmd_sync(ref or positional)
    except SyncError as exc:
        print(f"{_R}✗ {exc}{_Z}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 130
    return 0


if __name__ == "__main__":
    sys.exit(main())
