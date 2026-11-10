use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use frame_core::capabilities::AvailableEncoders;

use crate::bounded_process::{self, CleanupOutcome, Limits, StopReason};

use super::runtime::{Runtime, RuntimeIdentity};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize)]
pub enum HardwareEncoder {
    H264Videotoolbox,
    HevcVideotoolbox,
    H264Nvenc,
    HevcNvenc,
    Av1Nvenc,
}

impl HardwareEncoder {
    pub const ALL: [Self; 5] = [
        Self::H264Videotoolbox,
        Self::HevcVideotoolbox,
        Self::H264Nvenc,
        Self::HevcNvenc,
        Self::Av1Nvenc,
    ];

    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|encoder| encoder.name() == name)
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::H264Videotoolbox => "h264_videotoolbox",
            Self::HevcVideotoolbox => "hevc_videotoolbox",
            Self::H264Nvenc => "h264_nvenc",
            Self::HevcNvenc => "hevc_nvenc",
            Self::Av1Nvenc => "av1_nvenc",
        }
    }

    #[must_use]
    pub const fn videotoolbox(self) -> bool {
        matches!(self, Self::H264Videotoolbox | Self::HevcVideotoolbox)
    }

    #[must_use]
    pub const fn compiled(self, encoders: &AvailableEncoders) -> bool {
        match self {
            Self::H264Videotoolbox => encoders.h264_videotoolbox,
            Self::HevcVideotoolbox => encoders.hevc_videotoolbox,
            Self::H264Nvenc => encoders.h264_nvenc,
            Self::HevcNvenc => encoders.hevc_nvenc,
            Self::Av1Nvenc => encoders.av1_nvenc,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize)]
pub struct HardwareProbePolicy {
    pub encoder: HardwareEncoder,
    pub allow_software: bool,
}

impl HardwareProbePolicy {
    #[must_use]
    pub const fn new(encoder: HardwareEncoder, allow_software: bool) -> Self {
        Self {
            encoder,
            allow_software: encoder.videotoolbox() && allow_software,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum HardwareStatus {
    Available,
    Unavailable,
    Untested,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub enum HardwareReason {
    NotChecked,
    InitializationFailed,
    TimedOut,
    Cancelled,
    LaunchFailed,
    IncompleteProof,
    CleanupPending,
    RuntimeChanged,
}

impl HardwareReason {
    #[must_use]
    pub const fn caption(self) -> &'static str {
        match self {
            Self::NotChecked => "Encoder availability not checked",
            Self::InitializationFailed => "Encoder could not start",
            Self::TimedOut => "Encoder check timed out",
            Self::Cancelled => "Encoder check cancelled",
            Self::LaunchFailed => "Encoder check could not start",
            Self::IncompleteProof => "Encoder check did not finish",
            Self::CleanupPending => "Previous encoder check is still stopping",
            Self::RuntimeChanged => "FFmpeg changed during the check",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct HardwareProbeReport {
    pub policy: HardwareProbePolicy,
    pub status: HardwareStatus,
    pub reason: Option<HardwareReason>,
    pub diagnostics: String,
    pub checked_at: SystemTime,
}

impl HardwareProbeReport {
    fn new(
        policy: HardwareProbePolicy,
        status: HardwareStatus,
        reason: Option<HardwareReason>,
        diagnostics: String,
    ) -> Self {
        Self {
            policy,
            status,
            reason,
            diagnostics,
            checked_at: SystemTime::now(),
        }
    }

    #[must_use]
    pub const fn available(&self) -> bool {
        matches!(self.status, HardwareStatus::Available)
    }
}

#[derive(Clone, Debug, Default)]
pub struct HardwareCapabilities {
    pub reports: Vec<HardwareProbeReport>,
    pub inventory: EncoderInventory,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EncoderInventory {
    #[default]
    Unknown,
    Complete,
    Failed,
}

impl HardwareCapabilities {
    #[must_use]
    pub fn report(&self, policy: HardwareProbePolicy) -> Option<&HardwareProbeReport> {
        self.reports.iter().find(|report| report.policy == policy)
    }

    #[must_use]
    pub fn disabled_reason(&self, policy: HardwareProbePolicy) -> Option<&'static str> {
        self.report(policy)
            .map_or(Some(HardwareReason::NotChecked.caption()), |report| {
                if report.available() {
                    None
                } else {
                    Some(
                        report
                            .reason
                            .unwrap_or(HardwareReason::NotChecked)
                            .caption(),
                    )
                }
            })
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CacheKey {
    runtime: RuntimeIdentity,
    policy: HardwareProbePolicy,
}

#[derive(Default)]
struct ProbeCache {
    generation: u64,
    entries: HashMap<CacheKey, HardwareProbeReport>,
}

fn cache() -> &'static Mutex<ProbeCache> {
    static CACHE: OnceLock<Mutex<ProbeCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(ProbeCache::default()))
}

/// Invalidates both positive and negative results before an explicit rescan.
pub fn invalidate_hardware_cache() {
    let mut cache = cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    cache.generation = cache.generation.wrapping_add(1);
    cache.entries.clear();
}

fn probe_args(policy: HardwareProbePolicy) -> Vec<std::ffi::OsString> {
    let mut args: Vec<_> = [
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostdin",
        "-nostats",
        "-progress",
        "pipe:1",
        "-f",
        "rawvideo",
        "-pixel_format",
        "yuv420p",
        "-video_size",
        "128x128",
        "-framerate",
        "1",
        "-i",
        "pipe:0",
        "-frames:v",
        "1",
        "-an",
        "-c:v",
        policy.encoder.name(),
        "-pix_fmt",
        "yuv420p",
    ]
    .into_iter()
    .map(std::ffi::OsString::from)
    .collect();
    if policy.encoder.videotoolbox() {
        args.extend(
            [
                "-allow_sw",
                if policy.allow_software { "1" } else { "0" },
                "-require_sw",
                "0",
            ]
            .map(std::ffi::OsString::from),
        );
    }
    args.extend(["-f", "null", "-"].map(std::ffi::OsString::from));
    args
}

fn probe_input() -> Vec<u8> {
    let mut input = vec![16; 128 * 128];
    input.resize(128 * 128 * 3 / 2, 128);
    input
}

fn classify_output(
    policy: HardwareProbePolicy,
    output: &bounded_process::ProcessOutput,
    input_bytes: usize,
) -> HardwareProbeReport {
    let diagnostics = String::from_utf8_lossy(&output.stderr.bytes)
        .trim()
        .to_string();
    let (status, reason) = if !output.success {
        (
            HardwareStatus::Unavailable,
            Some(HardwareReason::InitializationFailed),
        )
    } else if output.stdin_written == input_bytes
        && output.stdin_error.is_none()
        && output.progress.ended
        && output.progress.frames.is_some_and(|frames| frames >= 1)
    {
        (HardwareStatus::Available, None)
    } else {
        (
            HardwareStatus::Untested,
            Some(HardwareReason::IncompleteProof),
        )
    };
    HardwareProbeReport::new(policy, status, reason, diagnostics)
}

fn store_report(key: CacheKey, generation: u64, report: &HardwareProbeReport) {
    if report.status == HardwareStatus::Untested {
        return;
    }
    let mut cache = cache()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if cache.generation == generation {
        if cache.entries.len() >= 64 {
            cache.entries.clear();
        }
        cache.entries.insert(key, report.clone());
    }
}

/// Tests one actual encoder initialization without writing an output file.
#[must_use]
pub fn probe_hardware_encoder(
    executable: &str,
    policy: HardwareProbePolicy,
    cancelled: impl Fn() -> bool,
) -> HardwareProbeReport {
    probe_hardware_encoder_with_limits(executable, policy, Limits::default(), cancelled)
}

fn probe_hardware_encoder_with_limits(
    executable: &str,
    policy: HardwareProbePolicy,
    limits: Limits,
    cancelled: impl Fn() -> bool,
) -> HardwareProbeReport {
    let unknown = |reason, diagnostic| {
        HardwareProbeReport::new(policy, HardwareStatus::Untested, Some(reason), diagnostic)
    };
    if cancelled() {
        return unknown(HardwareReason::Cancelled, String::new());
    }
    let mut runtime = match Runtime::resolve(executable) {
        Ok(runtime) => runtime,
        Err(error) => return unknown(HardwareReason::LaunchFailed, error.to_string()),
    };
    let key = runtime.identity.as_ref().map(|runtime| CacheKey {
        runtime: runtime.clone(),
        policy,
    });
    let generation = {
        let cache = cache()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(report) = key.as_ref().and_then(|key| cache.entries.get(key)) {
            return report.clone();
        }
        cache.generation
    };
    runtime.command.args = probe_args(policy);
    let input = probe_input();
    let report = match bounded_process::run(&runtime.command, &input, limits, &cancelled) {
        Ok(output) => classify_output(policy, &output, input.len()),
        Err(error) => {
            let reason = if error.cleanup == CleanupOutcome::Pending {
                HardwareReason::CleanupPending
            } else {
                match error.reason {
                    StopReason::TimedOut => HardwareReason::TimedOut,
                    StopReason::Cancelled => HardwareReason::Cancelled,
                    StopReason::Io(_) => HardwareReason::LaunchFailed,
                    StopReason::CleanupInProgress => HardwareReason::CleanupPending,
                }
            };
            unknown(
                reason,
                format!(
                    "{error}: {}\n{}",
                    String::from_utf8_lossy(&error.stderr.bytes).trim(),
                    String::from_utf8_lossy(&error.stdout.bytes).trim()
                ),
            )
        }
    };
    if cancelled() {
        return unknown(HardwareReason::Cancelled, report.diagnostics);
    }
    if !runtime.identity_unchanged() {
        return unknown(HardwareReason::RuntimeChanged, report.diagnostics);
    }
    if let Some(key) = key {
        store_report(key, generation, &report);
    }
    report
}

#[must_use]
pub fn detect_hardware_capabilities(
    executable: &str,
    compiled: &AvailableEncoders,
) -> HardwareCapabilities {
    let mut reports = Vec::new();
    for encoder in HardwareEncoder::ALL
        .into_iter()
        .filter(|encoder| encoder.compiled(compiled))
    {
        reports.push(probe_hardware_encoder(
            executable,
            HardwareProbePolicy::new(encoder, false),
            || false,
        ));
        if encoder.videotoolbox() {
            reports.push(probe_hardware_encoder(
                executable,
                HardwareProbePolicy::new(encoder, true),
                || false,
            ));
        }
    }
    HardwareCapabilities {
        reports,
        inventory: EncoderInventory::Complete,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_requires_input_delivery_and_a_complete_nonzero_final_frame_record() {
        let policy = HardwareProbePolicy::new(HardwareEncoder::H264Videotoolbox, false);
        for (success, written, frames, ended, write_error, expected) in [
            (
                true,
                24_576,
                Some(1),
                true,
                false,
                HardwareStatus::Available,
            ),
            (true, 24_576, None, false, false, HardwareStatus::Untested),
            (true, 24_576, Some(0), true, false, HardwareStatus::Untested),
            (
                true,
                24_576,
                Some(1),
                false,
                false,
                HardwareStatus::Untested,
            ),
            (true, 1024, Some(1), true, false, HardwareStatus::Untested),
            (true, 24_576, Some(1), true, true, HardwareStatus::Untested),
            (false, 0, None, false, true, HardwareStatus::Unavailable),
        ] {
            let output = bounded_process::ProcessOutput {
                success,
                code: Some(i32::from(!success)),
                stdin_written: written,
                stdin_error: write_error.then(|| "broken pipe".into()),
                stdout: bounded_process::CapturedOutput::default(),
                stderr: bounded_process::CapturedOutput::default(),
                progress: bounded_process::ProgressProof { frames, ended },
            };
            assert_eq!(classify_output(policy, &output, 24_576).status, expected);
        }
    }

    #[test]
    fn rescan_rejects_stale_cache_writes_and_unknown_results_are_never_cached() {
        let executable = std::env::current_exe().unwrap();
        let runtime = Runtime::resolve(executable.to_str().unwrap()).unwrap();
        let policy = HardwareProbePolicy::new(HardwareEncoder::H264Videotoolbox, false);
        let key = CacheKey {
            runtime: runtime.identity.unwrap(),
            policy,
        };
        invalidate_hardware_cache();
        let old_generation = cache().lock().unwrap().generation;
        invalidate_hardware_cache();
        let report =
            HardwareProbeReport::new(policy, HardwareStatus::Available, None, String::new());
        store_report(key.clone(), old_generation, &report);
        assert!(!cache().lock().unwrap().entries.contains_key(&key));
        let generation = cache().lock().unwrap().generation;
        let unknown = HardwareProbeReport::new(
            policy,
            HardwareStatus::Untested,
            Some(HardwareReason::TimedOut),
            String::new(),
        );
        store_report(key.clone(), generation, &unknown);
        assert!(!cache().lock().unwrap().entries.contains_key(&key));
        store_report(key.clone(), generation, &report);
        assert!(cache().lock().unwrap().entries.contains_key(&key));
        let relaxed = CacheKey {
            runtime: key.runtime,
            policy: HardwareProbePolicy::new(policy.encoder, true),
        };
        assert!(!cache().lock().unwrap().entries.contains_key(&relaxed));
        invalidate_hardware_cache();
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires the bundled runtime and operational VideoToolbox hardware"]
    fn bundled_videotoolbox_encoders_complete_a_real_strict_probe() {
        let executable = crate::runtime_binaries::ffmpeg_executable();
        let compiled =
            crate::capabilities::detect_available_encoders_with_executable(&executable).unwrap();
        for encoder in [
            HardwareEncoder::H264Videotoolbox,
            HardwareEncoder::HevcVideotoolbox,
        ] {
            assert!(encoder.compiled(&compiled));
            let report = probe_hardware_encoder(
                &executable,
                HardwareProbePolicy::new(encoder, false),
                || false,
            );
            assert!(report.available(), "{report:?}");
        }
    }

    #[test]
    fn hardware_probe_uses_a_complete_raw_frame_and_explicit_videotoolbox_policy() {
        let input = probe_input();
        assert_eq!(input.len(), 24_576);
        assert!(input[..16_384].iter().all(|byte| *byte == 16));
        assert!(input[16_384..].iter().all(|byte| *byte == 128));
        for allow in [false, true] {
            let args = probe_args(HardwareProbePolicy::new(
                HardwareEncoder::H264Videotoolbox,
                allow,
            ));
            assert!(
                args.windows(2)
                    .any(|pair| pair == ["-allow_sw", if allow { "1" } else { "0" }])
            );
            assert!(args.windows(2).any(|pair| pair == ["-require_sw", "0"]));
            assert!(args.windows(2).any(|pair| pair == ["-progress", "pipe:1"]));
            assert!(args.windows(2).any(|pair| pair == ["-frames:v", "1"]));
            assert_eq!(&args[args.len() - 3..], ["-f", "null", "-"]);
        }
        let policy = HardwareProbePolicy::new(HardwareEncoder::H264Nvenc, true);
        assert!(!policy.allow_software);
        assert!(!probe_args(policy).iter().any(|arg| arg == "-allow_sw"));
    }
}
