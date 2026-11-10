//! Strict observations used to plan a request-local ALAC stream-copy repair.
//!
//! Parsing is not evidence of process completion or safe normal presentation.
//! Callers must establish those facts before authorizing any changed export.

mod manifest;
mod matching;
mod numeric;
mod output;
mod planner;
mod postflight;
mod prepared;
mod source;

pub use manifest::{
    HashedData, ManifestLimits, ManifestPacket, ManifestStream, ParsedCopyManifest,
    parse_copy_manifest,
};
pub use numeric::{PacketFlags, PacketTimestamp, Sha256Digest, TimeBase};
pub use output::{
    AlacOutputEvent, ParsedNormalAlac, ParsedPhysicalAudio, PhysicalAudioTrack,
    parse_normal_alac_scan, parse_physical_audio_scan,
};
pub use planner::{
    AlacOutputMap, AlacRepairPlan, AlacTailCandidate, alac_tail_candidates, plan_alac_repair,
};
pub use postflight::{
    AlacPresentationSummary, PhysicalAlacProof, validate_alac_physical_output,
    verify_alac_normal_output,
};
pub use prepared::{PreparedAlacPacket, PreparedAlacTrack, prepare_alac_tracks};
pub use source::{
    AlacStreamFacts, ParsedAlacSource, SampleTrimming, SourceAudioFrame, SourceAudioPacket,
    SourcePacketFrame, parse_alac_source_scan,
};

/// A failed observation cannot authorize an ALAC repair.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ObservationError {
    #[error("invalid ALAC observation: {0}")]
    Invalid(&'static str),
    #[error("ALAC observation exceeded its {0} limit")]
    Limit(&'static str),
    #[error("ALAC observation arithmetic overflow")]
    Overflow,
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod source_tests;

#[cfg(test)]
mod planner_tests;

#[cfg(test)]
mod postflight_tests;
