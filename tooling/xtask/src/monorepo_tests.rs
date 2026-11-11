use super::*;

#[test]
fn flathub_archive_keeps_native_sources_and_excludes_only_the_root_website() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    let native_files = [
        "Cargo.toml",
        "Cargo.lock",
        "LICENSE",
        ".cargo/config.toml",
        "frame-app/Cargo.toml",
        "frame-app/src/main.rs",
        "frame-core/Cargo.toml",
        "frame-updater/Cargo.toml",
        "tooling/xtask/Cargo.toml",
        "vendor/gpui-ce/Cargo.toml",
        "packaging/flathub/io.github._66HEX.Frame.yml.in",
        "frame-app/website/fixture.txt",
    ];
    let website_files = ["website/package.json", "website/static/screenshot.png"];
    for file in native_files.iter().chain(&website_files) {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, format!("committed {file}\n")).unwrap();
    }
    git(root, &["init", "--quiet"]);
    git(root, &["add", "--force", "."]);
    git(root, &["commit", "--quiet", "-m", "archive fixture"]);
    fs::write(root.join("frame-app/src/main.rs"), "uncommitted change").unwrap();
    fs::write(root.join("untracked-file"), "not part of the release").unwrap();

    let archive = root.join("native.tar.gz");
    archive_flathub_source(root, &archive, "0.33.1").unwrap();
    let extracted = root.join("extracted");
    fs::create_dir(&extracted).unwrap();
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(&archive)
        .arg("-C")
        .arg(&extracted)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let source = extracted.join("frame-0.33.1");
    for file in native_files {
        assert_eq!(
            fs::read_to_string(source.join(file)).unwrap(),
            format!("committed {file}\n"),
            "the native archive must retain the committed version of {file}"
        );
    }
    assert!(!source.join("website").exists());
    assert!(!source.join("untracked-file").exists());

    // The release-specific exclusion must not affect a normal repository archive.
    let output = git(root, &["archive", "--format=zip", "HEAD", "website"]);
    let mut repository_archive = zip::ZipArchive::new(Cursor::new(output.stdout)).unwrap();
    for file in website_files {
        assert!(repository_archive.by_name(file).is_ok());
    }
}

fn git(root: &Path, args: &[&str]) -> std::process::Output {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Archive test",
            "-c",
            "user.email=archive-test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
        ])
        .arg(format!(
            "core.hooksPath={}",
            root.join("empty-hooks").display()
        ))
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
    output
}

#[test]
fn website_ci_checks_the_locked_project_without_deploying() {
    let workflow = website_workflow();
    for required in [
        "name: website checks",
        "  pull_request:",
        "  push:\n    branches:\n      - master",
        "working-directory: website",
        "node-version-file: website/.nvmrc",
        "const { packageManager } = require(\"./package.json\");",
        "if (!/^pnpm@\\d+\\.\\d+\\.\\d+$/.test(packageManager))",
        "execFileSync(\"npm\", [\"install\", \"--global\", packageManager]",
        "run: pnpm install --frozen-lockfile",
        "run: pnpm check",
        "run: pnpm test",
        "run: pnpm build",
        "run: pnpm exec wrangler deploy --dry-run --outdir .wrangler/dry-run",
    ] {
        assert!(workflow.contains(required), "missing {required}");
    }
    for forbidden in [
        "paths:",
        "paths-ignore:",
        "secrets.",
        "run: pnpm deploy",
        "pnpm/action-setup@",
    ] {
        assert!(!workflow.contains(forbidden), "unexpected {forbidden}");
    }
}

#[test]
fn native_required_checks_run_for_website_changes() {
    let workflow = ci_workflow();
    for name in ["cargo xtask ci", "cargo check windows", "cargo check macos"] {
        assert!(workflow.contains(&format!("name: {name}")));
    }
    for forbidden in ["paths:", "paths-ignore:", "pnpm", "node-version"] {
        assert!(!workflow.contains(forbidden), "unexpected {forbidden}");
    }
}

#[test]
fn codeql_scans_both_languages_with_distinct_checks_and_categories() {
    let workflow = codeql_workflow();
    for required in [
        "name: codeql rust",
        "languages: rust",
        "category: /language:rust",
        "name: codeql website",
        "languages: javascript-typescript",
        "category: /language:javascript-typescript/website",
        "config: |\n          paths:\n            - website",
    ] {
        assert!(workflow.contains(required), "missing {required}");
    }
    let triggers = workflow.split_once("permissions:").unwrap().0;
    assert!(!triggers.contains("paths:"));
    assert!(!triggers.contains("paths-ignore:"));
}
