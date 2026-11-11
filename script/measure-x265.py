#!/usr/bin/env python3
"""Measure x265 options on original FFmpeg packages with no Frame changes."""

import argparse
import importlib.util
import json
import os
import platform
import re
import statistics
import subprocess
import sys
import time
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "runtime_comparison", Path(__file__).with_name("compare-hevc-runtimes.py")
)
runtime = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runtime)
OPTIONS = {"default": None, "asm-0": "asm=0", "asm-sse2": "asm=sse2", "cutree-0": "cutree=0"}
FILTERS = ("copy", "vflip,copy")


def log_text(path):
    return path.read_text(encoding="utf-8", errors="replace")


def check_frames(tools, output, source, log):
    code = runtime.run([
        tools["ffmpeg"], "-v", "error", "-xerror", "-i", output, "-map", "0:v:0",
        "-threads:v", "1", "-f", "framemd5", "-",
    ], log)
    text = log_text(log)
    dimensions = re.search(r"^#dimensions 0: (\d+)x(\d+)$", text, re.M)
    frames = re.findall(r"^\s*0,.*?,\s*([a-f0-9]{32})\s*$", text, re.M)
    if code or not dimensions or tuple(map(int, dimensions.groups())) != source["size"]:
        return False
    return len(frames) == source["frames"]


def encode(tools, source, output, filters, option, log, crf=28, benchmark=False):
    command = [tools["ffmpeg"]]
    if benchmark:
        command += ["-benchmark"]
    command += [
        "-i", source["path"], "-c:v", "libx265", "-crf", str(crf),
        "-preset", "ultrafast", "-vf", filters,
    ]
    if option:
        command += ["-x265-params", option]
    command += ["-map", "0:v:0", "-an", "-dn", "-n", output]
    output.unlink(missing_ok=True)
    started = time.perf_counter()
    code = runtime.run(command, log)
    elapsed = time.perf_counter() - started
    text = log_text(log)
    rejected = re.search(r"(Unknown option|Invalid value for|Error parsing option)", text, re.I)
    decoded = not code and not rejected and check_frames(
        tools, output, source, log.with_suffix(".decode.log")
    )
    result = {
        "exit": code, "decoded": decoded, "parameter_rejected": bool(rejected),
        "wall_seconds": elapsed,
    }
    if benchmark and decoded:
        times = re.search(r"utime=([\d.]+)s stime=([\d.]+)s rtime=([\d.]+)s", text)
        fps = re.search(r"encoded \d+ frames in .*?\(([\d.]+) fps\)", text)
        if not times or not fps:
            raise RuntimeError(f"Missing benchmark timings in {log}")
        result.update(
            cpu_seconds=float(times[1]) + float(times[2]), ffmpeg_seconds=float(times[3]),
            encoder_fps=float(fps[1]), container_bytes=output.stat().st_size,
        )
    return result


def make_source(tools, folder, width, height, bits, frames):
    label = f"{width}x{height}-{bits}bit-{frames}f"
    path = folder / f"{label}.y4m"
    pixel_format = "yuv420p" if bits == 8 else "yuv420p10le"
    runtime.checked([
        tools["ffmpeg"], "-v", "error", "-f", "lavfi", "-i",
        f"testsrc2=size={width}x{height}:rate=25", "-frames:v", str(frames),
        "-pix_fmt", pixel_format, "-strict", "-1", "-f", "yuv4mpegpipe", path,
    ], folder / f"{label}.log")
    return {
        "path": path, "size": (width, height), "frames": frames,
        "pix_fmt": pixel_format, "sha256": runtime.digest(path), "label": label,
    }


def metrics(tools, source, output, folder, label):
    probe_log = folder / f"{label}.probe.log"
    runtime.checked([
        tools["ffprobe"], "-v", "error", "-select_streams", "v:0",
        "-show_entries", "stream=codec_name,width,height,pix_fmt:packet=size",
        "-of", "json", output,
    ], probe_log)
    probe = json.loads(log_text(probe_log).split("\n", 1)[1])
    stream, = probe["streams"]
    if (
        stream["codec_name"] != "hevc"
        or (stream["width"], stream["height"]) != source["size"]
        or stream["pix_fmt"] != source["pix_fmt"]
    ):
        raise RuntimeError(f"Unexpected output format in {probe_log}")
    log = folder / f"{label}.quality.log"
    runtime.checked([
        tools["ffmpeg"], "-hide_banner", "-nostats", "-xerror", "-i", output, "-i", source["path"],
        "-filter_complex",
        "[0:v]settb=AVTB,setpts=PTS-STARTPTS,split[a][b];"
        "[1:v]settb=AVTB,setpts=PTS-STARTPTS,split[c][d];"
        "[a][c]psnr=shortest=1[p];[b][d]ssim=shortest=1[s]",
        "-map", "[p]", "-map", "[s]", "-f", "null", "-",
    ], log)
    text = log_text(log)
    psnr = re.search(r"PSNR .*?average:([\d.]+|inf)", text)
    ssim = re.search(r"SSIM .*?All:([\d.]+)", text)
    if not psnr or not ssim:
        raise RuntimeError(f"Missing image metrics in {log}")
    hashes = re.findall(
        r"^\s*0,.*?,\s*([a-f0-9]{32})\s*$",
        log_text(folder / f"{label}.decode.log"), re.M,
    )
    return {
        "video_bytes": sum(int(packet["size"]) for packet in probe["packets"]),
        "psnr_db": "inf" if psnr[1] == "inf" else float(psnr[1]),
        "ssim": float(ssim[1]), "frame_hashes": hashes,
    }


def stability(tools, source, output, folder, repetitions):
    folder.mkdir()
    results = {f"{name}/{filters}": {"pass": 0, "failures": []}
               for name in OPTIONS for filters in FILTERS}
    for n in range(1, repetitions + 1):
        for name, option in OPTIONS.items():
            for filters in FILTERS:
                label = f"{n:03}-{name}-{'flip' if filters != 'copy' else 'plain'}"
                log = folder / f"{label}.log"
                result = encode(tools, source, output, filters, option, log)
                cell = results[f"{name}/{filters}"]
                if result["decoded"]:
                    cell["pass"] += 1
                else:
                    cell["failures"].append({"trial": n, "log": log.name, **result})
    return results


def benchmark(tools, source, output, folder, repetitions):
    folder.mkdir()
    results = {name: {"samples": [], "quality": None} for name in OPTIONS}
    # Rotate the order to reduce the influence of warm-up and runner load.
    for n in range(repetitions):
        names = list(OPTIONS)
        for name in names[n % len(names):] + names[:n % len(names)]:
            label = f"{n + 1:02}-{name}"
            log = folder / f"{label}.log"
            entry = results[name]
            result = encode(tools, source, output, "copy", OPTIONS[name], log, benchmark=True)
            entry["samples"].append(result)
            if result["decoded"] and entry["quality"] is None:
                entry["quality"] = metrics(tools, source, output, folder, label)
    for entry in results.values():
        valid = [sample for sample in entry["samples"] if sample["decoded"]]
        if valid:
            entry["median"] = {metric: statistics.median(x[metric] for x in valid) for metric in (
                "wall_seconds", "ffmpeg_seconds", "cpu_seconds", "encoder_fps", "container_bytes",
            )}
    control = results["default"]["quality"]
    if control:
        for name, entry in results.items():
            quality = entry["quality"]
            if name == "default" or not quality:
                continue
            target = control["video_bytes"]
            if abs(quality["video_bytes"] / target - 1) <= 0.03:
                continue
            closest = {"crf": 28, **quality}
            low, high = (28.0, 46.0) if quality["video_bytes"] > target else (10.0, 28.0)
            for n in range(7):
                crf = round((low + high) / 2, 3)
                label = f"match-{name}-{n + 1}"
                result = encode(
                    tools, source, output, "copy", OPTIONS[name], folder / f"{label}.log", crf,
                )
                if not result["decoded"]:
                    entry["match_failure"] = result
                    break
                candidate = metrics(tools, source, output, folder, label)
                size = candidate["video_bytes"]
                if abs(size - target) < abs(closest["video_bytes"] - target):
                    closest = {"crf": crf, **candidate}
                if abs(size / target - 1) <= 0.03:
                    break
                if size > target:
                    low = crf
                else:
                    high = crf
            entry["closest_bitrate"] = closest
    return results


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=runtime.ROOT / "target/x265-measurement")
    parser.add_argument("--repetitions", type=int, default=100)
    parser.add_argument("--boundary-repetitions", type=int, default=3)
    parser.add_argument("--benchmark-repetitions", type=int, default=5)
    args = parser.parse_args()
    if min(args.repetitions, args.boundary_repetitions, args.benchmark_repetitions) < 1:
        parser.error("repetition counts must be positive")
    output = args.output.resolve()
    root = output / "results"
    root.mkdir(parents=True)
    downloads = output / "downloads"
    downloads.mkdir(exist_ok=True)
    sources = output / "sources"
    sources.mkdir()
    manifest = json.loads(runtime.PACKAGES.read_text(encoding="utf-8"))
    runtime.write_json(root / "packages.json", manifest)
    key = runtime.host_key()
    cpu = {
        "linux-x86_64": ["lscpu"],
        "windows-x86_64": ["powershell", "-NoProfile", "-Command",
                           "Get-CimInstance Win32_Processor | Select-Object Name,NumberOfLogicalProcessors | ConvertTo-Json"],
        "macos-arm64": ["sysctl", "machdep.cpu.brand_string", "hw.ncpu"],
    }
    runtime.checked(cpu[key], root / "cpu.log")
    fixed = runtime.ROOT / manifest["fixture"]["path"]
    runtime.verify(fixed, manifest["fixture"]["sha256"])
    fixture = {"path": fixed, "size": (48, 32), "frames": 6, "sha256": runtime.digest(fixed)}
    data = {
        "platform": key, "os": platform.platform(), "runner_image": os.getenv("ImageVersion"),
        "commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=runtime.ROOT, text=True).strip(),
        "run": os.getenv("GITHUB_RUN_ID"), "attempt": os.getenv("GITHUB_RUN_ATTEMPT"),
        "repetitions": args.repetitions, "boundary_repetitions": args.boundary_repetitions,
        "benchmark_repetitions": args.benchmark_repetitions, "fixture_sha256": fixture["sha256"],
        "versions": {},
    }
    summary = root / "summary.json"
    runtime.write_json(summary, data)
    sets = {}
    for version, package in manifest["versions"].items():
        logs = root / version
        logs.mkdir()
        sets[version] = runtime.install(package["platforms"][key], output / version, downloads, logs)
    generator = sets["8.1.2"]
    boundary = [make_source(generator, sources, w, 64, b, 12)
                for w in (32, 48, 62, 64, 66) for b in (8, 10)]
    longer = [make_source(generator, sources, w, 96, b, 600)
              for w in (48, 64) for b in (8, 10)]
    data["sources"] = [{k: v for k, v in item.items() if k != "path"} for item in boundary + longer]
    failed = False
    for version, tools in sets.items():
        logs = root / version
        target = output / f"{version}.mkv"
        result = {"boundary": {}, "benchmarks": {}}
        data["versions"][version] = result
        result["historical"] = stability(tools, fixture, target, logs / "historical", args.repetitions)
        for case, cell in result["historical"].items():
            print(f"{key} {version} {case} historical: {cell['pass']}/{args.repetitions}", flush=True)
            failed |= bool(cell["failures"])
        runtime.write_json(summary, data)
        for item in boundary:
            checks = stability(tools, item, target, logs / item["label"], args.boundary_repetitions)
            result["boundary"][item["label"]] = checks
            failed |= any(cell["failures"] for cell in checks.values())
        runtime.write_json(summary, data)
        for item in longer:
            measurements = benchmark(tools, item, target, logs / item["label"], args.benchmark_repetitions)
            result["benchmarks"][item["label"]] = measurements
            failed |= any(not trial["decoded"] for m in measurements.values() for trial in m["samples"])
            runtime.write_json(summary, data)
        for entry in manifest["versions"][version]["platforms"][key]:
            runtime.verify(tools[entry["id"]], entry["sha256"])
        target.unlink(missing_ok=True)
        result["completed"] = True
        runtime.write_json(summary, data)
        print(f"{key} {version}: completed timing and quality measurements", flush=True)
    runtime.verify(fixed, manifest["fixture"]["sha256"])
    for item in boundary + longer:
        runtime.verify(item["path"], item["sha256"])
    data["completed"] = True
    runtime.write_json(summary, data)
    return int(failed)


if __name__ == "__main__":
    sys.exit(main())
