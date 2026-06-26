use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum FrameTextInputKind {
    MaxConcurrency,
    OutputName,
    AudioBitrate,
    VideoCustomWidth,
    VideoCustomHeight,
    VideoBitrate,
    GifLoop,
    MetadataTitle,
    MetadataArtist,
    MetadataAlbum,
    MetadataGenre,
    MetadataDate,
    MetadataComment,
    PresetName,
    SubtitleFontColorHex,
    SubtitleOutlineColorHex,
}

pub(in crate::app) struct FrameTextInputRuntime {
    pub(in crate::app) selected_range: Range<usize>,
    pub(in crate::app) selection_reversed: bool,
    pub(in crate::app) marked_range: Option<Range<usize>>,
    pub(in crate::app) last_layout: Option<ShapedLine>,
    pub(in crate::app) last_bounds: Option<Bounds<Pixels>>,
    pub(in crate::app) is_selecting: bool,
}

impl Default for FrameTextInputRuntime {
    fn default() -> Self {
        Self {
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
        }
    }
}
