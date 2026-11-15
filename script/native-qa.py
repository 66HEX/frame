#!/usr/bin/env python3
"""Build and verify the portable native QA package using the Python standard library."""

from dataclasses import dataclass
from datetime import datetime, timezone
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import signal
import stat
import struct
import subprocess
import sys
import tempfile
import time
import wave
import zipfile


@dataclass(frozen=True)
class Target:
    platform: str
    architecture: str
    triple: str
    suffix: str = ""

    @property
    def app_name(self):
        return f"frame-qa{self.suffix}"

    @property
    def runtime_names(self):
        return tuple(f"{tool}-{self.triple}{self.suffix}" for tool in ("ffmpeg", "ffprobe"))

    @property
    def archive_name(self):
        return f"frame-native-qa-{self.platform}-{self.architecture}.zip"


def identify_target(system=None, machine=None):
    system = (system or platform.system()).lower()
    machine = (machine or platform.machine()).lower()
    architecture = {"amd64": "x86_64", "x64": "x86_64", "arm64": "aarch64"}.get(machine, machine)
    targets = {
        ("windows", "x86_64"): Target("windows", "x86_64", "x86_64-pc-windows-msvc", ".exe"),
        ("linux", "x86_64"): Target("linux", "x86_64", "x86_64-unknown-linux-gnu"),
        ("linux", "aarch64"): Target("linux", "aarch64", "aarch64-unknown-linux-gnu"),
        ("darwin", "x86_64"): Target("macos", "x86_64", "x86_64-apple-darwin"),
        ("darwin", "aarch64"): Target("macos", "aarch64", "aarch64-apple-darwin"),
    }
    try:
        return targets[(system, architecture)]
    except KeyError as error:
        raise ValueError(f"Unsupported native QA target: {system}/{machine}") from error


def build_package(repo_root, output_dir, *, target, git_sha, pr_head_sha, build_id,
                  archive_limit_mib=100, source_dirty=False, binary_path=None, github_sha=None):
    if not math.isfinite(archive_limit_mib) or not 0 < archive_limit_mib <= 100:
        raise ValueError("Archive limit must be greater than zero and at most 100 MiB")
    repo_root, output_dir = Path(repo_root), Path(output_dir)
    inputs = {
        target.app_name: Path(binary_path) if binary_path else repo_root / "target" / "release" / target.app_name,
        **{
            f"binaries/{name}": repo_root / "frame-app" / "resources" / "binaries" / name
            for name in target.runtime_names
        },
    }
    for source in inputs.values():
        if source.is_symlink() or not source.is_file():
            raise ValueError(f"Missing regular QA binary: {source}")
    output_dir.mkdir(parents=True, exist_ok=True)
    archive = output_dir / target.archive_name
    with tempfile.TemporaryDirectory(prefix=".native-qa-build-", dir=output_dir) as staging:
        staged_archive = Path(staging) / target.archive_name
        files = {}
        with zipfile.ZipFile(staged_archive, "w", compression=zipfile.ZIP_DEFLATED) as package:
            for member, source in inputs.items():
                info = zipfile.ZipInfo(member)
                info.create_system = 3
                info.external_attr = (stat.S_IFREG | 0o755) << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                digest, size = hashlib.sha256(), 0
                with source.open("rb") as incoming, package.open(info, "w") as outgoing:
                    while chunk := incoming.read(1024 * 1024):
                        outgoing.write(chunk)
                        digest.update(chunk)
                        size += len(chunk)
                files[member] = {"sha256": digest.hexdigest(), "size": size, "executable": True}
            manifest = {
                "schemaVersion": 1,
                "gitSha": git_sha,
                "githubSha": github_sha,
                "prHeadSha": pr_head_sha,
                "buildId": build_id,
                "platform": target.platform,
                "architecture": target.architecture,
                "targetTriple": target.triple,
                "sourceDirty": source_dirty,
                "createdUtc": datetime.now(timezone.utc).isoformat(timespec="seconds"),
                "files": files,
            }
            package.writestr("manifest.json", json.dumps(manifest, indent=2, sort_keys=True) + "\n")
        if staged_archive.stat().st_size > archive_limit_mib * 1024 * 1024:
            raise ValueError(
                f"Archive exceeds {archive_limit_mib:g} MiB limit: "
                f"{staged_archive.stat().st_size / (1024 * 1024):.2f} MiB"
            )
        staged_archive.replace(archive)
    return archive


def sha256_file(path):
    digest = hashlib.sha256()
    with Path(path).open("rb") as source:
        while chunk := source.read(1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def unpack_verified(archive, destination, target):
    if Path(archive).stat().st_size > 100 * 1024 * 1024:
        raise ValueError("Archive exceeds the 100 MiB verification limit")
    expected = {target.app_name, *(f"binaries/{name}" for name in target.runtime_names)}
    with zipfile.ZipFile(archive) as package:
        members = package.namelist()
        if len(members) != len(set(members)) or set(members) != expected | {"manifest.json"}:
            raise ValueError("Unexpected or unsafe archive members")
        if package.getinfo("manifest.json").file_size > 1024 * 1024:
            raise ValueError("Manifest exceeds the size limit")
        if sum(info.file_size for info in package.infolist()) > 1024 * 1024 * 1024:
            raise ValueError("Unpacked package exceeds the size limit")
        manifest = json.loads(package.read("manifest.json"))
        if manifest.get("schemaVersion") != 1:
            raise ValueError("Unsupported package manifest version")
        if (manifest.get("platform"), manifest.get("architecture"), manifest.get("targetTriple")) != (
            target.platform, target.architecture, target.triple
        ):
            raise ValueError("Package target does not match this verification host")
        if set(manifest.get("files", {})) != expected:
            raise ValueError("Manifest does not describe exactly the packaged binaries")
        for member in sorted(expected):
            info = package.getinfo(member)
            if stat.S_ISLNK(info.external_attr >> 16):
                raise ValueError(f"Unsafe symbolic link in package: {member}")
            path = destination / member
            path.parent.mkdir(parents=True, exist_ok=True)
            digest, size = hashlib.sha256(), 0
            with package.open(info) as incoming, path.open("xb") as outgoing:
                while chunk := incoming.read(1024 * 1024):
                    outgoing.write(chunk)
                    digest.update(chunk)
                    size += len(chunk)
            expected_file = manifest["files"][member]
            if digest.hexdigest() != expected_file.get("sha256") or size != expected_file.get("size"):
                raise ValueError(f"Payload hash or size mismatch: {member}")
            path.chmod(0o755)
    return manifest


def run_command(args, *, cwd, env, timeout):
    if not math.isfinite(timeout) or not 0 < timeout <= 2_147_483:
        raise ValueError("QA command timeout must be finite, positive and representable")
    with tempfile.TemporaryDirectory(prefix="frame-qa-command-") as temporary:
        folder = Path(temporary)
        stdout, stderr = folder / "stdout", folder / "stderr"
        if os.name == "nt":
            code, timed_out = _run_windows_command(args, cwd, env, timeout, folder, stdout, stderr)
        else:
            code, timed_out = _run_posix_command(args, cwd, env, timeout, stdout, stderr)
        output = stdout.read_text(encoding="utf-8", errors="replace")
        errors = stderr.read_text(encoding="utf-8", errors="replace")
        if timed_out:
            raise subprocess.TimeoutExpired(args, timeout, output=output, stderr=errors)
        return subprocess.CompletedProcess(args, code, output, errors)


def _run_posix_command(args, cwd, env, timeout, stdout, stderr):
    with stdout.open("wb") as output, stderr.open("wb") as errors:
        process = subprocess.Popen(args, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                   stdout=output, stderr=errors, start_new_session=True)
        timed_out = False
        try:
            code = process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            timed_out, code = True, None
        finally:
            # The session belongs only to this command. Parent exit does not
            # release descendants, including ones holding inherited output.
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=5)
        return code, timed_out


def _run_windows_command(args, cwd, env, timeout, folder, stdout, stderr):
    request, response = folder / "request.json", folder / "response.json"
    request.write_text(json.dumps({
        "executable": str(args[0]), "arguments": subprocess.list2cmdline([str(arg) for arg in args[1:]]),
        "directory": str(Path(cwd).resolve()), "stdout": str(stdout), "stderr": str(stderr),
        "ffmpeg": env.get("FRAME_FFMPEG_PATH", ""), "ffprobe": env.get("FRAME_FFPROBE_PATH", ""),
        "timeoutMs": math.ceil(timeout * 1000),
    }), encoding="utf-8")
    shell = Path(os.environ["SystemRoot"]) / "System32/WindowsPowerShell/v1.0/powershell.exe"
    bridge = Path(__file__).resolve().parent / "qa/run-process.ps1"
    diagnostics = folder / "bridge.log"
    with diagnostics.open("wb") as log:
        process = subprocess.Popen(
            [str(shell), "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File",
             str(bridge), "-Request", str(request), "-Response", str(response)],
            cwd=cwd, env=env, stdin=subprocess.DEVNULL, stdout=log, stderr=log,
        )
        try:
            code = process.wait(timeout=timeout + 20)
        except BaseException:
            # Closing this PowerShell process closes its noninherited job handle,
            # which terminates the application and its descendants as well.
            process.kill()
            process.wait(timeout=5)
            raise
    if code or not response.is_file():
        raise ValueError("Windows QA process ownership failed: " + diagnostics.read_text(
            encoding="utf-8", errors="replace")[-4000:])
    result = json.loads(response.read_text(encoding="utf-8-sig"))
    if type(result.get("exitCode")) is not int or type(result.get("timedOut")) is not bool:
        raise ValueError("Invalid Windows QA process result")
    return result["exitCode"], result["timedOut"]


def create_synthetic_wav(path):
    samples = [round(12000 * math.sin(2 * math.pi * 440 * index / 48000)) for index in range(12000)]
    with wave.open(str(path), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(48000)
        output.writeframes(struct.pack(f"<{len(samples)}h", *samples))


def pcm_content(path):
    with wave.open(str(path), "rb") as source:
        return (source.getnchannels(), source.getsampwidth(), source.getframerate(),
                source.getnframes(), source.readframes(source.getnframes()))


def _verify_archive(archive, repo_root, runner, evidence, evidence_path):
    archive, repo_root = Path(archive).resolve(), Path(repo_root).resolve()
    runner = runner or run_command
    target = identify_target()
    with tempfile.TemporaryDirectory(prefix="frame-native-qa-verify-") as temporary:
        workspace = Path(temporary).resolve()
        evidence["workspace"] = str(workspace)
        if workspace.is_relative_to(repo_root):
            raise ValueError("Verification must unpack outside the checkout; choose an external TMPDIR")
        manifest = unpack_verified(archive, workspace, target)
        evidence["manifest"] = manifest
        app = workspace / target.app_name
        ffmpeg, ffprobe = (workspace / "binaries" / name for name in target.runtime_names)
        environment = dict(os.environ, FRAME_FFMPEG_PATH=str(ffmpeg), FRAME_FFPROBE_PATH=str(ffprobe))

        def invoke(args):
            args = [str(arg) for arg in args]
            record = {"argv": args, "cwd": str(workspace), "runtimeEnvironment": {
                "FRAME_FFMPEG_PATH": str(ffmpeg), "FRAME_FFPROBE_PATH": str(ffprobe),
            }}
            evidence["commands"].append(record)
            if evidence_path:
                write_report(evidence_path, evidence)
            started = time.monotonic()
            try:
                result = runner(args, cwd=workspace, env=environment, timeout=120)
                record.update(returnCode=result.returncode, stdout=result.stdout, stderr=result.stderr)
            except subprocess.TimeoutExpired as error:
                def text(value):
                    return value.decode("utf-8", errors="replace") if isinstance(value, bytes) else value or ""
                record.update(timeout=True, stdout=text(error.stdout), stderr=text(error.stderr))
                raise
            finally:
                record["durationMs"] = round((time.monotonic() - started) * 1000)
                if evidence_path:
                    write_report(evidence_path, evidence)
            if result.returncode:
                raise ValueError(f"QA command failed ({result.returncode}): {args[0]}\n{result.stderr[-4000:]}")
            return result.stdout

        versions = {
            "ffmpeg": invoke([ffmpeg, "-version"]).splitlines()[0],
            "ffprobe": invoke([ffprobe, "-version"]).splitlines()[0],
        }
        source = workspace / "Zażółć source.wav"
        create_synthetic_wav(source)
        source_hash = sha256_file(source)
        source_pcm = pcm_content(source)
        output = workspace / "output's space"
        output.mkdir()
        sentinel = output / "result.wav"
        sentinel.write_bytes(b"existing output must survive")
        sentinel_hash = sha256_file(sentinel)
        request = workspace / "request.json"
        request.write_text(json.dumps({
            "input": str(source), "outputDirectory": str(output), "outputName": "result.wav",
            "config": {"container": "wav", "audioCodec": "copy", "processingMode": "copy", "selectedAudioTracks": [0]},
        }), encoding="utf-8")
        evidence["request"] = json.loads(request.read_text(encoding="utf-8"))

        plan = json.loads(invoke([app, "--plan", request]))
        if (plan.get("normalizedConfig", {}).get("processingMode") != "copy"
                or plan.get("outputPath") != str(sentinel) or "-n" not in plan.get("args", [])):
            raise ValueError("QA plan did not preserve the requested Copy/output contract")
        if set(output.iterdir()) != {sentinel} or sha256_file(sentinel) != sentinel_hash:
            raise ValueError("Planning changed the output directory")
        if sha256_file(source) != source_hash:
            raise ValueError("Planning changed the source")

        events = [json.loads(line) for line in invoke([app, "--export", request]).splitlines() if line.strip()]
        completed = [event for event in events if event.get("event") == "conversion-completed"]
        if len(completed) != 1 or any(event.get("event") in {"conversion-error", "conversion-cancelled"} for event in events):
            raise ValueError("Export must report exactly one Completed event and no failure")
        actual_path = completed[0].get("payload", {}).get("output_path")
        if not isinstance(actual_path, str):
            raise ValueError("Completed event is missing output_path")
        result = Path(actual_path)
        if not result.is_absolute() or result.parent.resolve() != output or not result.is_file() or result.resolve() == sentinel:
            raise ValueError("Completed result must use a separate collision-safe path in the output directory")
        if sha256_file(source) != source_hash:
            raise ValueError("Export changed the source")
        if sha256_file(sentinel) != sentinel_hash:
            raise ValueError("Export changed the existing collision target")
        if set(output.iterdir()) != {sentinel, result}:
            raise ValueError("Export left unexpected staging files in the output directory")
        if pcm_content(result) != source_pcm:
            raise ValueError("Exported WAV samples do not match the source")
        probe = json.loads(invoke([ffprobe, "-v", "error", "-show_streams", "-of", "json", result]))
        audio = [stream for stream in probe.get("streams", []) if stream.get("codec_type") == "audio"]
        if len(audio) != 1 or audio[0].get("codec_name") != "pcm_s16le":
            raise ValueError("Bundled ffprobe did not confirm the expected PCM audio stream")
        return {
            "verified": True,
            "gitSha": manifest["gitSha"], "prHeadSha": manifest["prHeadSha"],
            "githubSha": manifest.get("githubSha"),
            "buildId": manifest["buildId"], "sourceDirty": manifest["sourceDirty"],
            "platform": target.platform, "architecture": target.architecture,
            "archiveSha256": sha256_file(archive), "payloadsVerified": len(manifest["files"]),
            "runtimeVersions": versions, "completedOutput": result.name, "sourceSha256": source_hash,
            "checks": {"unpackedOutsideCheckout": True, "planDoesNotWrite": True,
                       "sourceUnchanged": True, "collisionPreserved": True,
                       "completedReported": True, "audioSamplesMatch": True, "stagingCleaned": True},
        }


def verify_archive(archive, repo_root, *, runner=None, evidence_path=None):
    evidence = {"schemaVersion": 1, "verified": False, "archive": str(Path(archive).resolve()), "commands": []}
    try:
        evidence["archiveSha256"] = sha256_file(archive)
        report = _verify_archive(archive, repo_root, runner, evidence, evidence_path)
        evidence["verified"] = True
        evidence["report"] = report
        return report
    except Exception as error:
        evidence["error"] = str(error)
        raise
    finally:
        if evidence_path:
            write_report(evidence_path, evidence)


def repository_metadata(repo_root):
    def git(*arguments):
        result = subprocess.run(
            ["git", *arguments], cwd=repo_root, capture_output=True, text=True,
            encoding="utf-8", errors="replace", timeout=15, check=False,
        )
        if result.returncode:
            raise ValueError(f"Cannot record source revision: {result.stderr.strip()}")
        return result.stdout.strip()

    git_sha = git("rev-parse", "HEAD")
    github_sha = os.environ.get("GITHUB_SHA") or None
    selected_pr_head = os.environ.get("FRAME_QA_PR_HEAD") or None
    pr_head = selected_pr_head
    event_path = os.environ.get("GITHUB_EVENT_PATH")
    if not pr_head and event_path:
        event = json.loads(Path(event_path).read_text(encoding="utf-8"))
        pr_head = event.get("pull_request", {}).get("head", {}).get("sha")
    if pr_head and not re.fullmatch(r"[0-9a-fA-F]{40}|[0-9a-fA-F]{64}", pr_head):
        raise ValueError("PR head must be a full Git object ID")
    expected_checkout = selected_pr_head or github_sha
    if expected_checkout and git_sha != expected_checkout:
        raise ValueError("Checked-out revision differs from the requested checkout")
    run_id = os.environ.get("GITHUB_RUN_ID")
    build_id = f"{run_id}-{os.environ.get('GITHUB_RUN_ATTEMPT', '1')}" if run_id else (
        f"local-{datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ')}-{os.getpid()}"
    )
    return {
        "git_sha": git_sha, "github_sha": github_sha, "pr_head_sha": pr_head, "build_id": build_id,
        "source_dirty": bool(git("status", "--porcelain", "--untracked-files=normal")),
    }


def write_report(path, report):
    path = Path(path)
    with tempfile.NamedTemporaryFile("w", encoding="utf-8", dir=path.parent, prefix=".qa-report-", delete=False) as output:
        temporary = Path(output.name)
        json.dump(report, output, indent=2, sort_keys=True)
        output.write("\n")
    try:
        temporary.replace(path)
    finally:
        temporary.unlink(missing_ok=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    build = commands.add_parser("build", help="Package an already-built release QA application")
    build.add_argument("--output", type=Path, default=Path("target/native-qa"))
    build.add_argument("--archive-limit-mib", type=float, default=100)
    verify = commands.add_parser("verify", help="Unpack and exercise the package outside the checkout")
    verify.add_argument("--output", type=Path, default=Path("target/native-qa"))
    args = parser.parse_args(argv)
    repo_root = Path(__file__).resolve().parents[1]
    output = args.output.resolve()
    try:
        target = identify_target()
        if args.command == "build":
            metadata = repository_metadata(repo_root)
            build_dir = Path(os.environ.get("CARGO_TARGET_DIR", repo_root / "target"))
            if not build_dir.is_absolute():
                build_dir = repo_root / build_dir
            if cargo_target := os.environ.get("CARGO_BUILD_TARGET"):
                if cargo_target != target.triple:
                    raise ValueError("Cross-built packages require verification on their native target")
                build_dir /= cargo_target
            archive = build_package(
                repo_root, output, target=target, **metadata,
                archive_limit_mib=args.archive_limit_mib,
                binary_path=build_dir / "release" / target.app_name,
            )
            report = {
                "archive": str(archive), "archiveBytes": archive.stat().st_size,
                "archiveSha256": sha256_file(archive), "targetTriple": target.triple,
                "gitSha": metadata["git_sha"], "prHeadSha": metadata["pr_head_sha"],
                "githubSha": metadata["github_sha"],
                "buildId": metadata["build_id"], "sourceDirty": metadata["source_dirty"],
            }
            write_report(output / "build.json", report)
            write_report(output / "verification.json", {"verified": False, "archiveSha256": report["archiveSha256"]})
        else:
            archive = output / target.archive_name
            if output.is_dir():
                write_report(output / "verification.json", {"verified": False})
            report = verify_archive(archive, repo_root, evidence_path=output / "evidence.json" if output.is_dir() else None)
            write_report(output / "verification.json", report)
        print(json.dumps(report, sort_keys=True))
        return 0
    except (ValueError, OSError, subprocess.SubprocessError, zipfile.BadZipFile, wave.Error) as error:
        if args.command == "verify" and output.is_dir():
            write_report(output / "verification.json", {"verified": False, "error": str(error)})
        print(f"native-qa: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
