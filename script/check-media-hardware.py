#!/usr/bin/env python3
"""Exercise an installed Frame worker and independently decode its exports."""
import argparse
import hashlib
import json
import os
import platform
import shutil
import struct
import subprocess
import sys
import tempfile
from pathlib import Path


BACKENDS = {
    "nvenc": ("cuda", ("h264", "hevc", "av1")),
    "vaapi": ("vaapi", ("h264", "hevc", "av1")),
    "amf": ("d3d11va", ("hevc",)),
    "videotoolbox": ("videotoolbox", ("h264", "hevc")),
}
SAMPLES = 12
PROBE_FRAME = bytes([32]) * (128 * 128) + bytes([128]) * (128 * 128 // 2)


def call(program, *args, data=None):
    return subprocess.run([str(program), *map(str, args)], input=data,
                          capture_output=True, timeout=90, check=False)


def checked(program, *args, data=None):
    proc = call(program, *args, data=data)
    if proc.returncode:
        raise RuntimeError(f"{Path(program).name} {args[0]}: {proc.stderr.decode(errors='replace').strip()}")
    return proc


def reference_tool(name, explicit):
    if explicit:
        return str(Path(explicit).resolve())
    path = shutil.which(name)
    if path:
        return path
    machine = platform.machine().lower()
    arch = "aarch64" if machine in ("aarch64", "arm64") else "x86_64"
    system = {"darwin": "apple-darwin", "win32": "pc-windows-msvc"}.get(sys.platform, "unknown-linux-gnu")
    filename = f"{name}-{arch}-{system}" + (".exe" if sys.platform == "win32" else "")
    return str(Path(__file__).resolve().parents[1] / "frame-app/resources/binaries" / filename)


def metadata(ffprobe, source):
    result = checked(ffprobe, "-v", "error", "-select_streams", "v:0", "-count_frames",
                     "-show_entries", "stream=codec_name,profile,width,height,pix_fmt,nb_read_frames",
                     "-of", "json", source)
    return json.loads(result.stdout)["streams"][0]


def make_source(ffmpeg, ffprobe, folder, depth):
    source = folder / f"input-{depth}.mp4"
    # Low-order luma bits vary in the Main10 input. Feed the reference encoder
    # planar samples directly so a format conversion cannot turn this into 8-bit.
    raw = folder / f"input-{depth}.yuv"
    width, height = 320, 256
    pix_fmt = "yuv420p" if depth == 8 else "yuv420p10le"
    with raw.open("wb") as stream:
        for i in range(SAMPLES):
            if depth == 8:
                y, uv = bytes([32 + i * 16]), bytes([128])
            else:
                y, uv = struct.pack("<H", 128 + i * 65), struct.pack("<H", 512)
            stream.write(y * (width * height) + uv * (width * height // 2))
    options = ["-c:v", "libx264", "-qp", "12"] if depth == 8 else [
        "-c:v", "libx265", "-x265-params", "lossless=1:log-level=error"]
    checked(ffmpeg, "-v", "error", "-f", "rawvideo", "-pix_fmt", pix_fmt, "-s", "320x256",
            "-r", "24", "-i", raw, "-frames:v", SAMPLES, *options, "-n", source)
    stream = metadata(ffprobe, source)
    if stream["pix_fmt"] != pix_fmt or int(stream["nb_read_frames"]) != SAMPLES:
        raise RuntimeError(f"incorrect {depth}-bit fixture: {stream}")
    return source


def probe_encoder(worker, encoder, depth):
    pixel = "p010le" if depth == 10 else "yuv420p"
    result = call(worker, "capabilities", "hardware", encoder, "0", pixel, data=PROBE_FRAME)
    diagnostic = result.stderr.decode(errors="replace").strip()
    if result.returncode:
        return False, diagnostic
    if b"frame=1" not in result.stdout.splitlines() or b"progress=end" not in result.stdout.splitlines():
        raise RuntimeError(f"incomplete hardware probe: {result.stdout!r}, {diagnostic}")
    return True, diagnostic


def check_output(ffmpeg, ffprobe, output, codec, depth, filtered):
    stream = metadata(ffprobe, output)
    expected_fmt = "yuv420p10le" if depth == 10 else "yuv420p"
    if (stream["codec_name"], stream["pix_fmt"], stream["width"], stream["height"],
            int(stream["nb_read_frames"])) != (codec, expected_fmt, 320, 256, SAMPLES):
        raise RuntimeError(f"unexpected encoded stream: {stream}")
    samples = checked(ffmpeg, "-v", "error", "-xerror", "-i", output, "-map", "0:v:0",
                      "-vf", "scale=10:8:flags=neighbor,format=gray", "-fps_mode", "passthrough",
                      "-f", "rawvideo", "-").stdout
    centers = samples[45::80]
    if len(samples) != 80 * SAMPLES or not all(a < b for a, b in zip(centers, centers[1:])):
        raise RuntimeError(f"expected twelve pictures with increasing luma, got {list(centers)}")
    if filtered and (min(samples[0::80]) < 200 or max(samples[79::80]) > 15):
        raise RuntimeError("independent decoder did not find the bright watermark and black padding")
    return stream


def export_case(worker, ffmpeg, ffprobe, encoder, folder, source, depth, case, marker):
    codec, backend = encoder.split("_", 1)
    decoder = BACKENDS[backend][0]
    output = folder / f"{encoder}-{depth}-{case}.mp4"
    args = ["-hwaccel", decoder] if case != "cpu" else []
    args += ["-i", source]
    if case == "filtered":
        # Both frame edges are cropped, the content is scaled and padded back
        # to its original raster. Place a white marker in the black border.
        graph = ("[0:v:0]crop=316:252,scale=256:160,pad=320:256:32:48[base];"
                 "[base][1:v:0]overlay=0:0:format=auto[out]")
        args += ["-i", marker, "-filter_complex", graph, "-map", "[out]"]
    else:
        args += ["-map", "0:0"]
        if case == "fallback":
            args += ["-vf", "pad=320:256:0:0"]
    pixel = "yuv420p" if depth == 8 else "yuv420p10le"
    args += ["-c:v", encoder, "-b:v", "2M", "-pix_fmt", pixel]
    if backend == "videotoolbox":
        args += ["-allow_sw", "0"]
    if backend == "nvenc":
        args += ["-preset", "p4", "-spatial-aq", "1", "-temporal-aq", "1"]
    args += ["-n", output]
    route = json.loads(checked(worker, "preflight", *args).stdout)
    if output.exists():
        raise RuntimeError("preflight created the output")
    if (route["encoder"] != encoder or route["filters"] != "cpu"
            or route["upload"] != (backend == "vaapi")):
        raise RuntimeError(f"unexpected filter or encoder route: {route}")
    if backend == "vaapi" and not route["device"].startswith("/dev/dri/renderD"):
        raise RuntimeError(f"VA-API route did not identify a render node: {route}")
    if backend == "amf" and not route["device"].startswith("amd-"):
        raise RuntimeError(f"AMF route did not identify a Radeon adapter: {route}")
    if case == "fallback":
        if route["decoder"] != "cpu" or not route["fallback"]:
            raise RuntimeError(f"missing CPU fallback for the odd FFV1 source: {route}")
    elif case == "cpu":
        if route["decoder"] != "cpu" or route["fallback"]:
            raise RuntimeError(f"unexpected decoder with HW decoding off: {route}")
    elif route["decoder"] != decoder and (route["decoder"] != "cpu" or not route["fallback"]):
        raise RuntimeError(f"missing decoder or fallback: {route}")
    pin = ["-frame_decoder", route["decoder"], "-frame_device", route["device"],
           "-frame_format", route["format"]]
    run = checked(worker, "convert", *pin, *args)
    upload = "yes" if route["upload"] else "no"
    expected = (f"Video route: decoder={route['decoder']} filters=cpu samples={route['format']} "
                f"upload={upload} encoder={encoder} device={route['device']}")
    traces = [line for line in run.stderr.decode(errors="replace").splitlines() if line.startswith("Video route: ")]
    if traces != [expected]:
        raise RuntimeError(f"preflight/execution mismatch: {expected!r}, {traces!r}")
    return {"case": case, "input": source.name, "depth": depth, "preflight": route, "execution": traces[0],
            "output": check_output(ffmpeg, ffprobe, output, codec, depth, case == "filtered")}


def exercise(worker, ffmpeg, ffprobe, encoder, folder, sources, marker):
    results = {}
    for depth in ([8] if encoder.startswith("h264_") else [8, 10]):
        available, diagnostic = probe_encoder(worker, encoder, depth)
        if not available:
            results[depth] = {"status": "unavailable", "detail": diagnostic}
            if depth == 8:
                return {"status": "unavailable", "profiles": results}
            continue
        if depth not in sources:
            sources[depth] = make_source(ffmpeg, ffprobe, folder, depth)
        cases = []
        decoded = False
        passed = True
        for case in (["cpu", "filtered", "fallback"] if depth == 8 else ["cpu", "filtered"]):
            source = sources["odd"] if case == "fallback" else sources[depth]
            try:
                result = export_case(worker, ffmpeg, ffprobe, encoder, folder, source, depth, case, marker)
                if case == "filtered":
                    decoded = result["preflight"]["decoder"] != "cpu"
                cases.append(result)
            except (RuntimeError, subprocess.TimeoutExpired, KeyError, ValueError) as error:
                passed = False
                cases.append({"case": case, "source": source.name, "error": str(error)})
        results[depth] = {"status": "passed" if passed else "failed", "hardware_decode": decoded,
                          "probe": diagnostic, "exports": cases}
    failed = any(profile["status"] == "failed" for profile in results.values())
    return {"status": "failed" if failed else "passed", "profiles": results}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("worker", type=Path, help="candidate frame-media executable")
    parser.add_argument("--require", action="append", default=[],
                        help="require this encoder and successful hardware decoding of the 8-bit input")
    parser.add_argument("--require-10bit", action="append", default=[],
                        help="also require this encoder and HEVC Main10 hardware decoding at 10 bits")
    for name in ("ffmpeg", "ffprobe"):
        parser.add_argument(f"--{name}", default=os.environ.get(f"FRAME_TEST_{name.upper()}"),
                            help=f"path to the independent reference {name} executable")
    args = parser.parse_args()
    worker = args.worker.resolve()
    ffmpeg, ffprobe = reference_tool("ffmpeg", args.ffmpeg), reference_tool("ffprobe", args.ffprobe)
    families = {"darwin": ["videotoolbox"], "win32": ["nvenc", "amf"], "linux": ["nvenc", "vaapi"]}
    encoders = [f"{codec}_{family}" for family in families[sys.platform] for codec in BACKENDS[family][1]]
    report = {"platform": platform.platform(), "worker_sha256": hashlib.sha256(worker.read_bytes()).hexdigest(),
              "libraries": checked(worker, "capabilities", "version").stdout.decode().strip(),
              "library_paths": [os.fsdecode(p) for p in checked(worker, "capabilities", "libraries").stdout.split(b"\0") if p],
              "ffmpeg": ffmpeg, "ffprobe": ffprobe, "encoders": {}}
    if shutil.which("nvidia-smi"):
        info = call("nvidia-smi", "--query-gpu=name,driver_version", "--format=csv,noheader")
        report["gpu"] = (info.stdout or info.stderr).decode(errors="replace").strip()
    with tempfile.TemporaryDirectory() as folder:
        tmp = Path(folder)
        sources = {8: make_source(ffmpeg, ffprobe, tmp, 8), "odd": tmp / "odd.mkv"}
        checked(ffmpeg, "-v", "error", "-i", sources[8], "-vf", "format=yuv444p,crop=319:255",
                "-c:v", "ffv1", "-n", sources["odd"])
        marker = tmp / "white.ppm"
        marker.write_bytes(b"P6\n32 32\n255\n" + bytes([255]) * (32 * 32 * 3))
        for encoder in encoders:
            try:
                report["encoders"][encoder] = exercise(worker, ffmpeg, ffprobe, encoder, tmp, sources, marker)
            except (RuntimeError, subprocess.TimeoutExpired, KeyError, ValueError) as error:
                report["encoders"][encoder] = {"status": "failed", "detail": str(error)}
    results = report["encoders"]
    unmet = [f"{name}/{depth}" for depth, required in [(8, args.require), (10, args.require_10bit)]
             for name in required if not results.get(name, {}).get("profiles", {}).get(depth, {}).get("hardware_decode")]
    report["unmet_requirements"] = unmet
    print(json.dumps(report, indent=2))
    statuses = {row["status"] for row in results.values()}
    return int("passed" not in statuses or "failed" in statuses or bool(unmet))


if __name__ == "__main__":
    sys.exit(main())
