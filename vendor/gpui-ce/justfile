#!/usr/bin/env just

set shell := ["nu", "-c"]
set windows-shell := ["nu", "-c"]
set positional-arguments := true
set allow-duplicate-variables := true

project_root := justfile_directory()
msrv := "1.95.0"

# ▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰ #
#      Recipes      #
# ▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰▰ #

[doc('List all available recipes')]
default:
    @just --list


[doc('Check workspace for compilation errors')]
[group('build')]
check *flags:
    @echo "🔎 Checking workspace..."
    cargo check --workspace {{ flags }}

[doc('Build workspace (all targets, debug)')]
[group('build')]
build *flags:
    @echo "🔨 Building workspace..."
    cargo build --workspace --all-targets {{ flags }}

[doc('Build workspace in release mode')]
[group('build')]
build-release *flags:
    @echo "🚀 Building workspace (release)..."
    cargo build --workspace --release {{ flags }}

[doc('Build Linux-specific crates')]
[group('build')]
build-linux:
    @echo "🐧 Building Linux crates..."
    cargo build -p gpui_ce_linux

[doc('Build macOS-specific crates')]
[group('build')]
build-mac:
    @echo "🍎 Building macOS crates..."
    cargo build -p gpui_ce_macos

[doc('Build Windows-specific crates')]
[group('build')]
build-windows:
    @echo "🪟 Building Windows crates..."
    cargo build -p gpui_ce_windows

[doc('Build all GPUI examples')]
[group('build')]
build-examples:
    @echo "📐 Building examples..."
    cargo build --package gpui-ce --examples

[doc('Check WASM target (stable) — requires wasm32-unknown-unknown target to be installed')]
[group('build')]
check-wasm:
    @echo "🕸️ Checking WASM target (stable)..."
    cargo check --target wasm32-unknown-unknown --no-default-features -p gpui_ce_platform

[doc('Check WASM target with atomics (nightly) — requires nightly + rust-src component')]
[group('build')]
check-wasm-atomics:
    #!/usr/bin/env nu
    if ((which rustup | length) == 0) {
        print "⚠️  Skipping WASM atomics: rustup not installed (required for +nightly)"
        exit 0
    }
    print "🕸️ Checking WASM target with atomics (nightly)..."
    with-env {CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS: "-C target-feature=+atomics,+bulk-memory,+mutable-globals"} {
        cargo +nightly -Zbuild-std=std,panic_abort check --target wasm32-unknown-unknown -p gpui_ce_platform
    }

[doc('Check examples + WASM web package — requires wasm32-unknown-unknown target to be installed')]
[group('build')]
check-examples:
    @echo "📐 Checking examples..."
    cargo build --package gpui-ce --examples
    cargo check --package gpui_ce_web --target wasm32-unknown-unknown --no-default-features


[doc('Run all workspace unit and integration tests')]
[group('testing')]
test *flags:
    @echo "🧪 Running workspace tests..."
    cargo test --workspace --no-fail-fast {{ flags }}

[doc('Run all workspace doc tests')]
[group('testing')]
test-doc:
    @echo "🧪 Running doc tests..."
    cargo test --workspace --doc --no-fail-fast

[doc('Run tests with nextest (faster parallel runner)')]
[group('testing')]
test-nextest:
    @echo "🧪 Running tests with nextest..."
    cargo nextest run --workspace

[doc('Run workspace tests with a fixed seed for reproducibility')]
[group('testing')]
test-with-seed seed="12345":
    #!/usr/bin/env nu
    print $"🧪 Running tests with seed {{ seed }}..."
    with-env {SEED: '{{ seed }}'} { cargo test --workspace }

[doc('Run workspace tests with additional arguments')]
[group('testing')]
test-with +args:
    @echo "🧪 Running workspace tests with args..."
    cargo test --workspace -- {{ args }}


[doc('Format all Rust code in the workspace')]
[group('quality')]
fmt:
    @echo "💅 Formatting Rust code..."
    cargo fmt --all

[doc('Check Rust code formatting without modifying files')]
[group('quality')]
fmt-check:
    @echo "💅 Checking Rust code formatting..."
    cargo fmt --all -- --check

[doc('Lint with Clippy (all targets, deny warnings)')]
[group('quality')]
clippy *flags:
    @echo "🔍 Running Clippy..."
    cargo clippy --workspace --all-targets -- -D warnings {{ flags }}

[doc('Run Clippy for a specific target triple')]
[group('quality')]
clippy-target target:
    @echo "🔍 Running Clippy for target {{ target }}..."
    cargo clippy --workspace --all-targets --target {{ target }} -- -D warnings

[doc('Automatically fix Clippy lints where possible')]
[group('quality')]
clippy-fix:
    @echo "🩹 Fixing Clippy lints..."
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged -- -D warnings

[doc('Run typos spell checker')]
[group('quality')]
typos:
    @echo "🔤 Checking for typos..."
    typos


[doc('Check for unused dependencies')]
[group('quality')]
machete:
    @echo "⚙️ Checking for unused dependencies..."
    cargo machete

[doc('Check workspace against MSRV — requires {{ msrv }} toolchain to be installed')]
[group('quality')]
msrv-check:
    @echo "🦀 Checking MSRV ({{ msrv }})..."
    cargo +{{ msrv }} check --workspace

[doc('Run all quality checks: fmt-check, clippy, typos, machete')]
[group('quality')]
lint: fmt-check clippy typos machete

[doc('Verify packages conform to workspace standards')]
[group('quality')]
check-packages:
    @echo "✅ Package conformity checks passed."


[doc('Run full CI suite locally — mirrors GitHub Actions, reports all failures before exiting')]
[group('ci')]
ci:
    #!/usr/bin/env nu
    def run-check [name: string, block: closure] {
        print $"\n▶ ($name)"
        let code = try { do $block; $env.LAST_EXIT_CODE } catch { 1 }
        if $code == 0 {
            print $"✅ ($name)"
            {name: $name, ok: true}
        } else {
            print $"❌ ($name)"
            {name: $name, ok: false}
        }
    }

    def available [tool: string] {
        (which $tool | length) > 0
    }

    def skip [name: string, reason: string] {
        print $"\n⚠️  Skipping ($name): ($reason)"
        {name: $name, ok: true}
    }

    let msrv = '{{ msrv }}'

    let results = [
        (run-check "cargo fmt" { cargo fmt --all -- --check })
        (run-check "cargo clippy" { cargo clippy --workspace --all-targets -- -D warnings })
        (run-check "cargo build" { cargo build --workspace --all-targets })
        (run-check "cargo test" { cargo test --workspace --no-fail-fast })
        (run-check "doc tests" { cargo test --workspace --doc --no-fail-fast })
        (run-check "WASM stable" {
            cargo check --target wasm32-unknown-unknown --no-default-features -p gpui_ce_platform
        })
        (if (available "rustup") {
            run-check "WASM atomics" {
                with-env {CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS: "-C target-feature=+atomics,+bulk-memory,+mutable-globals"} {
                    cargo +nightly -Zbuild-std=std,panic_abort check --target wasm32-unknown-unknown -p gpui_ce_platform
                }
            }
        } else {
            skip "WASM atomics" "rustup not installed (required for +nightly)"
        })
        (run-check "check examples" {
            cargo build --package gpui-ce --examples
            cargo check --package gpui_ce_web --target wasm32-unknown-unknown
        })
        (if (available "typos") { run-check "typos" { typos } } else { skip "typos" "not installed" })
        (if (available "cargo-machete") { run-check "cargo machete" { cargo machete } } else { skip "cargo machete" "install: cargo install cargo-machete" })
        (if (available "rustup") {
            run-check $"MSRV ($msrv)" { run-external "cargo" $"+($msrv)" "check" "--workspace" }
        } else {
            skip $"MSRV ($msrv)" "rustup not installed"
        })
    ]

    let failures = ($results | where ok == false | get name)
    if ($failures | length) > 0 {
        let failed_list = ($failures | str join ", ")
        print $"\n❌ ($failures | length) checks failed: ($failed_list)"
        exit 1
    }
    print "\n✅ All checks passed!"

[doc('Run tidy checks only: fmt, clippy, typos, machete, MSRV')]
[group('ci')]
ci-tidy:
    #!/usr/bin/env nu
    def run-check [name: string, block: closure] {
        print $"\n▶ ($name)"
        let code = try { do $block; $env.LAST_EXIT_CODE } catch { 1 }
        if $code == 0 {
            print $"✅ ($name)"
            {name: $name, ok: true}
        } else {
            print $"❌ ($name)"
            {name: $name, ok: false}
        }
    }

    def available [tool: string] {
        (which $tool | length) > 0
    }

    def skip [name: string, reason: string] {
        print $"\n⚠️  Skipping ($name): ($reason)"
        {name: $name, ok: true}
    }

    let msrv = '{{ msrv }}'

    let results = [
        (run-check "cargo fmt" { cargo fmt --all -- --check })
        (run-check "cargo clippy" { cargo clippy --workspace --all-targets -- -D warnings })
        (if (available "typos") { run-check "typos" { typos } } else { skip "typos" "not installed" })
        (if (available "cargo-machete") { run-check "cargo machete" { cargo machete } } else { skip "cargo machete" "install: cargo install cargo-machete" })
        (if (available "rustup") {
            run-check $"MSRV ($msrv)" { run-external "cargo" $"+($msrv)" "check" "--workspace" }
        } else {
            skip $"MSRV ($msrv)" "rustup not installed"
        })
    ]

    let failures = ($results | where ok == false | get name)
    if ($failures | length) > 0 {
        let failed_list = ($failures | str join ", ")
        print $"\n❌ ($failures | length) checks failed: ($failed_list)"
        exit 1
    }
    print "\n✅ Tidy checks passed!"

[doc('Run build + unit tests + doc tests')]
[group('ci')]
ci-test:
    #!/usr/bin/env nu
    def run-check [name: string, block: closure] {
        print $"\n▶ ($name)"
        let code = try { do $block; $env.LAST_EXIT_CODE } catch { 1 }
        if $code == 0 {
            print $"✅ ($name)"
            {name: $name, ok: true}
        } else {
            print $"❌ ($name)"
            {name: $name, ok: false}
        }
    }

    let results = [
        (run-check "cargo build" { cargo build --workspace --all-targets })
        (run-check "cargo test" { cargo test --workspace --no-fail-fast })
        (run-check "doc tests" { cargo test --workspace --doc --no-fail-fast })
    ]

    let failures = ($results | where ok == false | get name)
    if ($failures | length) > 0 {
        let failed_list = ($failures | str join ", ")
        print $"\n❌ ($failures | length) checks failed: ($failed_list)"
        exit 1
    }
    print "\n✅ Tests passed!"

[doc('Check both WASM targets: stable and nightly atomics')]
[group('ci')]
ci-wasm: check-wasm check-wasm-atomics


[doc('Clear the target directory if it exceeds a size threshold in MB')]
[group('maintenance')]
clear-target threshold_mb="300" target_dir="target":
    #!/usr/bin/env nu
    let threshold_mb = ('{{ threshold_mb }}' | into int)
    let target_dir = '{{ target_dir }}'

    if not ($target_dir | path exists) {
        print $"Target directory '($target_dir)' does not exist, skipping."
        exit 0
    }

    let size_bytes = (du $target_dir | get physical | first | into int)
    let size_mb = ($size_bytes / 1048576 | into int)

    print $"Target directory size: ($size_mb) MB [threshold: ($threshold_mb) MB]"

    if $size_mb > $threshold_mb {
        print "Exceeds threshold, clearing build caches..."

        let debug_dir = ($target_dir | path join "debug")
        if ($debug_dir | path exists) {
            let stale = (
                [
                    (glob $"($debug_dir)/**/*.d")
                    (glob $"($debug_dir)/**/*.o")
                    (glob $"($debug_dir)/**/*.rlib")
                ] | flatten | first 10000
            )
            $stale | each { |f| rm --force $f }
        }

        print "Cleanup complete."
    } else {
        print "Target directory is within acceptable size."
    }

[doc('Update Cargo dependencies')]
[group('maintenance')]
update:
    @echo "🔄 Updating dependencies..."
    cargo update

[doc('Show outdated dependencies')]
[group('maintenance')]
outdated:
    @echo "📋 Checking for outdated dependencies..."
    cargo outdated

[doc('Clean build artifacts')]
[group('maintenance')]
clean:
    @echo "🧹 Cleaning build artifacts..."
    cargo clean


[doc('Publish GPUI crates to crates.io in dependency order (dry="true" for a dry run)')]
[group('release')]
publish dry="false":
    #!/usr/bin/env nu
    let dry_run = ('{{ dry }}' == "true")

    let status = (^git status --porcelain | str trim)
    if not $dry_run and ($status | str length) > 0 {
        error make {msg: "Working directory is not clean. Commit or stash changes before publishing."}
    }

    if not $dry_run and not ("CARGO_REGISTRY_TOKEN" in $env) {
        error make {msg: "CARGO_REGISTRY_TOKEN is not set"}
    }

    # Topological publish order. Vendored support crates first, then core,
    # then renderers/platform leaves, with gpui_platform LAST (it has
    # target-conditional deps on every platform crate).
    let crates = [
        "crates/gpui_ce_util/Cargo.toml"
        "crates/gpui_collections/Cargo.toml"
        "crates/gpui_derive_refineable/Cargo.toml"
        "crates/gpui_refineable/Cargo.toml"
        "crates/gpui_sum_tree/Cargo.toml"
        "crates/gpui_scheduler/Cargo.toml"
        "crates/gpui_media/Cargo.toml"
        "crates/gpui_path/Cargo.toml"
        "crates/gpui_zed_util/Cargo.toml"

        # core
        "crates/gpui_shared_string/Cargo.toml"
        "crates/gpui_macros/Cargo.toml"
        "crates/gpui/Cargo.toml"

        # renderers / platform leaves
        "crates/gpui_wgpu/Cargo.toml"
        "crates/gpui_tokio/Cargo.toml"
        "crates/gpui_macos/Cargo.toml"
        "crates/gpui_linux/Cargo.toml"
        "crates/gpui_windows/Cargo.toml"
        "crates/gpui_web/Cargo.toml"
        "crates/gpui_elements/Cargo.toml"

        "crates/gpui_platform/Cargo.toml"

    ]

    let dry_flag = if $dry_run { ["--dry-run"] } else { [] }

    # A `cargo publish --dry-run` resolves dependencies from crates.io. That
    # normally makes a dependency-ordered first release fail at the second
    # crate: its newly-versioned predecessor has not actually been uploaded.
    # Patch every release package into the temporary verification resolution so
    # Cargo still packages and compiles each tarball with `cargo publish`, but
    # validates the complete local release graph rather than stale registry
    # versions. The patches are passed only to the dry run and are never part
    # of the published manifests.
    for manifest in $crates {
        let name = ($manifest | path dirname | path basename)
        let metadata = (^cargo metadata --no-deps --format-version 1 --manifest-path $manifest | from json)
        let manifest_path = ($manifest | path expand)
        let package = ($metadata.packages | where manifest_path == $manifest_path | first)
        let package_name = ($package | get name)
        let package_version = ($package | get version)

        # crates.io versions are immutable. A version-bump PR usually changes
        # only the affected crates, so leave already-published packages alone
        # instead of making a later stable release fail at the first one.
        let already_published = if $dry_run {
            false
        } else {
            let response = (^curl
                --retry 3
                --retry-all-errors
                --silent
                --show-error
                --user-agent "gpui-ce release workflow"
                --output /dev/null
                --write-out "%{http_code}"
                $"https://crates.io/api/v1/crates/($package_name)/($package_version)"
                | complete)
            let status = ($response.stdout | str trim)
            if $response.exit_code != 0 or ($status != "200" and $status != "404") {
                error make {msg: $"Could not check crates.io for ($package_name) ($package_version): ($response.stderr) (HTTP ($status))"}
            }
            $status == "200"
        }
        if $already_published {
            print $"\n⏭️  Skipping ($name): ($package_name) ($package_version) is already on crates.io."
            continue
        }

        print $"\n📦 Publishing ($name)..."
        let patch_manifests = $crates
        let patch_flags = if $dry_run {
            $patch_manifests
                | each { |patch_manifest|
                    let patch_metadata = (^cargo metadata --no-deps --format-version 1 --manifest-path $patch_manifest | from json)
                    let patch_manifest_path = ($patch_manifest | path expand)
                    let patch_package = ($patch_metadata.packages | where manifest_path == $patch_manifest_path | first | get name)
                    ["--config" $"patch.crates-io.($patch_package).path=\"($patch_manifest | path dirname | path expand)\""]
                }
                | flatten
        } else {
            []
        }
        # `gpui-ce` has test/example-only edges to `gpui_ce_platform`, while the
        # Windows platform implementation has the reciprocal `gpui-ce` edge.
        # Cargo cannot represent the packaged crate and that local cycle in a
        # single verification lockfile. The release CI builds every target in
        # the workspace before this recipe; package the archive here and leave
        # its all-target compilation to that complete workspace check.
        let no_verify = if $name == "gpui" { ["--no-verify"] } else { [] }
        run-external "cargo" "publish" "--manifest-path" $manifest "--allow-dirty" ...$dry_flag ...$no_verify ...$patch_flags
        if $env.LAST_EXIT_CODE != 0 {
            error make {msg: $"Failed to publish ($name)"}
        }
    }

    print "\n✅ All crates published!"


[doc('Sync upstream Zed GPUI changes into this fork (local-only, never pushes)')]
[group('sync')]
sync-upstream *args:
    @python3 {{ project_root }}/scripts/sync-upstream/sync_upstream.py sync {{ args }}

[doc('One-time: record the upstream baseline to sync from (pass the upstream SHA explicitly)')]
[group('sync')]
sync-upstream-bootstrap *args:
    @python3 {{ project_root }}/scripts/sync-upstream/sync_upstream.py bootstrap {{ args }}

[doc('Show how far behind upstream Zed GPUI this fork is')]
[group('sync')]
sync-upstream-status:
    @python3 {{ project_root }}/scripts/sync-upstream/sync_upstream.py status


[doc('Generate project documentation')]
[group('docs')]
doc *flags:
    @echo "📚 Generating documentation..."
    cargo doc --workspace --no-deps {{ flags }}

[doc('Generate and open project documentation in browser')]
[group('docs')]
doc-open:
    @echo "📚 Opening documentation in browser..."
    cargo doc --workspace --no-deps --open


[doc('Run a specific example by name')]
[group('development')]
example name:
    @echo "▶️ Running example {{ name }}..."
    cargo run --example {{ name }}

alias b   := build
alias br  := build-release
alias c   := check
alias t   := test
alias td  := test-doc
alias f   := fmt
alias fc  := fmt-check
alias clp := clippy
alias clf := clippy-fix
alias l   := lint
alias d   := doc
alias up  := update
alias cl  := clean
alias sync := sync-upstream
