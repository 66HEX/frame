use std::{collections::HashSet, path::Path};

use frame_core::{args::build_output_path, types::ConversionTask};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PreparedOutputTarget {
    pub result_path: String,
    pub ffmpeg_sink_path: String,
    pub directory_to_create: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct PreparedConversionTask {
    pub task: ConversionTask,
    pub output: PreparedOutputTarget,
}

/// Resolves filesystem and in-batch collisions and prepares the distinct path
/// reported to the app and path consumed by `FFmpeg`.
pub(super) fn prepare_conversion_tasks(tasks: Vec<ConversionTask>) -> Vec<PreparedConversionTask> {
    let mut claimed_paths = HashSet::with_capacity(tasks.len());
    let mut prepared = Vec::with_capacity(tasks.len());

    for mut task in tasks {
        let desired_path = task_output_path(&task);
        let output_stem = output_stem_from_path(&desired_path);
        let is_sequence = task.config.image_output_mode == "sequence";

        for suffix in 1_u64.. {
            let candidate_name = if suffix == 1 {
                output_stem.to_string()
            } else {
                format!("{output_stem}_{suffix}")
            };
            let output = if is_sequence {
                sequence_output_target(&task, output_stem, suffix)
            } else {
                let result_path = build_output_path(
                    &task.output_directory,
                    &task.config.container,
                    Some(&candidate_name),
                );
                PreparedOutputTarget {
                    ffmpeg_sink_path: result_path.clone(),
                    result_path,
                    directory_to_create: None,
                }
            };

            if output_path_is_available(&output.result_path, &claimed_paths) {
                claimed_paths.insert(output_path_key(&output.result_path));
                if !is_sequence && suffix > 1 {
                    task.output_name = Some(candidate_name);
                }
                prepared.push(PreparedConversionTask { task, output });
                break;
            }
        }
    }

    prepared
}

fn sequence_output_target(
    task: &ConversionTask,
    output_stem: &str,
    suffix: u64,
) -> PreparedOutputTarget {
    let directory_name = if suffix == 1 {
        format!("{output_stem}_frames")
    } else {
        format!("{output_stem}_frames_{suffix}")
    };
    let result_file_shape = build_output_path(
        &task.output_directory,
        &task.config.container,
        Some(&directory_name),
    );
    let extension = format!(".{}", task.config.container);
    let result_path = result_file_shape
        .strip_suffix(&extension)
        .unwrap_or(&result_file_shape)
        .to_string();
    let ffmpeg_sink_path =
        build_output_path(&result_path, &task.config.container, Some("frame_%06d"));

    PreparedOutputTarget {
        ffmpeg_sink_path,
        result_path: result_path.clone(),
        directory_to_create: Some(result_path),
    }
}

/// Assigns deterministic suffixes to output names that would collide with an
/// earlier task or an existing filesystem entry.
pub fn disambiguate_output_paths(tasks: &mut [ConversionTask]) {
    let mut claimed_paths = HashSet::with_capacity(tasks.len());

    for task in tasks {
        if task.config.image_output_mode == "sequence" {
            continue;
        }
        let desired_path = task_output_path(task);
        if output_path_is_available(&desired_path, &claimed_paths) {
            claimed_paths.insert(output_path_key(&desired_path));
            continue;
        }

        let output_stem = output_stem_from_path(&desired_path);
        for suffix in 2_u64.. {
            let output_name = format!("{output_stem}_{suffix}");
            let candidate_path = build_output_path(
                &task.output_directory,
                &task.config.container,
                Some(&output_name),
            );
            if output_path_is_available(&candidate_path, &claimed_paths) {
                claimed_paths.insert(output_path_key(&candidate_path));
                task.output_name = Some(output_name);
                break;
            }
        }
    }
}

fn task_output_path(task: &ConversionTask) -> String {
    build_output_path(
        &task.output_directory,
        &task.config.container,
        task.output_name.as_deref(),
    )
}

fn output_path_is_available(path: &str, claimed_paths: &HashSet<String>) -> bool {
    !claimed_paths.contains(&output_path_key(path)) && !Path::new(path).exists()
}

fn output_stem_from_path(path: &str) -> &str {
    path.rsplit(['/', '\\'])
        .next()
        .and_then(|file_name| file_name.rsplit_once('.').map(|(stem, _)| stem))
        .filter(|stem| !stem.is_empty())
        .unwrap_or("output_converted")
}

fn output_path_key(path: &str) -> String {
    path.to_lowercase()
}
