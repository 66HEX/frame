const TEMPLATE: &str = r#"# Generated from xtask::native_qa.
# Rebuild with `cargo xtask workflows`.
name: native-qa
on:
  pull_request:
    types: [labeled]
permissions:
  contents: read
concurrency:
  group: native-qa-${{ github.event.pull_request.number }}
  cancel-in-progress: true
jobs:
  package:
    if: >-
      github.event.action == 'labeled' &&
      (github.event.label.name == 'qa-native' || github.event.label.name == 'qa-native-check') &&
      github.event.repository.private == false
    name: native QA (${{ matrix.platform }})
    strategy:
      fail-fast: false
      matrix:
        include:
          - runner: ubuntu-22.04
            platform: linux-x86_64
            archive_limit_mib: 180
          - runner: ubuntu-22.04-arm
            platform: linux-aarch64
            archive_limit_mib: 180
          - runner: windows-2022
            platform: windows-x86_64
            archive_limit_mib: 240
    runs-on: ${{ matrix.runner }}
    timeout-minutes: 90
    env:
      CARGO_BUILD_JOBS: 3
      FRAME_QA_PR_HEAD: ${{ github.event.pull_request.head.sha }}
    steps:
    - name: steps::checkout_repo
      uses: actions/checkout@v7.0.0
      with:
        persist-credentials: false
        ref: ${{ github.event.pull_request.head.sha }}
    - name: steps::setup_rust
      shell: bash
      run: |
        rustup toolchain install __RUST_VERSION__ --profile minimal
        rustup default __RUST_VERSION__
        test "$(rustc --version | awk '{print $2}')" = "__RUST_VERSION__"
__LINUX_SETUP__
    - name: qa::test_windows_job_helper
      if: runner.os == 'Windows'
      shell: powershell
      run: '& .\script\qa\tests\test-windows-job.ps1 -Runs .\target\native-qa-helper'
    - name: qa::setup_media_sdk
      run: python script/media-worker.py setup-ci
    - name: qa::build_media_worker
      run: cargo build --release --locked --package frame-media
    - name: qa::package_media_worker
      shell: bash
      run: |
        bundle=target/native-qa-worker
        libs="$bundle/lib"
        if [[ "$RUNNER_OS" == Windows ]]; then libs="$bundle"; fi
        python script/media-worker.py bundle --bin-dir "$bundle" --lib-dir "$libs"
    - name: qa::setup_media_tools
      run: cargo xtask setup-ffmpeg
    - name: qa::test_process_lifecycle
      run: cargo test --locked --release -p frame-app --features qa-harness --lib bounded_process -- --include-ignored --test-threads=1
    - name: qa::test_source_probe
      run: cargo test --locked --release -p frame-app --features qa-harness --lib source_probe -- --include-ignored --test-threads=1
    - name: qa::test_importer
      if: runner.os == 'Linux'
      run: cargo test --locked --release -p frame-app --features qa-harness --lib source_metadata -- --test-threads=1
    - name: qa::test_import_lifecycle
      run: cargo test --locked --release -p frame-app --features qa-harness --lib app::queue::metadata -- --test-threads=1
    - name: qa::test_publication
      run: cargo test --locked --release -p frame-app --features qa-harness --lib conversion_runner -- --include-ignored --test-threads=1
    - name: qa::test_source_frame_export
      run: cargo test --locked --release -p frame-app --features qa-harness --lib frame_export -- --include-ignored --test-threads=1
    - name: qa::test_preview_lifecycle
      run: cargo test --locked --release -p frame-app --features qa-harness --lib preview_engine -- --test-threads=1
    - name: qa::test_preview_pipe_throughput
      if: runner.os != 'Windows'
      run: cargo test --locked --release -p frame-app --features qa-harness --lib full_resolution_frame_does_not_pay_a_polling_sleep_for_each_producer_burst -- --ignored --test-threads=1
    - name: qa::test_preview_runtime
      run: cargo test --locked --release -p frame-app --features qa-harness --lib preview_engine::tests::ffmpeg_ -- --ignored --test-threads=1 --skip ffmpeg_preview_session_collects_audio_video_metrics_for_sync_fixture --skip ffmpeg_audio_prewarm_wakes_through_preview_session_lifecycle
    - name: qa::test_temporal_snapshot
      run: cargo test --locked --release -p frame-app --features qa-harness --lib preview_engine::tests::temporal_snapshot -- --ignored --test-threads=1
    - name: qa::test_small_source_preview
      run: cargo test --locked --release -p frame-app --features qa-harness --lib preview_engine::tests::small_sources::ffmpeg_ -- --ignored --test-threads=1
    - name: qa::test_isolated_profile
      run: cargo test --locked --release -p frame-app --features qa-harness --lib app::qa -- --test-threads=1
    - name: qa::test_workspace_recovery
      run: cargo test --locked --release -p frame-app --features qa-harness --lib recovery -- --test-threads=1
    - name: qa::test_command_line
      run: cargo test --locked --release -p frame-app --features qa-harness --test qa_harness -- --include-ignored --test-threads=1
    - name: qa::test_package_contract
      run: python -B -m unittest discover -s script/tests -p test_native_qa.py -v
    - name: qa::build
      run: cargo build --release --locked -p frame-app --features qa-harness --bin frame-qa
    - name: qa::package
      run: python script/native-qa.py build --output target/native-qa --archive-limit-mib ${{ matrix.archive_limit_mib }}
    - name: qa::verify_relocated_package
      run: python script/native-qa.py verify --output target/native-qa
    - name: qa::report_evidence
      if: always()
      shell: bash
      run: |
        python - <<'PY'
        import sys
        from pathlib import Path
        sys.stdout.reconfigure(encoding='utf-8')
        for name in ('build.json', 'verification.json', 'evidence.json'):
            path = Path('target/native-qa') / name
            if path.is_file():
                print(path.read_text(encoding='utf-8'))
        helper = Path('target/native-qa-helper/windows-job-self-test.json')
        if helper.is_file():
            print(helper.read_text(encoding='utf-8-sig'))
        PY
    - name: qa::upload
      if: github.event.label.name == 'qa-native'
      uses: actions/upload-artifact@v7.0.1
      with:
        name: frame-qa-${{ github.event.pull_request.head.sha }}-${{ matrix.platform }}
        path: target/native-qa/*.zip
        if-no-files-found: error
        compression-level: 0
        retention-days: 1
"#;

pub fn workflow() -> String {
    TEMPLATE.replace(
        "__LINUX_SETUP__",
        &super::linux_ci_setup_step().replace(
            "    - name: steps::setup_linux\n",
            "    - name: steps::setup_linux\n      if: runner.os == 'Linux'\n",
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_qa_requires_an_explicit_label_on_a_public_repository() {
        let workflow = workflow();
        assert!(workflow.contains("types: [labeled]"));
        assert!(workflow.contains("github.event.label.name == 'qa-native'"));
        assert!(workflow.contains("github.event.repository.private == false"));
        assert!(!workflow.contains("pull_request_target"));
        assert!(!workflow.contains("secrets."));
    }

    #[test]
    fn native_qa_accepts_only_package_or_check_labels_for_the_same_job() {
        let workflow = workflow();
        let condition = workflow
            .split_once("    if: >-\n")
            .unwrap()
            .1
            .split_once("\n    name:")
            .unwrap()
            .0;
        assert_eq!(
            condition.lines().map(str::trim).collect::<Vec<_>>(),
            [
                "github.event.action == 'labeled' &&",
                "(github.event.label.name == 'qa-native' || github.event.label.name == 'qa-native-check') &&",
                "github.event.repository.private == false",
            ]
        );
        assert_eq!(workflow.matches("- runner:").count(), 3);
        assert_eq!(workflow.matches("qa::package\n").count(), 1);
        assert_eq!(
            workflow.matches("qa::verify_relocated_package\n").count(),
            1
        );
    }

    #[test]
    fn native_qa_upload_requires_the_package_label() {
        let workflow = workflow();
        let upload = workflow.split_once("- name: qa::upload\n").unwrap().1;
        let guard = upload
            .lines()
            .map(str::trim)
            .find(|line| line.starts_with("if:"));
        assert_eq!(guard, Some("if: github.event.label.name == 'qa-native'"));
    }

    #[test]
    fn native_qa_uses_bounded_standard_runners_and_short_lived_artifacts() {
        let workflow = workflow();
        assert!(workflow.contains("ubuntu-22.04"));
        assert!(workflow.contains("ubuntu-22.04-arm"));
        assert!(workflow.contains("windows-2022"));
        assert!(!workflow.contains("self-hosted"));
        assert!(workflow.contains("timeout-minutes: 90"));
        assert!(workflow.contains("retention-days: 1"));
        assert!(!workflow.contains("actions/cache"));
    }

    #[test]
    fn native_qa_splits_the_same_archive_budget_between_platforms() {
        let workflow = workflow();
        let limits = workflow
            .lines()
            .filter_map(|line| line.trim().strip_prefix("archive_limit_mib: "))
            .map(|value| value.parse::<u32>().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(limits, [180, 180, 240]);
        assert_eq!(limits.iter().sum::<u32>(), 600);
        for (platform, limit) in [
            ("linux-x86_64", 180),
            ("linux-aarch64", 180),
            ("windows-x86_64", 240),
        ] {
            assert!(workflow.contains(&format!(
                "platform: {platform}\n            archive_limit_mib: {limit}"
            )));
        }
        assert!(workflow.contains("--archive-limit-mib ${{ matrix.archive_limit_mib }}"));
    }

    #[test]
    fn native_qa_verifies_the_relocated_package_before_uploading_it() {
        let workflow = workflow();
        let smoke = workflow
            .find("native-qa.py verify")
            .expect("package must be verified");
        let upload = workflow
            .find("actions/upload-artifact@")
            .expect("package upload");
        assert!(smoke < upload);
        assert!(workflow.contains("--features qa-harness"));
        assert!(!workflow.contains("bundle-linux --all"));
        assert!(!workflow.contains("flatpak"));
    }

    #[test]
    fn native_qa_checks_the_cli_and_package_contract_on_each_host() {
        let workflow = workflow();
        assert!(workflow.contains("--test qa_harness -- --include-ignored"));
        assert!(
            workflow
                .contains("python -B -m unittest discover -s script/tests -p test_native_qa.py -v")
        );
    }

    #[test]
    fn native_qa_keeps_failed_verification_evidence_in_job_logs() {
        let workflow = workflow();
        let report = workflow
            .find("qa::report_evidence")
            .expect("evidence log step");
        let upload = workflow.find("qa::upload").unwrap();
        let step = &workflow[report..upload];
        assert!(step.contains("if: always()"));
        assert!(step.contains("evidence.json"));
        assert!(step.contains("verification.json"));
        assert!(step.contains("print(path.read_text"));
    }

    #[test]
    fn native_qa_reports_unicode_evidence_with_a_legacy_stdout_encoding() {
        let workflow = workflow();
        let report = workflow
            .split_once("- name: qa::report_evidence")
            .unwrap()
            .1;
        let python = report
            .split_once("python - <<'PY'\n")
            .unwrap()
            .1
            .split_once("        PY\n")
            .unwrap()
            .0
            .lines()
            .map(|line| line.strip_prefix("        ").unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        let directory = tempfile::tempdir().unwrap();
        let helper_folder = directory.path().join("target/native-qa-helper");
        std::fs::create_dir_all(&helper_folder).unwrap();
        let helper = r#"{"status":"PASS","reportDirectory":"Zażółć's space"}"#;
        std::fs::write(
            helper_folder.join("windows-job-self-test.json"),
            format!("\u{feff}{helper}"),
        )
        .unwrap();
        let output_folder = directory.path().join("target/native-qa");
        std::fs::create_dir_all(&output_folder).unwrap();
        let evidence = r#"{"output":"Zażółć's space/klatka.png"}"#;
        for name in ["build.json", "verification.json", "evidence.json"] {
            std::fs::write(output_folder.join(name), evidence).unwrap();
        }
        let output = std::process::Command::new(if cfg!(windows) { "python" } else { "python3" })
            .args(["-c", &python])
            .env("PYTHONIOENCODING", "cp1252")
            .current_dir(directory.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "evidence reporter failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let lines = String::from_utf8(output.stdout).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            [evidence, evidence, evidence, helper]
        );
    }

    #[test]
    fn native_qa_includes_importer_and_small_source_integration() {
        let workflow = workflow();
        for (step, selector) in [
            ("qa::test_import_lifecycle", "app::queue::metadata --"),
            ("qa::test_workspace_recovery", "recovery --"),
            (
                "qa::test_temporal_snapshot",
                "preview_engine::tests::temporal_snapshot -- --ignored",
            ),
        ] {
            assert!(
                workflow.contains(&format!(
                    "- name: {step}\n      run: cargo test --locked --release -p frame-app --features qa-harness --lib {selector} --test-threads=1"
                )),
                "missing unconditional native product coverage: {step}"
            );
        }
        assert!(workflow.contains("--lib source_metadata -- --test-threads=1"));
        assert!(
            workflow.contains("--lib preview_engine::tests::small_sources::ffmpeg_ -- --ignored")
        );
    }

    #[test]
    fn native_qa_exercises_source_frame_publication_and_runtime_oracles() {
        let workflow = workflow();
        assert!(workflow.contains("--lib frame_export -- --include-ignored --test-threads=1"));
        let step = workflow.find("qa::test_source_frame_export").unwrap();
        let end = workflow[step..].find("\n    - name:").unwrap() + step;
        assert!(!workflow[step..end].contains("if:"));
    }

    #[test]
    fn native_qa_checks_windows_qa_job_ownership_with_powershell_five() {
        let workflow = workflow();
        let step = workflow.find("qa::test_windows_job_helper").unwrap();
        let end = workflow[step..].find("\n    - name:").unwrap() + step;
        let helper = &workflow[step..end];
        assert!(helper.contains("if: runner.os == 'Windows'"));
        assert!(helper.contains("shell: powershell"));
        assert!(helper.contains("test-windows-job.ps1"));
        assert!(workflow.contains("target/native-qa-helper/windows-job-self-test.json"));
    }
}
