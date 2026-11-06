# Inwentaryzacja UI Frame

Wygenerowano z kodu przez `python3 script/inventory-ui.py`. Zestawienie obejmuje konstruktory elementów i każde miejsce ich użycia poza testami. Listy plików, kodeków, presetów, fontów i ścieżek są dynamiczne; nie mają stałej liczby instancji.

Rodzaje pól tekstowych: 24; konstruktory UI: 287; miejsca użycia: 833; jawne identyfikatory lub ich wzorce: 41. Pełny indeks z numerami linii jest w [inventory.json](inventory.json).

## Pola tekstowe

| Rodzaj | Etykieta dostępności |
| --- | --- |
| `MaxConcurrency` | Maximum concurrent conversions |
| `OutputName` | Output name |
| `AudioBitrate` | Audio bitrate |
| `CropWidth` | Crop width |
| `CropHeight` | Crop height |
| `VideoCustomWidth` | Video width |
| `VideoCustomHeight` | Video height |
| `VideoBitrate` | Video bitrate |
| `GifLoop` | GIF loop count |
| `PreviewStartTime` | Preview start time |
| `PreviewEndTime` | Preview end time |
| `MetadataTitle` | Metadata title |
| `MetadataArtist` | Metadata artist |
| `MetadataAlbum` | Metadata album |
| `MetadataGenre` | Metadata genre |
| `MetadataDate` | Metadata date |
| `MetadataComment` | Metadata comment |
| `MetadataServiceName` | MPEG transport stream service name |
| `MetadataServiceProvider` | MPEG transport stream service provider |
| `PresetName` | Preset name |
| `ExternalSubtitleLanguage` | Selectable subtitle language |
| `ExternalSubtitleTitle` | Selectable subtitle title |
| `SubtitleFontColorHex` | Subtitle font color |
| `SubtitleOutlineColorHex` | Subtitle outline color |

## Parametry suwaków

- `SettingsVideoRangeTarget`: Crf, Quality.
- `AudioFilterRangeTarget`: Volume, Limiter, Bass, Treble, HighPass, LowPass, NoiseReduction, DeEsser, StereoWidth.
- `SettingsAudioRangeTarget`: Quality.
- `SettingsImageRangeTarget`: JpegQuality, WebpQuality, WebpCompression, PngCompression.
- `VideoFilterRangeTarget`: Brightness, Contrast, Saturation, Gamma, Hue, Temperature, Sharpen, GaussianBlur, Deband, Vignette.

Dodatkowo: pozycja odtwarzania, początek i koniec przycięcia, przezroczystość nakładki, hue i saturation/value w pickerze kolorów.

## Komponenty i miejsca użycia

| Konstruktor | Źródło | Liczba wywołań |
| --- | --- | --- |
| `apply_accessible_toggle_button` | [accessibility.rs](../../src/app/accessibility.rs#L445) | 2 |
| `apply_accessible_checkbox` | [accessibility.rs](../../src/app/accessibility.rs#L457) | 3 |
| `apply_accessible_checkbox_with_focus` | [accessibility.rs](../../src/app/accessibility.rs#L485) | 1 |
| `apply_accessible_select_trigger` | [accessibility.rs](../../src/app/accessibility.rs#L514) | 1 |
| `apply_accessible_select_trigger_with_focus` | [accessibility.rs](../../src/app/accessibility.rs#L540) | 1 |
| `apply_accessible_select_option` | [accessibility.rs](../../src/app/accessibility.rs#L567) | 1 |
| `apply_accessible_select_option_with_focus` | [accessibility.rs](../../src/app/accessibility.rs#L592) | 1 |
| `apply_accessible_slider` | [accessibility.rs](../../src/app/accessibility.rs#L620) | 4 |
| `apply_accessible_slider_with_focus` | [accessibility.rs](../../src/app/accessibility.rs#L652) | 1 |
| `apply_accessible_text_input` | [accessibility.rs](../../src/app/accessibility.rs#L681) | 1 |
| `titlebar_navigation` | [chrome/navigation.rs](../../src/app/chrome/navigation.rs#L14) | 2 |
| `titlebar_stats` | [chrome/navigation.rs](../../src/app/chrome/navigation.rs#L51) | 2 |
| `titlebar_stat` | [chrome/navigation.rs](../../src/app/chrome/navigation.rs#L72) | 2 |
| `titlebar_segment` | [chrome/navigation.rs](../../src/app/chrome/navigation.rs#L89) | 2 |
| `current` | [chrome/titlebar.rs](../../src/app/chrome/titlebar.rs#L28) | 2 |
| `titlebar_for_platform` | [chrome/titlebar.rs](../../src/app/chrome/titlebar.rs#L48) | 1 |
| `macos_titlebar` | [chrome/titlebar.rs](../../src/app/chrome/titlebar.rs#L62) | 1 |
| `windows_titlebar` | [chrome/titlebar.rs](../../src/app/chrome/titlebar.rs#L108) | 1 |
| `linux_titlebar` | [chrome/titlebar.rs](../../src/app/chrome/titlebar.rs#L124) | 1 |
| `titlebar_drag_surface` | [chrome/titlebar.rs](../../src/app/chrome/titlebar.rs#L140) | 3 |
| `platform_titlebar_content` | [chrome/titlebar.rs](../../src/app/chrome/titlebar.rs#L196) | 2 |
| `titlebar_shows_workspace_controls` | [chrome/titlebar.rs](../../src/app/chrome/titlebar.rs#L248) | 2 |
| `titlebar_add_source_button` | [chrome/titlebar.rs](../../src/app/chrome/titlebar.rs#L280) | 2 |
| `titlebar_start_button` | [chrome/titlebar.rs](../../src/app/chrome/titlebar.rs#L302) | 2 |
| `frame_logo` | [chrome/titlebar.rs](../../src/app/chrome/titlebar.rs#L340) | 1 |
| `platform_frame_logo` | [chrome/titlebar.rs](../../src/app/chrome/titlebar.rs#L356) | 1 |
| `titlebar_divider` | [chrome/titlebar.rs](../../src/app/chrome/titlebar.rs#L371) | 2 |
| `platform_titlebar_divider` | [chrome/titlebar.rs](../../src/app/chrome/titlebar.rs#L375) | 2 |
| `macos_native_window_controls_placeholder` | [chrome/window_controls.rs](../../src/app/chrome/window_controls.rs#L18) | 1 |
| `windows_window_controls` | [chrome/window_controls.rs](../../src/app/chrome/window_controls.rs#L27) | 1 |
| `linux_window_controls` | [chrome/window_controls.rs](../../src/app/chrome/window_controls.rs#L108) | 1 |
| `titlebar_window_button` | [chrome/window_controls.rs](../../src/app/chrome/window_controls.rs#L203) | 6 |
| `frame_toggle_button_base` | [components/button.rs](../../src/app/components/button.rs#L47) | 0 |
| `frame_text_button` | [components/button.rs](../../src/app/components/button.rs#L73) | 9 |
| `frame_text_button_with_focus` | [components/button.rs](../../src/app/components/button.rs#L92) | 5 |
| `frame_text_button_inner` | [components/button.rs](../../src/app/components/button.rs#L120) | 4 |
| `frame_button_base` | [components/button.rs](../../src/app/components/button.rs#L155) | 7 |
| `transitions` | [components/button.rs](../../src/app/components/button.rs#L185) | 15 |
| `frame_icon_button` | [components/button.rs](../../src/app/components/button.rs#L315) | 8 |
| `frame_icon_swap_button` | [components/button.rs](../../src/app/components/button.rs#L345) | 1 |
| `frame_icon_button_inner` | [components/button.rs](../../src/app/components/button.rs#L381) | 2 |
| `frame_icon_button_content` | [components/button.rs](../../src/app/components/button.rs#L414) | 1 |
| `frame_checkbox_indicator` | [components/checkbox.rs](../../src/app/components/checkbox.rs#L16) | 3 |
| `frame_checkbox_row` | [components/checkbox.rs](../../src/app/components/checkbox.rs#L71) | 7 |
| `frame_checkbox_row_with_focus` | [components/checkbox.rs](../../src/app/components/checkbox.rs#L90) | 1 |
| `frame_checkbox_row_inner` | [components/checkbox.rs](../../src/app/components/checkbox.rs#L118) | 2 |
| `frame_collapsible` | [components/collapsible.rs](../../src/app/components/collapsible.rs#L17) | 1 |
| `frame_color_picker_panel` | [components/color_picker.rs](../../src/app/components/color_picker.rs#L17) | 1 |
| `frame_color_picker_sv_canvas` | [components/color_picker.rs](../../src/app/components/color_picker.rs#L45) | 1 |
| `frame_color_picker_hue_track` | [components/color_picker.rs](../../src/app/components/color_picker.rs#L99) | 1 |
| `frame_color_picker_hue_handle` | [components/color_picker.rs](../../src/app/components/color_picker.rs#L132) | 1 |
| `frame_color_picker_hue_color` | [components/color_picker.rs](../../src/app/components/color_picker.rs#L149) | 1 |
| `frame_handle_hit_region` | [components/hit_target.rs](../../src/app/components/hit_target.rs#L10) | 2 |
| `frame_list_group` | [components/list_item.rs](../../src/app/components/list_item.rs#L10) | 3 |
| `frame_list_item` | [components/list_item.rs](../../src/app/components/list_item.rs#L18) | 1 |
| `interactive_item` | [components/list_item.rs](../../src/app/components/list_item.rs#L60) | 1 |
| `frame_option_tabs` | [components/option_tabs.rs](../../src/app/components/option_tabs.rs#L10) | 2 |
| `frame_tab_trigger` | [components/option_tabs.rs](../../src/app/components/option_tabs.rs#L107) | 2 |
| `input_surface` | [components/patterns.rs](../../src/app/components/patterns.rs#L9) | 1 |
| `frame_field_group` | [components/patterns.rs](../../src/app/components/patterns.rs#L80) | 16 |
| `frame_label` | [components/patterns.rs](../../src/app/components/patterns.rs#L88) | 6 |
| `frame_description` | [components/patterns.rs](../../src/app/components/patterns.rs#L97) | 2 |
| `frame_section` | [components/patterns.rs](../../src/app/components/patterns.rs#L108) | 1 |
| `frame_badge` | [components/patterns.rs](../../src/app/components/patterns.rs#L115) | 1 |
| `frame_panel_header` | [components/patterns.rs](../../src/app/components/patterns.rs#L135) | 2 |
| `frame_toolbar` | [components/patterns.rs](../../src/app/components/patterns.rs#L146) | 2 |
| `frame_empty` | [components/patterns.rs](../../src/app/components/patterns.rs#L155) | 1 |
| `frame_tabs_list` | [components/patterns.rs](../../src/app/components/patterns.rs#L173) | 4 |
| `frame_vertical_scrollbar` | [components/scrollbar.rs](../../src/app/components/scrollbar.rs#L43) | 2 |
| `frame_vertical_scrollbar_clipped` | [components/scrollbar.rs](../../src/app/components/scrollbar.rs#L52) | 2 |
| `paint_scrollbar_thumb` | [components/scrollbar.rs](../../src/app/components/scrollbar.rs#L157) | 1 |
| `frame_select_trigger_with_focus` | [components/select.rs](../../src/app/components/select.rs#L30) | 1 |
| `frame_select_trigger_content` | [components/select.rs](../../src/app/components/select.rs#L63) | 1 |
| `frame_select_trigger_content_with_focus` | [components/select.rs](../../src/app/components/select.rs#L82) | 1 |
| `frame_select_trigger_content_inner` | [components/select.rs](../../src/app/components/select.rs#L110) | 3 |
| `frame_select_popover` | [components/select.rs](../../src/app/components/select.rs#L175) | 1 |
| `frame_select_options_list` | [components/select.rs](../../src/app/components/select.rs#L203) | 1 |
| `frame_select_scrollbar` | [components/select.rs](../../src/app/components/select.rs#L218) | 1 |
| `frame_select_content_height` | [components/select.rs](../../src/app/components/select.rs#L234) | 2 |
| `frame_select_option_inner` | [components/select.rs](../../src/app/components/select.rs#L274) | 1 |
| `frame_color_select_value` | [components/select.rs](../../src/app/components/select.rs#L331) | 1 |
| `frame_color_swatch` | [components/select.rs](../../src/app/components/select.rs#L353) | 1 |
| `search` | [components/select_control.rs](../../src/app/components/select_control.rs#L52) | 7 |
| `new` | [components/select_field.rs](../../src/app/components/select_field.rs#L18) | 160 |
| `frame_select_field` | [components/select_field.rs](../../src/app/components/select_field.rs#L81) | 4 |
| `frame_boolean_field` | [components/select_field.rs](../../src/app/components/select_field.rs#L144) | 2 |
| `frame_range_track` | [components/slider.rs](../../src/app/components/slider.rs#L22) | 2 |
| `frame_progress_track` | [components/slider.rs](../../src/app/components/slider.rs#L30) | 1 |
| `range_track` | [components/slider.rs](../../src/app/components/slider.rs#L37) | 2 |
| `frame_slider_content_bounds` | [components/slider.rs](../../src/app/components/slider.rs#L75) | 2 |
| `frame_slider_handle` | [components/slider.rs](../../src/app/components/slider.rs#L225) | 6 |
| `new` | [components/tooltip.rs](../../src/app/components/tooltip.rs#L41) | 160 |
| `should_handle_text_input` | [input/element.rs](../../src/app/input/element.rs#L38) | 1 |
| `settings_audio_tab` | [inspector/audio.rs](../../src/app/inspector/audio.rs#L28) | 1 |
| `settings_audio_channels_grid` | [inspector/audio.rs](../../src/app/inspector/audio.rs#L96) | 1 |
| `settings_audio_encoding_controls` | [inspector/audio.rs](../../src/app/inspector/audio.rs#L165) | 1 |
| `settings_audio_bitrate_mode_grid` | [inspector/audio.rs](../../src/app/inspector/audio.rs#L237) | 1 |
| `settings_audio_bitrate_field` | [inspector/audio.rs](../../src/app/inspector/audio.rs#L275) | 1 |
| `settings_audio_range_field` | [inspector/audio.rs](../../src/app/inspector/audio.rs#L309) | 1 |
| `settings_audio_range_slider` | [inspector/audio.rs](../../src/app/inspector/audio.rs#L353) | 1 |
| `apply_settings_audio_range_value` | [inspector/audio.rs](../../src/app/inspector/audio.rs#L448) | 4 |
| `settings_audio_codec_list` | [inspector/audio.rs](../../src/app/inspector/audio.rs#L480) | 1 |
| `settings_audio_track_button` | [inspector/audio.rs](../../src/app/inspector/audio.rs#L516) | 1 |
| `settings_audio_filter_reset` | [inspector/audio_filters/controls.rs](../../src/app/inspector/audio_filters/controls.rs#L19) | 1 |
| `settings_audio_normalize_control` | [inspector/audio_filters/controls.rs](../../src/app/inspector/audio_filters/controls.rs#L53) | 1 |
| `settings_audio_compressor_control` | [inspector/audio_filters/controls.rs](../../src/app/inspector/audio_filters/controls.rs#L78) | 1 |
| `settings_audio_filters_reset_all` | [inspector/audio_filters/controls.rs](../../src/app/inspector/audio_filters/controls.rs#L147) | 1 |
| `settings_audio_filter_range_field` | [inspector/audio_filters/range.rs](../../src/app/inspector/audio_filters/range.rs#L47) | 9 |
| `settings_audio_filter_range_slider` | [inspector/audio_filters/range.rs](../../src/app/inspector/audio_filters/range.rs#L135) | 1 |
| `settings_audio_filters_tab` | [inspector/audio_filters/view.rs](../../src/app/inspector/audio_filters/view.rs#L21) | 1 |
| `settings_images_pixel_format_section` | [inspector/images.rs](../../src/app/inspector/images.rs#L101) | 1 |
| `settings_images_encoding_section` | [inspector/images.rs](../../src/app/inspector/images.rs#L141) | 1 |
| `settings_image_jpeg_section` | [inspector/images.rs](../../src/app/inspector/images.rs#L162) | 1 |
| `settings_image_webp_section` | [inspector/images.rs](../../src/app/inspector/images.rs#L195) | 1 |
| `settings_image_png_section` | [inspector/images.rs](../../src/app/inspector/images.rs#L261) | 1 |
| `settings_image_tiff_section` | [inspector/images.rs](../../src/app/inspector/images.rs#L294) | 1 |
| `settings_image_webp_mode_grid` | [inspector/images.rs](../../src/app/inspector/images.rs#L312) | 1 |
| `settings_image_range_field` | [inspector/images.rs](../../src/app/inspector/images.rs#L352) | 4 |
| `settings_image_range_slider` | [inspector/images.rs](../../src/app/inspector/images.rs#L396) | 1 |
| `apply_settings_image_range_value` | [inspector/images.rs](../../src/app/inspector/images.rs#L487) | 4 |
| `settings_image_range_slider_id` | [inspector/images.rs](../../src/app/inspector/images.rs#L520) | 1 |
| `settings_metadata_tab` | [inspector/metadata.rs](../../src/app/inspector/metadata.rs#L15) | 1 |
| `settings_metadata_mode_grid` | [inspector/metadata.rs](../../src/app/inspector/metadata.rs#L56) | 1 |
| `settings_metadata_fields` | [inspector/metadata.rs](../../src/app/inspector/metadata.rs#L93) | 1 |
| `settings_output_tab` | [inspector/output.rs](../../src/app/inspector/output.rs#L18) | 1 |
| `settings_processing_mode_grid` | [inspector/output.rs](../../src/app/inspector/output.rs#L85) | 1 |
| `settings_container_grid` | [inspector/output.rs](../../src/app/inspector/output.rs#L128) | 1 |
| `settings_output_name_field` | [inspector/output.rs](../../src/app/inspector/output.rs#L166) | 1 |
| `settings_panel` | [inspector/panel.rs](../../src/app/inspector/panel.rs#L30) | 1 |
| `settings_tab_button` | [inspector/panel.rs](../../src/app/inspector/panel.rs#L116) | 1 |
| `settings_tab_for_key` | [inspector/panel.rs](../../src/app/inspector/panel.rs#L178) | 2 |
| `settings_output_section` | [inspector/panel.rs](../../src/app/inspector/panel.rs#L263) | 1 |
| `settings_section` | [inspector/panel.rs](../../src/app/inspector/panel.rs#L372) | 6 |
| `settings_field` | [inspector/panel.rs](../../src/app/inspector/panel.rs#L380) | 46 |
| `settings_fields_grid` | [inspector/panel.rs](../../src/app/inspector/panel.rs#L388) | 3 |
| `settings_presets_tab` | [inspector/presets/view.rs](../../src/app/inspector/presets/view.rs#L30) | 1 |
| `settings_presets_notice` | [inspector/presets/view.rs](../../src/app/inspector/presets/view.rs#L62) | 1 |
| `settings_presets_save_row` | [inspector/presets/view.rs](../../src/app/inspector/presets/view.rs#L82) | 1 |
| `settings_save_preset_button` | [inspector/presets/view.rs](../../src/app/inspector/presets/view.rs#L115) | 1 |
| `settings_preset_row` | [inspector/presets/view.rs](../../src/app/inspector/presets/view.rs#L139) | 1 |
| `settings_preset_icon_button` | [inspector/presets/view.rs](../../src/app/inspector/presets/view.rs#L230) | 2 |
| `settings_field_label` | [inspector/shared.rs](../../src/app/inspector/shared.rs#L9) | 15 |
| `settings_value_badge` | [inspector/shared.rs](../../src/app/inspector/shared.rs#L16) | 5 |
| `settings_hint_text` | [inspector/shared.rs](../../src/app/inspector/shared.rs#L23) | 18 |
| `settings_value_row` | [inspector/shared.rs](../../src/app/inspector/shared.rs#L30) | 2 |
| `settings_tab_icon` | [inspector/shared.rs](../../src/app/inspector/shared.rs#L52) | 1 |
| `settings_source_tab` | [inspector/source.rs](../../src/app/inspector/source.rs#L10) | 1 |
| `settings_source_tracks` | [inspector/source.rs](../../src/app/inspector/source.rs#L88) | 1 |
| `settings_track_header` | [inspector/source.rs](../../src/app/inspector/source.rs#L106) | 1 |
| `settings_subtitle_burn_file_row` | [inspector/subtitles/burn_in.rs](../../src/app/inspector/subtitles/burn_in.rs#L28) | 1 |
| `settings_subtitle_clear_button` | [inspector/subtitles/burn_in.rs](../../src/app/inspector/subtitles/burn_in.rs#L63) | 1 |
| `settings_subtitle_style_controls` | [inspector/subtitles/burn_in.rs](../../src/app/inspector/subtitles/burn_in.rs#L99) | 1 |
| `settings_subtitle_position_grid` | [inspector/subtitles/burn_in.rs](../../src/app/inspector/subtitles/burn_in.rs#L208) | 1 |
| `into_element` | [inspector/subtitles/color_picker.rs](../../src/app/inspector/subtitles/color_picker.rs#L50) | 0 |
| `settings_subtitle_color_picker` | [inspector/subtitles/color_picker.rs](../../src/app/inspector/subtitles/color_picker.rs#L261) | 1 |
| `settings_subtitle_sv_square` | [inspector/subtitles/color_picker.rs](../../src/app/inspector/subtitles/color_picker.rs#L340) | 1 |
| `settings_subtitle_hue_slider` | [inspector/subtitles/color_picker.rs](../../src/app/inspector/subtitles/color_picker.rs#L431) | 1 |
| `settings_subtitle_file_button` | [inspector/subtitles/file_button.rs](../../src/app/inspector/subtitles/file_button.rs#L10) | 2 |
| `settings_subtitle_font_select` | [inspector/subtitles/font_select.rs](../../src/app/inspector/subtitles/font_select.rs#L10) | 1 |
| `settings_subtitle_font_size_select` | [inspector/subtitles/size_select.rs](../../src/app/inspector/subtitles/size_select.rs#L10) | 1 |
| `settings_external_subtitles_section` | [inspector/subtitles/tracks.rs](../../src/app/inspector/subtitles/tracks.rs#L21) | 1 |
| `has_any` | [inspector/subtitles/tracks.rs](../../src/app/inspector/subtitles/tracks.rs#L134) | 1 |
| `settings_external_subtitle_editor` | [inspector/subtitles/tracks.rs](../../src/app/inspector/subtitles/tracks.rs#L211) | 1 |
| `settings_subtitle_track_button` | [inspector/subtitles/tracks.rs](../../src/app/inspector/subtitles/tracks.rs#L317) | 1 |
| `settings_subtitles_tab` | [inspector/subtitles/view.rs](../../src/app/inspector/subtitles/view.rs#L108) | 1 |
| `settings_subtitle_mode_grid` | [inspector/subtitles/view.rs](../../src/app/inspector/subtitles/view.rs#L129) | 1 |
| `settings_selectable_subtitles_content` | [inspector/subtitles/view.rs](../../src/app/inspector/subtitles/view.rs#L166) | 1 |
| `settings_burn_in_subtitles_content` | [inspector/subtitles/view.rs](../../src/app/inspector/subtitles/view.rs#L196) | 1 |
| `settings_video_encoder_section` | [inspector/video/encoding.rs](../../src/app/inspector/video/encoding.rs#L19) | 1 |
| `settings_video_pixel_format_section` | [inspector/video/encoding.rs](../../src/app/inspector/video/encoding.rs#L60) | 1 |
| `settings_video_preset_section` | [inspector/video/encoding.rs](../../src/app/inspector/video/encoding.rs#L108) | 1 |
| `settings_video_quality_section` | [inspector/video/encoding.rs](../../src/app/inspector/video/encoding.rs#L156) | 1 |
| `settings_video_bitrate_mode_grid` | [inspector/video/encoding.rs](../../src/app/inspector/video/encoding.rs#L242) | 1 |
| `settings_video_nvenc_section` | [inspector/video/encoding.rs](../../src/app/inspector/video/encoding.rs#L277) | 1 |
| `settings_video_videotoolbox_section` | [inspector/video/encoding.rs](../../src/app/inspector/video/encoding.rs#L324) | 1 |
| `settings_video_hw_section` | [inspector/video/encoding.rs](../../src/app/inspector/video/encoding.rs#L351) | 1 |
| `settings_video_checkbox_row` | [inspector/video/encoding.rs](../../src/app/inspector/video/encoding.rs#L380) | 4 |
| `settings_video_resolution_section` | [inspector/video/geometry.rs](../../src/app/inspector/video/geometry.rs#L18) | 2 |
| `settings_resolution_grid` | [inspector/video/geometry.rs](../../src/app/inspector/video/geometry.rs#L50) | 1 |
| `settings_custom_dimensions_grid` | [inspector/video/geometry.rs](../../src/app/inspector/video/geometry.rs#L80) | 1 |
| `settings_video_scaling_section` | [inspector/video/geometry.rs](../../src/app/inspector/video/geometry.rs#L136) | 2 |
| `settings_video_fps_section` | [inspector/video/geometry.rs](../../src/app/inspector/video/geometry.rs#L175) | 1 |
| `settings_video_gif_colors_section` | [inspector/video/geometry.rs](../../src/app/inspector/video/geometry.rs#L206) | 1 |
| `settings_video_gif_dither_section` | [inspector/video/geometry.rs](../../src/app/inspector/video/geometry.rs#L243) | 1 |
| `settings_video_gif_loop_section` | [inspector/video/geometry.rs](../../src/app/inspector/video/geometry.rs#L282) | 1 |
| `settings_video_range_field` | [inspector/video/quality.rs](../../src/app/inspector/video/quality.rs#L41) | 1 |
| `settings_video_range_slider` | [inspector/video/quality.rs](../../src/app/inspector/video/quality.rs#L86) | 1 |
| `apply_settings_video_range_value` | [inspector/video/quality.rs](../../src/app/inspector/video/quality.rs#L183) | 4 |
| `settings_video_tab` | [inspector/video/view.rs](../../src/app/inspector/video/view.rs#L26) | 1 |
| `settings_video_filter_reset` | [inspector/video_filters/controls.rs](../../src/app/inspector/video_filters/controls.rs#L20) | 1 |
| `settings_video_denoise_control` | [inspector/video_filters/controls.rs](../../src/app/inspector/video_filters/controls.rs#L54) | 1 |
| `settings_video_grayscale_control` | [inspector/video_filters/controls.rs](../../src/app/inspector/video_filters/controls.rs#L120) | 1 |
| `settings_video_deinterlace_control` | [inspector/video_filters/controls.rs](../../src/app/inspector/video_filters/controls.rs#L145) | 1 |
| `settings_video_filters_reset_all` | [inspector/video_filters/controls.rs](../../src/app/inspector/video_filters/controls.rs#L184) | 1 |
| `settings_video_filter_range_field` | [inspector/video_filters/range.rs](../../src/app/inspector/video_filters/range.rs#L48) | 10 |
| `settings_video_filter_range_slider` | [inspector/video_filters/range.rs](../../src/app/inspector/video_filters/range.rs#L135) | 1 |
| `settings_video_filters_tab` | [inspector/video_filters/view.rs](../../src/app/inspector/video_filters/view.rs#L21) | 1 |
| `logs_view` | [logs/view.rs](../../src/app/logs/view.rs#L30) | 1 |
| `logs_tab_strip` | [logs/view.rs](../../src/app/logs/view.rs#L73) | 1 |
| `log_tab_button` | [logs/view.rs](../../src/app/logs/view.rs#L140) | 1 |
| `logs_body` | [logs/view.rs](../../src/app/logs/view.rs#L221) | 1 |
| `log_lines_list` | [logs/view.rs](../../src/app/logs/view.rs#L276) | 1 |
| `log_tab_id_for_key` | [logs/view.rs](../../src/app/logs/view.rs#L350) | 2 |
| `log_scroll_to_bottom_button` | [logs/view.rs](../../src/app/logs/view.rs#L473) | 1 |
| `logs_copy_button` | [logs/view.rs](../../src/app/logs/view.rs#L510) | 1 |
| `log_line_tone` | [logs/view.rs](../../src/app/logs/view.rs#L551) | 4 |
| `css_ease` | [motion.rs](../../src/app/motion.rs#L14) | 3 |
| `app_settings_appearance_section` | [preferences/appearance.rs](../../src/app/preferences/appearance.rs#L22) | 1 |
| `options` | [preferences/appearance.rs](../../src/app/preferences/appearance.rs#L92) | 2 |
| `app_settings_drawer` | [preferences/view.rs](../../src/app/preferences/view.rs#L72) | 1 |
| `app_settings_version_label` | [preferences/view.rs](../../src/app/preferences/view.rs#L291) | 1 |
| `app_settings_output_directory_section` | [preferences/view.rs](../../src/app/preferences/view.rs#L304) | 1 |
| `app_settings_concurrency_control` | [preferences/view.rs](../../src/app/preferences/view.rs#L362) | 1 |
| `app_settings_close_button` | [preferences/view.rs](../../src/app/preferences/view.rs#L407) | 2 |
| `app_settings_apply_button` | [preferences/view.rs](../../src/app/preferences/view.rs#L437) | 1 |
| `crop_select` | [preview/crop_options/select.rs](../../src/app/preview/crop_options/select.rs#L16) | 2 |
| `preview_crop_options` | [preview/crop_options/view.rs](../../src/app/preview/crop_options/view.rs#L12) | 1 |
| `preview_crop_custom_fields` | [preview/crop_options/view.rs](../../src/app/preview/crop_options/view.rs#L22) | 1 |
| `preview_crop_overlay` | [preview/crop_overlay.rs](../../src/app/preview/crop_overlay.rs#L33) | 1 |
| `preview_crop_visual_rect` | [preview/crop_overlay.rs](../../src/app/preview/crop_overlay.rs#L128) | 1 |
| `crop_outline_rect` | [preview/crop_overlay.rs](../../src/app/preview/crop_overlay.rs#L150) | 1 |
| `crop_vertical_guide_line` | [preview/crop_overlay.rs](../../src/app/preview/crop_overlay.rs#L209) | 2 |
| `crop_horizontal_guide_line` | [preview/crop_overlay.rs](../../src/app/preview/crop_overlay.rs#L219) | 2 |
| `preview_crop_handle` | [preview/crop_overlay.rs](../../src/app/preview/crop_overlay.rs#L229) | 8 |
| `crop_handle_cursor` | [preview/crop_overlay.rs](../../src/app/preview/crop_overlay.rs#L296) | 1 |
| `crop_handle_screen_cursor` | [preview/crop_overlay.rs](../../src/app/preview/crop_overlay.rs#L309) | 1 |
| `compact_text_button` | [preview/crop_overlay.rs](../../src/app/preview/crop_overlay.rs#L441) | 1 |
| `compact_text_button_variant` | [preview/crop_overlay.rs](../../src/app/preview/crop_overlay.rs#L458) | 1 |
| `compact_text_button_variant_inner` | [preview/crop_overlay.rs](../../src/app/preview/crop_overlay.rs#L474) | 2 |
| `preview_overlay_controls` | [preview/overlay/controls.rs](../../src/app/preview/overlay/controls.rs#L19) | 1 |
| `preview_overlay_icon_button_with_focus` | [preview/overlay/controls.rs](../../src/app/preview/overlay/controls.rs#L177) | 2 |
| `preview_overlay_icon_button_inner` | [preview/overlay/controls.rs](../../src/app/preview/overlay/controls.rs#L205) | 2 |
| `preview_overlay_layer` | [preview/overlay/layer.rs](../../src/app/preview/overlay/layer.rs#L34) | 1 |
| `overlay_handle_cursor` | [preview/overlay/layer.rs](../../src/app/preview/overlay/layer.rs#L296) | 1 |
| `into_element` | [preview/overlay/opacity.rs](../../src/app/preview/overlay/opacity.rs#L27) | 0 |
| `default` | [preview/panel.rs](../../src/app/preview/panel.rs#L68) | 71 |
| `preview_timecode_field` | [preview/timeline/timecode.rs](../../src/app/preview/timeline/timecode.rs#L18) | 3 |
| `preview_timeline_label` | [preview/timeline/timecode.rs](../../src/app/preview/timeline/timecode.rs#L77) | 3 |
| `into_element` | [preview/timeline/track.rs](../../src/app/preview/timeline/track.rs#L41) | 0 |
| `preview_timeline_handle` | [preview/timeline/track.rs](../../src/app/preview/timeline/track.rs#L296) | 2 |
| `preview_timeline` | [preview/timeline/view.rs](../../src/app/preview/timeline/view.rs#L15) | 1 |
| `preview_timeline_labels` | [preview/timeline/view.rs](../../src/app/preview/timeline/view.rs#L103) | 1 |
| `preview_toolbar_height` | [preview/toolbar.rs](../../src/app/preview/toolbar.rs#L25) | 1 |
| `preview_zoom_toolbar` | [preview/toolbar.rs](../../src/app/preview/toolbar.rs#L157) | 1 |
| `preview_toolbar_vertical_separator` | [preview/toolbar.rs](../../src/app/preview/toolbar.rs#L215) | 3 |
| `preview_tool_button` | [preview/toolbar.rs](../../src/app/preview/toolbar.rs#L229) | 6 |
| `preview_tool_button_with_focus` | [preview/toolbar.rs](../../src/app/preview/toolbar.rs#L260) | 2 |
| `preview_tool_button_inner` | [preview/toolbar.rs](../../src/app/preview/toolbar.rs#L292) | 2 |
| `into_element` | [preview/viewport/media.rs](../../src/app/preview/viewport/media.rs#L19) | 0 |
| `preview_viewport` | [preview/viewport/view.rs](../../src/app/preview/viewport/view.rs#L34) | 1 |
| `preview_viewport_content` | [preview/viewport/view.rs](../../src/app/preview/viewport/view.rs#L144) | 1 |
| `preview_canvas_pan_enabled` | [preview/viewport/view.rs](../../src/app/preview/viewport/view.rs#L357) | 2 |
| `button_colors` | [primitives.rs](../../src/app/primitives.rs#L32) | 8 |
| `icon_svg` | [primitives.rs](../../src/app/primitives.rs#L142) | 10 |
| `parse_hex` | [primitives.rs](../../src/app/primitives.rs#L150) | 5 |
| `panel_bottom_separator` | [primitives.rs](../../src/app/primitives.rs#L170) | 6 |
| `drag_drop_overlay` | [queue/drop_overlay.rs](../../src/app/queue/drop_overlay.rs#L8) | 1 |
| `file_list_panel` | [queue/view.rs](../../src/app/queue/view.rs#L27) | 1 |
| `file_list_header` | [queue/view.rs](../../src/app/queue/view.rs#L48) | 1 |
| `queue_navigation_state` | [queue/view.rs](../../src/app/queue/view.rs#L139) | 1 |
| `file_list_row` | [queue/view.rs](../../src/app/queue/view.rs#L282) | 1 |
| `header_label` | [queue/view.rs](../../src/app/queue/view.rs#L386) | 4 |
| `row_label` | [queue/view.rs](../../src/app/queue/view.rs#L395) | 4 |
| `row_actions_cell` | [queue/view.rs](../../src/app/queue/view.rs#L410) | 1 |
| `row_primary_action_button` | [queue/view.rs](../../src/app/queue/view.rs#L456) | 1 |
| `row_checkbox_control` | [queue/view.rs](../../src/app/queue/view.rs#L630) | 1 |
| `render_overlays` | [render/overlays.rs](../../src/app/render/overlays.rs#L20) | 1 |
| `linux_window_frame` | [render/window_frame.rs](../../src/app/render/window_frame.rs#L11) | 1 |
| `linux_window_frame` | [render/window_frame.rs](../../src/app/render/window_frame.rs#L33) | 1 |
| `update_dialog` | [updates/dialog.rs](../../src/app/updates/dialog.rs#L30) | 1 |
| `update_dialog_panel` | [updates/dialog.rs](../../src/app/updates/dialog.rs#L93) | 1 |
| `update_dialog_header` | [updates/dialog.rs](../../src/app/updates/dialog.rs#L146) | 1 |
| `update_dialog_body` | [updates/dialog.rs](../../src/app/updates/dialog.rs#L203) | 1 |
| `update_dialog_download_state` | [updates/dialog.rs](../../src/app/updates/dialog.rs#L250) | 1 |
| `update_dialog_footer` | [updates/dialog.rs](../../src/app/updates/dialog.rs#L279) | 1 |
| `update_dialog_primary_action` | [updates/dialog.rs](../../src/app/updates/dialog.rs#L324) | 1 |
| `app_settings_updates_section` | [updates/preferences.rs](../../src/app/updates/preferences.rs#L25) | 1 |
| `update_status_label` | [updates/preferences.rs](../../src/app/updates/preferences.rs#L94) | 1 |
| `update_status_text` | [updates/preferences.rs](../../src/app/updates/preferences.rs#L115) | 1 |
| `update_download_detail` | [updates/preferences.rs](../../src/app/updates/preferences.rs#L167) | 2 |
| `update_action_row` | [updates/preferences.rs](../../src/app/updates/preferences.rs#L194) | 1 |
| `update_release_notes_text` | [updates/release_notes.rs](../../src/app/updates/release_notes.rs#L25) | 1 |
| `update_release_notes_content_height` | [updates/release_notes.rs](../../src/app/updates/release_notes.rs#L90) | 1 |
| `workspace_view` | [workspace.rs](../../src/app/workspace.rs#L24) | 1 |
| `welcome_view` | [workspace.rs](../../src/app/workspace.rs#L68) | 2 |
| `settings_panel_for_selection` | [workspace.rs](../../src/app/workspace.rs#L162) | 1 |
| `empty_settings_panel` | [workspace.rs](../../src/app/workspace.rs#L175) | 1 |

## Jawne identyfikatory elementów

Identyfikatory obejmują również kontenery, które mają znaczenie dla testów geometrii i dostępności. Pełne argumenty dynamicznych kontrolek są w podanych konstruktorach.

| Identyfikator lub wzorzec | Źródło |
| --- | --- |
| `logs-tab-list` | [logs/view.rs](../../src/app/logs/view.rs#L90) |
| `logs-body` | [logs/view.rs](../../src/app/logs/view.rs#L232) |
| `app-settings-drawer` | [preferences/view.rs](../../src/app/preferences/view.rs#L98) |
| `app-settings-backdrop` | [preferences/view.rs](../../src/app/preferences/view.rs#L126) |
| `app-settings-panel` | [preferences/view.rs](../../src/app/preferences/view.rs#L142) |
| `app-settings-scroll-body` | [preferences/view.rs](../../src/app/preferences/view.rs#L209) |
| `app-settings-max-concurrency-error` | [preferences/view.rs](../../src/app/preferences/view.rs#L263) |
| `app-settings-output-directory-path` | [preferences/view.rs](../../src/app/preferences/view.rs#L342) |
| `app-settings-output-directory-error` | [preferences/view.rs](../../src/app/preferences/view.rs#L351) |
| `app-settings-appearance-error` | [preferences/appearance.rs](../../src/app/preferences/appearance.rs#L72) |
| `update-dialog-release-notes-content` | [updates/release_notes.rs](../../src/app/updates/release_notes.rs#L52) |
| `update-dialog-release-notes` | [updates/release_notes.rs](../../src/app/updates/release_notes.rs#L65) |
| `update-dialog` | [updates/dialog.rs](../../src/app/updates/dialog.rs#L49) |
| `update-dialog-panel` | [updates/dialog.rs](../../src/app/updates/dialog.rs#L101) |
| `update-dialog-error-alert` | [updates/dialog.rs](../../src/app/updates/dialog.rs#L234) |
| `app-settings-update-status` | [updates/preferences.rs](../../src/app/updates/preferences.rs#L107) |
| `app-settings-update-progress` | [updates/preferences.rs](../../src/app/updates/preferences.rs#L158) |
| `tooltip-{}` | [components/tooltip.rs](../../src/app/components/tooltip.rs#L199) |
| `{id}-indicator` | [components/checkbox.rs](../../src/app/components/checkbox.rs#L140) |
| `file-list-header-checkbox-hit-area` | [queue/view.rs](../../src/app/queue/view.rs#L55) |
| `file-list-header-checkbox` | [queue/view.rs](../../src/app/queue/view.rs#L81) |
| `file-list-body` | [queue/view.rs](../../src/app/queue/view.rs#L188) |
| `drag-drop-overlay` | [queue/drop_overlay.rs](../../src/app/queue/drop_overlay.rs#L25) |
| `range-fill` | [components/slider.rs](../../src/app/components/slider.rs#L58) |
| `{id}-thumb` | [components/slider.rs](../../src/app/components/slider.rs#L196) |
| `titlebar-main-view-tabs` | [chrome/navigation.rs](../../src/app/chrome/navigation.rs#L28) |
| `preview-crop-move-handle` | [preview/crop_overlay.rs](../../src/app/preview/crop_overlay.rs#L159) |
| `preview-crop-handle-{}` | [preview/crop_overlay.rs](../../src/app/preview/crop_overlay.rs#L239) |
| `preview-crop-toolbar` | [preview/crop_overlay.rs](../../src/app/preview/crop_overlay.rs#L380) |
| `preview-viewport` | [preview/viewport/view.rs](../../src/app/preview/viewport/view.rs#L43) |
| `preview-canvas-pan-layer` | [preview/viewport/view.rs](../../src/app/preview/viewport/view.rs#L151) |
| `preview-empty-state` | [preview/viewport/view.rs](../../src/app/preview/viewport/view.rs#L210) |
| `preview-media-stage` | [preview/viewport/view.rs](../../src/app/preview/viewport/view.rs#L293) |
| `preview-timeline-track` | [preview/timeline/track.rs](../../src/app/preview/timeline/track.rs#L139) |
| `settings-source-metadata-error` | [inspector/source.rs](../../src/app/inspector/source.rs#L26) |
| `settings-presets-notice` | [inspector/presets/view.rs](../../src/app/inspector/presets/view.rs#L67) |
| `settings-tab-list` | [inspector/panel.rs](../../src/app/inspector/panel.rs#L56) |
| `settings-panel-body` | [inspector/panel.rs](../../src/app/inspector/panel.rs#L100) |
| `preview-overlay-layer` | [preview/overlay/layer.rs](../../src/app/preview/overlay/layer.rs#L54) |
| `preview-overlay-handle-{}` | [preview/overlay/layer.rs](../../src/app/preview/overlay/layer.rs#L229) |
| `preview-overlay-toolbar` | [preview/overlay/controls.rs](../../src/app/preview/overlay/controls.rs#L34) |
