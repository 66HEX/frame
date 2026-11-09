use super::{
    build_ffmpeg_args, collect_selected_audio_tracks, collect_selected_subtitle_tracks,
    container_supports_subtitles, has_overlay, is_audio_only_container, is_copy_mode,
};
use crate::alac_copy::{AlacOutputMap, AlacRepairPlan, ObservationError, ParsedCopyManifest};
use crate::error::ConversionError;
use crate::types::{ConversionConfig, ProbeMetadata};
use std::collections::BTreeSet;

/// Actual audio mapping order, including codecs that need no ALAC repair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CopyAudioOutput {
    pub mapping: AlacOutputMap,
    pub codec: String,
}

/// A validated ordinary Copy command with one explicit replaceable output sink.
#[derive(Clone, Debug)]
pub struct AlacCopyObserver {
    ordinary: Vec<String>,
    audio: Vec<CopyAudioOutput>,
    stream_types: Vec<&'static str>,
    dependencies: Vec<String>,
}

impl AlacCopyObserver {
    #[must_use]
    pub fn audio_outputs(&self) -> &[CopyAudioOutput] {
        &self.audio
    }

    #[must_use]
    pub fn alac_mappings(&self) -> Vec<AlacOutputMap> {
        self.audio
            .iter()
            .filter(|output| output.codec == "alac")
            .map(|output| output.mapping)
            .collect()
    }

    #[must_use]
    pub fn dependencies(&self) -> &[String] {
        &self.dependencies
    }

    /// Checks the header against actual mapping decisions, before tail analysis.
    ///
    /// # Errors
    /// Returns an error if a mapped stream is omitted, added, reordered or changed.
    pub fn validate_manifest(&self, manifest: &ParsedCopyManifest) -> Result<(), ObservationError> {
        if manifest.streams.len() != self.stream_types.len()
            || manifest
                .streams
                .iter()
                .zip(&self.stream_types)
                .any(|(stream, expected)| stream.media_type != *expected)
        {
            return Err(ObservationError::Invalid("actual copy map headers"));
        }
        for output in &self.audio {
            let index = usize::try_from(output.mapping.output_stream_index)
                .map_err(|_| ObservationError::Overflow)?;
            if manifest.streams.get(index).is_none_or(|stream| {
                stream.index != output.mapping.output_stream_index || stream.codec != output.codec
            }) {
                return Err(ObservationError::Invalid("actual copied audio codec"));
            }
        }
        Ok(())
    }

    /// Preserves the validated command, changing only its known final sink.
    ///
    /// The execution layer attaches a retained OS-path completion sidecar.
    ///
    /// # Errors
    /// Returns an error for duplicate repairs or a repair for another output map.
    pub fn observer_args(
        &self,
        repairs: &[AlacRepairPlan],
    ) -> Result<Vec<String>, ObservationError> {
        let mut args: Vec<_> = ["-nostdin", "-hide_banner", "-loglevel", "error", "-xerror"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        args.extend_from_slice(&self.ordinary[..self.ordinary.len() - 2]);
        self.append_repairs(&mut args, repairs)?;
        args.extend(
            [
                "-hash",
                "sha256",
                "-format_version",
                "2",
                "-f",
                "framehash",
                "pipe:1",
            ]
            .map(str::to_owned),
        );
        Ok(args)
    }

    /// Keeps the user's output unchanged; the runner may substitute an owned OS path.
    ///
    /// # Errors
    /// Returns an error for duplicate repairs or a repair for another output map.
    pub fn export_args(&self, repairs: &[AlacRepairPlan]) -> Result<Vec<String>, ObservationError> {
        let split = self.ordinary.len() - 2;
        let mut args = self.ordinary[..split].to_vec();
        self.append_repairs(&mut args, repairs)?;
        args.extend_from_slice(&self.ordinary[split..]);
        Ok(args)
    }

    fn append_repairs(
        &self,
        args: &mut Vec<String>,
        repairs: &[AlacRepairPlan],
    ) -> Result<(), ObservationError> {
        let mut ordinals = BTreeSet::new();
        for repair in repairs {
            let mapping = repair.mapping();
            if !ordinals.insert(mapping.audio_output_ordinal)
                || !self
                    .audio
                    .iter()
                    .any(|output| output.codec == "alac" && output.mapping == mapping)
            {
                return Err(ObservationError::Invalid("repair output mapping"));
            }
            // The validated Copy branch currently emits no audio BSF. This is
            // its sole audio-BSF composition site; subtitle BSFs stay untouched.
            args.extend(repair.bitstream_filter_args());
        }
        Ok(())
    }
}

/// Builds the first-scope ALAC observer from the same ordinary argument builder.
///
/// Other modes/containers and Copy without selected ALAC return `None` after
/// ordinary command validation. Positive-duration tracks remain ordinary later.
///
/// # Errors
/// Returns the ordinary builder's errors, or an error if its scoped output-sink
/// contract changes. No middle argument strings are interpreted as option names.
pub fn build_alac_copy_observer(
    input: &str,
    output: &str,
    config: &ConversionConfig,
    probe: &ProbeMetadata,
) -> Result<Option<AlacCopyObserver>, ConversionError> {
    let ordinary = build_ffmpeg_args(input, output, config, probe)?;
    if !is_copy_mode(config)
        || !["mov", "mp4", "m4a"]
            .iter()
            .any(|container| config.container.eq_ignore_ascii_case(container))
    {
        return Ok(None);
    }
    let audio_tracks = collect_selected_audio_tracks(config, probe)?;
    if !audio_tracks.iter().any(|track| track.codec == "alac") {
        return Ok(None);
    }
    if ordinary.len() < 2
        || ordinary[ordinary.len() - 2] != "-n"
        || ordinary.last().map(String::as_str) != Some(output)
    {
        return Err(ConversionError::Worker(
            "Unexpected ALAC Copy output sink".into(),
        ));
    }
    let video_count = usize::from(
        !is_audio_only_container(&config.container) && probe.video_stream_index.is_some(),
    );
    let mut stream_types = vec!["video"; video_count];
    let mut audio = Vec::with_capacity(audio_tracks.len());
    for (ordinal, track) in audio_tracks.into_iter().enumerate() {
        let mapping = AlacOutputMap {
            source_stream_index: track.index,
            output_stream_index: u32::try_from(stream_types.len())
                .map_err(|_| ConversionError::Worker("Too many output streams".into()))?,
            audio_output_ordinal: u32::try_from(ordinal)
                .map_err(|_| ConversionError::Worker("Too many audio streams".into()))?,
        };
        stream_types.push("audio");
        audio.push(CopyAudioOutput {
            mapping,
            codec: track.codec.clone(),
        });
    }
    if container_supports_subtitles(&config.container) {
        let count = collect_selected_subtitle_tracks(config, probe)?.len()
            + config.external_subtitle_tracks.len();
        stream_types.extend(std::iter::repeat_n("subtitle", count));
    }
    let mut dependencies = vec![input.to_owned()];
    if has_overlay(config)
        && let Some(overlay) = &config.overlay
    {
        dependencies.push(overlay.path.clone());
    }
    dependencies.extend(
        config
            .external_subtitle_tracks
            .iter()
            .map(|track| track.path.clone()),
    );
    Ok(Some(AlacCopyObserver {
        ordinary,
        audio,
        stream_types,
        dependencies,
    }))
}
