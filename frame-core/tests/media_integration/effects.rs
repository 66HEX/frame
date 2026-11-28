//! Exercise each persisted effect independently through the normal export plan.
use super::*;
use frame_core::types::{
    AudioFiltersConfig, DeinterlaceMode, FilterStrength, FilterValue, VideoFiltersConfig,
};

const fn enabled<T>(value: T) -> FilterValue<T> {
    FilterValue {
        enabled: true,
        value,
    }
}

type Effect<T> = (&'static str, fn(&mut T));

fn video_effects() -> Vec<Effect<VideoFiltersConfig>> {
    vec![
        ("baseline", |_| {}),
        ("brightness", |f| f.color.brightness = enabled(29)),
        ("contrast", |f| f.color.contrast = enabled(155)),
        ("saturation", |f| f.color.saturation = enabled(155)),
        ("gamma", |f| f.color.gamma = enabled(180)),
        ("hue", |f| f.hue = enabled(49)),
        ("temperature", |f| f.temperature = enabled(4000)),
        ("sharpen", |f| f.sharpen = enabled(70)),
        ("gaussian-blur", |f| f.gaussian_blur = enabled(34)),
        ("stabilization", |f| f.stabilization_enabled = true),
        ("denoise-low", |f| {
            f.denoise_enabled = true;
            f.denoise_strength = FilterStrength::Low;
        }),
        ("denoise-medium", |f| f.denoise_enabled = true),
        ("denoise-high", |f| {
            f.denoise_enabled = true;
            f.denoise_strength = FilterStrength::High;
        }),
        ("deband", |f| f.deband = enabled(82)),
        ("vignette", |f| f.vignette = enabled(65)),
        ("grayscale", |f| f.grayscale = true),
        ("deinterlace-auto", |f| {
            f.deinterlace = DeinterlaceMode::Auto;
        }),
        ("deinterlace-on", |f| f.deinterlace = DeinterlaceMode::On),
    ]
}

fn audio_effects() -> Vec<Effect<AudioFiltersConfig>> {
    vec![
        ("baseline", |_| {}),
        ("compressor-low", |f| {
            f.compressor_enabled = true;
            f.compressor_strength = FilterStrength::Low;
        }),
        ("compressor-medium", |f| f.compressor_enabled = true),
        ("compressor-high", |f| {
            f.compressor_enabled = true;
            f.compressor_strength = FilterStrength::High;
        }),
        ("limiter", |f| f.limiter = enabled(-10)),
        ("bass", |f| f.bass = enabled(10)),
        ("treble", |f| f.treble = enabled(-9)),
        ("high-pass", |f| f.high_pass = enabled(700)),
        ("low-pass", |f| f.low_pass = enabled(2500)),
        ("noise-reduction", |f| f.noise_reduction = enabled(20)),
        ("de-esser", |f| f.de_esser = enabled(65)),
        ("stereo-width", |f| f.stereo_width = enabled(35)),
        ("normalize", |_| {}),
        ("volume", |_| {}),
    ]
}

fn decoded_pair(
    tools: &Toolchain,
    source: &Path,
    config: &ConversionConfig,
    sandbox: &Sandbox,
    name: &str,
    decode_args: &[&str],
) -> TestResult<[Vec<u8>; 2]> {
    let metadata = probe_media(tools, source)?;
    let input = path_arg(source);
    validate_task_input(&input, config).map_err(|e| e.to_string())?;
    let mut results = [Vec::new(), Vec::new()];
    for (index, result) in results.iter_mut().enumerate() {
        let output = sandbox.path(&format!("{name}-{index}.{}", config.container));
        let plan = build_ffmpeg_args(&input, &path_arg(&output), config, &metadata)
            .map_err(|e| e.to_string())?;
        if index == 0 {
            run_tool(&tools.ffmpeg, &plan)?;
        } else {
            tools.native("convert", &plan)?;
        }
        let mut decode = args(&["-v", "error", "-i", &path_arg(&output)]);
        decode.extend(args(decode_args));
        decode.push("pipe:1".into());
        *result = run_tool_output(&tools.ffmpeg, &decode)?;
    }
    Ok(results)
}

#[test]
#[ignore = "requires fixture tools; run with --ignored"]
fn every_video_effect_matches_the_reference_raster() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("video_effects")?;
    let source = sandbox.path("shaking-interlaced.mkv");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-f",
            "lavfi",
            "-i",
            "nullsrc=s=320x224:r=15:d=0.8,geq=lum='16+205*gt(mod(floor(X/6)*31+floor(Y/5)*17+floor(X/13)*floor(Y/12),13),6)+mod(X*3+Y*7,11)':cb='64+mod(X,128)':cr='192-mod(Y,128)'",
            "-vf",
            "crop=256:192:x=32+8*sin(n*0.4):y=16+7*cos(n*0.6),setfield=tff",
            "-c:v",
            "ffv1",
            &path_arg(&source),
        ]),
    )?;
    let mut baseline = Vec::new();
    for (name, apply) in video_effects() {
        let mut config = video_config("mkv", "libx264", "aac");
        config.crf = 0;
        config.pixel_format = "yuv420p".into();
        apply(&mut config.video_filters);
        let [expected, actual] = decoded_pair(
            &tools,
            &source,
            &config,
            &sandbox,
            name,
            &[
                "-map",
                "0:v:0",
                "-fps_mode",
                "passthrough",
                "-pix_fmt",
                "yuv420p",
                "-f",
                "rawvideo",
            ],
        )?;
        assert!(
            !expected.is_empty() && actual.len() == expected.len(),
            "{name} frame count"
        );
        // Deband uses spatial sampling and some older builds round individual
        // pixels differently. Keep the RMS error below one 8-bit level.
        let error: u64 = expected
            .iter()
            .zip(&actual)
            .map(|(a, b)| u64::from(a.abs_diff(*b)).pow(2))
            .sum();
        assert!(error <= expected.len() as u64, "{name} raster mismatch");
        if name == "baseline" {
            baseline = expected;
        } else {
            assert!(
                expected != baseline,
                "{name} did not change the reference fixture"
            );
        }
    }
    Ok(())
}

#[test]
#[ignore = "requires fixture tools; run with --ignored"]
fn every_audio_effect_matches_the_reference_waveform() -> TestResult {
    let tools = Toolchain::discover()?;
    let sandbox = Sandbox::new("audio_effects")?;
    let source = sandbox.path("stereo.wav");
    run_tool(
        &tools.ffmpeg,
        &args(&[
            "-f",
            "lavfi",
            "-i",
            "aevalsrc=0.57*sin(2*PI*100*t)+0.36*sin(2*PI*7200*t)|0.41*cos(2*PI*300*t)+0.4*sin(2*PI*6400*t):s=48000:d=0.7",
            "-c:a",
            "pcm_s16le",
            &path_arg(&source),
        ]),
    )?;
    let mut baseline = Vec::new();
    for (name, apply) in audio_effects() {
        let mut config = audio_config("wav", "pcm_s16le");
        config.selected_audio_tracks = vec![0];
        apply(&mut config.audio_filters);
        if name == "normalize" {
            config.audio_normalize = true;
        }
        if name == "volume" {
            config.audio_volume = 73.0;
        }
        let [expected, actual] = decoded_pair(
            &tools,
            &source,
            &config,
            &sandbox,
            name,
            &["-map", "0:a:0", "-ar", "48000", "-f", "s16le"],
        )?;
        assert!(
            !expected.is_empty() && actual.len() == expected.len(),
            "{name} sample count"
        );
        let samples = |s: &[u8]| i16::from_le_bytes([s[0], s[1]]);
        let error: u64 = expected
            .chunks_exact(2)
            .zip(actual.chunks_exact(2))
            .map(|(a, b)| u64::from(samples(a).abs_diff(samples(b))).pow(2))
            .sum();
        // Two sample levels RMS allow alternate SIMD rounding.
        assert!(
            error <= (expected.len() as u64 / 2) * 4,
            "{name} sample mismatch"
        );
        if name == "baseline" {
            baseline = expected;
        } else {
            assert!(
                expected != baseline,
                "{name} did not change the reference fixture"
            );
        }
    }
    Ok(())
}
