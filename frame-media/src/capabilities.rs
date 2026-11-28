//! Lists linked codecs and filters, and verifies a hardware encoder with one frame.
use std::ffi::{CString, OsStr, OsString, c_char, c_int};
use std::io::Read;

unsafe extern "C" {
    fn frame_library_paths() -> c_int;
    fn frame_library_versions() -> c_int;
    fn frame_inventory(kind: c_int) -> c_int;
    fn frame_check_encoder(
        name: *const c_char,
        allow_sw: c_int,
        sample_format: *const c_char,
        input: *const u8,
    ) -> c_int;
}

/// # Errors
/// Returns a diagnostic when the registry cannot be read or the requested encoder fails.
pub fn run(args: &[OsString]) -> Result<(), String> {
    if let [operation] = args {
        // SAFETY: both functions read static library information and write to stdout.
        let result = match operation.to_str() {
            Some("libraries") => Some(unsafe { frame_library_paths() }),
            Some("version") => Some(unsafe { frame_library_versions() }),
            _ => None,
        };
        if let Some(code) = result {
            return (code >= 0)
                .then_some(())
                .ok_or_else(|| "could not inspect loaded libraries".into());
        }
    }
    let last = args.last().and_then(|s| s.to_str());
    let kind = match last {
        Some("-encoders") => 0,
        Some("-filters") => 1,
        Some("-bsfs") => 2,
        _ => {
            let [operation, encoder, allow_sw, extra @ ..] = args else {
                return Err("invalid encoder check".into());
            };
            if operation != OsStr::new("hardware") || extra.len() > 1 {
                return Err("invalid capability request".into());
            }
            let name = CString::new(encoder.to_str().ok_or("invalid encoder")?)
                .map_err(|e| e.to_string())?;
            let format = extra
                .first()
                .map(|fmt| CString::new(fmt.to_str().unwrap_or_default()))
                .transpose()
                .map_err(|e| e.to_string())?;
            let mut frame = vec![0; 128 * 128 * 3 / 2];
            std::io::stdin()
                .read_exact(&mut frame)
                .map_err(|e| e.to_string())?;
            // SAFETY: the native encoder reads exactly one packed 128x128 YUV420P frame.
            let rc = unsafe {
                frame_check_encoder(
                    name.as_ptr(),
                    i32::from(allow_sw == "1"),
                    format.as_ref().map_or(std::ptr::null(), |f| f.as_ptr()),
                    frame.as_ptr(),
                )
            };
            return if rc < 0 {
                Err(format!("encoder check failed ({rc})"))
            } else {
                println!("frame=1\nprogress=end");
                Ok(())
            };
        }
    };
    // SAFETY: the integer selects one registry. All iteration stays inside the native call.
    if unsafe { frame_inventory(kind) } < 0 {
        return Err("could not write capability inventory".into());
    }
    Ok(())
}
