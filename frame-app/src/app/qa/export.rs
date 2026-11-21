use crate::settings::ConversionConfig;
use frame_core::types::{ConversionTask, ProbeMetadata};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExportRequest {
    input: String,
    output_directory: String,
    output_name: Option<String>,
    config: ConversionConfig,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPlan {
    #[serde(skip_serializing)]
    pub task: ConversionTask,
    pub normalized_config: ConversionConfig,
    pub output_path: String,
    pub args: Vec<String>,
}

/// # Errors
/// Returns an error for unreadable requests, failed probes, or invalid export settings.
pub fn read_plan(path: &Path) -> Result<ExportPlan, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("Could not read request: {error}"))?;
    let request: ExportRequest =
        serde_json::from_slice(&bytes).map_err(|error| format!("Invalid request JSON: {error}"))?;
    let probe = crate::source_probe::probe(
        &crate::runtime_binaries::ffprobe_executable(),
        Path::new(&request.input),
        || false,
    )
    .map_err(|error| error.to_string())?;
    let plan = prepare_export(&request, &probe)?;
    frame_core::args::validate_task_input(&plan.task.file_path, &plan.task.config)
        .map_err(|error| error.to_string())?;
    Ok(plan)
}

fn prepare_export(request: &ExportRequest, probe: &ProbeMetadata) -> Result<ExportPlan, String> {
    let metadata = crate::source_metadata::source_metadata_from_probe(probe.clone());
    let mut normalized_config = request.config.clone();
    crate::settings::normalize_output_config(&mut normalized_config, Some(&metadata));
    let mut file = crate::file_queue::FileItem::from_path("qa-export", &request.input, 0);
    file.config = normalized_config.clone();
    if let Some(name) = &request.output_name {
        file.output_name.clone_from(name);
    }
    let task =
        crate::conversion_runner::conversion_task_from_file(&file, &request.output_directory);
    let output_path = frame_core::args::build_output_path(
        &task.output_directory,
        &task.config.container,
        task.output_name.as_deref(),
    );
    let args =
        frame_core::args::build_ffmpeg_args(&task.file_path, &output_path, &task.config, probe)
            .map_err(|error| error.to_string())?;
    Ok(ExportPlan {
        task,
        normalized_config,
        output_path,
        args,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::ProcessingMode;
    use frame_core::types::AudioTrack;

    fn audio_probe() -> ProbeMetadata {
        ProbeMetadata {
            media_kind: "audio".into(),
            source_format: Some("nut".into()),
            duration: Some("2.213".into()),
            audio_codec: Some("opus".into()),
            audio_tracks: vec![AudioTrack {
                index: 0,
                codec: "opus".into(),
                channels: "1".into(),
                sample_rate: Some("48000".into()),
                ..AudioTrack::default()
            }],
            ..ProbeMetadata::default()
        }
    }

    fn copy_request() -> ExportRequest {
        ExportRequest {
            input: "/inputs/source.mov".into(),
            output_directory: "/output".into(),
            output_name: Some("audio.opus".into()),
            config: ConversionConfig {
                container: "opus".into(),
                audio_codec: "copy".into(),
                processing_mode: ProcessingMode::Copy,
                selected_audio_tracks: vec![0],
                ..ConversionConfig::default()
            },
        }
    }

    #[test]
    fn export_plan_uses_the_frame_copy_builder_without_mutating_the_request() {
        let request = copy_request();
        let before = request.config.clone();
        let plan = prepare_export(&request, &audio_probe()).expect("Opus copy must be planned");

        assert_eq!(plan.output_path, "/output/audio.opus");
        assert_eq!(plan.args.last(), Some(&plan.output_path));
        assert!(plan.args.windows(2).any(|pair| pair == ["-c", "copy"]));
        assert!(plan.args.windows(2).any(|pair| pair == ["-map", "0:0"]));
        assert!(plan.args.iter().any(|arg| arg == "-n"));
        assert_eq!(request.config, before);
    }

    #[test]
    fn export_plan_keeps_the_app_trim_clock() {
        let mut request = copy_request();
        request.config.start_time = Some("0.1370009".into());
        request.config.end_time = Some("0.5110042".into());

        let plan = prepare_export(&request, &audio_probe()).unwrap();

        assert!(
            plan.args
                .windows(2)
                .any(|pair| pair == ["-ss", "0.1370009"])
        );
        assert!(plan.args.windows(2).any(|pair| pair == ["-t", "0.374003"]));
    }

    #[test]
    fn export_plan_rejects_an_unknown_selected_track() {
        let mut request = copy_request();
        request.config.selected_audio_tracks = vec![99];

        let error = prepare_export(&request, &audio_probe()).unwrap_err();

        assert!(
            error.contains("99"),
            "invalid track must be diagnosed: {error}"
        );
    }
}
