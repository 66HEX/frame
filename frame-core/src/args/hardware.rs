//! The negotiated frame route returned by the installed media worker.

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct VideoRoute {
    pub encoder: String,
    pub decoder: String,
    pub filters: String,
    pub upload: bool,
    pub device: String,
    pub format: String,
    pub fallback: Option<String>,
}

impl VideoRoute {
    /// Reuse the probed decoder, adapter and pixel format when starting the export.
    pub fn pin(&self, args: &mut Vec<String>) {
        args.splice(
            0..0,
            [
                "-frame_decoder".to_owned(),
                self.decoder.clone(),
                "-frame_device".to_owned(),
                self.device.clone(),
                "-frame_filters".to_owned(),
                self.filters.clone(),
                "-frame_format".to_owned(),
                self.format.clone(),
            ],
        );
    }
}
