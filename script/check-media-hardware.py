#!/usr/bin/env python3
"""Exercise the hardware encoders in an installed Frame package and report JSON."""
import argparse
import hashlib
import json
import platform
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def call(worker, *args, data=None):
    return subprocess.run([str(worker), *map(str, args)], input=data, capture_output=True,
                          timeout=60, check=False)


def checked(worker, *args):
    proc = call(worker, *args)
    if proc.returncode:
        raise RuntimeError(proc.stderr.decode(errors="replace").strip())
    return proc.stdout


def write_pattern(path):
    # A moving luma level checks all 12 decoded pictures, including encoder drain.
    with path.open("wb") as stream:
        stream.write(b"YUV4MPEG2 W320 H256 F24:1 Ip A1:1 C420jpeg\n")
        for i in range(12):
            stream.write(b"FRAME\n" + bytes([32 + 16*i]) * (320*256) +
                         bytes([128]) * (320*256//2))


def exercise(worker, encoder, source, tmp):
    probe = call(worker, "capabilities", "hardware", encoder, "0", data=bytes(128*128*3//2))
    if probe.returncode:
        return {"status": "unavailable", "detail": probe.stderr.decode(errors="replace").strip()}
    codec = encoder.split("_")[0]
    device = "cuda" if encoder.endswith("_nvenc") else "videotoolbox"
    options = ["-preset", "p4", "-spatial-aq", "1", "-temporal-aq", "1"] if device == "cuda" else ["-allow_sw", "0"]
    results = []
    for depth in ([8] if codec == "h264" else [8, 10]):
        output = tmp / f"{encoder}-{depth}.mp4"
        checked(worker, "convert", "-hwaccel", device, "-i", source, "-map", "0:0",
                "-c:v", encoder, *options, "-pix_fmt", "yuv420p" if depth == 8 else "p010le",
                "-b:v", "2M", "-n", output)
        meta = json.loads(checked(worker, "probe", "metadata", output))["streams"][0]
        pix_fmt = "yuv420p" if depth == 8 else "yuv420p10le"
        if (meta["codec_name"], meta["pix_fmt"]) != (codec, pix_fmt):
            raise RuntimeError(f"unexpected encoded stream {meta}")
        raw = checked(worker, "preview", "-i", output, "-map", "0:0", "-fps_mode", "passthrough",
                      "-vf", "scale=1:1", "-pix_fmt", "bgra", "-f", "rawvideo", "pipe:1")
        luma = raw[::4]
        if len(raw) != 48 or not all(a < b for a, b in zip(luma, luma[1:])):
            raise RuntimeError(f"expected twelve pictures with increasing luma, got {list(luma)}")
        results.append({"pixel_format": pix_fmt, "decoded_frames": len(luma)})
    return {"status": "passed", "outputs": results}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("worker", type=Path, help="path to the installed frame-media executable")
    parser.add_argument("--require", action="append", default=[], help="require this encoder to pass")
    args = parser.parse_args()
    worker = args.worker.resolve()
    encoders = (["h264_videotoolbox", "hevc_videotoolbox"] if sys.platform == "darwin"
                else ["h264_nvenc", "hevc_nvenc", "av1_nvenc"])
    report = {"platform": platform.platform(), "worker_sha256": hashlib.sha256(worker.read_bytes()).hexdigest(),
              "libraries": checked(worker, "capabilities", "version").decode().strip(), "encoders": {}}
    if shutil.which("nvidia-smi"):
        info = subprocess.run(["nvidia-smi", "--query-gpu=name,driver_version", "--format=csv,noheader"],
                              timeout=15, capture_output=True, text=True)
        report["gpu"] = info.stdout.strip() or info.stderr.strip()
    with tempfile.TemporaryDirectory() as folder:
        tmp = Path(folder)
        raw, source = tmp / "pattern.y4m", tmp / "input.mp4"
        write_pattern(raw)
        checked(worker, "convert", "-i", raw, "-map", "0:0", "-c:v", "libx264", "-n", source)
        for encoder in encoders:
            try:
                report["encoders"][encoder] = exercise(worker, encoder, source, tmp)
            except (RuntimeError, subprocess.TimeoutExpired, KeyError, ValueError) as error:
                report["encoders"][encoder] = {"status": "failed", "detail": str(error)}
    print(json.dumps(report, indent=2))
    statuses = {k: row["status"] for k, row in report["encoders"].items()}
    return int("passed" not in statuses.values() or "failed" in statuses.values()
               or any(statuses.get(required) != "passed" for required in args.require))


if __name__ == "__main__":
    sys.exit(main())
