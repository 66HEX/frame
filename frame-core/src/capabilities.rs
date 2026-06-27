use regex::Regex;

const FFMPEG_ENCODER_LIST_ARGS: [&str; 1] = ["-encoders"];

#[derive(serde::Serialize, Clone, Debug, Default, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "encoder availability is represented as explicit frontend feature flags"
)]
pub struct AvailableEncoders {
    pub h264_videotoolbox: bool,
    pub h264_nvenc: bool,
    pub hevc_videotoolbox: bool,
    pub hevc_nvenc: bool,
    pub av1_nvenc: bool,
    pub libfdk_aac: bool,
    pub libmp3lame: bool,
}

#[must_use]
pub const fn ffmpeg_encoder_list_args() -> [&'static str; 1] {
    FFMPEG_ENCODER_LIST_ARGS
}

#[must_use]
pub fn parse_available_encoders(ffmpeg_encoders_stdout: impl AsRef<str>) -> AvailableEncoders {
    let stdout = ffmpeg_encoders_stdout.as_ref();

    AvailableEncoders {
        h264_videotoolbox: encoder_list_contains(stdout, "h264_videotoolbox"),
        h264_nvenc: encoder_list_contains(stdout, "h264_nvenc"),
        hevc_videotoolbox: encoder_list_contains(stdout, "hevc_videotoolbox"),
        hevc_nvenc: encoder_list_contains(stdout, "hevc_nvenc"),
        av1_nvenc: encoder_list_contains(stdout, "av1_nvenc"),
        libfdk_aac: encoder_list_contains(stdout, "libfdk_aac"),
        libmp3lame: encoder_list_contains(stdout, "libmp3lame"),
    }
}

fn encoder_list_contains(stdout: &str, name: &str) -> bool {
    let pattern = format!(r"(?m)^\s*[A-Z.]+\s+{}\s+", regex::escape(name));
    Regex::new(&pattern).map_or_else(|_| stdout.contains(name), |re| re.is_match(stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffmpeg_encoder_list_args_match_sidecar_contract() {
        assert_eq!(ffmpeg_encoder_list_args(), ["-encoders"]);
    }

    #[test]
    fn parse_available_encoders_detects_ffmpeg_encoder_rows() {
        let stdout = "\
Encoders:
 V..... h264_videotoolbox VideoToolbox H.264 Encoder
 V..... hevc_videotoolbox VideoToolbox H.265 Encoder
 V....D h264_nvenc NVIDIA NVENC H.264 encoder
 V....D hevc_nvenc NVIDIA NVENC hevc encoder
 V....D av1_nvenc NVIDIA NVENC av1 encoder
 A..... libfdk_aac Fraunhofer FDK AAC
 A..... libmp3lame libmp3lame MP3
";

        let actual = parse_available_encoders(stdout);

        assert_eq!(
            actual,
            AvailableEncoders {
                h264_videotoolbox: true,
                h264_nvenc: true,
                hevc_videotoolbox: true,
                hevc_nvenc: true,
                av1_nvenc: true,
                libfdk_aac: true,
                libmp3lame: true,
            }
        );
    }

    #[test]
    fn parse_available_encoders_rejects_substring_matches() {
        let stdout = "\
Encoders:
 V..... not_h264_nvenc should not match
 A..... libmp3lame_extra should not match
";

        let actual = parse_available_encoders(stdout);

        assert_eq!(actual, AvailableEncoders::default());
    }
}
