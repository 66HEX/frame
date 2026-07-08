use regex::Regex;

const FFMPEG_ENCODER_LIST_ARGS: [&str; 1] = ["-encoders"];
const FFMPEG_FILTER_LIST_ARGS: [&str; 1] = ["-filters"];

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

#[derive(serde::Serialize, Clone, Debug, Default, Eq, PartialEq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "filter availability is represented as explicit frontend feature flags"
)]
pub struct AvailableFilters {
    pub eq: bool,
    pub hue: bool,
    pub colortemperature: bool,
    pub unsharp: bool,
    pub gblur: bool,
    pub hqdn3d: bool,
    pub deband: bool,
    pub vignette: bool,
    pub bwdif: bool,
    pub highpass: bool,
    pub lowpass: bool,
    pub afftdn: bool,
    pub deesser: bool,
    pub bass: bool,
    pub treble: bool,
    pub acompressor: bool,
    pub loudnorm: bool,
    pub volume: bool,
    pub stereotools: bool,
    pub alimiter: bool,
}

#[must_use]
pub const fn ffmpeg_encoder_list_args() -> [&'static str; 1] {
    FFMPEG_ENCODER_LIST_ARGS
}

#[must_use]
pub const fn ffmpeg_filter_list_args() -> [&'static str; 1] {
    FFMPEG_FILTER_LIST_ARGS
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

#[must_use]
pub fn parse_available_filters(ffmpeg_filters_stdout: impl AsRef<str>) -> AvailableFilters {
    let stdout = ffmpeg_filters_stdout.as_ref();

    AvailableFilters {
        eq: filter_list_contains(stdout, "eq"),
        hue: filter_list_contains(stdout, "hue"),
        colortemperature: filter_list_contains(stdout, "colortemperature"),
        unsharp: filter_list_contains(stdout, "unsharp"),
        gblur: filter_list_contains(stdout, "gblur"),
        hqdn3d: filter_list_contains(stdout, "hqdn3d"),
        deband: filter_list_contains(stdout, "deband"),
        vignette: filter_list_contains(stdout, "vignette"),
        bwdif: filter_list_contains(stdout, "bwdif"),
        highpass: filter_list_contains(stdout, "highpass"),
        lowpass: filter_list_contains(stdout, "lowpass"),
        afftdn: filter_list_contains(stdout, "afftdn"),
        deesser: filter_list_contains(stdout, "deesser"),
        bass: filter_list_contains(stdout, "bass"),
        treble: filter_list_contains(stdout, "treble"),
        acompressor: filter_list_contains(stdout, "acompressor"),
        loudnorm: filter_list_contains(stdout, "loudnorm"),
        volume: filter_list_contains(stdout, "volume"),
        stereotools: filter_list_contains(stdout, "stereotools"),
        alimiter: filter_list_contains(stdout, "alimiter"),
    }
}

fn encoder_list_contains(stdout: &str, name: &str) -> bool {
    let pattern = format!(r"(?m)^\s*[A-Z.]+\s+{}\s+", regex::escape(name));
    Regex::new(&pattern).map_or_else(|_| stdout.contains(name), |re| re.is_match(stdout))
}

fn filter_list_contains(stdout: &str, name: &str) -> bool {
    let pattern = format!(r"(?m)^\s*[A-Z.|]+\s+{}\s+", regex::escape(name));
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
    fn ffmpeg_filter_list_args_match_sidecar_contract() {
        assert_eq!(ffmpeg_filter_list_args(), ["-filters"]);
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

    #[test]
    fn parse_available_filters_detects_ffmpeg_filter_rows() {
        let stdout = "\
Filters:
 TSC eq                V->V       Adjust brightness, contrast, gamma, and saturation.
 T.C hue               V->V       Adjust the hue and saturation.
 ... colortemperature  V->V       Adjust color temperature.
 ... unsharp           V->V       Sharpen or blur the input video.
 ... gblur             V->V       Apply Gaussian Blur filter.
 ... hqdn3d            V->V       Apply a High Quality 3D Denoiser.
 ... deband            V->V       Debands video.
 ... vignette          V->V       Make or reverse a vignette effect.
 ... bwdif             V->V       Deinterlace the input image.
 T.C highpass          A->A       Apply a high-pass filter.
 T.C lowpass           A->A       Apply a low-pass filter.
 ... afftdn            A->A       Denoise audio samples.
 ... deesser           A->A       Apply de-essing to the audio.
 T.C bass              A->A       Boost or cut lower frequencies.
 T.C treble            A->A       Boost or cut upper frequencies.
 ... acompressor       A->A       Audio compressor.
 ... loudnorm          A->A       EBU R128 loudness normalization
 T.C volume            A->A       Change input volume.
 ... stereotools       A->A       Apply stereo tools.
 ... alimiter          A->A       Audio lookahead limiter.
";

        let actual = parse_available_filters(stdout);

        assert_eq!(
            actual,
            AvailableFilters {
                eq: true,
                hue: true,
                colortemperature: true,
                unsharp: true,
                gblur: true,
                hqdn3d: true,
                deband: true,
                vignette: true,
                bwdif: true,
                highpass: true,
                lowpass: true,
                afftdn: true,
                deesser: true,
                bass: true,
                treble: true,
                acompressor: true,
                loudnorm: true,
                volume: true,
                stereotools: true,
                alimiter: true,
            }
        );
    }

    #[test]
    fn parse_available_filters_rejects_substring_matches() {
        let stdout = "\
Filters:
 ... not_eq             V->V should not match
 ... deesser_extra      A->A should not match
";

        let actual = parse_available_filters(stdout);

        assert_eq!(actual, AvailableFilters::default());
    }
}
