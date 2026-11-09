use super::*;
use std::collections::HashSet;

fn assert_protocol(bytes: &[u8], expected: bool) {
    let offset = bytes.len().saturating_sub(TAIL_LIMIT);
    let result = complete_final_record(&bytes[offset..], offset as u64);
    assert_eq!(result.is_ok(), expected, "protocol result: {result:?}");
}

#[test]
fn terminal_record_accepts_audio_images_empty_values_and_optional_fields() {
    for bytes in [
        b"progress=end\n".as_slice(),
        b"total_size=N/A\nout_time_us=-20000\nprogress=end\n",
        b"frame=0\nstream_0_0_q=-1.0\ncustom=\nfuture=a=b\nprogress=end\n",
        b"custom=za\xc5\xbc\xc3\xb3\xc5\x82\xc4\x87\r\nprogress=end\r\n",
        b"frame=1\nprogress=continue\nframe=2\nprogress=end\n",
    ] {
        assert_protocol(bytes, true);
    }
}

#[test]
fn terminal_record_rejects_incomplete_malformed_and_nonterminal_proof() {
    for bytes in [
        b"".as_slice(),
        b"progress=end",
        b"progress=end\r",
        b"progress=continue\n",
        b"progress=end\npartial",
        b"progress=end\nframe=1\n",
        b"progress=end\nprogress=continue\n",
        b"=value\nprogress=end\n",
        b"bad key=value\nprogress=end\n",
        b"missing equals\nprogress=end\n",
        b"\nprogress=end\n",
        b"custom=\xff\nprogress=end\n",
        b"custom=bad\0value\nprogress=end\n",
        b"progress=END\n",
        b"progress=end \n",
    ] {
        assert_protocol(bytes, false);
    }
}

#[test]
fn tail_requires_a_proven_complete_final_record_not_only_a_visible_end() {
    let mut history = b"frame=1\nprogress=continue\n".repeat(TAIL_LIMIT / 10);
    history.extend_from_slice(b"custom=\nprogress=end\n");
    assert_protocol(&history, true);

    let mut oversized = b"custom=".to_vec();
    oversized.extend(std::iter::repeat_n(b'x', TAIL_LIMIT + 1));
    oversized.extend_from_slice(b"\nprogress=end\n");
    assert_protocol(&oversized, false);

    // The first line could be a cut suffix of custom=...progress=continue.
    assert!(complete_final_record(b"progress=continue\ncustom=ok\nprogress=end\n", 1).is_err());
    assert!(complete_final_record(b"cut\nprogress=continue\ncustom=ok\nprogress=end\n", 1).is_ok());
    // Invalid bytes in a discarded old record are not final-record UTF8 errors.
    assert!(
        complete_final_record(
            b"\xbc\nold=\xff\nprogress=continue\ncustom=ok\nprogress=end\n",
            1
        )
        .is_ok()
    );
    assert!(
        complete_final_record(b"cut\nprogress=continue\ncustom=\xff\nprogress=end\n", 1).is_err()
    );
}

#[test]
fn owner_reads_bounded_history_through_its_regular_retained_handle() {
    let mut owner = CompletionFile::create().expect("private owner should be created");
    assert!(owner.path().is_absolute());
    let mut bytes = b"custom=history\nprogress=continue\n".repeat(TAIL_LIMIT / 8);
    bytes.extend_from_slice(b"total_size=N/A\nprogress=end\n");
    fs::write(owner.path(), &bytes).expect("writer should reopen progress");
    owner
        .verify()
        .expect("small complete final record should pass");
    fs::write(owner.path(), vec![b'x'; TAIL_LIMIT * 2]).expect("oversized data should write");
    assert!(owner.verify().is_err());
    let path = owner.path().to_path_buf();
    let directory = owner.directory.clone();
    assert!(owner.cleanup().is_empty());
    assert!(!path.exists() && !directory.exists());
}

#[test]
fn replacement_path_cannot_supply_or_remove_proof_from_the_original_handle() {
    for original_valid in [false, true] {
        let mut owner = CompletionFile::create().expect("owner should be created");
        if original_valid {
            fs::write(owner.path(), b"progress=end\n").unwrap();
        }
        let moved = owner.directory.join("retained-original");
        fs::rename(owner.path(), &moved).expect("normal shared owner should allow rename");
        fs::write(
            owner.path(),
            if original_valid {
                b"broken".as_slice()
            } else {
                b"progress=end\n"
            },
        )
        .unwrap();
        assert_eq!(owner.verify().is_ok(), original_valid);
        fs::remove_file(moved).unwrap();
        assert!(owner.cleanup().is_empty());
    }
}

#[test]
fn cleanup_preserves_unowned_entries_and_reports_failure_separately() {
    let owner = CompletionFile::create().unwrap();
    let path = owner.path().to_path_buf();
    let directory = owner.directory.clone();
    let extra = directory.join("unowned");
    fs::write(&extra, b"keep").unwrap();
    let diagnostics = owner.cleanup();
    assert_eq!(diagnostics.len(), 1);
    assert!(!path.exists());
    assert_eq!(fs::read(&extra).unwrap(), b"keep");
    fs::remove_file(extra).unwrap();
    fs::remove_dir(directory).unwrap();
}

#[test]
fn concurrent_owners_are_unique_and_creation_failure_is_controlled() {
    let owners = (0..12)
        .map(|_| std::thread::spawn(|| CompletionFile::create().unwrap()))
        .collect::<Vec<_>>()
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    let unique = owners
        .iter()
        .map(CompletionFile::path)
        .collect::<HashSet<_>>();
    assert_eq!(unique.len(), owners.len());
    let invalid = CompletionFile::create_in(owners[0].path());
    assert!(invalid.is_err());
    for owner in owners {
        assert!(owner.cleanup().is_empty());
    }
}

#[test]
fn unreadable_retained_handle_does_not_fall_back_to_valid_path_content() {
    let mut owner = CompletionFile::create().unwrap();
    fs::write(owner.path(), b"progress=end\n").unwrap();
    owner.file = Some(OpenOptions::new().write(true).open(owner.path()).unwrap());
    assert!(owner.verify().is_err());
    assert!(owner.cleanup().is_empty());
}

#[cfg(unix)]
#[test]
fn private_directory_and_file_have_restrictive_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let owner = CompletionFile::create().unwrap();
    assert_eq!(
        fs::metadata(&owner.directory).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        owner
            .file
            .as_ref()
            .unwrap()
            .metadata()
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert!(owner.cleanup().is_empty());
}
