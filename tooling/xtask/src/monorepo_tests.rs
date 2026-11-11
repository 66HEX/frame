use std::io::Write;

use super::*;

#[test]
fn linux_ci_preserves_apt_trust_and_suites_with_either_archive() {
    let cases = [
        (
            "deb http://azure.archive.ubuntu.com/ubuntu jammy main restricted\n",
            "deb https://archive.ubuntu.com/ubuntu jammy main restricted\n",
        ),
        (
            "deb-src https://azure.archive.ubuntu.com/ubuntu/ jammy-updates universe\n",
            "deb-src https://archive.ubuntu.com/ubuntu/ jammy-updates universe\n",
        ),
        (
            "deb [arch=amd64 signed-by=/keys/ubuntu.gpg] mirror+file:/etc/apt/apt-mirrors.txt jammy-security main\n",
            "deb [arch=amd64 signed-by=/keys/ubuntu.gpg] https://archive.ubuntu.com/ubuntu/ jammy-security main\n",
        ),
        (
            "Types: deb\nURIs: mirror+file:/etc/apt/apt-mirrors.txt\nSuites: jammy jammy-updates jammy-security\nComponents: main universe\nSigned-By: /keys/ubuntu.gpg\n",
            "Types: deb\nURIs: https://archive.ubuntu.com/ubuntu/\nSuites: jammy jammy-updates jammy-security\nComponents: main universe\nSigned-By: /keys/ubuntu.gpg\n",
        ),
        (
            "deb https://archive.ubuntu.com/ubuntu/ jammy main\n",
            "deb https://archive.ubuntu.com/ubuntu/ jammy main\n",
        ),
        (
            "deb http://archive.ubuntu.com/ubuntu jammy main\n",
            "deb https://archive.ubuntu.com/ubuntu jammy main\n",
        ),
        (
            "deb [signed-by=/keys/microsoft.gpg] https://packages.microsoft.com/ubuntu/22.04/prod jammy main\n",
            "deb [signed-by=/keys/microsoft.gpg] https://packages.microsoft.com/ubuntu/22.04/prod jammy main\n",
        ),
    ];
    for primary_dns_resolves in [true, false] {
        for (input, primary_expected) in cases {
            let expected = if primary_dns_resolves {
                primary_expected.to_string()
            } else {
                primary_expected.replace(
                    "https://archive.ubuntu.com/ubuntu",
                    "https://mirrors.edge.kernel.org/ubuntu",
                )
            };
            assert_eq!(rewrite_apt_source(input, primary_dns_resolves), expected);
            assert_eq!(
                rewrite_apt_source(&expected, primary_dns_resolves),
                expected
            );
        }
    }

    let workflow = ci_workflow();
    let rewrite = workflow.find(UBUNTU_ARCHIVE_SUBSTITUTIONS).unwrap();
    assert!(rewrite < workflow.find("sudo apt-get update").unwrap());
    assert!(workflow.contains("-name '*.list' -o -name '*.sources'"));
    for forbidden in [
        "--allow-unauthenticated",
        "trusted=yes",
        "Verify-Peer=false",
        "--allow-insecure-repositories",
        "Check-Valid-Until=false",
    ] {
        assert!(!workflow.contains(forbidden));
    }
}

#[test]
fn ubuntu_archive_falls_back_only_when_the_primary_dns_lookup_fails() {
    for (resolves, expected) in [
        (true, "https://archive.ubuntu.com/ubuntu"),
        (false, "https://mirrors.edge.kernel.org/ubuntu"),
    ] {
        let script = format!(
            "{}\nprintf '%s' \"$ubuntu_archive\"",
            mock_ubuntu_archive_selection(resolves)
        );
        let output = Command::new("bash")
            .args(["-e", "-c", &script])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }
}

fn mock_ubuntu_archive_selection(primary_dns_resolves: bool) -> String {
    let status = if primary_dns_resolves { 0 } else { 2 };
    format!(
        "getent() {{ test \"$*\" = 'ahosts archive.ubuntu.com' || exit 3; return {status}; }}\n{UBUNTU_ARCHIVE_SELECTION}"
    )
}

fn rewrite_apt_source(input: &str, primary_dns_resolves: bool) -> String {
    let script = format!(
        "{}\nsed -E \"{UBUNTU_ARCHIVE_SUBSTITUTIONS}\"",
        mock_ubuntu_archive_selection(primary_dns_resolves)
    );
    let mut child = Command::new("bash")
        .args(["-e", "-c", &script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    String::from_utf8(output.stdout).unwrap()
}

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
