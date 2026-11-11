#!/usr/bin/env python3
"""Generate the local media fixtures used by capture-frame-docs.rs."""

import argparse
from pathlib import Path
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "output", nargs="?", type=Path,
        default=Path(tempfile.gettempdir()) / "frame-docs-media",
        help="directory for the generated documentation fixtures",
    )
    output = parser.parse_args().output.resolve()
    ffmpeg = shutil.which("ffmpeg")
    if not ffmpeg:
        parser.error("ffmpeg must be installed and available on PATH")

    output.mkdir(parents=True, exist_ok=True)
    source = Path(__file__).resolve().parents[1] / "static/media/alpine-lake.jpg"
    shutil.copyfile(source, output / "alpine-lake.jpg")
    (output / "captions.srt").write_text(
        "1\n00:00:00,500 --> 00:00:03,500\nA quiet morning at the lake.\n\n"
        "2\n00:00:04,000 --> 00:00:07,500\nKeep the details that matter.\n",
        encoding="utf-8",
    )
    base = [ffmpeg, "-hide_banner", "-loglevel", "error", "-y"]
    subprocess.run(base + [
        "-loop", "1", "-framerate", "24", "-i", str(output / "alpine-lake.jpg"),
        "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:duration=8",
        "-i", str(output / "captions.srt"),
        "-map", "0:v", "-map", "1:a", "-map", "2:s",
        "-vf", "scale=1280:-2,crop=1280:720,setsar=1",
        "-af", "volume=0.08,afade=t=in:d=0.3,afade=t=out:st=7.5:d=0.5",
        "-t", "8", "-c:v", "libx264", "-preset", "fast", "-crf", "18",
        "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "192k", "-ac", "2",
        "-c:s", "mov_text", "-metadata", "title=Alpine lake",
        "-metadata", "comment=Frame documentation sample",
        "-metadata:s:a:0", "title=Reference tone", "-metadata:s:s:0", "language=eng",
        "-movflags", "+faststart", str(output / "alpine-lake.mp4"),
    ], check=True)
    subprocess.run(base + [
        "-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000:duration=12",
        "-af", "volume=0.08,afade=t=in:d=0.3,afade=t=out:st=11.5:d=0.5",
        "-ac", "2", "-c:a", "pcm_s24le", "-metadata", "title=Studio reference",
        "-metadata", "artist=Frame", str(output / "studio-reference.wav"),
    ], check=True)
    (output / "unreadable-sample.mp4").write_bytes(
        b"Frame documentation fixture: this is deliberately not valid video data.\n"
    )
    print(f"Prepared documentation media in {output}")


if __name__ == "__main__":
    main()
