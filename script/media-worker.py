#!/usr/bin/env python3
"""Prepare the release SDK and build a relocatable Frame media worker."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import wave

ROOT = Path(__file__).resolve().parents[1]
TARGET_DIR = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
CACHE = TARGET_DIR / "media-sdk"
PORTS = ROOT / "packaging/media"
RELEASE = "https://github.com/FFmpeg-Builds/FFmpeg-Builds/releases/download/autobuild-2026-10-31-12-59"
ARCHIVES = {
    "x86_64-unknown-linux-gnu": ("linux64", "90a42a7d9689c031beacc3d03b139fd8243351d616988e8a74188df949fe5d08"),
    "aarch64-unknown-linux-gnu": ("linuxarm64", "d0a01c8a3c2e31f8613b42c2f7190869e2b87bb3044748702ec4fef71c0bd1b9"),
    "x86_64-pc-windows-msvc": ("win64", "1d9823c35c7a7e2f54d1438cf61030d6efabe3278de01c191309d1e828bb74df"),
}
MAC_TRIPLETS = {"aarch64-apple-darwin": "arm64-frame", "x86_64-apple-darwin": "x64-frame"}
LIBRARIES = ("avcodec", "avformat", "avutil", "avfilter", "swscale", "swresample")


def digest(path):
    value = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def fetch_sdk(target):
    platform, checksum = ARCHIVES[target]
    name = f"ffmpeg-n9.0.2-39-g7adfbe40dc-{platform}-gpl-shared-9.0"
    archive = name + (".zip" if platform == "win64" else ".tar.xz")
    destination = CACHE / name
    if destination.is_dir():
        return destination
    with tempfile.TemporaryDirectory(dir=CACHE) as temporary:
        staging = Path(temporary)
        data = staging / archive
        subprocess.run(["curl", "--fail", "--location", "--retry", "3", "--proto", "=https",
                        "-o", data, f"{RELEASE}/{archive}"], check=True)
        if digest(data) != checksum:
            raise ValueError(f"Unexpected SHA-256 for {archive}")
        subprocess.run(["tar", "-xf", data, "-C", staging], check=True)
        try:
            (staging / name).rename(destination)
        except OSError:
            if not destination.is_dir():
                raise
    return destination


def mac_sdk(target):
    version = json.loads((PORTS / "vcpkg.json").read_text())["builtin-baseline"]
    checkout = CACHE / f"vcpkg-{version}"
    if not checkout.exists():
        with tempfile.TemporaryDirectory(dir=CACHE) as temporary:
            source = Path(temporary) / "source"
            subprocess.run(["git", "init", "--quiet", source], check=True)
            subprocess.run(["git", "-C", source, "fetch", "--depth=1",
                            "https://github.com/microsoft/vcpkg", version], check=True)
            subprocess.run(["git", "-C", source, "checkout", "--quiet", "FETCH_HEAD"], check=True)
            subprocess.run([source / "bootstrap-vcpkg.sh", "-disableMetrics"], check=True)
            source.rename(checkout)
    # Keep upstream port patches and license fixes; append x265's multilib
    # build to the exact builtin port selected by the pinned baseline.
    overlay = CACHE / f"ports-{version}"
    extra = (PORTS / "x265-multilib.cmake").read_bytes()
    overlay = overlay / hashlib.sha256(extra).hexdigest()
    if not overlay.exists():
        overlay.parent.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=overlay.parent) as temporary:
            stage = Path(temporary) / "ports"
            shutil.copytree(checkout / "ports/x265", stage / "x265")
            with (stage / "x265/portfile.cmake").open("ab") as output:
                output.write(b"\nvcpkg_find_acquire_program(NINJA)\n" + extra)
            stage.rename(overlay)
    triplet = MAC_TRIPLETS[target]
    installed = CACHE / "installed"
    subprocess.run([checkout / "vcpkg", "install", f"--x-manifest-root={PORTS}",
                    f"--overlay-triplets={PORTS}", f"--overlay-ports={overlay}", f"--x-install-root={installed}",
                    "--triplet", triplet], check=True)
    return installed / triplet


def sdk_for(target):
    CACHE.mkdir(parents=True, exist_ok=True)
    if os.environ.get("FFMPEG_DIR"):
        return Path(os.environ["FFMPEG_DIR"]).resolve()
    return mac_sdk(target) if target in MAC_TRIPLETS else fetch_sdk(target)


def runtime_libraries(sdk, target):
    if target in MAC_TRIPLETS:
        directory, pattern = "lib", r"lib{}\.\d+\.dylib"
    elif target.endswith("msvc"):
        directory, pattern = "bin", r"{}-\d+\.dll"
    else:
        directory, pattern = "lib", r"lib{}\.so\.\d+"
    files = list((sdk / directory).iterdir())
    for name in LIBRARIES:
        matches = [file for file in files if re.fullmatch(pattern.format(name), file.name)]
        if len(matches) != 1:
            raise ValueError(f"Expected one versioned {name} library in {sdk / directory}")
        yield matches[0]


def relocate_macos(worker, libraries):
    # Vcpkg install names can contain the SDK prefix. Resolve every libav edge
    # within Contents/Frameworks, and reject accidental dependencies on the host.
    for binary in [*libraries, worker]:
        destination = "@loader_path/"
        if binary == worker:
            destination += os.path.relpath(libraries[0].parent, worker.parent) + "/"
        args = [] if binary == worker else ["-id", f"@rpath/{binary.name}"]
        deps = subprocess.check_output(["otool", "-L", binary], text=True).splitlines()[1:]
        if binary != worker:
            deps = deps[1:]  # LC_ID_DYLIB is the first entry
        for entry in deps:
            dep = entry.strip().split(" (compatibility version")[0]
            if dep.startswith(("/usr/lib/", "/System/Library/")):
                continue
            name = Path(dep).name
            if name not in {lib.name for lib in libraries}:
                raise ValueError(f"{binary.name} requires an unpackaged dylib: {dep}")
            args.extend(["-change", dep, destination + name])
        if args:
            subprocess.run(["install_name_tool", *args, binary], check=True)
        subprocess.run(["codesign", "--force", "--sign", "-", binary], check=True)


def smoke(worker):
    env = {k: v for k, v in os.environ.items() if k not in {
        "LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH", "DYLD_FALLBACK_LIBRARY_PATH", "FFMPEG_DIR"}}
    if os.name == "nt":
        # The actual installer lives beside the six DLLs. Exclude developer
        # and runner SDK directories from Windows' fallback search path.
        env["PATH"] = str(Path(env["SystemRoot"]) / "System32")
    with tempfile.TemporaryDirectory() as temporary:
        fixture = Path(temporary) / "relocation test.wav"
        with wave.open(str(fixture), "wb") as output:
            output.setparams((1, 2, 8000, 0, "NONE", ""))
            output.writeframes(bytes(1600))
        def invoke(args):
            return subprocess.check_output([worker.resolve(), *args], cwd=temporary, env=env,
                                           text=True, timeout=30)
        encoders = invoke(["capabilities", "-encoders"])
        for codec in ("libx264", "libx265", "libvpx-vp9", "libsvtav1", "libopus", "libmp3lame", "libwebp"):
            if not re.search(r"\b" + re.escape(codec) + r"\b", encoders):
                raise ValueError(f"SDK missing encoder {codec}")
        filters = invoke(["capabilities", "-filters"])
        for name in ("subtitles", "deshake", "palettegen", "paletteuse", "loudnorm"):
            if not re.search(r"\b" + name + r"\b", filters):
                raise ValueError(f"SDK missing filter {name}")
        info = json.loads(invoke(["probe", "metadata", fixture]))
        if info["streams"][0]["codec_name"] != "pcm_s16le":
            raise ValueError("Packaged worker failed PCM probe")


def bundle(target, sdk, bin_dir, lib_dir, profile):
    env = dict(os.environ, FFMPEG_DIR=str(sdk))
    flags = []
    if target.endswith("linux-gnu"):
        relative = os.path.relpath(lib_dir, bin_dir)
        flags = ["-C", f"link-arg=-Wl,--disable-new-dtags,-rpath,$ORIGIN/{relative}",
                 "-C", f"link-arg=-Wl,-rpath-link,{sdk / 'lib'}"]
    if target in MAC_TRIPLETS:
        flags = ["-C", "link-arg=-Wl,-headerpad_max_install_names"]
    args = ["cargo", "rustc", "--locked", "-p", "frame-media", "--bin", "frame-media", "--target", target]
    if profile == "release":
        args.append("--release")
    subprocess.run([*args, "--", *flags], cwd=ROOT, env=env, check=True)
    name = "frame-media.exe" if target.endswith("msvc") else "frame-media"
    bin_dir.mkdir(parents=True, exist_ok=True)
    lib_dir.mkdir(parents=True, exist_ok=True)
    worker = bin_dir / name
    shutil.copy2(TARGET_DIR / target / profile / name, worker)
    installed = [Path(shutil.copy2(path, lib_dir)) for path in runtime_libraries(sdk, target)]
    if target in MAC_TRIPLETS:
        relocate_macos(worker, installed)
    smoke(worker)
    notices = bin_dir / "frame-media.LICENSE"
    if (sdk / "LICENSE.txt").is_file():
        shutil.copy2(sdk / "LICENSE.txt", notices)
    else:
        copyrights = sorted((sdk / "share").glob("*/copyright"))
        if not copyrights:
            raise ValueError(f"SDK is missing distribution licenses: {sdk}")
        with notices.open("wb") as output:
            for path in copyrights:
                output.write(f"\n{path.parent.name}\n\n".encode())
                output.write(path.read_bytes())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("setup-ci", "bundle", "verify"))
    parser.add_argument("--target")
    parser.add_argument("--bin-dir", type=Path)
    parser.add_argument("--lib-dir", type=Path)
    parser.add_argument("--profile", choices=("debug", "release"), default="release")
    args = parser.parse_args()
    host = re.search(r"^host: (.+)$", subprocess.check_output(["rustc", "-vV"], text=True), re.M)[1]
    target = args.target or host
    if target not in ARCHIVES and target not in MAC_TRIPLETS:
        parser.error(f"Unsupported target {target}")
    if args.action == "verify":
        smoke(args.bin_dir / ("frame-media.exe" if target.endswith("msvc") else "frame-media"))
        return
    sdk = sdk_for(target)
    if args.action == "setup-ci":
        with open(os.environ["GITHUB_ENV"], "a", encoding="utf8") as output:
            print(f"FFMPEG_DIR={sdk}", file=output)
            if target.endswith("linux-gnu"):
                print(f"LD_LIBRARY_PATH={sdk / 'lib'}", file=output)
                print(f"LIBRARY_PATH={sdk / 'lib'}", file=output)
            elif target in MAC_TRIPLETS:
                print(f"DYLD_LIBRARY_PATH={sdk / 'lib'}", file=output)
        if target.endswith("msvc"):
            with open(os.environ["GITHUB_PATH"], "a", encoding="utf8") as output:
                print(sdk / "bin", file=output)
    else:
        if not args.bin_dir or not args.lib_dir:
            parser.error("bundle requires --bin-dir and --lib-dir")
        bundle(target, sdk, args.bin_dir, args.lib_dir, args.profile)


if __name__ == "__main__":
    main()
