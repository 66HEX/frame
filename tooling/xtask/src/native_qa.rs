pub fn workflow() -> String {
    r#"# Generated from xtask::native_qa.
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
      github.event.label.name == 'qa-native' &&
      github.event.repository.private == false
    name: native QA (${{ matrix.platform }})
    strategy:
      fail-fast: false
      matrix:
        include:
          - runner: ubuntu-22.04
            platform: linux-x86_64
          - runner: ubuntu-22.04-arm
            platform: linux-aarch64
          - runner: windows-2022
            platform: windows-x86_64
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
    - name: qa::setup_media_tools
      run: cargo xtask setup-ffmpeg
    - name: qa::test_process_lifecycle
      run: cargo test --locked --release -p frame-app --features qa-harness --lib bounded_process -- --test-threads=1
    - name: qa::test_source_probe
      run: cargo test --locked --release -p frame-app --features qa-harness --lib source_probe -- --include-ignored --test-threads=1
    - name: qa::test_publication
      run: cargo test --locked --release -p frame-app --features qa-harness --lib conversion_runner -- --include-ignored --test-threads=1
    - name: qa::test_preview_lifecycle
      run: cargo test --locked --release -p frame-app --features qa-harness --lib preview_engine -- --test-threads=1
    - name: qa::test_preview_runtime
      run: cargo test --locked --release -p frame-app --features qa-harness --lib preview_engine::tests::ffmpeg_ -- --ignored --test-threads=1 --skip ffmpeg_preview_session_collects_audio_video_metrics_for_sync_fixture --skip ffmpeg_audio_prewarm_wakes_through_preview_session_lifecycle
    - name: qa::test_isolated_profile
      run: cargo test --locked --release -p frame-app --features qa-harness --lib app::qa -- --test-threads=1
    - name: qa::test_command_line
      run: cargo test --locked --release -p frame-app --features qa-harness --test qa_harness -- --include-ignored --test-threads=1
    - name: qa::test_package_contract
      run: python -B -m unittest discover -s script/tests -p test_native_qa.py -v
    - name: qa::build
      run: cargo build --release --locked -p frame-app --features qa-harness --bin frame-qa
    - name: qa::package
      run: python script/native-qa.py build --output target/native-qa --archive-limit-mib 100
    - name: qa::verify_relocated_package
      run: python script/native-qa.py verify --output target/native-qa
    - name: qa::report_evidence
      if: always()
      shell: bash
      run: |
        python - <<'PY'
        from pathlib import Path
        for name in ('build.json', 'verification.json', 'evidence.json'):
            path = Path('target/native-qa') / name
            if path.is_file():
                print(path.read_text(encoding='utf-8'))
        PY
    - name: qa::upload
      uses: actions/upload-artifact@v7.0.1
      with:
        name: frame-qa-${{ github.event.pull_request.head.sha }}-${{ matrix.platform }}
        path: target/native-qa/*.zip
        if-no-files-found: error
        compression-level: 0
        retention-days: 1
"#
    .replace(
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
    fn native_qa_uses_bounded_standard_runners_and_short_lived_artifacts() {
        let workflow = workflow();
        assert!(workflow.contains("ubuntu-22.04"));
        assert!(workflow.contains("ubuntu-22.04-arm"));
        assert!(workflow.contains("windows-2022"));
        assert!(!workflow.contains("self-hosted"));
        assert!(workflow.contains("timeout-minutes: 90"));
        assert!(workflow.contains("retention-days: 1"));
        assert!(workflow.contains("--archive-limit-mib 100"));
        assert!(!workflow.contains("actions/cache"));
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
}
