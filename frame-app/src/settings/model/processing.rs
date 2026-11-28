use serde::{Deserialize, Serialize};

/// The two original values stay compatible with saved presets and queues.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProcessingMode {
    #[default]
    Reencode,
    Copy,
    CopyVideo,
    CopyAudio,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessingStream {
    Video,
    Audio,
}

impl ProcessingMode {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Reencode => "reencode",
            Self::Copy => "copy",
            Self::CopyVideo => "copyVideo",
            Self::CopyAudio => "copyAudio",
        }
    }

    #[must_use]
    pub const fn copies_video(self) -> bool {
        matches!(self, Self::Copy | Self::CopyVideo)
    }

    #[must_use]
    pub const fn copies_audio(self) -> bool {
        matches!(self, Self::Copy | Self::CopyAudio)
    }

    #[must_use]
    pub const fn with_copy(self, stream: ProcessingStream, copy: bool) -> Self {
        let modes = match stream {
            ProcessingStream::Video => (copy, self.copies_audio()),
            ProcessingStream::Audio => (self.copies_video(), copy),
        };
        match modes {
            (false, false) => Self::Reencode,
            (true, true) => Self::Copy,
            (true, false) => Self::CopyVideo,
            (false, true) => Self::CopyAudio,
        }
    }
}
