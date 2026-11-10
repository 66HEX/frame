# Frame preview transport

This opt-in patch for FFmpeg n8.1.2 binds preview pixels to the output raster
and sample aspect ratio of the same decoded frame, including CFR duplicates.
The normal FFmpeg CLI and rawvideo output stay unchanged unless the new options
are enabled.

`lock.json` pins the upstream archive, patch and hashes of all eight changed
source files. Prepare a separate source tree with:

```sh
python3 tooling/ffmpeg/frame-preview-v1/build_pinned.py \
  --archive /path/to/ffmpeg-n8.1.2.tar.gz --output /path/to/new-build
```

The archive URL is in the lock file. Add `--build` to compile the included
minimal macOS ARM profile, or pass `--configure-json` with an explicit array of
configure arguments. The destination must not exist. The script records its
commands, logs and resulting hashes. It does not install files or replace the
runtime selected by Frame.

The minimal profile is for protocol testing. It lacks many codecs and libraries
in Frame's release bundles. This recipe does not pin the compiler, SDK or every
external dependency, and does not promise bit-for-bit binary reproducibility.
Full release builds need their own dependency locks and native platform tests.
The existing downloaded release runtimes do not contain this patch.

The [complete macOS ARM64 recipe](full-macos-arm64/README.md) builds the
external dependencies in a private directory and targets macOS 12. Its source
lock and verification requirements are separate from the minimal profile.

## Wire format

`-frame_preview_framed` requires a single rawvideo BGRA output stream. Add
`frame_preview_capture=1` to the last preview `scale` filter, followed only by
`setsar=1,format=bgra`. The scale filter captures its input raster and SAR after
autorotation and before preview downscaling. The encoder attaches that capture
to the corresponding rawvideo packet after CFR synchronization.

`-native_frame_sar:v:0` is an independent input option that preserves the
decoder's per-frame SAR. Frame uses it for MPEG-TS Source/Copy preview. It must
stay off where container SAR takes precedence, including MP4/MOV.

Each output record contains this 80-byte little-endian header, then exactly
`payload_bytes` BGRA bytes:

| Offset | Type | Value |
| --- | --- | --- |
| 0 | 8 bytes | `FRMPV001` |
| 8 | u16 | Header size, 80 |
| 10 | u16 | Bits 0/1/2: known SAR/PTS/decoder index; other bits zero |
| 12 | u32 | Pixel format, 1 for BGRA |
| 16 | u64 | Payload size |
| 24 | u64 | Consecutive output sequence, starting at zero |
| 32 | i64 | PTS, or `INT64_MIN` when unknown |
| 40, 44 | u32 | Positive time-base numerator, denominator; unknown PTS uses `0/0` |
| 48, 52 | u32 | Prepared texture width, height |
| 56, 60 | u32 | Raster width, height before preview scaling |
| 64, 68 | u32 | Positive SAR numerator, denominator; unknown is `0/1` |
| 72 | u64 | Decoder frame index, or `UINT64_MAX` when unknown |

The reader validates the prepared dimensions, payload size, flags, sentinels
and sequence before allocating pixels. EOF is valid only between records.
A malformed record ends that stream; the reader never searches for another
magic marker or replaces frame metadata with initial probe values. Header PTS
is observational and does not change Frame's playback clock.

The patch clears its private capture flag on every decoded input frame, then
sets it only after the scale filter has written the capture. This prevents
source-file metadata from impersonating the capture. The contract assumes
Frame's fixed filter suffix; arbitrary filters inserted after capture are
outside that contract.

## Operational probes

`probes/` contains synthetic input media and accepted wire output, with SHA-256
hashes in `manifest.json`. The static NUT fixture has two 4×2 frames with SAR
3:2. The MPEG-TS fixture has 36 identical frames whose SAR changes from 1:1 to
2:1 and back. Identical pixels make a metadata/payload pairing error observable.

Frame tests the executable it will actually launch. A version string or option
listing alone does not establish support. Runtime replacement invalidates the
result. The initial framed streaming owner requires an observed native executable
identity. Script wrappers and unresolved PATH names keep the existing static
preview path; Frame does not substitute another executable for them.

Capability probes and framed snapshots use Frame's bounded process-group/job
owner. Streaming manages the selected native FFmpeg child directly. Its
15-second first-frame timeout is followed by child termination and reader
cleanup; it is not a guaranteed deadline for arbitrary native programs whose
descendants retain the output pipes. Runtime identity and operational probes
establish compatibility with the selected executable, not its authenticity.

The upstream source and patch remain subject to FFmpeg's applicable licenses.
Distributing patched binaries requires retaining matching sources, build
configuration and dependency/license information for those binaries.
