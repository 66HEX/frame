use super::*;

#[test]
#[ignore = "requires the worker, FFmpeg and FFprobe"]
fn damaged_aac_access_units_preserve_recoverable_audio_and_strict_failures() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("decoder_recovery")?;
    let clean = sandbox.path("clean.m4a");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "anullsrc=r=48000:cl=mono:d=0.96",
            "-c:a",
            "aac",
            &path_arg(&clean),
        ]),
    )?;
    let packets = run_tool_output(
        &tools.ffprobe,
        &args(&[
            "-v",
            "error",
            "-show_packets",
            "-show_entries",
            "packet=pos,size",
            "-of",
            "json",
            &path_arg(&clean),
        ]),
    )?;
    let metadata: serde_json::Value =
        serde_json::from_slice(&packets).map_err(|e| e.to_string())?;
    let packets = metadata["packets"].as_array().unwrap();
    assert!(packets.len() > 30);
    let damaged = sandbox.path("damaged.m4a");
    for count in [1, packets.len() / 2, packets.len() * 4 / 5] {
        let mut bytes = fs::read(&clean).map_err(|e| e.to_string())?;
        for packet in packets.iter().skip(2).take(count) {
            let index = |key: &str| packet[key].as_str().unwrap().parse::<usize>().unwrap();
            let pos = index("pos");
            bytes[pos..pos + index("size")].fill(0);
        }
        fs::write(&damaged, bytes).map_err(|e| e.to_string())?;
        let mut plan = args(&[
            "-loglevel",
            "error",
            "-i",
            &path_arg(&damaged),
            "-map",
            "0:0",
            "-c:a",
            "pcm_s16le",
            "-f",
            "s16le",
            "pipe:1",
        ]);
        let expected = run_tool_output(&tools.ffmpeg, &plan);
        let actual = tools.native("convert", &plan);
        if count * 3 > packets.len() * 2 {
            assert!(
                expected.is_err() && actual.is_err(),
                "too many damaged access units"
            );
        } else {
            let expected = expected?;
            assert!(!expected.is_empty());
            assert_bytes_match(&actual?, &expected, "surviving decoded samples")?;
        }
        plan.insert(0, "-xerror".into());
        assert!(run_tool_output(&tools.ffmpeg, &plan).is_err());
        assert!(tools.native("convert", &plan).is_err());
    }
    Ok(())
}
