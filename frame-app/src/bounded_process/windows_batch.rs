// Derived from Rust 1.95.0 std::sys::args::windows (MIT).
// https://github.com/rust-lang/rust/blob/1.95.0/library/std/src/sys/args/windows.rs
// Copyright/license notice: RUST_LICENSE_MIT.txt. Keep this compatibility shim
// pinned to the runtime toolchain; its tests are string goldens, not Windows QA.
// Inputs are encoded with Windows OsStrExt::encode_wide in the native adapter.
// Raw arguments are deliberately absent: Frame uses ordinary Command::arg(s).

pub(super) fn make_bat_command_line(
    script: &[u16],
    args: &[Vec<u16>],
    force_quotes: bool,
) -> Result<Vec<u16>, &'static str> {
    let script = script.strip_suffix(&[0]).unwrap_or(script);
    // resolve_exe/to_u16s already reject embedded NULs upstream in std.
    if script.contains(&0) {
        return Err("nul in script");
    }
    if script.contains(&(u16::from(b'"'))) || script.last() == Some(&(u16::from(b'\\'))) {
        return Err("invalid script name");
    }
    let mut cmd: Vec<u16> = "cmd.exe /e:ON /v:OFF /d /c \"".encode_utf16().collect();
    cmd.push(u16::from(b'"'));
    cmd.extend_from_slice(script);
    cmd.push(u16::from(b'"'));
    for arg in args {
        if arg.contains(&(u16::from(b'\r'))) || arg.contains(&(u16::from(b'\n'))) {
            return Err("batch file arguments are invalid");
        }
        cmd.push(u16::from(b' '));
        append_bat_arg(&mut cmd, arg, force_quotes)?;
    }
    cmd.push(u16::from(b'"'));
    Ok(cmd) // Native caller adds one terminal NUL before CreateProcessW.
}

fn append_bat_arg(cmd: &mut Vec<u16>, arg: &[u16], mut quote: bool) -> Result<(), &'static str> {
    if arg.contains(&0) {
        return Err("nul in argument");
    }
    if arg.is_empty() || arg.last() == Some(&(u16::from(b'\\'))) {
        quote = true;
    }
    for cp in char::decode_utf16(arg.iter().copied()).flatten() {
        const UNQUOTED: &str = r"#$*+-./:?@\_";
        if (cp.is_ascii() && !(cp.is_ascii_alphanumeric() || UNQUOTED.contains(cp)))
            || cp.is_control()
        {
            quote = true;
        }
    }
    if quote {
        cmd.push(u16::from(b'"'));
    }
    let mut backslashes = 0;
    for &x in arg {
        if x == u16::from(b'\\') {
            backslashes += 1;
        } else {
            if x == u16::from(b'"') {
                cmd.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes));
                cmd.push(u16::from(b'"'));
            } else if x == u16::from(b'%') || x == u16::from(b'\r') {
                cmd.extend("%%cd:~,".encode_utf16());
            }
            backslashes = 0;
        }
        cmd.push(x);
    }
    if quote {
        cmd.extend(std::iter::repeat_n(u16::from(b'\\'), backslashes));
        cmd.push(u16::from(b'"'));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn arg(value: &str) -> Result<String, &'static str> {
        let mut out = Vec::new();
        append_bat_arg(&mut out, &value.encode_utf16().collect::<Vec<_>>(), false)?;
        Ok(String::from_utf16(&out).unwrap())
    }
    #[test]
    fn golden_regular_arguments() {
        for (input, expected) in [
            ("alpha-012_./:$#*?@+", "alpha-012_./:$#*?@+"),
            ("", "\"\""),
            ("a b", "\"a b\""),
            ("a\tb", "\"a\tb\""),
            ("&|<>()^=;,`~{}[]!'", "\"&|<>()^=;,`~{}[]!'\""),
            ("%PATH%", "\"%%cd:~,%PATH%%cd:~,%\""),
            ("100%", "\"100%%cd:~,%\""),
            ("a\"b", "\"a\"\"b\""),
            (r"C:\trailing\", "\"C:\\trailing\\\\\""),
            ("a\\\"b", "\"a\\\\\"\"b\""),
            ("Zażółć-日本-🙂", "Zażółć-日本-🙂"),
            ("a\u{0085}b", "\"a\u{0085}b\""),
        ] {
            assert_eq!(arg(input), Ok(expected.to_owned()), "{input:?}");
        }
    }
    #[test]
    fn complete_command_and_empty_list() {
        let script: Vec<u16> = "C:\\safe & dir\\ffmpeg.CmD\0".encode_utf16().collect();
        let args = ["-hide_banner", "-encoders", "%PATH%"].map(|s| s.encode_utf16().collect());
        let output = make_bat_command_line(&script, &args, false).unwrap();
        assert_eq!(
            String::from_utf16(&output).unwrap(),
            "cmd.exe /e:ON /v:OFF /d /c \"\"C:\\safe & dir\\ffmpeg.CmD\" -hide_banner -encoders \"%%cd:~,%PATH%%cd:~,%\"\""
        );
        let output = make_bat_command_line(&script, &[], false).unwrap();
        assert!(
            String::from_utf16(&output)
                .unwrap()
                .ends_with("ffmpeg.CmD\"\"")
        );
    }
    #[test]
    fn nul_cr_lf_rejected_before_spawn() {
        let script: Vec<u16> = "C:\\safe.cmd".encode_utf16().collect();
        for s in ["a\0b", "a\nb", "a\rb", "a\r\nb"] {
            assert!(make_bat_command_line(&script, &[s.encode_utf16().collect()], false).is_err());
        }
        for s in ["C:\\has\"quote.cmd", "C:\\folder\\", "C:\\nul\0bad.cmd"] {
            assert!(
                make_bat_command_line(&s.encode_utf16().collect::<Vec<_>>(), &[], false).is_err()
            );
        }
    }
    #[test]
    fn unpaired_surrogates_preserved() {
        let script: Vec<u16> = "C:\\safe.cmd".encode_utf16().collect();
        let input = vec![0xd800, u16::from(b'-'), 0xdc00];
        let output = make_bat_command_line(&script, std::slice::from_ref(&input), false).unwrap();
        assert!(output.windows(input.len()).any(|s| s == input));
    }
    #[test]
    fn force_quotes_and_nul_termination_contract() {
        let script: Vec<u16> = "C:\\safe.cmd".encode_utf16().collect();
        let out = make_bat_command_line(&script, &["abc".encode_utf16().collect()], true).unwrap();
        assert!(String::from_utf16(&out).unwrap().ends_with(" \"abc\"\""));
        assert!(!out.contains(&0));
    }
}
