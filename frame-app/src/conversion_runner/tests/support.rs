use super::*;

pub(super) struct ConversionRunnerSandbox {
    pub(super) root: PathBuf,
    keep: bool,
}

impl ConversionRunnerSandbox {
    pub(super) fn new(name: &str) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "frame-runner-test-{}-{}-{now}",
            std::process::id(),
            name
        ));
        fs::create_dir_all(&root).expect("runner temp directory should be created");
        Self {
            root,
            keep: std::env::var_os("FRAME_KEEP_MEDIA_TESTS").is_some(),
        }
    }

    pub(super) fn path(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }
}

impl Drop for ConversionRunnerSandbox {
    fn drop(&mut self) {
        if self.keep {
            eprintln!(
                "keeping conversion runner test artifacts in {}",
                self.root.display()
            );
            return;
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub(super) fn generate_runner_source(output: &Path) {
    let status = Command::new(crate::runtime_binaries::ffmpeg_executable())
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=64x48:rate=12:duration=0.5",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=48000:duration=0.5",
            "-shortest",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-crf",
            "28",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "96k",
            "-y",
        ])
        .arg(output)
        .status()
        .expect("ffmpeg should start to generate runner fixture");

    assert!(
        status.success(),
        "ffmpeg fixture generation failed with {status}"
    );
}

pub(super) fn generate_runner_image_source(output: &Path) {
    let status = Command::new(crate::runtime_binaries::ffmpeg_executable())
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=64x48:rate=1",
            "-frames:v",
            "1",
            "-update",
            "1",
            "-y",
        ])
        .arg(output)
        .status()
        .expect("ffmpeg should start to generate image fixture");

    assert!(
        status.success(),
        "ffmpeg image fixture generation failed with {status}"
    );
}
