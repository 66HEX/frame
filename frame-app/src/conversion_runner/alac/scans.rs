//! Separate packet-only and decoding queries; mentioning frame entries enables
//! frame reporting in ffprobe even without an explicit `-show_frames` flag.

use std::ffi::OsString;
use std::path::Path;

const PACKETS: &str =
    "packet=codec_type,stream_index,pts,dts,duration,pos,size,flags,data_hash:packet_side_data";
const FRAMES: &str = "frame=media_type,stream_index,pts,pkt_dts,pkt_pos,nb_samples,sample_fmt,channels,channel_layout:frame_side_data";
const STREAMS: &str = "stream=index,codec_name,codec_type,sample_fmt,sample_rate,channels,channel_layout,time_base,initial_padding,bits_per_raw_sample,extradata_size,extradata_hash";

pub(super) enum Scan {
    Source(u32),
    Physical,
    Normal(u32),
}

impl Scan {
    pub fn args(&self, path: &Path) -> Vec<OsString> {
        let mut args: Vec<OsString> = ["-v", "error"].map(Into::into).into();
        match self {
            Self::Source(_) => args.extend(["-flags2", "+skip_manual"].map(Into::into)),
            Self::Physical => args.extend(["-ignore_editlist", "1"].map(Into::into)),
            Self::Normal(_) => {}
        }
        let selection = match self {
            Self::Source(index) | Self::Normal(index) => index.to_string(),
            Self::Physical => "a".into(),
        };
        args.extend([OsString::from("-select_streams"), selection.into()]);
        args.push("-show_packets".into());
        let selector = if matches!(self, Self::Physical) {
            format!("{PACKETS}:{STREAMS}")
        } else {
            args.push("-show_frames".into());
            format!("{PACKETS}:{FRAMES}:{STREAMS}")
        };
        args.extend(
            [
                "-show_streams",
                "-show_data_hash",
                "sha256",
                "-show_entries",
            ]
            .map(Into::into),
        );
        args.push(selector.into());
        args.extend(["-of", "json=compact=1"].map(Into::into));
        args.push(path.as_os_str().to_owned());
        args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_scan_cannot_enable_decoding_through_its_selector() {
        let physical = Scan::Physical.args(Path::new("output.m4a"));
        assert!(
            physical
                .windows(2)
                .any(|pair| pair == ["-ignore_editlist", "1"])
        );
        assert!(!physical.iter().any(|arg| arg == "-show_frames"));
        let selector = physical
            .windows(2)
            .find(|pair| pair[0] == "-show_entries")
            .unwrap();
        assert!(!selector[1].to_str().unwrap().contains("frame"));
        assert!(selector[1].to_str().unwrap().contains(":packet_side_data:"));
        for (scan, manual) in [(Scan::Source(3), true), (Scan::Normal(7), false)] {
            let args = scan.args(Path::new("input.m4a"));
            assert!(args.iter().any(|arg| arg == "-show_frames"));
            assert_eq!(args.iter().any(|arg| arg == "+skip_manual"), manual);
            assert!(!args.iter().any(|arg| arg == "-ignore_editlist"));
            let selector = args
                .windows(2)
                .find(|pair| pair[0] == "-show_entries")
                .unwrap();
            assert!(selector[1].to_str().unwrap().contains(":frame_side_data:"));
        }
    }

    #[cfg(unix)]
    #[test]
    fn source_and_output_paths_remain_os_strings() {
        use std::os::unix::ffi::OsStringExt;
        let path = OsString::from_vec(b"/fixture-\xff/source.m4a".to_vec());
        for scan in [Scan::Source(3), Scan::Physical, Scan::Normal(1)] {
            let args = scan.args(Path::new(&path));
            assert_eq!(args.last(), Some(&path));
        }
    }
}
