import hashlib
import importlib.util
import io
import json
import os
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch
import zipfile


SCRIPT = Path(__file__).resolve().parents[1] / "native-qa.py"
SPEC = importlib.util.spec_from_file_location("native_qa", SCRIPT)
native_qa = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = native_qa
SPEC.loader.exec_module(native_qa)


class PackageFixture(unittest.TestCase):
    def setUp(self):
        self.sandbox = tempfile.TemporaryDirectory(prefix="frame-native-qa-test-")
        self.addCleanup(self.sandbox.cleanup)
        self.repo = Path(self.sandbox.name) / "checkout"
        self.output = self.repo / "target" / "native-qa"
        self.target = native_qa.identify_target("Linux", "x86_64")

    def create_inputs(self, target=None):
        target = target or self.target
        inputs = {
            target.app_name: self.repo / "target" / "release" / target.app_name,
            **{name: self.repo / "target/native-qa-worker" / name for name in target.worker_members},
            **{
                f"binaries/{name}": self.repo / "frame-app" / "resources" / "binaries" / name
                for name in target.runtime_names
            },
        }
        for member, path in inputs.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(f"binary fixture for {member}".encode())
        return inputs

    def build(self, **kwargs):
        return native_qa.build_package(
            self.repo, self.output, target=self.target,
            git_sha="a" * 40, pr_head_sha="b" * 40, build_id="12345-2",
            **kwargs,
        )


class PackageTests(PackageFixture):
    def test_build_contains_only_portable_binaries_and_hashed_manifest(self):
        for system, machine in [("Windows", "AMD64"), ("Linux", "x86_64"), ("Linux", "aarch64")]:
            with self.subTest(system=system, machine=machine):
                self.target = native_qa.identify_target(system, machine)
                inputs = self.create_inputs()
                archive = self.build()
                self.assertTrue(archive.is_file(), "build must produce the portable ZIP")
                with zipfile.ZipFile(archive) as package:
                    self.assertEqual(set(package.namelist()), {*inputs, "manifest.json"})
                    manifest = json.loads(package.read("manifest.json"))
                    self.assertEqual(manifest["gitSha"], "a" * 40)
                    self.assertEqual(manifest["prHeadSha"], "b" * 40)
                    self.assertEqual(manifest["buildId"], "12345-2")
                    self.assertEqual(manifest["targetTriple"], self.target.triple)
                    for member, source in inputs.items():
                        self.assertEqual(package.read(member), source.read_bytes())
                        self.assertEqual(manifest["files"][member]["sha256"], hashlib.sha256(source.read_bytes()).hexdigest())
                        self.assertEqual(manifest["files"][member]["size"], source.stat().st_size)

    def test_archive_budget_fails_without_replacing_an_existing_package(self):
        self.create_inputs()
        self.output.mkdir(parents=True)
        previous = self.output / self.target.archive_name
        previous.write_bytes(b"previous package")
        with self.assertRaisesRegex(ValueError, "limit"):
            self.build(archive_limit_mib=0.00001)
        self.assertEqual(previous.read_bytes(), b"previous package")

    def test_missing_runtime_does_not_publish_an_incomplete_package(self):
        inputs = self.create_inputs()
        inputs[f"binaries/{self.target.runtime_names[1]}"].unlink()
        with self.assertRaisesRegex(ValueError, "Missing"):
            self.build()
        self.assertFalse((self.output / self.target.archive_name).exists())

    def test_archive_size_policy_is_target_specific_and_keeps_ci_total_bounded(self):
        limits = {}
        for system, machine, expected in [
            ("Windows", "AMD64", 240), ("Linux", "x86_64", 180),
            ("Linux", "aarch64", 180), ("Darwin", "x86_64", 180),
            ("Darwin", "arm64", 180),
        ]:
            with self.subTest(system=system, machine=machine):
                target = native_qa.identify_target(system, machine)
                self.assertEqual(target.archive_limit_mib, expected)
                limits[(target.platform, target.architecture)] = target.archive_limit_mib
        self.assertEqual(sum(value for (system, _), value in limits.items() if system != "macos"), 600)

    def test_explicit_archive_limits_cannot_exceed_the_target_policy(self):
        for system, machine, maximum in [
            ("Windows", "AMD64", 240), ("Linux", "x86_64", 180),
            ("Linux", "aarch64", 180), ("Darwin", "x86_64", 180),
        ]:
            with self.subTest(system=system, machine=machine):
                self.target = native_qa.identify_target(system, machine)
                self.create_inputs()
                for limit in (0, -1, float("nan"), float("inf"), maximum + 1):
                    with self.subTest(limit=limit), self.assertRaisesRegex(ValueError, str(maximum)):
                        self.build(archive_limit_mib=limit)
                self.assertTrue(self.build(archive_limit_mib=maximum).is_file())
                self.assertTrue(self.build(archive_limit_mib=1).is_file())

    def test_default_windows_cli_budget_builds_and_verifies_more_than_100_mib(self):
        self.target = native_qa.identify_target("Windows", "AMD64")
        inputs = self.create_inputs()
        chunk = os.urandom(1024 * 1024)
        expected_hash = hashlib.sha256()
        with inputs[self.target.app_name].open("wb") as output:
            for _ in range(101):
                output.write(chunk)
                expected_hash.update(chunk)
        metadata = {"git_sha": "a" * 40, "pr_head_sha": "a" * 40,
                    "github_sha": "a" * 40, "build_id": "size-test", "source_dirty": False}
        stdout, stderr = io.StringIO(), io.StringIO()
        with patch.object(native_qa, "__file__", str(self.repo / "script/native-qa.py")), \
             patch.object(native_qa, "identify_target", return_value=self.target), \
             patch.object(native_qa, "repository_metadata", return_value=metadata), \
             patch.dict(os.environ, {"CARGO_TARGET_DIR": str(self.repo / "target"), "CARGO_BUILD_TARGET": ""}), \
             redirect_stdout(stdout), redirect_stderr(stderr):
            code = native_qa.main(["build", "--output", str(self.output)])
        self.assertEqual(code, 0, stderr.getvalue())
        archive = self.output / self.target.archive_name
        self.assertEqual(json.loads(stdout.getvalue())["archiveBytes"], archive.stat().st_size)
        self.assertGreater(archive.stat().st_size, 100 * 1024 * 1024)
        self.assertLessEqual(archive.stat().st_size, 240 * 1024 * 1024)
        relocated = self.repo.parent / "relocated"
        manifest = native_qa.unpack_verified(archive, relocated, self.target)
        self.assertEqual(manifest["files"][self.target.app_name]["size"], 101 * 1024 * 1024)
        self.assertEqual(native_qa.sha256_file(relocated / self.target.app_name), expected_hash.hexdigest())

    def test_unpack_rejects_an_archive_above_each_target_budget(self):
        oversized = self.repo.parent / "oversized.zip"
        for system, machine, maximum in [
            ("Windows", "AMD64", 240), ("Linux", "x86_64", 180),
            ("Linux", "aarch64", 180), ("Darwin", "arm64", 180),
        ]:
            with self.subTest(system=system, machine=machine):
                with oversized.open("wb") as output:
                    output.truncate(maximum * 1024 * 1024 + 1)
                destination = self.repo.parent / "must-not-unpack"
                with self.assertRaisesRegex(ValueError, str(maximum)):
                    native_qa.unpack_verified(oversized, destination,
                                             native_qa.identify_target(system, machine))
                self.assertFalse(destination.exists())


class CommandTests(unittest.TestCase):
    def test_help_exposes_both_build_and_verify_commands(self):
        result = subprocess.run([sys.executable, "-B", str(SCRIPT), "--help"], capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0)
        self.assertIn("build", result.stdout)
        self.assertIn("verify", result.stdout)

    def test_missing_package_is_a_nonzero_cli_failure(self):
        with tempfile.TemporaryDirectory(prefix="frame-native-qa-missing-") as directory:
            result = subprocess.run([sys.executable, "-B", str(SCRIPT), "verify", "--output", directory], capture_output=True, text=True, check=False)
        self.assertNotEqual(result.returncode, 0)


class OwnedProcessTests(unittest.TestCase):
    @staticmethod
    def process_running(pid):
        if os.name == "nt":
            import ctypes
            from ctypes import wintypes
            kernel = ctypes.WinDLL("kernel32", use_last_error=True)
            kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
            kernel.OpenProcess.restype = wintypes.HANDLE
            kernel.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
            kernel.WaitForSingleObject.restype = wintypes.DWORD
            kernel.CloseHandle.argtypes = [wintypes.HANDLE]
            handle = kernel.OpenProcess(0x100000, False, pid)
            if not handle:
                if ctypes.get_last_error() != 87:
                    raise ctypes.WinError(ctypes.get_last_error())
                return False
            try:
                result = kernel.WaitForSingleObject(handle, 0)
                if result not in (0, 0x102):
                    raise ctypes.WinError(ctypes.get_last_error())
                return result == 0x102
            finally:
                kernel.CloseHandle(handle)
        try:
            os.kill(pid, 0)
        except ProcessLookupError:
            return False
        status = Path(f"/proc/{pid}/stat")
        if status.is_file():
            try:
                if status.read_text().split(")", 1)[1].split()[0] == "Z":
                    return False
            except FileNotFoundError:
                return False
        return True

    def assert_owned_descendant_stopped(self, parent_exits):
        with tempfile.TemporaryDirectory(prefix="frame QA Zażółć's space ") as temporary:
            folder = Path(temporary)
            marker = folder / "child.pid"
            code = (
                "import subprocess,sys,time,pathlib; "
                "child=subprocess.Popen([sys.executable,'-c','import time; time.sleep(30)']); "
                "pathlib.Path(sys.argv[1]).write_text(str(child.pid)); "
                "print('parent-output',flush=True); print('parent-error',file=sys.stderr,flush=True); "
                + ("sys.exit(0)" if parent_exits else "time.sleep(30)")
            )
            started = time.monotonic()
            try:
                args = [sys.executable, "-c", code, str(marker)]
                if parent_exits:
                    result = native_qa.run_command(args, cwd=folder, env=dict(os.environ), timeout=2)
                    self.assertEqual(result.returncode, 0)
                    self.assertIn("parent-output", result.stdout)
                    self.assertIn("parent-error", result.stderr)
                else:
                    with self.assertRaises(subprocess.TimeoutExpired) as caught:
                        native_qa.run_command(args, cwd=folder, env=dict(os.environ), timeout=2)
                    output = caught.exception.stdout
                    output = output.decode() if isinstance(output, bytes) else output
                    self.assertIn("parent-output", output)
                self.assertLess(time.monotonic() - started, 15, "parent exit and timeout must be bounded")
                child = int(marker.read_text())
                deadline = time.monotonic() + 2
                while self.process_running(child) and time.monotonic() < deadline:
                    time.sleep(0.02)
                self.assertFalse(self.process_running(child), "owned descendant survived QA command completion")
            finally:
                if marker.exists():
                    child = int(marker.read_text())
                    if self.process_running(child):
                        import signal
                        os.kill(child, signal.SIGTERM if os.name == "nt" else signal.SIGKILL)

    def test_timeout_stops_the_owned_descendant_and_preserves_partial_output(self):
        self.assert_owned_descendant_stopped(parent_exits=False)

    def test_parent_exit_stops_descendants_without_waiting_for_inherited_output(self):
        self.assert_owned_descendant_stopped(parent_exits=True)

    def test_argument_environment_and_working_directory_round_trip_without_a_shell(self):
        with tempfile.TemporaryDirectory(prefix="frame QA Unicode Zażółć's ") as temporary:
            arguments = ["space word", 'double"quote', "Zażółć's", "backslash\\", "$(echo should-not-run)"]
            code = "import json,sys,os; print(json.dumps([sys.argv[1:],os.environ['FRAME_QA_TEST_VALUE'],os.getcwd()]))"
            result = native_qa.run_command([sys.executable, "-c", code, *arguments], cwd=temporary,
                                          env=dict(os.environ, FRAME_QA_TEST_VALUE="value's space"), timeout=10)
            self.assertEqual(result.returncode, 0, result.stderr)
            actual_arguments, environment, directory = json.loads(result.stdout)
            self.assertEqual(actual_arguments, arguments)
            self.assertEqual(environment, "value's space")
            self.assertEqual(Path(directory).resolve(), Path(temporary).resolve())

    def test_normal_exit_124_is_not_misreported_as_a_timeout(self):
        result = native_qa.run_command([sys.executable, "-c", "raise SystemExit(124)"], cwd=Path.cwd(),
                                      env=dict(os.environ), timeout=10)
        self.assertEqual(result.returncode, 124)


class RevisionTests(unittest.TestCase):
    def git_result(self, args, **_kwargs):
        return subprocess.CompletedProcess(args, 0, "a" * 40 if args[1] == "rev-parse" else "", "")

    def test_explicit_pr_head_checkout_records_the_distinct_merge_sha(self):
        with patch.dict(os.environ, {"FRAME_QA_PR_HEAD": "a" * 40, "GITHUB_SHA": "c" * 40, "GITHUB_EVENT_PATH": "", "PR_HEAD_SHA": ""}), patch.object(native_qa.subprocess, "run", side_effect=self.git_result):
            try:
                metadata = native_qa.repository_metadata(Path.cwd())
            except ValueError as error:
                self.fail(f"Explicit PR head checkout must be accepted: {error}")
        self.assertEqual(metadata["git_sha"], "a" * 40)
        self.assertEqual(metadata["pr_head_sha"], "a" * 40)
        self.assertEqual(metadata["github_sha"], "c" * 40)

    def test_explicit_pr_head_must_match_the_checked_out_revision(self):
        with patch.dict(os.environ, {"FRAME_QA_PR_HEAD": "b" * 40, "GITHUB_SHA": "a" * 40, "GITHUB_EVENT_PATH": "", "PR_HEAD_SHA": ""}), patch.object(native_qa.subprocess, "run", side_effect=self.git_result):
            with self.assertRaisesRegex(ValueError, "checkout|revision"):
                native_qa.repository_metadata(Path.cwd())


class FakeRunner:
    def __init__(self, *, corrupt_source=False, omit_completed=False, export_returncode=0):
        self.calls = []
        self.corrupt_source = corrupt_source
        self.omit_completed = omit_completed
        self.export_returncode = export_returncode

    def __call__(self, args, *, cwd, env, timeout):
        self.calls.append((list(args), Path(cwd), dict(env)))
        if "-version" in args:
            stdout = "bundled runtime test version\n"
        elif "--plan" in args or "--export" in args:
            request = json.loads(Path(args[-1]).read_text())
            directory = Path(request["outputDirectory"])
            if "--plan" in args:
                stdout = json.dumps({
                    "normalizedConfig": request["config"],
                    "outputPath": str(directory / request["outputName"]),
                    "args": ["-n", "-i", request["input"], "-map", "0:0"],
                })
            else:
                result = directory / "result (2).wav"
                shutil.copyfile(request["input"], result)
                if self.corrupt_source:
                    Path(request["input"]).write_bytes(b"source was changed")
                stdout = "" if self.omit_completed else json.dumps({
                    "event": "conversion-completed",
                    "payload": {"id": "qa-export", "output_path": str(result)},
                }) + "\n"
        else:
            stdout = json.dumps({"streams": [{"codec_type": "audio", "codec_name": "pcm_s16le", "sample_rate": "48000", "channels": 1}]})
        code = self.export_returncode if "--export" in args else 0
        return subprocess.CompletedProcess(args, code, stdout, "simulated export diagnostic" if code else "")


class VerificationTests(PackageFixture):
    def setUp(self):
        super().setUp()
        self.target = native_qa.identify_target()
        self.create_inputs()
        self.archive = self.build()

    def test_verify_runs_unpacked_qa_outside_checkout_with_explicit_packed_runtime(self):
        runner = FakeRunner()
        report = native_qa.verify_archive(self.archive, self.repo, runner=runner)
        self.assertTrue(report["verified"])
        self.assertTrue(report["checks"]["sourceUnchanged"])
        self.assertTrue(report["checks"]["collisionPreserved"])
        self.assertTrue(report["checks"]["audioSamplesMatch"])
        self.assertTrue(runner.calls)
        for args, cwd, env in runner.calls:
            self.assertFalse(cwd.is_relative_to(self.repo))
            self.assertTrue(Path(args[0]).is_relative_to(cwd))
            for key, tool in [("FRAME_FFMPEG_PATH", "ffmpeg"), ("FRAME_FFPROBE_PATH", "ffprobe")]:
                self.assertTrue(Path(env[key]).is_relative_to(cwd))
                self.assertIn(tool, Path(env[key]).name)
        self.assertFalse(runner.calls[0][1].exists(), "temporary unpacked files must be cleaned")

    def test_payload_tampering_is_rejected_before_any_execution(self):
        replacement = self.archive.with_suffix(".tampered.zip")
        with zipfile.ZipFile(self.archive) as original, zipfile.ZipFile(replacement, "w") as changed:
            for info in original.infolist():
                data = original.read(info.filename)
                changed.writestr(info, data + b"changed" if info.filename == self.target.app_name else data)
        runner = FakeRunner()
        with self.assertRaisesRegex(ValueError, "hash"):
            native_qa.verify_archive(replacement, self.repo, runner=runner)
        self.assertEqual(runner.calls, [])

    def test_extra_archive_members_are_rejected_before_extraction(self):
        with zipfile.ZipFile(self.archive, "a") as package:
            package.writestr("../escaped.txt", "must not escape")
        runner = FakeRunner()
        with self.assertRaisesRegex(ValueError, "Unexpected|unsafe"):
            native_qa.verify_archive(self.archive, self.repo, runner=runner)
        self.assertEqual(runner.calls, [])
        self.assertFalse((self.repo.parent / "escaped.txt").exists())

    def test_export_exit_zero_without_completed_event_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "Completed"):
            native_qa.verify_archive(self.archive, self.repo, runner=FakeRunner(omit_completed=True))

    def test_source_mutation_is_rejected_even_if_completed_was_reported(self):
        with self.assertRaisesRegex(ValueError, "source"):
            native_qa.verify_archive(self.archive, self.repo, runner=FakeRunner(corrupt_source=True))

    def test_failed_cli_verification_keeps_raw_command_evidence_after_cleanup(self):
        runner = FakeRunner(export_returncode=7)
        with patch.object(native_qa, "run_command", runner), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            status = native_qa.main(["verify", "--output", str(self.output)])
        self.assertEqual(status, 1)
        path = self.output / "evidence.json"
        self.assertTrue(path.is_file(), "failed verification must retain raw command evidence")
        evidence = json.loads(path.read_text())
        self.assertFalse(evidence["verified"])
        failed = evidence["commands"][-1]
        self.assertIn("--export", failed["argv"])
        self.assertEqual(failed["returnCode"], 7)
        self.assertIn("conversion-completed", failed["stdout"])
        self.assertEqual(failed["stderr"], "simulated export diagnostic")
        self.assertFalse(Path(failed["cwd"]).exists())


if __name__ == "__main__":
    unittest.main()
