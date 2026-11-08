//! Windows runtime selection follows Rust 1.95, not `PATHEXT` or shell search.
//!
//! Derived from std/sys/process/windows.rs, std/sys/path/windows.rs, and
//! std/sys/args/windows.rs at rust-lang/rust tag 1.95.0 (MIT).
//! Copyright/license notice: `../RUST_LICENSE_MIT.txt`.

use std::ffi::{OsStr, OsString};
use std::io;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use windows::Win32::Storage::FileSystem::{
    GetFileAttributesW, GetFullPathNameW, INVALID_FILE_ATTRIBUTES,
};
use windows::Win32::System::SystemInformation::{GetSystemDirectoryW, GetWindowsDirectoryW};
use windows::core::PCWSTR;

const SEP: u16 = 92;
const ALT_SEP: u16 = 47;
const COLON: u16 = 58;
const VERBATIM: &[u16] = &[92, 92, 63, 92];
const NT: &[u16] = &[92, 63, 63, 92];
const UNC: &[u16] = &[92, 92, 63, 92, 85, 78, 67, 92];

pub(super) fn wide(value: &OsStr) -> io::Result<Vec<u16>> {
    let units: Vec<_> = value.encode_wide().collect();
    if units.contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "process argument contains NUL",
        ));
    }
    Ok(units)
}

fn fill(mut call: impl FnMut(&mut [u16]) -> u32) -> io::Result<Vec<u16>> {
    let mut buffer = vec![0; 512];
    loop {
        let count = usize::try_from(call(&mut buffer)).unwrap_or(usize::MAX);
        if count == 0 {
            return Err(io::Error::last_os_error());
        }
        if count < buffer.len() {
            buffer.truncate(count);
            return Ok(buffer);
        }
        buffer.resize(
            count
                .checked_add(1)
                .ok_or_else(|| io::Error::other("Windows path is too long"))?,
            0,
        );
    }
}

fn full_path(terminated: &[u16]) -> io::Result<Vec<u16>> {
    // SAFETY: input is NUL-terminated; fill owns the writable output allocation.
    fill(|buffer| unsafe { GetFullPathNameW(PCWSTR(terminated.as_ptr()), Some(buffer), None) })
}

fn verbatim(path: &[u16]) -> bool {
    path.starts_with(VERBATIM) || path.starts_with(NT)
}

pub(super) fn to_user_path(path: &Path) -> io::Result<Vec<u16>> {
    let mut units = wide(path.as_os_str())?;
    units.push(0);
    // Keep std's early long-path behavior before its nonverbatim conversion.
    if units.len() > 260 {
        return Ok(units);
    }
    if units.starts_with(VERBATIM) {
        let candidate = if matches!(&units[4..], [_, COLON, SEP, ..]) {
            Some(units[4..].to_vec())
        } else if units.starts_with(UNC) {
            let mut candidate = vec![SEP, SEP];
            candidate.extend_from_slice(&units[8..]);
            Some(candidate)
        } else {
            None
        };
        if let Some(candidate) = candidate {
            let normalized = full_path(&candidate)?;
            if normalized == candidate[..candidate.len() - 1] {
                let mut normalized = normalized;
                normalized.push(0);
                return Ok(normalized);
            }
            return Ok(units);
        }
    }
    if verbatim(&units) || units == [0] {
        return Ok(units);
    }
    if units.len() < 248 {
        match units.as_slice() {
            [drive, COLON, 0] | [drive, COLON, SEP | ALT_SEP, ..]
                if *drive != SEP && *drive != ALT_SEP =>
            {
                return Ok(units);
            }
            [SEP | ALT_SEP, SEP | ALT_SEP, ..] => return Ok(units),
            _ => {}
        }
    }
    let absolute = full_path(&units)?;
    units.clear();
    if absolute.len() + 1 >= 248 {
        match absolute.as_slice() {
            [_, COLON, SEP, ..] => units.extend_from_slice(VERBATIM),
            [SEP, SEP, 46, SEP, ..] => {
                units.extend_from_slice(VERBATIM);
                units.extend_from_slice(&absolute[4..]);
                units.push(0);
                return Ok(units);
            }
            [SEP, SEP, ..] => {
                units.extend_from_slice(UNC);
                units.extend_from_slice(&absolute[2..]);
                units.push(0);
                return Ok(units);
            }
            _ => {}
        }
    }
    units.extend(absolute);
    units.push(0);
    Ok(units)
}

fn exists(path: &Path) -> Option<Vec<u16>> {
    let units = to_user_path(path).ok()?;
    // SAFETY: initialized NUL-terminated path. Match std attributes semantics,
    // including directories, rather than silently choosing a later executable.
    (unsafe { GetFileAttributesW(PCWSTR(units.as_ptr())) } != INVALID_FILE_ATTRIBUTES)
        .then_some(units)
}

fn select(
    program: &OsStr,
    roots: &[PathBuf],
    mut exists: impl FnMut(&Path) -> Option<Vec<u16>>,
) -> io::Result<Vec<u16>> {
    let units = wide(program)?;
    let has_trailing_slash = units
        .last()
        .is_some_and(|unit| *unit == SEP || (!units.starts_with(VERBATIM) && *unit == ALT_SEP));
    if units.is_empty() || has_trailing_slash {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "program path has no file name",
        ));
    }
    let bytes = program.as_encoded_bytes();
    let suffix_exe = bytes
        .len()
        .checked_sub(4)
        .and_then(|start| bytes.get(start..))
        .is_some_and(|suffix| suffix.eq_ignore_ascii_case(b".exe"));
    // std is_file_name tests separators, not Path.components or drive prefixes.
    if units.iter().any(|unit| matches!(*unit, SEP | ALT_SEP)) {
        if suffix_exe {
            return to_user_path(Path::new(program));
        }
        let mut with_exe = program.to_owned();
        with_exe.push(".exe");
        return exists(Path::new(&with_exe)).map_or_else(|| to_user_path(Path::new(program)), Ok);
    }
    let has_dot = units.contains(&u16::from(b'.'));
    for root in roots {
        let mut candidate = root.join(program);
        if !has_dot {
            candidate.set_extension("exe");
        }
        if let Some(selected) = exists(&candidate) {
            return Ok(selected);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "FFmpeg executable was not found",
    ))
}

fn system_directory() -> io::Result<PathBuf> {
    // SAFETY: fill supplies a live correctly sized UTF-16 buffer.
    fill(|buffer| unsafe { GetSystemDirectoryW(Some(buffer)) })
        .map(|units| PathBuf::from(OsString::from_wide(&units)))
}

fn search_path_entries(paths: &OsStr) -> impl Iterator<Item = PathBuf> + '_ {
    std::env::split_paths(paths).filter(|path| !path.as_os_str().is_empty())
}

pub(super) fn batch_interpreter(program: &Path) -> io::Result<Option<PathBuf>> {
    let mut units = wide(program.as_os_str())?;
    units.push(0);
    let normalized = if verbatim(&units) {
        units[..units.len() - 1].to_vec()
    } else {
        full_path(&units)?
    };
    let batch = matches!(
        normalized
            .len()
            .checked_sub(4)
            .and_then(|start| normalized.get(start..)),
        Some([46, 98 | 66, 97 | 65, 116 | 84] | [46, 99 | 67, 109 | 77, 100 | 68])
    );
    if batch {
        Ok(Some(system_directory()?.join("cmd.exe")))
    } else {
        Ok(None)
    }
}

pub(super) fn resolve(program: &OsStr) -> io::Result<(PathBuf, Option<PathBuf>)> {
    let mut roots = Vec::new();
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        roots.push(directory.to_owned());
    }
    if let Ok(directory) = system_directory() {
        roots.push(directory);
    }
    // SAFETY: fill supplies a live correctly sized UTF-16 buffer.
    if let Ok(directory) = fill(|buffer| unsafe { GetWindowsDirectoryW(Some(buffer)) }) {
        roots.push(OsString::from_wide(&directory).into());
    }
    if let Some(paths) = std::env::var_os("PATH") {
        roots.extend(search_path_entries(&paths));
    }
    let selected = select(program, &roots, exists)?;
    let absolute = if verbatim(&selected) {
        selected[..selected.len() - 1].to_vec()
    } else {
        full_path(&selected)?
    };
    let path = PathBuf::from(OsString::from_wide(&absolute));
    let interpreter = batch_interpreter(&path)?;
    Ok((path, interpreter))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selected_path(units: &[u16]) -> PathBuf {
        OsString::from_wide(units.strip_suffix(&[0]).unwrap()).into()
    }

    fn found(path: &Path) -> Vec<u16> {
        let mut units = wide(path.as_os_str()).unwrap();
        units.push(0);
        units
    }
    #[test]
    fn std_resolution_prefers_application_directory_without_pathext_guessing() {
        let roots = [
            PathBuf::from(r"C:\app"),
            PathBuf::from(r"C:\system"),
            PathBuf::from(r"C:\path"),
        ];
        let existing = [
            PathBuf::from(r"C:\app\ffmpeg.exe"),
            PathBuf::from(r"C:\path\ffmpeg.exe"),
            PathBuf::from(r"C:\path\only.cmd"),
        ];
        let lookup = |path: &Path| {
            existing.contains(&path.to_path_buf()).then(|| {
                let mut value = wide(path.as_os_str()).unwrap();
                value.push(0);
                value
            })
        };
        assert_eq!(
            selected_path(&select(OsStr::new("ffmpeg"), &roots, lookup).unwrap()),
            PathBuf::from(r"C:\app\ffmpeg.exe")
        );
        assert!(select(OsStr::new("only"), &roots, lookup).is_err());
        assert!(select(OsStr::new("only.cmd"), &roots, lookup).is_ok());
    }

    #[test]
    fn dotted_names_and_explicit_paths_follow_different_extension_rules() {
        let roots = [PathBuf::from(r"C:\path")];
        let mut attempts = Vec::new();
        assert!(
            select(OsStr::new("ffmpeg.tool"), &roots, |path| {
                attempts.push(path.to_owned());
                None
            })
            .is_err()
        );
        assert_eq!(attempts, [PathBuf::from(r"C:\path\ffmpeg.tool")]);
        for input in [r".\ffmpeg.tool", r".\Zażółć dir\ffmpeg.cmd"] {
            let actual = select(OsStr::new(input), &roots, |path| Some(found(path))).unwrap();
            assert_eq!(
                selected_path(&actual),
                PathBuf::from(format!("{input}.exe"))
            );
        }
        let actual = select(OsStr::new(r"C:\bin\foo.cmd"), &roots, |path| {
            Some(found(path))
        })
        .unwrap();
        assert_eq!(selected_path(&actual), PathBuf::from(r"C:\bin\foo.cmd.exe"));
        assert!(
            batch_interpreter(&selected_path(&actual))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn first_existing_directory_candidate_is_not_skipped_and_empty_path_entries_are_ignored() {
        let roots: Vec<_> = search_path_entries(OsStr::new(r";C:\earlier;;C:\later;")).collect();
        assert_eq!(
            roots,
            [PathBuf::from(r"C:\earlier"), PathBuf::from(r"C:\later")]
        );
        // GetFileAttributesW also succeeds for a directory: std selects it and
        // subsequently reports a launch error instead of searching further.
        let mut attempts = Vec::new();
        let actual = select(OsStr::new("ffmpeg"), &roots, |path| {
            attempts.push(path.to_owned());
            Some(found(path))
        })
        .unwrap();
        assert_eq!(
            selected_path(&actual),
            PathBuf::from(r"C:\earlier\ffmpeg.exe")
        );
        assert_eq!(attempts.len(), 1);
    }

    #[test]
    fn batch_detection_uses_the_normalized_path_and_system_interpreter() {
        for script in [r"C:\dir\part\..\tool.CmD", r".\Zażółć dir\tool.BaT"] {
            assert_eq!(
                batch_interpreter(Path::new(script)).unwrap(),
                Some(system_directory().unwrap().join("cmd.exe"))
            );
        }
        assert!(
            batch_interpreter(Path::new(r"C:\dir.cmd\tool.exe"))
                .unwrap()
                .is_none()
        );
    }
}
