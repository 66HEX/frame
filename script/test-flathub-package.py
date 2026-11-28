#!/usr/bin/env python3
"""Build the production Flathub manifest and compare its worker with the runtime's media inventory."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import tempfile


APP_ID = "io.github._66HEX.Frame"


def release_manifest(root, output):
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"], cwd=root
    ))
    version = next(p["version"] for p in metadata["packages"] if p["name"] == "frame-app")
    subprocess.run(["cargo", "xtask", "flathub-sources", "--version", version,
                    "--out-dir", output], cwd=root, check=True)
    values = dict(line.split("=", 1) for line in (output / "checksums.env").read_text().splitlines())
    subprocess.run([
        "cargo", "xtask", "flathub-manifest", "--version", version, "--out", output,
        "--source-url", (output / values["FRAME_SOURCE_ARCHIVE"]).as_uri(),
        "--source-sha256", values["FRAME_SOURCE_SHA256"],
        "--vendor-url", (output / values["FRAME_CARGO_VENDOR_ARCHIVE"]).as_uri(),
        "--vendor-sha256", values["FRAME_CARGO_VENDOR_SHA256"],
    ], cwd=root, check=True)
    return output / f"{APP_ID}.yml"


def inventory_names(option, data):
    if option == "-bsfs":
        return set(re.findall(r"^[\w-]+$", data, re.M))
    if option == "-filters":
        return set(re.findall(r"^\s*[TSC.]{2,3}\s+([\w-]+)\s+\S*->", data, re.M))
    return set(re.findall(r"^\s*[VAS][A-Z.]{5}\s+([\w-]+)\s", data, re.M))


def smoke():
    with tempfile.TemporaryDirectory(prefix="Frame Flatpak ", dir=Path.home()) as temporary:
        directory = Path(temporary)

        def invoke(command, *args):
            return subprocess.check_output([
                "flatpak", "run", "--user", f"--filesystem={directory}",
                "--env=PATH=", f"--command={command}", APP_ID, *args
            ], timeout=60)

        worker = "/app/bin/frame-media"
        for option in ("-encoders", "-filters", "-bsfs"):
            previous = inventory_names(option, invoke("/usr/bin/ffmpeg", "-hide_banner", option).decode())
            current = inventory_names(option, invoke(worker, "capabilities", option).decode())
            if not previous or current != previous:
                raise ValueError(f"{option} inventory differs: missing {previous-current}, extra {current-previous}")
            print(f"{len(current)} {option[1:]} match the Flatpak runtime", flush=True)

        paths = invoke(worker, "capabilities", "libraries").rstrip(b"\x00").split(b"\x00")
        if len(set(paths)) != 6 or any(not p.startswith(b"/usr/") for p in paths):
            raise ValueError(f"Unexpected Flatpak runtime library paths: {paths}")

        source = directory / "source.ppm"
        source.write_bytes(b"P6\n32 32\n255\n" + bytes([45, 90, 128]) * 1024)
        jxl = directory / "image.jxl"
        invoke(worker, "convert", "-i", source, "-map", "0:v:0", "-c:v", "libjxl",
               "-distance", "0", "-frames:v", "1", "-n", jxl)
        pixels = invoke("/usr/bin/ffmpeg", "-v", "error", "-i", jxl, "-frames:v", "1",
                        "-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1")
        if pixels != bytes([45, 90, 128]) * 1024:
            raise ValueError("Flatpak JPEG XL lossless pixel mismatch")

        output = directory / "encoded.mp4"
        invoke(worker, "convert", "-i", source, "-map", "0:v:0", "-c:v", "libx264",
               "-pix_fmt", "yuv420p", "-frames:v", "1", "-n", output)
        info = json.loads(invoke(worker, "probe", "metadata", output))
        stream = info["streams"][0]
        if (stream["codec_name"], stream["width"], stream["height"]) != ("h264", 32, 32):
            raise ValueError(f"Unexpected encoded stream: {stream}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", type=Path, help="Use an already rendered release manifest")
    parser.add_argument("--output", type=Path, default=Path("target/flathub-ci"))
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    manifest = args.manifest.resolve() if args.manifest else release_manifest(Path(__file__).resolve().parents[1], output)
    subprocess.run(["flatpak-builder", "--user", "--force-clean", "--install",
                    "--repo=repo", "app", manifest], cwd=output, check=True)
    smoke()


if __name__ == "__main__":
    main()
