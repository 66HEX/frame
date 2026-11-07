# Preview engine benchmark

`preview-benchmark` drives the public `PreviewSession` API against an existing video file. It measures wall time to the first published frame, paused precise and fast seeks, brightness changes, Play readiness, Pause, and Stop. During playback it acknowledges new frames every 2 ms and records frame intervals, dropped frames, conversion time, bytes, and process-spawn counters.

```sh
cargo build --locked -p frame-app --example preview-benchmark
FRAME_FFMPEG_PATH=/opt/homebrew/bin/ffmpeg \
  target/debug/examples/preview-benchmark \
  /tmp/preview-fixture.mp4 1920 1080 12 \
  --iterations 10 --warmup 1 --playback-seconds 2 \
  --output /tmp/preview-baseline.json
```

The positional dimensions and duration must describe the source, not the preview. Preview defaults to 1280×720 at 30 fps; use `--max-width`, `--max-height`, and `--fps` to change it. `--help` lists all arguments. Run the same binary profile, FFmpeg version, source file, and options before and after a change; keep other heavy work idle. The default warmup iteration is excluded, so these are warm-file-cache measurements, not cold-disk measurements. Raw samples and nearest-rank p50/p95 are retained in JSON; p95 from a small sample set is only a rough tail estimate.

Each measured iteration starts a fresh session, seeks to 20%, 70%, and 40% of source duration in both precise and fast modes, enables brightness at 10% then 20%, restores the original filter config, seeks to zero, and plays for the sampling interval. Playback metrics cover Play startup plus that interval. The first-frame measurements require a newly published, nonempty frame; stalled playback fails rather than producing a misleading success result. The per-iteration watchdog exits with code 124 after `--timeout-seconds` (default 30); inspect and terminate leftover FFmpeg children after a timeout.

After the original session stops, a separate session compares `SeekPrecise` followed by `Play` with atomic `SeekAndPlay`. Both start paused at zero and seek to 20% of the source duration. Order alternates each iteration; session creation and resets are outside the timed sections. `iterations[].seek_and_play` retains both times and process counts and verifies identical first-frame timestamp, dimensions and BGRA bytes. It also verifies that another `Play` leaves the active stream and pending frame intact without spawning another process.

This is a headless video-engine benchmark. It does not measure GPUI layout, GPU upload/presentation, A/V synchronization, or visual correctness. Audio is unselected by default for repeatability. Add `--audio-track INDEX` using the source stream index to measure audio-device readiness and callbacks as part of startup; this enables the real audio path with volume set to zero. Keep the same output device during comparisons. Use integration tests for pixel/seek correctness and a native UI run for presentation and audio. The app's renderer benchmark remains separate (`render-benchmark`).

Do not infer zero process work from a counter without checking its coverage: historical engine revisions count streaming video process spawns but omit single-frame decode spawns. Current measurements should use the corrected counter on both sides of a comparison. The operation wall times remain directly comparable across those revisions.

## Measurements: 7 November 2026

Host: Apple M4 Pro, 12 CPU cores, 24 GiB RAM, macOS arm64, Homebrew FFmpeg 8.1. The harness used Cargo's development profile throughout. Sources were synthetic H.264 1080p and 4K, 30 fps, 12 seconds, with a 120-frame GOP and B-frames. Preview output was 1280×720 at 30 fps. These are warm-cache measurements on this machine; they do not establish performance for every codec or the bundled FFmpeg build.

| Workload | Before | After | Measurement |
| --- | ---: | ---: | --- |
| 20 queued seeks, 1080p | 1269.80 ms | 128.54 ms | Median of 3, first and final decode retained |
| 20 queued seeks, 4K | 3736.75 ms | 379.01 ms | Median of 3, 20 → 2 FFmpeg processes |
| Paused seek + play, 1080p | 132.49 ms | 64.29 ms | Paired median of 5, 2 → 1 video processes |
| Paused seek + play, 4K | 392.99 ms | 192.27 ms | Paired median of 5, 2 → 1 video processes |
| Paused seek + play, 4K with audio | 399.65 ms | 195.86 ms | Paired median of 3, real output device, muted |
| Paused preview timer | 600 wakeups | 1 wakeup, then stopped | GPUI test clock, 600 × 16 ms; not a CPU-utilization measurement |

The backlog benchmark deliberately holds the first worker while 19 newer targets arrive. It compares an uncoalesced FIFO reference with the actual coalescing queue against real FFmpeg. The former dispatcher submitted every request, but its concurrent tasks did not guarantee FIFO mutex acquisition. The benchmark excludes initial session creation and does **not** measure end-to-end UI event latency. Every trial checks the exact final timestamp and BGRA bytes. The paired seek-and-play cases likewise check identical first-frame pixels and target timestamps. Playback produced approximately 30 frames per second with zero dropped frames in the measured windows; the audio runs reported zero output underruns. These short runs do not prove perceptual A/V synchronization.

Single-operation decoding and filter work remain largely unchanged: for example, 4K precise seek p50 was 200.78 → 206.08 ms and filter reconfiguration 152.24 → 151.35 ms. The reduction in 1080p startup time in the final run is not attributed to an optimization because the startup decode path was not changed. The benefits above come from removing redundant work, not from faster decoding of every individual frame.

Changes retained:

- One decoder command worker coalesces pending ordinary seeks while retaining Play/Pause boundaries. Trim requests share that worker, so a late drag request cannot overtake the final precise seek and resume.
- Atomic seek-and-play starts the stream directly; repeated Play is idempotent while the stream is already playing. Replay after EOF still starts at the configured trim start.
- Paused/static previews stop their 16 ms ticker after pending frame, animation and debounce work settles. Operation completion wakes it again.
- Failed starts or rebuilds are cached by request key instead of retried on every redraw. Source/config changes permit retry, and successful transport commands preserve a pending rebuild error.
- Process metrics include single-frame decodes and count successful process spawns.

Experiments rejected:

- Fixed decoder thread limits reduced some startup costs but regressed seeking. In the 4K 2.4-second seek sweep, automatic threading took 204.40 ms versus 887.35 ms with one decoder thread. Filter/output thread caps offered no consistent material gain. All compared output hashes matched.
- Removing video input `-readrate 1` did not improve these workloads. Through the same wrapper on both sides, 4K Play readiness was 117.31 → 127.09 ms; both policies delivered about 30 fps without drops. Existing pacing remains intact.

Raw samples, queue benchmark output and rejected-experiment data are in [benchmarks/preview-2026-11-07](benchmarks/preview-2026-11-07/). `baseline-*` captures the earlier engine with corrected single-frame counters; `final-*` includes the paired atomic comparison. The thread sweep uses direct FFmpeg subprocess timings; the readrate experiment uses a Python exec wrapper on both sides, so those absolute times should not be compared with the unwrapped harness.

Final validation passed: 891 `frame-app` library tests (12 ignored), 117 `frame-core` unit tests (41 integration tests ignored), and 7 explicitly enabled real-FFmpeg engine tests. The focused runtime/queue suite passed 22 tests. The real tests cover first-frame publication, play/pause/seek, reconfiguration, EOF, audio metrics, atomic seek-and-play pixels/process counts, repeated Play, and replay from trim start. Strict all-target Clippy, formatting, diff whitespace checks and the `frame` development binary build also passed. The application binary was built but not launched for these measurements.

### Reproduce the fixtures and queue benchmark

```sh
mkdir -p /tmp/frame-preview-perf
ffmpeg -hide_banner -y -f lavfi -i testsrc2=size=1920x1080:rate=30 \
  -f lavfi -i sine=frequency=440:sample_rate=48000 -t 12 \
  -c:v libx264 -preset fast -crf 20 -g 120 -bf 3 \
  -c:a aac -b:a 128k /tmp/frame-preview-perf/h264-1080p.mp4
ffmpeg -hide_banner -y -f lavfi -i testsrc2=size=3840x2160:rate=30 \
  -f lavfi -i anullsrc=channel_layout=stereo:sample_rate=48000 -t 12 \
  -c:v libx264 -preset fast -crf 20 -g 120 -bf 3 \
  -c:a aac -b:a 128k /tmp/frame-preview-perf/h264-4k.mp4
FRAME_FFMPEG_PATH=/opt/homebrew/bin/ffmpeg \
  FRAME_PREVIEW_BENCH_PATH=/tmp/frame-preview-perf/h264-4k.mp4 \
  cargo test --locked -p frame-app --lib benchmark_real_ffmpeg_seek_backlog \
  -- --ignored --nocapture --test-threads=1
```

The queue benchmark defaults to 3840×2160, 12 seconds. Set `FRAME_PREVIEW_BENCH_WIDTH=1920` and `FRAME_PREVIEW_BENCH_HEIGHT=1080` for the other fixture. Run it with an external timeout when testing an unfamiliar FFmpeg executable. It intentionally fails if the final frames differ or decode counts deviate from 20 and 2.
