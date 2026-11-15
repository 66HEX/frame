use crate::error::ConversionError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceFrameOptions {
    pub position_microseconds: u64,
    pub video_stream_index: u32,
}

/// Builds a full-resolution source PNG at or after the requested input-relative time.
///
/// Source orientation is applied by `FFmpeg`, then SAR is normalized without reducing
/// either decoded raster axis. Conversion effects are deliberately absent from this API.
///
/// # Errors
/// Rejects an empty input or a timestamp outside `FFmpeg`'s signed microsecond range.
pub fn build_source_frame_args(
    input: &str,
    options: SourceFrameOptions,
) -> Result<Vec<String>, ConversionError> {
    if input.is_empty() || options.position_microseconds > i64::MAX.unsigned_abs() {
        return Err(ConversionError::InvalidInput(
            "Invalid source frame path or position".into(),
        ));
    }
    let seconds = options.position_microseconds / 1_000_000;
    let fraction = options.position_microseconds % 1_000_000;
    Ok(vec![
        "-hide_banner".into(), "-nostdin".into(), "-loglevel".into(), "error".into(),
        "-ss".into(), format!("{seconds}.{fraction:06}"),
        "-i".into(), input.into(),
        "-map".into(), format!("0:{}", options.video_stream_index),
        "-an".into(), "-sn".into(), "-dn".into(),
        "-filter_threads".into(), "1".into(),
        "-vf".into(), "scale=w='round(iw*max(sar,1))':h='round(ih/if(gt(sar,0),min(sar,1),1))':flags=lanczos,setsar=1".into(),
        "-frames:v".into(), "1".into(),
        "-c:v".into(), "png".into(), "-threads".into(), "1".into(),
        "-fps_mode".into(), "passthrough".into(),
        "-f".into(), "image2pipe".into(), "pipe:1".into(),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option<'a>(args: &'a [String], key: &str) -> &'a str {
        args.windows(2).find(|pair| pair[0] == key).unwrap()[1].as_str()
    }

    #[test]
    fn source_frame_keeps_microseconds_and_maps_the_observed_stream() {
        let args = build_source_frame_args(
            "source.mov",
            SourceFrameOptions {
                position_microseconds: 40_001,
                video_stream_index: 7,
            },
        )
        .unwrap();
        assert_eq!(option(&args, "-ss"), "0.040001");
        assert_eq!(option(&args, "-map"), "0:7");
        assert!(args.iter().position(|arg| arg == "-ss") < args.iter().position(|arg| arg == "-i"));
        assert!(!args.iter().any(|arg| matches!(
            arg.as_str(),
            "-noaccurate_seek" | "-seek_timestamp" | "-noautorotate"
        )));
    }

    #[test]
    fn source_frame_plan_is_one_full_resolution_png_to_stdout() {
        let args = build_source_frame_args(
            "source.mov",
            SourceFrameOptions {
                position_microseconds: 0,
                video_stream_index: 0,
            },
        )
        .unwrap();
        assert_eq!(option(&args, "-frames:v"), "1");
        assert_eq!(option(&args, "-c:v"), "png");
        assert_eq!(option(&args, "-f"), "image2pipe");
        assert_eq!(args.last().unwrap(), "pipe:1");
        let filter = option(&args, "-vf");
        assert!(filter.contains("iw") && filter.contains("ih") && filter.contains("sar"));
        assert!(filter.ends_with("setsar=1"));
        assert!(!filter.contains("fps=") && !filter.contains("1280") && !filter.contains("720"));
    }

    #[test]
    fn source_frame_rejects_empty_input_and_unrepresentable_time() {
        for (path, position_microseconds) in [("", 0), ("source.mov", u64::MAX)] {
            assert!(
                build_source_frame_args(
                    path,
                    SourceFrameOptions {
                        position_microseconds,
                        video_stream_index: 0,
                    }
                )
                .is_err()
            );
        }
    }
}
