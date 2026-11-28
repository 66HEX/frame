import importlib.util
import json
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest
import wave

SCRIPT = Path(__file__).resolve().parents[1] / "media-worker.py"
spec = importlib.util.spec_from_file_location("media_worker", SCRIPT)
worker = importlib.util.module_from_spec(spec)
spec.loader.exec_module(worker)


class MediaWorkerPackages(unittest.TestCase):
    def test_one_abi_library_for_each_component_on_every_release_target(self):
        targets = set(worker.ARCHIVES) | set(worker.MAC_TRIPLETS)
        self.assertEqual(len(targets), 5)
        for target in targets:
            with self.subTest(target=target), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                windows = target.endswith("msvc")
                lib = root / ("bin" if windows else "lib")
                lib.mkdir()
                wanted = []
                for component in worker.LIBRARIES:
                    if windows:
                        name = f"{component}-63.dll"
                    elif target in worker.MAC_TRIPLETS:
                        name = f"lib{component}.63.dylib"
                    else:
                        name = f"lib{component}.so.63"
                    wanted.append(lib / name)
                    wanted[-1].touch()
                    (lib / f"lib{component}.so.63.1.0").touch()
                    (lib / f"lib{component}.dylib").touch()
                self.assertCountEqual(worker.runtime_libraries(root, target), wanted)
                wanted[0].unlink()
                with self.assertRaisesRegex(ValueError, "avcodec"):
                    list(worker.runtime_libraries(root, target))

    def test_release_package_rejects_cli_after_upgrade_or_repack(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            resources = root / "Contents/Resources/binaries"
            resources.mkdir(parents=True)
            for binary in ["ffmpeg", "ffprobe.exe", "ffmpeg-aarch64-apple-darwin"]:
                path = resources / binary
                path.touch()
                with self.assertRaisesRegex(ValueError, "legacy CLI"):
                    worker.verify_no_cli(root)
                path.unlink()
            (resources / "frame-media.LICENSE").touch()
            worker.verify_no_cli(root)

    def test_mixed_abi_input_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "lib").mkdir()
            for major in (62, 63):
                (root / f"lib/libavcodec.so.{major}").touch()
            with self.assertRaisesRegex(ValueError, "avcodec"):
                list(worker.runtime_libraries(root, "x86_64-unknown-linux-gnu"))

    @unittest.skipUnless(shutil.which("perl"), "requires release workflow Perl")
    def test_nix_upgrade_preserves_downstream_changes_and_adds_worker(self):
        workflow = (SCRIPT.parents[1] / ".github/workflows/publish_nixpkgs.yml").read_text()
        changes = [line.strip().split("'", 2)[1] for line in workflow.splitlines()
                   if "perl -0pi" in line and ("cargo(?:Build|Test)?Flags" in line or "buildInputs" in line)]
        self.assertEqual(len(changes), 3)
        for previous in ('cargoFlags = [ "--package" "frame-app" ];',
                         'cargoBuildFlags = [ "--package" "frame-app" ];\ncargoTestFlags = [ "--package" "frame-app" ];'):
            with self.subTest(previous=previous), tempfile.TemporaryDirectory() as temporary:
                package = Path(temporary) / "package.nix"
                package.write_text(previous + "\nbuildInputs = [\n  alsa-lib\n  vulkan-loader\n];")
                for pass_number in range(2):
                    for change in changes:
                        subprocess.run(["perl", "-0pi", "-e", change, package], check=True)
                    updated = package.read_text()
                    self.assertEqual(updated.count('"frame-media"'), previous.count('"frame-app"'))
                    self.assertEqual(updated.count('ffmpeg'), 1)
                    self.assertIn("(ffmpeg.override { withJxl = true; })", updated)
                    self.assertIn("vulkan-loader", updated)

    @unittest.skipUnless(os.environ.get("FFMPEG_DIR"), "requires release media SDK")
    def test_worker_invokes_packaged_libraries_after_relocation(self):
        host = next(line[6:] for line in subprocess.check_output(
            ["rustc", "-vV"], text=True).splitlines() if line.startswith("host: "))
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            original = root / "unsigned package"
            bin_dir = original if host.endswith("msvc") else original / "bin"
            lib_dir = (bin_dir if host.endswith("msvc") else
                       original / ("Frameworks" if host in worker.MAC_TRIPLETS else "lib"))
            worker.bundle(host, Path(os.environ["FFMPEG_DIR"]), bin_dir, lib_dir, "debug")
            relocated = root / "moved Frame ü"
            shutil.move(original, relocated)
            name = "frame-media.exe" if host.endswith("msvc") else "frame-media"
            worker.smoke(relocated / bin_dir.relative_to(original) / name, relocated / lib_dir.relative_to(original))
            if os.environ.get("FRAME_TEST_QA_RELOCATION") == "1":
                self.check_relocated_application(relocated / bin_dir.relative_to(original))

    def check_relocated_application(self, bin_dir):
        name = "frame-qa.exe" if os.name == "nt" else "frame-qa"
        qa = shutil.copy2(worker.TARGET_DIR / "debug" / name, bin_dir)
        worker.verify_no_cli(bin_dir.parent)
        with tempfile.TemporaryDirectory() as temporary:
            data = Path(temporary)
            source, request = data / "source ü.wav", data / "export.json"
            samples = bytes(range(200)) * 20
            with wave.open(str(source), "wb") as output:
                output.setparams((1, 2, 8000, 0, "NONE", ""))
                output.writeframes(samples)
            request.write_text(json.dumps({
                "input": str(source), "outputDirectory": str(data), "outputName": "result.wav",
                "config": {"container": "wav", "audioCodec": "copy", "processingMode": "copy",
                           "selectedAudioTracks": [0]},
            }), encoding="utf8")
            env = dict(os.environ)
            for key in ("LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH", "FFMPEG_DIR",
                        "FRAME_FFMPEG_PATH", "FRAME_FFPROBE_PATH"):
                env.pop(key, None)
            env["PATH"] = str(Path(os.environ["SystemRoot"]) / "System32") if os.name == "nt" else ""
            events = subprocess.check_output([qa, "--export", request], cwd=data, env=env,
                                             text=True, timeout=60)
            self.assertTrue(any(json.loads(line).get("event") == "conversion-completed"
                                for line in events.splitlines() if line.strip()))
            with wave.open(str(data / "result.wav")) as result:
                self.assertEqual(result.readframes(result.getnframes()), samples)



if __name__ == "__main__":
    unittest.main()
