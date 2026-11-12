use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "frame-import-probe-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn executable(&self, body: &str) -> String {
        let path = self.0.join("ffprobe");
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path.to_str().unwrap().into()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const VALID_METADATA: &str = "printf '%s\\n' '{\"streams\":[{\"index\":0,\"codec_type\":\"video\",\"codec_name\":\"h264\",\"width\":16,\"height\":16}],\"format\":{\"duration\":\"1\"}}'";

#[test]
fn importer_uses_the_production_probe_deadline_and_can_probe_again() {
    let _serial = crate::source_probe::serial_test_probes();
    let fixture = Fixture::new();
    let executable = fixture.executable(&format!("sleep 16\n{VALID_METADATA}"));
    let started = Instant::now();
    let result = probe_source_metadata_with_executable("source.mp4", &executable);

    assert!(
        matches!(&result, Err(ConversionError::Probe(message)) if message.contains("timed out")),
        "silent import probe ignored its production deadline: {result:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(17),
        "probe exceeded its deadline and cleanup budget"
    );
    let executable = fixture.executable(VALID_METADATA);
    let metadata = probe_source_metadata_with_executable("source.mp4", &executable).unwrap();
    assert_eq!(metadata.media_kind, Some(SourceKind::Video));
    assert_eq!(metadata.width, Some(16));
}

#[test]
fn importer_rejects_metadata_larger_than_the_bounded_probe_output() {
    let _serial = crate::source_probe::serial_test_probes();
    let fixture = Fixture::new();
    let executable = fixture.executable("dd if=/dev/zero bs=1024 count=4097 2>/dev/null");
    let result = probe_source_metadata_with_executable("source.mp4", &executable);

    assert!(
        matches!(&result, Err(ConversionError::Probe(message)) if message.contains("metadata exceeded")),
        "oversized metadata did not report the bounded output limit: {result:?}"
    );
}
