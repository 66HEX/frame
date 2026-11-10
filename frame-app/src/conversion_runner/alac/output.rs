use super::prepare::PreparedRepair;
use super::process::{Failure, Helpers};
use super::scans::Scan;
use super::staging::StagedOutput;
use frame_core::alac_copy::{
    ManifestLimits, parse_normal_alac_scan, parse_physical_audio_scan,
    validate_alac_physical_output, verify_alac_normal_output,
};

pub(super) struct Postflight {
    pub result: Result<(), Failure>,
    pub diagnostics: Vec<String>,
}

pub(super) fn verify(
    prepared: &PreparedRepair,
    stage: &StagedOutput,
    cancelled: &dyn Fn() -> bool,
    log: &mut dyn FnMut(String),
) -> Postflight {
    // Final conversion time is excluded. All postflight helpers share this new
    // phase budget, while serialized evidence keeps the request-wide accounting.
    let mut helpers = Helpers::new(prepared.captured_bytes);
    let result = verify_with_helpers(prepared, stage, &mut helpers, cancelled, log);
    Postflight {
        result,
        diagnostics: helpers.diagnostics,
    }
}

fn check(
    prepared: &PreparedRepair,
    stage: &StagedOutput,
    cancelled: &dyn Fn() -> bool,
) -> Result<(), Failure> {
    prepared.bindings.check(cancelled)?;
    stage
        .ensure_unchanged()
        .map_err(|error| Failure::invalid(&error))
}

fn checkpoint(
    prepared: &PreparedRepair,
    stage: &StagedOutput,
    message: &str,
    cancelled: &dyn Fn() -> bool,
    log: &mut dyn FnMut(String),
) -> Result<(), Failure> {
    prepared.bindings.checkpoint(message, cancelled, log)?;
    stage
        .ensure_unchanged()
        .map_err(|error| Failure::invalid(&error))
}

fn verify_with_helpers(
    prepared: &PreparedRepair,
    stage: &StagedOutput,
    helpers: &mut Helpers,
    cancelled: &dyn Fn() -> bool,
    log: &mut dyn FnMut(String),
) -> Result<(), Failure> {
    check(prepared, stage, cancelled)?;
    checkpoint(
        prepared,
        stage,
        "inspecting physical output packets",
        cancelled,
        log,
    )?;
    let bytes = helpers.execute(
        prepared.bindings.probe_runtime()?,
        Scan::Physical.args(stage.path()),
        false,
        cancelled,
    )?;
    check(prepared, stage, cancelled)?;
    let physical = parse_physical_audio_scan(&bytes, ManifestLimits::default())?;
    let proofs = validate_alac_physical_output(&prepared.candidate, &prepared.tracks, &physical)?;
    drop(physical);
    drop(bytes);
    checkpoint(
        prepared,
        stage,
        "physical output packets verified",
        cancelled,
        log,
    )?;
    for proof in proofs {
        let index = proof.actual_stream_index();
        checkpoint(
            prepared,
            stage,
            &format!("decoding output track #{index}"),
            cancelled,
            log,
        )?;
        let bytes = helpers.execute(
            prepared.bindings.probe_runtime()?,
            Scan::Normal(index).args(stage.path()),
            false,
            cancelled,
        )?;
        check(prepared, stage, cancelled)?;
        let normal = parse_normal_alac_scan(&bytes, index, ManifestLimits::default())?;
        verify_alac_normal_output(&proof, &normal)?;
        drop(normal);
        drop(bytes);
        checkpoint(
            prepared,
            stage,
            &format!("output track #{index} verified"),
            cancelled,
            log,
        )?;
    }
    Ok(())
}
