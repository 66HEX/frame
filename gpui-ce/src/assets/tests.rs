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
            "chevrons-up-down.svg",
            "close.svg",
            "crop.svg",
            "file-down.svg",
            "file-image.svg",
            "file-up.svg",
            "file-video.svg",
            "flip-horizontal.svg",
            "flip-vertical.svg",
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
    fn preview_transform_assets_preserve_original_filled_paths() {
        let loaded = FrameAssets
            .load(ICON_ROTATE_CW)
            .expect("asset load should not fail")
            .expect("rotate asset should exist");
        let svg = std::str::from_utf8(loaded.as_ref()).expect("svg should be utf8");

        assert!(svg.contains(r#"fill="currentColor""#));
        assert!(svg.contains("M240,56v48"));
        assert!(!svg.contains(r#"stroke-width="18""#));
    }

    #[test]
    fn traffic_light_symbols_preserve_original_hover_glyphs() {
        let loaded = FrameAssets
            .load(ICON_TRAFFIC_CLOSE_SYMBOL)
            .expect("asset load should not fail")
            .expect("traffic light asset should exist");
        let svg = std::str::from_utf8(loaded.as_ref()).expect("svg should be utf8");

        assert!(svg.contains(r#"viewBox="-10 -10 20 20""#));
        assert!(svg.contains(r#"M-1.8 -1.8 L1.8 1.8 M1.8 -1.8 L-1.8 1.8"#));
        assert!(svg.contains(r#"stroke-width="1.5""#));
    }

    #[test]
    fn frame_font_family_matches_bundled_font_name_table_family() {
        assert_eq!(FRAME_FONT_FAMILY, "Instrument Sans");
    }

    #[test]
    fn frame_font_weight_matches_bundled_font_face_weight() {
        assert_eq!(FRAME_FONT_WEIGHT, gpui::FontWeight::NORMAL);
    }

    #[test]
    fn frame_font_alias_matches_bundled_font_alias() {
        assert_eq!(FRAME_FONT_ALIAS, "InstrumentSans");
    }

    #[test]
    fn frame_font_features_enable_requested_opentype_tags() {
        let features = frame_font_features();

        assert_eq!(
            features.tag_value_list(),
            [
                ("liga".to_string(), 1),
                ("ss02".to_string(), 1),
                ("ss05".to_string(), 1),
                ("kern".to_string(), 1),
            ]
        );
    }

    #[test]
    fn list_returns_bundled_font_asset() {
        let listed = FrameAssets
            .list("fonts")
            .expect("asset list should not fail");

        assert!(
            listed
                .iter()
                .any(|name| name.as_ref() == "InstrumentSans-Variable.ttf")
        );
    }
}
