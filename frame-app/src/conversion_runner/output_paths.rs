use std::{collections::HashSet, path::Path};

use frame_core::{args::build_output_path, types::ConversionTask};

/// Assigns deterministic suffixes to output names that would collide with an
/// earlier task or an existing filesystem entry.
pub fn disambiguate_output_paths(tasks: &mut [ConversionTask]) {
    let mut claimed_paths = HashSet::with_capacity(tasks.len());

    for task in tasks {
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
