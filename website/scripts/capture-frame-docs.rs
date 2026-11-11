//! Reproducible native Frame documentation capture helper.
//!
//! First, from the website checkout, prepare the same real sample media:
//! python3 scripts/prepare-docs-media.py /tmp/frame-docs-media
//! The recipe copies the attributed photograph, generates MP4 and WAV files with
//! FFmpeg, and writes subtitle cues and a deliberately invalid troubleshooting sample.
//!
//! This is an executable example for the sibling Frame application, not website code.
//! Copy it to frame-app/examples/render-docs-capture.rs in a clean Frame checkout, then:
//! cargo build -p frame-app --example render-docs-capture --features gpui_platform/test-support --locked
//!
//! Arguments: <scene-id> <output.png> <real-source-path> [more-real-source-paths...]
//! Set FRAME_VISUAL_THEME to light/dark and FRAME_FFMPEG_PATH / FRAME_FFPROBE_PATH
//! to the native app's bundled binaries. Run on a 2x display for 2400 x 1600 output.
//! Example, from the Frame checkout with those environment variables set:
//! mkdir -p /tmp/frame-docs-captures
//! target/debug/examples/render-docs-capture video-output /tmp/frame-docs-captures/video-output-dark.png /tmp/frame-docs-media/alpine-lake.mp4
//! Use studio-reference.wav for audio scenes, alpine-lake.jpg for images-output
//! and crop-preview, and unreadable-sample.mp4 only for source-error. For
//! batch-queue pass the MP4, JPEG, and WAV paths in that order. All other scenes
//! use alpine-lake.mp4. Repeat with FRAME_VISUAL_THEME=light and a new output name.
//! Remove the temporary example after capture. PNGs are encoded with
//! cwebp -lossless -exact -m 6 and compared pixel-for-pixel before publication.
//!
//! FrameRoot::new creates an isolated in-memory application with no persistence and
//! disabled notifications. Media enters through the application's real file-drop
//! handler; inspector changes use its real accessibility bounds and mouse events.
//! No injected app state, private state mutation, screen recording, compositing, or
//! conversion are used. The source-error scene deliberately imports an invalid file.
use frame_app::{
    app::{FrameRoot, frame_window_options, init_app},
    assets::{self, FrameAssets},
};
use gpui::{
    AppContext, Bounds, ExternalPaths, FileDropEvent, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PlatformInput, point, px, size,
};
use std::{path::PathBuf, time::Duration};

fn main() {
    let mut args = std::env::args().skip(1);
    let scene = args.next().expect("pass scene ID");
    let output = args.next().expect("pass screenshot destination");
    let sources: Vec<PathBuf> = args.map(PathBuf::from).collect();
    assert!(!sources.is_empty(), "pass real media paths");
    gpui_platform::application()
        .with_assets(FrameAssets)
        .run(move |cx| {
            assets::load_frame_fonts(cx).expect("load Frame fonts");
            init_app(cx, "Frame native website capture");
            cx.set_reduce_motion(true);
            let bounds = Bounds::centered(None, size(px(1200.0), px(800.0)), cx);
            let mut options = frame_window_options(bounds);
            options.show = false;
            options.focus = false;
            let handle = cx
                .open_window(options, |_, cx| {
                    cx.new(|cx| {
                        let mut root = FrameRoot::new();
                        root.load_runtime_capabilities(cx);
                        root
                    })
                })
                .expect("open hidden Frame render surface");
            cx.defer(move |cx| {
                cx.update_window(handle.into(), |_, window, cx| {
                    for _ in 0..3 {
                        window.refresh();
                        window.draw(cx).clear(cx);
                    }
                    let position = point(px(400.0), px(350.0));
                    window.dispatch_event(
                        PlatformInput::FileDrop(FileDropEvent::Entered {
                            position,
                            paths: ExternalPaths(sources.into_iter().collect()),
                        }),
                        cx,
                    );
                    window.draw(cx).clear(cx);
                    window.dispatch_event(
                        PlatformInput::FileDrop(FileDropEvent::Pending { position }),
                        cx,
                    );
                    window.draw(cx).clear(cx);
                    window.dispatch_event(
                        PlatformInput::FileDrop(FileDropEvent::Submit { position }),
                        cx,
                    );
                    window.dispatch_event(
                        PlatformInput::MouseMove(MouseMoveEvent {
                            position: point(px(-1000.0), px(-1000.0)),
                            ..Default::default()
                        }),
                        cx,
                    );
                    window.draw(cx).clear(cx);
                })
                .expect("import source through the native application");
            });
            cx.spawn(async move |cx| {
                for _ in 0..12 {
                    cx.background_executor()
                        .timer(Duration::from_millis(500))
                        .await;
                    cx.update(|cx| {
                        cx.update_window(handle.into(), |_, window, cx| {
                            window.refresh();
                            window.draw(cx).clear(cx);
                        })
                        .expect("settle native media preview");
                    });
                }
                cx.update(|cx| {
                    cx.update_window(handle.into(), |_, window, cx| {
                        window.set_a11y_forced(true);
                        window.draw(cx).clear(cx);
                        let labels: &[&str] = match scene.as_str() {
                            "workspace" | "batch-queue" => &["Output", "General"],
                            "presets" => &["Presets"],
                            "video-output" => &["Output", "Video"],
                            "video-filters" => &["Output", "Video filters"],
                            "audio-output" => &["Output", "Audio"],
                            "audio-filters" => &["Output", "Audio filters"],
                            "images-output" => &["Output", "General", "Images"],
                            "crop-preview" => &["Output", "Crop"],
                            "subtitles" => &["Output", "Subtitles"],
                            "source-metadata" | "source-error" => &[],
                            "output-metadata" => &["Output", "Metadata"],
                            "settings" => &["Settings"],
                            "trim-preview" => &["Output", "General", "Trim preview"],
                            _ => panic!("unknown screenshot scene: {scene}"),
                        };
                        for label in labels {
                            if *label == "Trim preview" {
                                window.dispatch_event(
                                    PlatformInput::MouseMove(MouseMoveEvent {
                                        position: point(px(400.0), px(350.0)),
                                        ..Default::default()
                                    }),
                                    cx,
                                );
                                for _ in 0..3 {
                                    window.refresh();
                                    window.draw(cx).clear(cx);
                                }
                            }
                            click_label(window, cx, label);
                            for _ in 0..3 {
                                window.refresh();
                                window.draw(cx).clear(cx);
                            }
                        }
                        window.dispatch_event(
                            PlatformInput::MouseMove(MouseMoveEvent {
                                position: point(px(-1000.0), px(-1000.0)),
                                ..Default::default()
                            }),
                            cx,
                        );
                    })
                    .expect("show output controls");
                });
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                cx.update(|cx| {
                    cx.update_window(handle.into(), |_, window, cx| {
                        for _ in 0..3 {
                            window.refresh();
                            window.draw(cx).clear(cx);
                        }
                        if let Some(tree) = window.a11y_tree() {
                            let labels: Vec<_> = tree
                                .nodes
                                .iter()
                                .filter_map(|(id, node)| {
                                    node.label().map(|label| {
                                        format!(
                                            "{:?}\t{}\t{:?}",
                                            node.role(),
                                            label,
                                            window.a11y_node_bounds(*id)
                                        )
                                    })
                                })
                                .collect();
                            std::fs::write(format!("{output}.a11y.txt"), labels.join("\n"))
                                .expect("save native accessibility evidence");
                        }
                        window
                            .render_to_image()
                            .expect("native GPU readback")
                            .save(&output)
                            .expect("save native screenshot");
                    })
                    .expect("capture Frame window");
                    cx.quit();
                });
            })
            .detach();
        });
}

fn click_label(window: &mut gpui::Window, cx: &mut gpui::App, label: &str) {
    let node = window.a11y_tree().and_then(|tree| {
        tree.nodes
            .iter()
            .filter(|(_, node)| node.label() == Some(label) && node.role() != gpui::Role::TabPanel)
            .next_back()
            .map(|(id, _)| *id)
    });
    let Some(position) = node
        .and_then(|node| window.a11y_node_bounds(node))
        .map(|bounds| bounds.center())
    else {
        panic!("No native control labeled {label}");
    };
    window.dispatch_event(
        PlatformInput::MouseDown(MouseDownEvent {
            button: MouseButton::Left,
            position,
            click_count: 1,
            ..Default::default()
        }),
        cx,
    );
    window.dispatch_event(
        PlatformInput::MouseUp(MouseUpEvent {
            button: MouseButton::Left,
            position,
            click_count: 1,
            ..Default::default()
        }),
        cx,
    );
}
