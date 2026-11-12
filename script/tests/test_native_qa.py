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

    def test_archive_limit_cannot_exceed_the_ci_storage_policy(self):
        self.create_inputs()
        with self.assertRaisesRegex(ValueError, "100"):
            self.build(archive_limit_mib=101)


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
