use crate::settings::model::defaults::{
    DEFAULT_AUDIO_FILTER_DE_ESSER, DEFAULT_AUDIO_FILTER_HIGH_PASS, DEFAULT_AUDIO_FILTER_LIMITER,
    DEFAULT_AUDIO_FILTER_LOW_PASS, DEFAULT_AUDIO_FILTER_NOISE_REDUCTION, DEFAULT_AUDIO_VOLUME,
    DEFAULT_VIDEO_FILTER_BLUR, DEFAULT_VIDEO_FILTER_DEBAND, DEFAULT_VIDEO_FILTER_SHARPEN,
    DEFAULT_VIDEO_FILTER_TEMPERATURE, DEFAULT_VIDEO_FILTER_VIGNETTE,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FilterValue<T> {
    pub enabled: bool,
    pub value: T,
}

impl<T: Default> Default for FilterValue<T> {
    fn default() -> Self {
        Self {
            enabled: false,
            value: T::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FilterStrength {
    Low,
    #[default]
    Medium,
    High,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DeinterlaceMode {
    #[default]
    Off,
    Auto,
    On,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct VideoColorFiltersConfig {
    pub brightness: FilterValue<i32>,
    pub contrast: FilterValue<u32>,
    pub saturation: FilterValue<u32>,
    pub gamma: FilterValue<u32>,
}

impl Default for VideoColorFiltersConfig {
    fn default() -> Self {
        Self {
            brightness: FilterValue {
                enabled: false,
                value: 0,
            },
            contrast: FilterValue {
                enabled: false,
                value: 100,
            },
            saturation: FilterValue {
                enabled: false,
                value: 100,
            },
            gamma: FilterValue {
                enabled: false,
                value: 100,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct VideoFiltersConfig {
    pub color: VideoColorFiltersConfig,
    pub hue: FilterValue<i32>,
    pub temperature: FilterValue<u32>,
    pub sharpen: FilterValue<u32>,
    pub gaussian_blur: FilterValue<u32>,
    pub denoise_enabled: bool,
    pub denoise_strength: FilterStrength,
    pub deband: FilterValue<u32>,
    pub vignette: FilterValue<u32>,
    pub grayscale: bool,
    pub deinterlace: DeinterlaceMode,
}

impl Default for VideoFiltersConfig {
    fn default() -> Self {
        Self {
            color: VideoColorFiltersConfig::default(),
            hue: FilterValue {
                enabled: false,
                value: 0,
            },
            temperature: FilterValue {
                enabled: false,
                value: DEFAULT_VIDEO_FILTER_TEMPERATURE,
            },
            sharpen: FilterValue {
                enabled: false,
                value: DEFAULT_VIDEO_FILTER_SHARPEN,
            },
            gaussian_blur: FilterValue {
                enabled: false,
                value: DEFAULT_VIDEO_FILTER_BLUR,
            },
            denoise_enabled: false,
            denoise_strength: FilterStrength::Medium,
            deband: FilterValue {
                enabled: false,
                value: DEFAULT_VIDEO_FILTER_DEBAND,
            },
            vignette: FilterValue {
                enabled: false,
                value: DEFAULT_VIDEO_FILTER_VIGNETTE,
            },
            grayscale: false,
            deinterlace: DeinterlaceMode::Off,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AudioFiltersConfig {
    pub compressor_enabled: bool,
    pub compressor_strength: FilterStrength,
    pub limiter: FilterValue<i32>,
    pub bass: FilterValue<i32>,
    pub treble: FilterValue<i32>,
    pub high_pass: FilterValue<u32>,
    pub low_pass: FilterValue<u32>,
    pub noise_reduction: FilterValue<u32>,
    pub de_esser: FilterValue<u32>,
    pub stereo_width: FilterValue<u32>,
}

impl Default for AudioFiltersConfig {
    fn default() -> Self {
        Self {
            compressor_enabled: false,
            compressor_strength: FilterStrength::Medium,
            limiter: FilterValue {
                enabled: false,
                value: DEFAULT_AUDIO_FILTER_LIMITER,
            },
            bass: FilterValue {
                enabled: false,
                value: 0,
            },
            treble: FilterValue {
                enabled: false,
                value: 0,
            },
            high_pass: FilterValue {
                enabled: false,
                value: DEFAULT_AUDIO_FILTER_HIGH_PASS,
            },
            low_pass: FilterValue {
                enabled: false,
                value: DEFAULT_AUDIO_FILTER_LOW_PASS,
            },
            noise_reduction: FilterValue {
                enabled: false,
                value: DEFAULT_AUDIO_FILTER_NOISE_REDUCTION,
            },
            de_esser: FilterValue {
                enabled: false,
                value: DEFAULT_AUDIO_FILTER_DE_ESSER,
            },
            stereo_width: FilterValue {
                enabled: false,
                value: DEFAULT_AUDIO_VOLUME,
            },
        }
    }
}
