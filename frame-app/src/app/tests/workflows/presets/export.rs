use super::*;
use crate::conversion_runner::{conversion_task_from_file, run_conversion_task};
use crate::runtime_binaries::{ffmpeg_executable, ffprobe_executable};
use crate::settings::initialize_output_config;
use crate::source_metadata::probe_source_metadata;
use frame_core::events::ConversionEvent;
use std::path::Path;
use std::process::Command;

fn ffmpeg(dir: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new(ffmpeg_executable())
        .current_dir(dir)
        .args(["-nostdin", "-v", "error"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn make_sources(dir: &Path) {
    ffmpeg(
        dir,
        &[
            "-f",
            "lavfi",
            "-i",
            "color=size=160x96:rate=24",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:sample_rate=48000",
            "-t",
            "0.5",
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-map",
            "2:a",
            "-c:v",
            "libx264",
            "-c:a",
            "pcm_s16le",
            "-n",
            "multi.mkv",
        ],
    );
    for (name, streams) in [
        ("video-first.mkv", &["0:v", "0:a:0"][..]),
        ("audio-first.mkv", &["0:a:0", "0:v"]),
        ("silent.mkv", &["0:v"]),
    ] {
        let mut args = vec!["-i", "multi.mkv"];
        for stream in streams {
            args.extend(["-map", stream]);
        }
        args.extend(["-c", "copy", "-n", name]);
        ffmpeg(dir, &args);
    }
}

fn import(root: &mut FrameRoot, dir: &Path, name: &str) {
    let source = dir.join(name);
    let path = source.to_str().unwrap();
    let mut item = FileItem::from_path(name, path, 1);
    let metadata = probe_source_metadata(path).unwrap();
    initialize_output_config(&mut item.config, Some(&metadata));
    root.file_queue.add_file(item);
    root.source_metadata.mark_ready(name, metadata);
}

fn audio_stream_count(path: &str) -> usize {
    let output = Command::new(ffprobe_executable())
        .args([
            "-v",
            "error",
            "-select_streams",
            "a",
            "-show_entries",
            "stream=index",
            "-of",
            "csv=p=0",
            path,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().lines().count()
}

#[test]
#[ignore = "requires a built media worker and reference FFmpeg/FFprobe"]
fn reloaded_audio_preset_exports_each_targets_selected_tone() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path();
    make_sources(folder);
    let persistence = AppPersistence::from_settings_path(folder.join("settings.json"));
    let mut root = FrameRoot::new_with_persistence(persistence.clone());
    import(&mut root, folder, "video-first.mkv");
    assert_eq!(root.selected_config().unwrap().selected_audio_tracks, [1]);
    root.update_selected_config(|config| {
        config.container = "mkv".into();
        config.audio_codec = "flac".into();
        true
    });
    root.settings_ui.preset_name_draft = "Archive with audio".into();
    assert!(root.save_preset_from_draft());
    drop(root);

    let mut root = FrameRoot::new_with_persistence(persistence);
    let preset = root
        .presets
        .iter()
        .find(|preset| preset.name == "Archive with audio")
        .unwrap()
        .clone();
    assert!(preset.enable_source_audio);
    assert!(preset.config.selected_audio_tracks.is_empty());
    for name in [
        "video-first.mkv",
        "audio-first.mkv",
        "silent.mkv",
        "multi.mkv",
    ] {
        import(&mut root, folder, name);
    }
    let excluded = &mut root
        .file_queue
        .file_by_id_mut("audio-first.mkv")
        .unwrap()
        .config
        .selected_audio_tracks;
    assert_eq!(*excluded, [0]);
    excluded.clear();
    root.file_queue
        .file_by_id_mut("multi.mkv")
        .unwrap()
        .config
        .selected_audio_tracks = vec![2];
    assert!(root.apply_preset_to_all_pending(&preset));
    for (name, expected_track, stream) in [
        ("video-first.mkv", Some(1), "0:a:0"),
        ("audio-first.mkv", Some(0), "0:a:0"),
        ("silent.mkv", None, ""),
        ("multi.mkv", Some(2), "0:a:1"),
    ] {
        let file = root.file_queue.file_by_id(name).unwrap();
        assert_eq!(file.config.selected_audio_tracks, expected_track.as_slice());
        let mut output_path = None;
        run_conversion_task(
            conversion_task_from_file(file, folder.to_str().unwrap()),
            |event| {
                if let ConversionEvent::Completed(event) = event {
                    output_path = Some(event.output_path);
                }
            },
        )
        .unwrap();
        let output = output_path.unwrap();
        assert_eq!(
            audio_stream_count(&output),
            usize::from(expected_track.is_some()),
            "{name}"
        );
        if expected_track.is_some() {
            let decode = |path, map| {
                ffmpeg(
                    folder,
                    &[
                        "-i", path, "-map", map, "-ac", "1", "-ar", "48000", "-f", "s16le", "-",
                    ],
                )
            };
            let expected = decode(name, stream);
            assert!(expected.len() >= 47_000 && expected.iter().any(|&byte| byte != 0));
            assert!(
                decode(&output, "0:a:0") == expected,
                "{name} audio samples differ"
            );
        }
    }
}
