use super::{Bounds, FocusHandle, Pixels, Range, ShapedLine, TEXT_INPUT_CARET_WIDTH, Task, px};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::app) enum FrameTextInputKind {
    MaxConcurrency,
    OutputName,
    AudioBitrate,
    VideoCustomWidth,
    VideoCustomHeight,
    VideoBitrate,
    GifLoop,
    PreviewStartTime,
    PreviewEndTime,
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

impl FrameTextInputKind {
    pub(in crate::app) const ALL: [Self; 18] = [
        Self::MaxConcurrency,
        Self::OutputName,
        Self::AudioBitrate,
        Self::VideoCustomWidth,
        Self::VideoCustomHeight,
        Self::VideoBitrate,
        Self::GifLoop,
        Self::PreviewStartTime,
        Self::PreviewEndTime,
        Self::MetadataTitle,
        Self::MetadataArtist,
        Self::MetadataAlbum,
        Self::MetadataGenre,
        Self::MetadataDate,
        Self::MetadataComment,
        Self::PresetName,
        Self::SubtitleFontColorHex,
        Self::SubtitleOutlineColorHex,
    ];
}

pub(in crate::app) struct FrameTextInputRuntime {
    pub(in crate::app) selected_range: Range<usize>,
    pub(in crate::app) selection_reversed: bool,
    pub(in crate::app) marked_range: Option<Range<usize>>,
    pub(in crate::app) last_layout: Option<ShapedLine>,
    pub(in crate::app) last_bounds: Option<Bounds<Pixels>>,
    pub(in crate::app) scroll_x: Pixels,
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
            scroll_x: Pixels::ZERO,
            is_selecting: false,
        }
    }
}

pub(in crate::app) fn clamp_text_input_scroll_x(
    scroll_x: Pixels,
    content_width: Pixels,
    viewport_width: Pixels,
) -> Pixels {
    scroll_x.clamp(
        Pixels::ZERO,
        (content_width - viewport_width).max(Pixels::ZERO),
    )
}

pub(in crate::app) fn text_input_scroll_x_for_cursor(
    current: Pixels,
    cursor_x: Pixels,
    content_width: Pixels,
    viewport_width: Pixels,
) -> Pixels {
    let mut next = current;
    let trailing_edge = (viewport_width - px(TEXT_INPUT_CARET_WIDTH)).max(Pixels::ZERO);

    if cursor_x - next > trailing_edge {
        next = cursor_x - trailing_edge;
    } else if cursor_x < next {
        next = cursor_x;
    }

    clamp_text_input_scroll_x(next, content_width, viewport_width)
}

#[derive(Default)]
pub(in crate::app) struct FrameTextInputStore {
    max_concurrency: FrameTextInputRuntime,
    output_name: FrameTextInputRuntime,
    audio_bitrate: FrameTextInputRuntime,
    video_width: FrameTextInputRuntime,
    video_height: FrameTextInputRuntime,
    video_bitrate: FrameTextInputRuntime,
    gif_loop: FrameTextInputRuntime,
    preview_start_time: FrameTextInputRuntime,
    preview_end_time: FrameTextInputRuntime,
    metadata_title: FrameTextInputRuntime,
    metadata_artist: FrameTextInputRuntime,
    metadata_album: FrameTextInputRuntime,
    metadata_genre: FrameTextInputRuntime,
    metadata_date: FrameTextInputRuntime,
    metadata_comment: FrameTextInputRuntime,
    preset_name: FrameTextInputRuntime,
    subtitle_font_color: FrameTextInputRuntime,
    subtitle_outline_color: FrameTextInputRuntime,
}

impl FrameTextInputStore {
    pub(in crate::app) const fn runtime(&self, kind: FrameTextInputKind) -> &FrameTextInputRuntime {
        match kind {
            FrameTextInputKind::MaxConcurrency => &self.max_concurrency,
            FrameTextInputKind::OutputName => &self.output_name,
            FrameTextInputKind::AudioBitrate => &self.audio_bitrate,
            FrameTextInputKind::VideoCustomWidth => &self.video_width,
            FrameTextInputKind::VideoCustomHeight => &self.video_height,
            FrameTextInputKind::VideoBitrate => &self.video_bitrate,
            FrameTextInputKind::GifLoop => &self.gif_loop,
            FrameTextInputKind::PreviewStartTime => &self.preview_start_time,
            FrameTextInputKind::PreviewEndTime => &self.preview_end_time,
            FrameTextInputKind::MetadataTitle => &self.metadata_title,
            FrameTextInputKind::MetadataArtist => &self.metadata_artist,
            FrameTextInputKind::MetadataAlbum => &self.metadata_album,
            FrameTextInputKind::MetadataGenre => &self.metadata_genre,
            FrameTextInputKind::MetadataDate => &self.metadata_date,
            FrameTextInputKind::MetadataComment => &self.metadata_comment,
            FrameTextInputKind::PresetName => &self.preset_name,
            FrameTextInputKind::SubtitleFontColorHex => &self.subtitle_font_color,
            FrameTextInputKind::SubtitleOutlineColorHex => &self.subtitle_outline_color,
        }
    }

    pub(in crate::app) const fn runtime_mut(
        &mut self,
        kind: FrameTextInputKind,
    ) -> &mut FrameTextInputRuntime {
        match kind {
            FrameTextInputKind::MaxConcurrency => &mut self.max_concurrency,
            FrameTextInputKind::OutputName => &mut self.output_name,
            FrameTextInputKind::AudioBitrate => &mut self.audio_bitrate,
            FrameTextInputKind::VideoCustomWidth => &mut self.video_width,
            FrameTextInputKind::VideoCustomHeight => &mut self.video_height,
            FrameTextInputKind::VideoBitrate => &mut self.video_bitrate,
            FrameTextInputKind::GifLoop => &mut self.gif_loop,
            FrameTextInputKind::PreviewStartTime => &mut self.preview_start_time,
            FrameTextInputKind::PreviewEndTime => &mut self.preview_end_time,
            FrameTextInputKind::MetadataTitle => &mut self.metadata_title,
            FrameTextInputKind::MetadataArtist => &mut self.metadata_artist,
            FrameTextInputKind::MetadataAlbum => &mut self.metadata_album,
            FrameTextInputKind::MetadataGenre => &mut self.metadata_genre,
            FrameTextInputKind::MetadataDate => &mut self.metadata_date,
            FrameTextInputKind::MetadataComment => &mut self.metadata_comment,
            FrameTextInputKind::PresetName => &mut self.preset_name,
            FrameTextInputKind::SubtitleFontColorHex => &mut self.subtitle_font_color,
            FrameTextInputKind::SubtitleOutlineColorHex => &mut self.subtitle_outline_color,
        }
    }
}

#[derive(Default)]
pub(in crate::app) struct FrameTextInputFocusStore {
    max_concurrency: Option<FocusHandle>,
    output_name: Option<FocusHandle>,
    audio_bitrate: Option<FocusHandle>,
    video_width: Option<FocusHandle>,
    video_height: Option<FocusHandle>,
    video_bitrate: Option<FocusHandle>,
    gif_loop: Option<FocusHandle>,
    preview_start_time: Option<FocusHandle>,
    preview_end_time: Option<FocusHandle>,
    metadata_title: Option<FocusHandle>,
    metadata_artist: Option<FocusHandle>,
    metadata_album: Option<FocusHandle>,
    metadata_genre: Option<FocusHandle>,
    metadata_date: Option<FocusHandle>,
    metadata_comment: Option<FocusHandle>,
    preset_name: Option<FocusHandle>,
    subtitle_font_color: Option<FocusHandle>,
    subtitle_outline_color: Option<FocusHandle>,
}

impl FrameTextInputFocusStore {
    pub(in crate::app) const fn focus(&self, kind: FrameTextInputKind) -> Option<&FocusHandle> {
        match kind {
            FrameTextInputKind::MaxConcurrency => self.max_concurrency.as_ref(),
            FrameTextInputKind::OutputName => self.output_name.as_ref(),
            FrameTextInputKind::AudioBitrate => self.audio_bitrate.as_ref(),
            FrameTextInputKind::VideoCustomWidth => self.video_width.as_ref(),
            FrameTextInputKind::VideoCustomHeight => self.video_height.as_ref(),
            FrameTextInputKind::VideoBitrate => self.video_bitrate.as_ref(),
            FrameTextInputKind::GifLoop => self.gif_loop.as_ref(),
            FrameTextInputKind::PreviewStartTime => self.preview_start_time.as_ref(),
            FrameTextInputKind::PreviewEndTime => self.preview_end_time.as_ref(),
            FrameTextInputKind::MetadataTitle => self.metadata_title.as_ref(),
            FrameTextInputKind::MetadataArtist => self.metadata_artist.as_ref(),
            FrameTextInputKind::MetadataAlbum => self.metadata_album.as_ref(),
            FrameTextInputKind::MetadataGenre => self.metadata_genre.as_ref(),
            FrameTextInputKind::MetadataDate => self.metadata_date.as_ref(),
            FrameTextInputKind::MetadataComment => self.metadata_comment.as_ref(),
            FrameTextInputKind::PresetName => self.preset_name.as_ref(),
            FrameTextInputKind::SubtitleFontColorHex => self.subtitle_font_color.as_ref(),
            FrameTextInputKind::SubtitleOutlineColorHex => self.subtitle_outline_color.as_ref(),
        }
    }

    pub(in crate::app) const fn focus_mut(
        &mut self,
        kind: FrameTextInputKind,
    ) -> &mut Option<FocusHandle> {
        match kind {
            FrameTextInputKind::MaxConcurrency => &mut self.max_concurrency,
            FrameTextInputKind::OutputName => &mut self.output_name,
            FrameTextInputKind::AudioBitrate => &mut self.audio_bitrate,
            FrameTextInputKind::VideoCustomWidth => &mut self.video_width,
            FrameTextInputKind::VideoCustomHeight => &mut self.video_height,
            FrameTextInputKind::VideoBitrate => &mut self.video_bitrate,
            FrameTextInputKind::GifLoop => &mut self.gif_loop,
            FrameTextInputKind::PreviewStartTime => &mut self.preview_start_time,
            FrameTextInputKind::PreviewEndTime => &mut self.preview_end_time,
            FrameTextInputKind::MetadataTitle => &mut self.metadata_title,
            FrameTextInputKind::MetadataArtist => &mut self.metadata_artist,
            FrameTextInputKind::MetadataAlbum => &mut self.metadata_album,
            FrameTextInputKind::MetadataGenre => &mut self.metadata_genre,
            FrameTextInputKind::MetadataDate => &mut self.metadata_date,
            FrameTextInputKind::MetadataComment => &mut self.metadata_comment,
            FrameTextInputKind::PresetName => &mut self.preset_name,
            FrameTextInputKind::SubtitleFontColorHex => &mut self.subtitle_font_color,
            FrameTextInputKind::SubtitleOutlineColorHex => &mut self.subtitle_outline_color,
        }
    }

    pub(in crate::app) fn clear(&mut self, kind: FrameTextInputKind) {
        *self.focus_mut(kind) = None;
    }
}

pub(in crate::app) struct FrameTextInputUiState {
    pub(in crate::app) active: Option<FrameTextInputKind>,
    pub(in crate::app) runtimes: FrameTextInputStore,
    pub(in crate::app) focuses: FrameTextInputFocusStore,
    pub(in crate::app) cursor_visible: bool,
    pub(in crate::app) cursor_paused: bool,
    pub(in crate::app) cursor_epoch: usize,
    pub(in crate::app) cursor_task: Task<()>,
}

impl Default for FrameTextInputUiState {
    fn default() -> Self {
        Self {
            active: None,
            runtimes: FrameTextInputStore::default(),
            focuses: FrameTextInputFocusStore::default(),
            cursor_visible: false,
            cursor_paused: false,
            cursor_epoch: 0,
            cursor_task: Task::ready(()),
        }
    }
}
