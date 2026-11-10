use crate::settings::model::processing::ProcessingMode;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputModeOption {
    pub mode: ProcessingMode,
    pub label: &'static str,
    pub hint: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputContainerOption {
    pub container: String,
    pub is_selected: bool,
    pub is_disabled: bool,
    pub disabled_reason: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioCodecOption {
    pub codec: &'static str,
    pub label: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
    pub disabled_reason: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioChannelOption {
    pub id: &'static str,
    pub label: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VideoCodecOption {
    pub codec: &'static str,
    pub label: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
    pub disabled_reason: Option<&'static str>,
    pub allow_software_fallback: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VideoPixelFormatOption {
    pub id: &'static str,
    pub label: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
    pub caption: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VideoPresetOption {
    pub preset: &'static str,
    pub label: &'static str,
    pub caption: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ImageEncodingOption {
    pub id: &'static str,
    pub label: &'static str,
    pub caption: &'static str,
    pub is_selected: bool,
    pub is_disabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioQualityRange {
    pub min: u32,
    pub max: u32,
    pub lower_is_better: bool,
    pub default_value: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioTrackOption {
    pub copy_error: Option<&'static str>,
    pub index: u32,
    pub index_label: String,
    pub codec: String,
    pub detail: String,
    pub bitrate: String,
    pub is_selected: bool,
    pub is_disabled: bool,
}
