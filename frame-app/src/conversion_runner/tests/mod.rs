#![expect(
    clippy::float_cmp,
    reason = "Conversion runner tests compare exact parsed metadata fixture values."
)]

use super::*;
use crate::settings::{
    AudioFiltersConfig, CropSettings, DeinterlaceMode, ExternalSubtitleTrack, FilterStrength,
    FilterValue, MetadataConfig, MetadataMode, ProcessingMode, VideoColorFiltersConfig,
    VideoFiltersConfig,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use support::*;

mod config;
mod controller;
mod ffmpeg;
mod paths;
mod progress;

mod support;
