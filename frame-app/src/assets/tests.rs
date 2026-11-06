use super::*;

mod frame_assets {
    use super::*;

    #[test]
    fn asset_registry_resolves_all_bundled_icons_and_fonts() {
        let registry: gpui::AssetRegistry = FrameAssets.into();
        for directory in ["icons", "fonts"] {
            for name in FrameAssets.list(directory).unwrap() {
                let path = format!("{directory}/{name}");
                let expected = FrameAssets.load(&path).unwrap().unwrap();
                assert_eq!(
                    registry.load(&path).as_deref(),
                    Some(expected.as_ref()),
                    "{path}"
                );
            }
        }
    }

    #[test]
    fn load_returns_frame_icon_svg() {
        let loaded = FrameAssets
            .load(ICON_FRAME)
            .expect("asset load should not fail");

        assert!(
            loaded
                .as_deref()
                .is_some_and(|bytes| bytes.starts_with(b"<svg"))
        );
    }

    #[test]
    fn load_returns_none_for_unknown_asset() {
        let loaded = FrameAssets
            .load("icons/missing.svg")
            .expect("asset load should not fail");

        assert!(loaded.is_none());
    }

    #[test]
    fn list_returns_original_frame_icon_assets() {
        let listed = FrameAssets
            .list("icons")
            .expect("asset list should not fail");

        for icon_name in [
            "arrow-down.svg",
            "bookmark.svg",
            "captions.svg",
            "check.svg",
            "close.svg",
            "copy.svg",
            "crop.svg",
            "download-02.svg",
            "file.svg",
            "file-down.svg",
            "file-image.svg",
            "file-import.svg",
            "file-up.svg",
            "file-video.svg",
            "flip-horizontal.svg",
            "flip-vertical.svg",
            "folder-import.svg",
            "hard-drive.svg",
            "audio-filters.svg",
            "layout-list.svg",
            "list-checks.svg",
            "minus.svg",
            "music.svg",
            "pause.svg",
            "pause2.svg",
            "play.svg",
            "plus.svg",
            "refresh.svg",
            "rotate-cw.svg",
            "settings.svg",
            "spinner.svg",
            "square.svg",
            "tags.svg",
            "terminal.svg",
            "trash.svg",
            "unfold-more.svg",
            "video-filters.svg",
        ] {
            assert!(
                listed.iter().any(|name| name.as_ref() == icon_name),
                "{icon_name} should be listed"
            );
            let path = format!("icons/{icon_name}");
            assert!(
                FrameAssets
                    .load(&path)
                    .expect("asset load should not fail")
                    .is_some(),
                "{path} should load"
            );
        }
    }

    #[test]
    fn ui_icon_assets_render_with_nucleo_outline_style() {
        let renderer = gpui::SvgRenderer::new(std::sync::Arc::new(FrameAssets.into()));

        for icon in [
            ICON_ARROW_DOWN,
            ICON_LAYOUT_LIST,
            ICON_LIST_CHECKS,
            ICON_TERMINAL,
            ICON_CHECK,
            ICON_COPY,
            ICON_UNFOLD_MORE,
            ICON_CLOSE,
            ICON_DOWNLOAD_02,
            ICON_FILE,
            ICON_FILE_UP,
            ICON_FILE_DOWN,
            ICON_FILE_IMPORT,
            ICON_FOLDER_IMPORT,
            ICON_HARD_DRIVE,
            ICON_FILE_VIDEO,
            ICON_FILE_IMAGE,
            ICON_MUSIC,
            ICON_VIDEO_FILTERS,
            ICON_AUDIO_FILTERS,
            ICON_CAPTIONS,
            ICON_TAGS,
            ICON_BOOKMARK,
            ICON_SETTINGS,
            ICON_PLUS,
            ICON_MINUS,
            ICON_PLAY,
            ICON_PAUSE,
            ICON_PAUSE_2,
            ICON_ROTATE_CW,
            ICON_REFRESH,
            ICON_FLIP_HORIZONTAL,
            ICON_FLIP_VERTICAL,
            ICON_CROP,
            ICON_SPINNER,
            ICON_SQUARE,
            ICON_TRASH,
        ] {
            let loaded = FrameAssets
                .load(icon)
                .expect("asset load should not fail")
                .expect("icon asset should exist");
            let svg = std::str::from_utf8(loaded.as_ref()).expect("svg should be utf8");

            assert!(
                svg.contains(r#"viewBox="0 0 18 18""#),
                "{icon} should use the Nucleo UI 18px grid"
            );
            assert!(
                svg.contains(r#"fill="none""#),
                "{icon} should use stroke rendering"
            );
            assert!(
                svg.contains(r#"stroke="currentColor""#),
                "{icon} should inherit GPUI text_color"
            );
            assert!(
                svg.contains(r#"stroke-width="1.5""#),
                "{icon} should use the Mira stroke weight"
            );
            assert!(
                !svg.contains(r##"="#000""##) && !svg.contains(r#"="black""#),
                "{icon} should not retain fixed colors from the source SVG"
            );
            for size in [12.0, 16.0, 18.0, 20.0, 24.0] {
                let image = renderer
                    .render_single_frame(loaded.as_ref(), size / 18.0)
                    .unwrap_or_else(|error| panic!("{icon} should render at {size}px: {error}"));
                assert!(
                    image
                        .as_bytes(0)
                        .expect("rendered icon should have a frame")
                        .chunks_exact(4)
                        .any(|pixel| pixel[3] > 0),
                    "{icon} should have visible pixels at {size}px"
                );
            }
        }
    }

    #[test]
    fn frame_font_features_enable_requested_opentype_tags() {
        let features = frame_font_features();

        assert_eq!(features.tag_value_list(), [("kern".to_string(), 1)]);
    }

    #[test]
    fn frame_tabular_number_font_features_enable_requested_opentype_tags() {
        let features = frame_tabular_number_font_features();

        assert_eq!(
            features.tag_value_list(),
            [("kern".to_string(), 1), ("tnum".to_string(), 1)]
        );
    }

    #[test]
    fn list_returns_bundled_font_faces() {
        let listed = FrameAssets
            .list("fonts")
            .expect("asset list should not fail");

        assert_eq!(
            listed,
            [
                SharedString::from("Inter-Regular.ttf"),
                SharedString::from("Inter-Medium.ttf"),
            ]
        );
    }

    #[test]
    fn load_returns_each_bundled_font_face() {
        for path in [FRAME_FONT_REGULAR_PATH, FRAME_FONT_MEDIUM_PATH] {
            assert!(
                FrameAssets
                    .load(path)
                    .expect("asset load should not fail")
                    .is_some(),
                "{path} should load"
            );
        }
    }
}
