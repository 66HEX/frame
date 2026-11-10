use super::process::{Failure, Helpers};
use super::scans::Scan;
use crate::capabilities::runtime::{Runtime, RuntimeIdentity};
use frame_core::alac_copy::{
    AlacRepairPlan, AlacTailCandidate, ManifestLimits, ParsedCopyManifest, PreparedAlacTrack,
    alac_tail_candidates, parse_alac_source_scan, parse_copy_manifest, plan_alac_repair,
    prepare_alac_tracks,
};
use frame_core::args::AlacCopyObserver;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Both ordinary and repaired results remain tied to what was observed. A
/// wrapper can establish a positive tail without becoming a native repair proof.
pub(super) struct Bindings {
    pub ffmpeg: Runtime,
    pub ffprobe: Option<Runtime>,
    dependencies: Vec<PathBuf>,
    source: Option<RuntimeIdentity>,
}

impl Bindings {
    pub fn new(observer: &AlacCopyObserver, executable: &str) -> Result<Self, Failure> {
        let dependencies = observer
            .dependencies()
            .iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        Ok(Self {
            ffmpeg: Runtime::resolve(executable).map_err(|error| Failure::invalid(&error))?,
            ffprobe: None,
            source: RuntimeIdentity::capture_paths(&dependencies),
            dependencies,
        })
    }

    pub fn check(&self, cancelled: &dyn Fn() -> bool) -> Result<(), Failure> {
        if cancelled() {
            return Err(Failure::Cancelled);
        }
        if !self.ffmpeg.identity_unchanged()
            || self
                .ffprobe
                .as_ref()
                .is_some_and(|runtime| !runtime.identity_unchanged())
            || self
                .source
                .as_ref()
                .is_some_and(|identity| !identity.matches_paths(&self.dependencies))
        {
            return Err(Failure::invalid(
                "source dependency or runtime changed during verification",
            ));
        }
        Ok(())
    }

    fn require_repair_runtime(&mut self, executable: &str) -> Result<(), Failure> {
        if self.source.is_none() || self.ffmpeg.identity.is_none() {
            return Err(Failure::invalid(
                "repair requires observed source dependencies and a native FFmpeg runtime",
            ));
        }
        let runtime = Runtime::resolve(executable).map_err(|error| Failure::invalid(&error))?;
        if runtime.identity.is_none() {
            return Err(Failure::invalid(
                "repair requires an observed native ffprobe runtime",
            ));
        }
        self.ffprobe = Some(runtime);
        Ok(())
    }

    pub fn probe_runtime(&self) -> Result<&Runtime, Failure> {
        self.ffprobe
            .as_ref()
            .ok_or_else(|| Failure::invalid("missing repair probe runtime"))
    }

    pub fn checkpoint(
        &self,
        message: &str,
        cancelled: &dyn Fn() -> bool,
        log: &mut dyn FnMut(String),
    ) -> Result<(), Failure> {
        log(format!("[INFO] ALAC verification: {message}"));
        self.check(cancelled)
    }
}

pub(super) enum Preparation {
    Ordinary(Box<Bindings>),
    Repair(Box<PreparedRepair>),
}

pub(super) struct PreparedRepair {
    pub bindings: Bindings,
    pub args: Vec<String>,
    pub candidate: ParsedCopyManifest,
    pub tracks: Vec<PreparedAlacTrack>,
    pub captured_bytes: usize,
}

pub(super) struct Preflight {
    pub result: Result<Preparation, Failure>,
    pub diagnostics: Vec<String>,
}

/// The caller builds the ordinary command first. Only a scoped core observer
/// enters here; no probe or callback runs for other output modes/codecs.
pub(super) fn prepare(
    observer: &AlacCopyObserver,
    input: &Path,
    ffmpeg: &str,
    ffprobe: &str,
    cancelled: &dyn Fn() -> bool,
    log: &mut dyn FnMut(String),
) -> Preflight {
    let mut helpers = Helpers::new(0);
    let result = Bindings::new(observer, ffmpeg).and_then(|bindings| {
        prepare_bound(
            observer,
            input,
            ffprobe,
            bindings,
            &mut helpers,
            cancelled,
            log,
        )
    });
    Preflight {
        result,
        diagnostics: helpers.diagnostics,
    }
}

fn prepare_bound(
    observer: &AlacCopyObserver,
    input: &Path,
    ffprobe: &str,
    mut bindings: Bindings,
    helpers: &mut Helpers,
    cancelled: &dyn Fn() -> bool,
    log: &mut dyn FnMut(String),
) -> Result<Preparation, Failure> {
    bindings.check(cancelled)?;
    bindings.checkpoint("observing copied packets", cancelled, log)?;
    let original = observe(observer, &bindings, &[], helpers, cancelled)?;
    bindings.checkpoint("copied packet observation complete", cancelled, log)?;
    let candidates = alac_tail_candidates(&original, &observer.alac_mappings())?;
    if candidates.is_empty() {
        return Ok(Preparation::Ordinary(Box::new(bindings)));
    }
    bindings.require_repair_runtime(ffprobe)?;
    bindings.check(cancelled)?;
    let plans = source_plans(
        input,
        &original,
        &candidates,
        &bindings,
        helpers,
        cancelled,
        log,
    )?;
    bindings.checkpoint("observing repaired packet timing", cancelled, log)?;
    let candidate = observe(observer, &bindings, &plans, helpers, cancelled)?;
    let tracks = prepare_alac_tracks(&original, &plans, &candidate)?;
    bindings.checkpoint("repaired packet timing verified", cancelled, log)?;
    Ok(Preparation::Repair(Box::new(PreparedRepair {
        args: observer.export_args(&plans)?,
        bindings,
        candidate,
        tracks,
        captured_bytes: helpers.captured_bytes,
    })))
}

fn observe(
    observer: &AlacCopyObserver,
    bindings: &Bindings,
    repairs: &[AlacRepairPlan],
    helpers: &mut Helpers,
    cancelled: &dyn Fn() -> bool,
) -> Result<ParsedCopyManifest, Failure> {
    bindings.check(cancelled)?;
    let args = observer.observer_args(repairs)?.into_iter().map(Into::into);
    let bytes = helpers.execute(&bindings.ffmpeg, args, true, cancelled)?;
    bindings.check(cancelled)?;
    let manifest = parse_copy_manifest(&bytes, ManifestLimits::default())?;
    observer.validate_manifest(&manifest)?;
    Ok(manifest)
}

fn source_plans(
    input: &Path,
    original: &ParsedCopyManifest,
    candidates: &[AlacTailCandidate],
    bindings: &Bindings,
    helpers: &mut Helpers,
    cancelled: &dyn Fn() -> bool,
    log: &mut dyn FnMut(String),
) -> Result<Vec<AlacRepairPlan>, Failure> {
    let mut seen = BTreeSet::new();
    let mut plans = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let index = candidate.mapping().source_stream_index;
        if !seen.insert(index) {
            continue;
        }
        bindings.checkpoint(&format!("decoding source track #{index}"), cancelled, log)?;
        let bytes = helpers.execute(
            bindings.probe_runtime()?,
            Scan::Source(index).args(input),
            false,
            cancelled,
        )?;
        bindings.check(cancelled)?;
        let source = parse_alac_source_scan(&bytes, index, ManifestLimits::default())?;
        for candidate in candidates
            .iter()
            .filter(|candidate| candidate.mapping().source_stream_index == index)
        {
            plans.push(plan_alac_repair(original, *candidate, &source)?);
        }
        // Release a full-source observation before reading the next source track.
        drop(source);
        drop(bytes);
        bindings.checkpoint(&format!("source track #{index} verified"), cancelled, log)?;
    }
    plans.sort_by_key(|plan| plan.mapping().audio_output_ordinal);
    Ok(plans)
}

#[cfg(test)]
mod tests;
