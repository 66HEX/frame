use super::*;

mod frame_assets {
    use super::*;

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
            "file-down.svg",
            "file-image.svg",
            "file-import.svg",
            "file-up.svg",
            "file-video.svg",
            "flip-horizontal.svg",
            "flip-vertical.svg",
            "folder-import.svg",
            "hard-drive.svg",
            "layout-list.svg",
            "list-checks.svg",
            "minus.svg",
            "music.svg",
            "pause.svg",
            "pause2.svg",
            "play.svg",
            "plus.svg",
            "rotate-cw.svg",
            "settings.svg",
            "spinner.svg",
            "square.svg",
            "tags.svg",
            "terminal.svg",
            "trash.svg",
            "unfold-more.svg",
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
    fn ui_icon_assets_use_hugeicons_stroke_style() {
        let renderer = gpui::SvgRenderer::new(std::sync::Arc::new(FrameAssets));

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
            ICON_FILE_UP,
            ICON_FILE_DOWN,
            ICON_FILE_IMPORT,
            ICON_FOLDER_IMPORT,
            ICON_HARD_DRIVE,
            ICON_FILE_VIDEO,
            ICON_FILE_IMAGE,
            ICON_MUSIC,
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
                svg.contains(r#"viewBox="0 0 24 24""#),
                "{icon} should use the Hugeicons 24px grid"
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
                "{icon} should use the Hugeicons stroke weight"
            );
            assert!(
                !svg.contains(r#" key=""#),
                "{icon} should not include React-only metadata"
            );
            renderer
                .render_single_frame(loaded.as_ref(), 1.0)
                .unwrap_or_else(|error| panic!("{icon} should render: {error}"));
        }
    }

    #[test]
    fn traffic_light_symbols_preserve_original_hover_glyphs() {
        let loaded = FrameAssets
            .load(ICON_TRAFFIC_CLOSE_SYMBOL)
            .expect("asset load should not fail")
            .expect("traffic light asset should exist");
        let svg = std::str::from_utf8(loaded.as_ref()).expect("svg should be utf8");

        assert!(svg.contains(r#"viewBox="-10 -10 20 20""#));
        assert!(svg.contains("M-1.8 -1.8 L1.8 1.8 M1.8 -1.8 L-1.8 1.8"));
        assert!(svg.contains(r#"stroke-width="1.5""#));
    }

    #[test]
    fn frame_font_family_matches_bundled_font_name_table_family() {
        assert_eq!(FRAME_FONT_FAMILY, "Overused Grotesk");
    }

    #[test]
    fn frame_font_alias_matches_bundled_font_alias() {
        assert_eq!(FRAME_FONT_ALIAS, "OverusedGrotesk");
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
                SharedString::from("OverusedGrotesk-Roman.ttf"),
                SharedString::from("OverusedGrotesk-Medium.ttf"),
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

    #[test]
    fn frame_font_bytes_registers_regular_and_medium_faces() {
        assert_eq!(frame_font_bytes().len(), 2);
    }
}
