#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::settings) struct AudioCodecDefinition {
    pub(in crate::settings) codec: &'static str,
    pub(in crate::settings) label: &'static str,
}

pub(in crate::settings) const AUDIO_CODEC_DEFINITIONS: [AudioCodecDefinition; 9] = [
    AudioCodecDefinition {
        codec: "aac",
        label: "AAC / Stereo",
    },
    AudioCodecDefinition {
        codec: "ac3",
        label: "Dolby Digital",
    },
    AudioCodecDefinition {
        codec: "libopus",
        label: "Opus",
    },
    AudioCodecDefinition {
        codec: "mp3",
        label: "MP3",
    },
    AudioCodecDefinition {
        codec: "alac",
        label: "ALAC (Lossless)",
    },
    AudioCodecDefinition {
        codec: "flac",
        label: "FLAC (Lossless)",
    },
    AudioCodecDefinition {
        codec: "pcm_s16le",
        label: "PCM / WAV",
    },
    AudioCodecDefinition {
        codec: "mp2",
        label: "MPEG Layer II",
    },
    AudioCodecDefinition {
        codec: "pcm_bluray",
        label: "Blu-ray PCM (Lossless)",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::settings) struct OptionalAudioCodecDefinition {
    pub(in crate::settings) codec: &'static str,
    pub(in crate::settings) label: &'static str,
}

pub(in crate::settings) const OPTIONAL_AUDIO_CODEC_DEFINITIONS: [OptionalAudioCodecDefinition; 1] =
    [OptionalAudioCodecDefinition {
        codec: "libfdk_aac",
        label: "AAC (Fraunhofer FDK)",
    }];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::settings) struct AudioChannelDefinition {
    pub(in crate::settings) id: &'static str,
    pub(in crate::settings) label: &'static str,
}

pub(in crate::settings) const AUDIO_CHANNEL_DEFINITIONS: [AudioChannelDefinition; 3] = [
    AudioChannelDefinition {
        id: "original",
        label: "Original",
    },
    AudioChannelDefinition {
        id: "stereo",
        label: "Stereo",
    },
    AudioChannelDefinition {
        id: "mono",
        label: "Mono",
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::settings) struct VideoCodecDefinition {
    pub(in crate::settings) codec: &'static str,
    pub(in crate::settings) label: &'static str,
    pub(in crate::settings) capability: Option<VideoCodecCapability>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::settings) enum VideoCodecCapability {
    Mpeg2video,
    H264Videotoolbox,
    H264Nvenc,
    HevcVideotoolbox,
    HevcNvenc,
    Av1Nvenc,
}

pub(in crate::settings) const VIDEO_CODEC_DEFINITIONS: [VideoCodecDefinition; 12] = [
    VideoCodecDefinition {
        codec: "libx264",
        label: "H.264 / AVC",
        capability: None,
    },
    VideoCodecDefinition {
        codec: "libx265",
        label: "H.265 / HEVC",
        capability: None,
    },
    VideoCodecDefinition {
        codec: "vp9",
        label: "VP9 / Web",
        capability: None,
    },
    VideoCodecDefinition {
        codec: "prores",
        label: "Apple ProRes",
        capability: None,
    },
    VideoCodecDefinition {
        codec: "libsvtav1",
        label: "AV1 / SVT",
        capability: None,
    },
    VideoCodecDefinition {
        codec: "mpeg2video",
        label: "MPEG-2 Video",
        capability: Some(VideoCodecCapability::Mpeg2video),
    },
    VideoCodecDefinition {
        codec: "gif",
        label: "GIF / Palette",
        capability: None,
    },
    VideoCodecDefinition {
        codec: "h264_videotoolbox",
        label: "H.264 (Apple Silicon)",
        capability: Some(VideoCodecCapability::H264Videotoolbox),
    },
    VideoCodecDefinition {
        codec: "h264_nvenc",
        label: "H.264 (NVIDIA)",
        capability: Some(VideoCodecCapability::H264Nvenc),
    },
    VideoCodecDefinition {
        codec: "hevc_videotoolbox",
        label: "H.265 (Apple Silicon)",
        capability: Some(VideoCodecCapability::HevcVideotoolbox),
    },
    VideoCodecDefinition {
        codec: "hevc_nvenc",
        label: "H.265 (NVIDIA)",
        capability: Some(VideoCodecCapability::HevcNvenc),
    },
    VideoCodecDefinition {
        codec: "av1_nvenc",
        label: "AV1 (NVIDIA)",
        capability: Some(VideoCodecCapability::Av1Nvenc),
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::settings) struct VideoPixelFormatDefinition {
    pub(in crate::settings) id: &'static str,
    pub(in crate::settings) label: &'static str,
}

pub(in crate::settings) const VIDEO_PIXEL_FORMAT_DEFINITIONS: [VideoPixelFormatDefinition; 7] = [
    VideoPixelFormatDefinition {
        id: "auto",
        label: "Auto",
    },
    VideoPixelFormatDefinition {
        id: "yuv420p",
        label: "YUV 4:2:0 (8-bit)",
    },
    VideoPixelFormatDefinition {
        id: "yuv422p",
        label: "YUV 4:2:2 (8-bit)",
    },
    VideoPixelFormatDefinition {
        id: "yuv444p",
        label: "YUV 4:4:4 (8-bit)",
    },
    VideoPixelFormatDefinition {
        id: "yuv420p10le",
        label: "YUV 4:2:0 (10-bit)",
    },
    VideoPixelFormatDefinition {
        id: "yuv422p10le",
        label: "YUV 4:2:2 (10-bit)",
    },
    VideoPixelFormatDefinition {
        id: "yuv444p10le",
        label: "YUV 4:4:4 (10-bit)",
    },
];

pub(in crate::settings) const VIDEO_PRESETS: [&str; 9] = [
    "ultrafast",
    "superfast",
    "veryfast",
    "faster",
    "fast",
    "medium",
    "slow",
    "slower",
    "veryslow",
];

pub(in crate::settings) const RESOLUTION_OPTIONS: [&str; 5] =
    ["original", "1080p", "720p", "480p", "custom"];

pub(in crate::settings) const SCALING_ALGORITHM_OPTIONS: [&str; 4] =
    ["bicubic", "lanczos", "bilinear", "nearest"];

pub(in crate::settings) const FPS_OPTIONS: [&str; 4] = ["original", "24", "30", "60"];

pub(in crate::settings) const GIF_FPS_OPTIONS: [&str; 8] =
    ["original", "8", "10", "12", "15", "20", "24", "30"];

pub(in crate::settings) const GIF_COLOR_OPTIONS: [u16; 4] = [32, 64, 128, 256];

pub(in crate::settings) const GIF_DITHER_OPTIONS: [&str; 4] =
    ["sierra2_4a", "floyd_steinberg", "bayer", "none"];
