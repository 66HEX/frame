#!/usr/bin/env python3
"""Compare unmodified, hash-pinned FFmpeg packages with the actual HEVC tests."""

import argparse
import hashlib
import json
import os
import platform
from pathlib import Path
import shutil
import signal
import subprocess
import sys
import time
import zipfile

ROOT = Path(__file__).resolve().parent.parent
PACKAGES = ROOT / "frame-core/tests/fixtures/hevc-runtime-comparison/packages.json"
CASES = {
    "copy": "hevc_mkv_reencode_should_write_hevc_video",
    "vflip-copy": "hevc_mkv_vflip_reencode_should_write_hevc_video",
}


def digest(path):
    checksum = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            checksum.update(block)
    return checksum.hexdigest()


def verify(path, expected):
    actual = digest(path)
    if actual != expected:
        raise RuntimeError(f"SHA-256 mismatch for {path}: {actual} != {expected}")


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


def run(command, log, env=None, timeout=120):
    """Retain combined output and stop the whole process tree if a probe hangs."""
    command = [str(part) for part in command]
    with log.open("wb") as stream:
        stream.write((json.dumps(command) + "\n").encode())
        stream.flush()
        process = subprocess.Popen(
            command, cwd=ROOT, env=env, stdout=stream, stderr=subprocess.STDOUT,
            start_new_session=os.name != "nt",
            creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0,
        )
        try:
            return process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            if os.name == "nt":
                subprocess.run(
                    ["taskkill", "/PID", str(process.pid), "/T", "/F"],
                    stdout=stream, stderr=subprocess.STDOUT, check=False,
                )
            else:
                os.killpg(process.pid, signal.SIGKILL)
            process.wait()
            stream.write(b"\nCOMPARISON TIMEOUT: process tree terminated\n")
            return 124


def checked(command, log, timeout=120):
    code = run(command, log, timeout=timeout)
    if code:
        raise RuntimeError(f"Command exited {code}; see {log.name}")


def host_key():
    system = platform.system()
    machine = platform.machine().lower()
    if system == "Linux" and machine == "x86_64":
        return "linux-x86_64"
    if system == "Darwin" and machine == "arm64":
        return "macos-arm64"
    if system == "Windows" and machine in ("amd64", "x86_64"):
        return "windows-x86_64"
    raise RuntimeError(f"No pinned native packages for {system}/{machine}")


def install(entries, directory, downloads, logs):
    directory.mkdir(parents=True, exist_ok=True)
    binaries = {}
    for entry in entries:
        archive = downloads / (entry["archive_sha256"] + ".zip")
        if not archive.exists():
            checked([
                "curl", "--fail", "--location", "--retry", "2",
                "--connect-timeout", "30", "--max-time", "300",
                "--proto", "=https", "--tlsv1.2",
                "--output", archive, entry["url"],
            ], logs / (entry["id"] + "-download.log"), timeout=1000)
        verify(archive, entry["archive_sha256"])
        binary = directory / entry["member_name"]
        with zipfile.ZipFile(archive) as zipped:
            members = [
                member for member in zipped.infolist()
                if not member.is_dir()
                and Path(member.filename).name == entry["member_name"]
            ]
            if len(members) != 1:
                raise RuntimeError(f"Expected one {entry['member_name']} in {archive.name}")
            with zipped.open(members[0]) as source, binary.open("wb") as target:
                shutil.copyfileobj(source, target)
        verify(binary, entry["sha256"])
        if os.name != "nt":
            binary.chmod(0o755)
        binaries[entry["id"]] = binary
        checked([binary, "-version"], logs / (entry["id"] + "-version.log"))
    return binaries


def build_test_binary(results):
    log = results / "build.log"
    checked([
        "cargo", "test", "--locked", "-p", "frame-core", "--test", "media_integration",
        "--no-run", "--message-format=json",
    ], log, timeout=1200)
    executables = []
    for line in log.read_text(encoding="utf-8", errors="replace").splitlines():
        try:
            item = json.loads(line)
        except json.JSONDecodeError:
            continue
        if (
            isinstance(item, dict)
            and item.get("reason") == "compiler-artifact"
            and item.get("target", {}).get("name") == "media_integration"
            and item.get("executable")
        ):
            executables.append(Path(item["executable"]))
    if len(executables) != 1:
        raise RuntimeError(f"Expected one media test executable, got {executables}")
    binary = executables[0]
    checked([binary, "--list", "--ignored"], results / "test-list.log")
    names = (results / "test-list.log").read_text(encoding="utf-8")
    for case in CASES.values():
        if f"{case}: test" not in names:
            raise RuntimeError(f"Missing independently runnable test: {case}")
    return binary


def save_summary(results, summary):
    write_json(results / "summary.json", summary)


def compare(output, repetitions):
    results = output / "results"
    downloads = output / "downloads"
    results.mkdir(parents=True, exist_ok=False)
    downloads.mkdir(parents=True, exist_ok=True)
    manifest = json.loads(PACKAGES.read_text(encoding="utf-8"))
    write_json(results / "packages.json", manifest)
    fixture = ROOT / manifest["fixture"]["path"]
    verify(fixture, manifest["fixture"]["sha256"])
    shutil.copyfile(fixture, results / "source.mp4")
    key = host_key()
    summary = {
        "platform": key, "os": platform.platform(), "machine": platform.machine(),
        "processor": platform.processor(),
        "commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "runner_image": os.environ.get("ImageVersion"),
        "run_id": os.environ.get("GITHUB_RUN_ID"),
        "run_attempt": os.environ.get("GITHUB_RUN_ATTEMPT"),
        "fixture_sha256": digest(fixture), "repetitions_per_case": repetitions,
        "versions": {}, "success": False,
    }
    save_summary(results, summary)
    cpu_command = {
        "linux-x86_64": ["lscpu"],
        "macos-arm64": ["sysctl", "machdep.cpu.brand_string", "hw.ncpu", "hw.physicalcpu", "hw.logicalcpu"],
        "windows-x86_64": [
            "powershell", "-NoProfile", "-Command",
            "Get-CimInstance Win32_Processor | Select-Object Name,Architecture,NumberOfCores,NumberOfLogicalProcessors | ConvertTo-Json",
        ],
    }[key]
    run(cpu_command, results / "cpu.log")
    binary = build_test_binary(results)
    for version, package in manifest["versions"].items():
        logs = results / version
        logs.mkdir()
        record = {
            "source_commit": package["source_commit"],
            "packages": package["platforms"][key], "trials": [], "success": False,
        }
        summary["versions"][version] = record
        save_summary(results, summary)
        try:
            tools = install(package["platforms"][key], output / "runtimes" / version, downloads, logs)
            checked([
                tools["ffprobe"], "-v", "error", "-count_frames", "-show_streams",
                "-show_format", "-of", "json", fixture,
            ], logs / "source-probe.log")
            probe_text = (logs / "source-probe.log").read_text(encoding="utf-8")
            probe = json.loads(probe_text.split("\n", 1)[1])
            video = [stream for stream in probe["streams"] if stream["codec_type"] == "video"]
            if len(video) != 1 or (
                video[0].get("codec_name"), video[0].get("width"),
                video[0].get("height"), video[0].get("nb_read_frames")
            ) != ("h264", 48, 32, "6"):
                raise RuntimeError("Pinned source does not probe as six 48x32 H.264 frames")
            for number in range(1, repetitions + 1):
                for label, name in CASES.items():
                    trial = logs / f"{number:02}-{label}"
                    trial.mkdir()
                    temporary = trial / "temporary"
                    temporary.mkdir()
                    environment = dict(os.environ, **{
                        "FRAME_TEST_FFMPEG": str(tools["ffmpeg"]),
                        "FRAME_TEST_FFPROBE": str(tools["ffprobe"]),
                        "FRAME_TEST_HEVC_SOURCE": str(fixture),
                        "FRAME_KEEP_MEDIA_TESTS": "1",
                        "FRAME_TEST_TRACE_TOOLS": "1",
                        "TMPDIR": str(temporary), "TMP": str(temporary), "TEMP": str(temporary),
                    })
                    started = time.monotonic()
                    code = run([
                        binary, name, "--exact", "--ignored", "--nocapture", "--test-threads=1",
                    ], trial / "test.log", env=environment)
                    text = (trial / "test.log").read_text(encoding="utf-8", errors="replace")
                    success = code == 0 and "test result: ok. 1 passed; 0 failed; 0 ignored;" in text
                    outcome = {
                        "case": label, "repetition": number, "exit_code": code,
                        "success": success, "seconds": round(time.monotonic() - started, 3),
                        "log": str((trial / "test.log").relative_to(results)),
                    }
                    record["trials"].append(outcome)
                    write_json(trial / "result.json", outcome)
                    save_summary(results, summary)
                    print(f"{key} {version} {label} {number}/{repetitions}: {'PASS' if success else 'FAIL'} (exit {code})", flush=True)
            record["success"] = (
                len(record["trials"]) == repetitions * len(CASES)
                and all(trial["success"] for trial in record["trials"])
            )
            # Confirm that no probe or conversion changed the supplied tools or fixture.
            for entry in package["platforms"][key]:
                verify(tools[entry["id"]], entry["sha256"])
            verify(fixture, manifest["fixture"]["sha256"])
        except Exception as error:
            record["error"] = str(error)
            record["success"] = False
            print(f"{key} {version}: ERROR: {error}", flush=True)
        save_summary(results, summary)
    summary["success"] = all(record["success"] for record in summary["versions"].values())
    save_summary(results, summary)
    markdown = ["| Platform | FFmpeg | Case | Passed | Failed |", "| --- | --- | --- | --- | --- |"]
    for version, record in summary["versions"].items():
        for label in CASES:
            trials = [trial for trial in record["trials"] if trial["case"] == label]
            passed = sum(trial["success"] for trial in trials)
            markdown.append(f"| {key} | {version} | {label} | {passed}/{repetitions} | {len(trials) - passed} |")
        if record.get("error"):
            markdown.append(f"\n{version}: setup/execution error: {record['error']}\n")
    report = "\n".join(markdown) + "\n"
    (results / "summary.md").write_text(report, encoding="utf-8")
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as stream:
            stream.write(report)
    print(report, flush=True)
    return 0 if summary["success"] else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "target/hevc-runtime-comparison")
    parser.add_argument("--repetitions", type=int, default=20)
    args = parser.parse_args()
    if args.repetitions < 1:
        parser.error("--repetitions must be positive")
    try:
        return compare(args.output.resolve(), args.repetitions)
    except Exception as error:
        print(f"COMPARISON ERROR: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
