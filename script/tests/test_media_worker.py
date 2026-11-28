import importlib.util
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest

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

    def test_mixed_abi_input_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "lib").mkdir()
            for major in (62, 63):
                (root / f"lib/libavcodec.so.{major}").touch()
            with self.assertRaisesRegex(ValueError, "avcodec"):
                list(worker.runtime_libraries(root, "x86_64-unknown-linux-gnu"))

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
            worker.smoke(relocated / bin_dir.relative_to(original) / name)


if __name__ == "__main__":
    unittest.main()
