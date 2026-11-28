use crate::{media_rules::audio_track_metadata_support, types::AudioTrackOverride};
use std::collections::BTreeMap;

// libavformat/isom.c, mov_mdhd_language_map and ff_mov_iso639_to_lang:
// https://github.com/FFmpeg/FFmpeg/blob/n9.0.2/libavformat/isom.c#L91-L258
// The space-padded two-letter entries are normalized separately below.
const QUICKTIME_LANGUAGE_CODES: &[&str] = &[
    "alb", "amh", "ara", "arm", "asm", "aym", "aze", "baq", "bel", "ben", "bul", "bur", "cat",
    "ces", "chi", "dan", "dut", "dzo", "eng", "epo", "est", "fin", "fra", "geo", "ger", "gre",
    "grn", "guj", "heb", "hin", "hun", "ice", "ind", "iri", "ita", "jav", "jpn", "kan", "kas",
    "kaz", "khm", "kin", "kir", "kor", "kur", "lao", "lat", "lav", "lit", "mac", "mal", "mar",
    "may", "mlg", "mlt", "mol", "mon", "nep", "nor", "nya", "ori", "orm", "per", "pol", "por",
    "pus", "que", "ron", "run", "rus", "san", "sin", "slk", "slv", "smi", "snd", "som", "spa",
    "sve", "swa", "tam", "tat", "tel", "tgk", "tgl", "tha", "tib", "tir", "tuk", "tur", "uig",
    "ukr", "urd", "uzb", "vie", "wel", "yid",
];

fn requires_default(container: &str) -> bool {
    ["mp4", "mov", "m4a"]
        .iter()
        .any(|name| container.eq_ignore_ascii_case(name))
}

/// Reports a conflict among selected source tracks. Duplicate mappings of the
/// same source track represent one choice.
#[must_use]
pub fn audio_track_default_error(
    container: &str,
    selected: &[u32],
    edits: &BTreeMap<u32, AudioTrackOverride>,
) -> Option<String> {
    conflicting_defaults_error(container, selected, edits)
        .or_else(|| missing_audio_default_error(container, selected, edits))
}

fn conflicting_defaults_error(
    container: &str,
    selected: &[u32],
    edits: &BTreeMap<u32, AudioTrackOverride>,
) -> Option<String> {
    if selected.is_empty() || !audio_track_metadata_support(container).default_flag {
        return None;
    }
    let mut chosen = None;
    for &index in selected {
        if edits.get(&index).and_then(|edit| edit.is_default) == Some(true)
            && chosen.replace(index).is_some_and(|prior| prior != index)
        {
            return Some("Select only one default audio track.".into());
        }
    }
    None
}

/// Reports a required playback default once all selected tracks explicitly opt out.
#[must_use]
pub fn missing_audio_default_error(
    container: &str,
    selected: &[u32],
    edits: &BTreeMap<u32, AudioTrackOverride>,
) -> Option<String> {
    if !selected.is_empty()
        && requires_default(container)
        && selected
            .iter()
            .all(|index| edits.get(index).and_then(|edit| edit.is_default) == Some(false))
    {
        return Some(format!(
            "{} requires a default audio track. Set Default to Yes on one track.",
            container.to_ascii_uppercase()
        ));
    }
    None
}

/// Validates only the edits that apply to the selected streams and metadata mode.
#[must_use]
pub fn audio_track_metadata_error(
    container: &str,
    selected: &[u32],
    edits: &BTreeMap<u32, AudioTrackOverride>,
    preserve_text: bool,
) -> Option<String> {
    missing_audio_default_error(container, selected, edits)
        .or_else(|| audio_track_metadata_edit_error(container, selected, edits, preserve_text))
}

/// Validates individual edits while the user is still choosing a default track.
#[must_use]
pub fn audio_track_metadata_edit_error(
    container: &str,
    selected: &[u32],
    edits: &BTreeMap<u32, AudioTrackOverride>,
    preserve_text: bool,
) -> Option<String> {
    if edits.is_empty() {
        return None;
    }
    if let Some(reason) = conflicting_defaults_error(container, selected, edits) {
        return Some(reason);
    }
    if preserve_text {
        let support = audio_track_metadata_support(container);
        for (index, edit) in edits {
            if !selected.contains(index) {
                continue;
            }
            if support.language
                && let Some(language) = &edit.language
                && let Err(reason) = audio_track_language_value(container, language)
            {
                return Some(format!("Audio track #{index}: {reason}"));
            }
            if support.title
                && edit
                    .title
                    .as_deref()
                    .is_some_and(|title| title.chars().any(char::is_control))
            {
                return Some(format!(
                    "Audio track #{index}: title contains a control character."
                ));
            }
        }
    }
    None
}

/// Returns per-output changes to the default flag. `None` preserves the source.
///
/// MP4-family muxers enable the first track when no track is enabled. Choose a
/// remaining eligible track explicitly when the user has cleared that first one.
///
/// # Errors
/// Returns the reason the selected defaults cannot be represented.
pub fn audio_track_default_actions(
    container: &str,
    selected: &[u32],
    edits: &BTreeMap<u32, AudioTrackOverride>,
    source_default: impl Fn(u32) -> bool,
) -> Result<Vec<Option<bool>>, String> {
    if let Some(error) = audio_track_default_error(container, selected, edits) {
        return Err(error);
    }
    if !audio_track_metadata_support(container).default_flag {
        return Ok(vec![None; selected.len()]);
    }
    let mut actions: Vec<_> = selected
        .iter()
        .map(|index| edits.get(index).and_then(|edit| edit.is_default))
        .collect();
    if let Some(chosen) = actions.iter().position(|action| *action == Some(true)) {
        actions.fill(Some(false));
        actions[chosen] = Some(true);
    } else if requires_default(container)
        && actions.contains(&Some(false))
        && !selected
            .iter()
            .zip(&actions)
            .any(|(&index, action)| action.unwrap_or_else(|| source_default(index)))
        && let Some(fallback) = actions.iter_mut().find(|action| action.is_none())
    {
        *fallback = Some(true);
    }
    Ok(actions)
}

/// Produces the track language spelling the selected muxer can store.
///
/// # Errors
/// Returns a validation message for invalid tags and unsupported `QuickTime` codes.
pub fn audio_track_language_value(container: &str, value: &str) -> Result<String, String> {
    if value.chars().any(char::is_control) {
        return Err("Audio language contains a control character.".into());
    }
    let value = value.trim();
    if value.is_empty()
        || ["mkv", "webm", "opus"]
            .iter()
            .any(|c| container.eq_ignore_ascii_case(c))
    {
        return Ok(value.to_owned());
    }
    let lower = value.to_ascii_lowercase();
    if container.eq_ignore_ascii_case("mov") {
        // QuickTime mdhd uses the legacy Macintosh language table. The muxer
        // silently writes "unspecified" for unknown codes. Map the equivalent
        // ISO spellings to the tags actually read back by libavformat.
        let code = match lower.as_str() {
            "und" => "",
            "deu" => "ger",
            "nld" => "dut",
            "zho" => "chi",
            "swe" => "sve",
            "ell" => "gre",
            "isl" => "ice",
            "fas" => "per",
            "gle" => "iri",
            "sqi" => "alb",
            "mkd" => "mac",
            "hye" => "arm",
            "kat" => "geo",
            "bod" => "tib",
            "mya" => "bur",
            "msa" => "may",
            "cym" => "wel",
            "eus" => "baq",
            "fre" => "fra",
            "rum" => "ron",
            "cze" => "ces",
            "slo" => "slk",
            "hr" | "hrv" => "hr ",
            "fo" | "fao" => "fo ",
            "sr" | "srp" => "sr ",
            "pa" | "pan" => "pa ",
            code if QUICKTIME_LANGUAGE_CODES.contains(&code) => code,
            _ => {
                return Err(format!(
                    "MOV cannot store the audio language '{value}' in its QuickTime language table."
                ));
            }
        };
        return Ok(code.to_owned());
    }
    if lower.len() != 3 || !lower.bytes().all(|c| c.is_ascii_lowercase()) {
        return Err(format!(
            "{} audio language requires a three-letter language code, such as eng.",
            container.to_ascii_uppercase()
        ));
    }
    Ok(lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_defaults_skip_rejected_sources_and_dormant_edits() {
        let mut overrides = BTreeMap::new();
        let edit = |is_default| AudioTrackOverride {
            is_default: Some(is_default),
            ..Default::default()
        };
        overrides.insert(9, edit(true));
        overrides.insert(7, edit(false));
        let inherited = |index| index == 7;
        for container in ["mp4", "mov", "m4a"] {
            assert_eq!(
                audio_track_default_actions(container, &[7, 3, 7], &overrides, inherited).unwrap(),
                [Some(false), Some(true), Some(false)]
            );
            assert!(audio_track_default_error(container, &[7, 7], &overrides).is_some());
            assert!(audio_track_default_error(container, &[], &overrides).is_none());
        }
        assert_eq!(
            audio_track_default_actions("mkv", &[7, 3], &overrides, inherited).unwrap(),
            [Some(false), None]
        );
        overrides.insert(3, edit(true));
        assert!(audio_track_default_error("mkv", &[7, 9, 3], &overrides).is_some());
        assert!(audio_track_default_error("opus", &[7, 9, 3], &overrides).is_none());
    }

    #[test]
    fn audio_language_edits_follow_the_muxers_actual_representation() {
        for (container, input, expected) in [
            ("mp4", " ENG ", "eng"),
            ("mov", "DEU", "ger"),
            ("mov", "zho", "chi"),
            ("mov", "deu", "ger"),
            ("mov", "pol", "pol"),
            ("mov", "hrv", "hr "),
            ("m2t", "POL", "pol"),
            ("mkv", "pl-PL", "pl-PL"),
            ("webm", "en-US", "en-US"),
            ("opus", "fr-CA", "fr-CA"),
        ] {
            assert_eq!(
                audio_track_language_value(container, input).unwrap(),
                expected
            );
        }
        for (container, input) in [
            ("mp4", "de-DE"),
            ("m4a", "x"),
            ("mov", "xyz"),
            ("mkv", "eng\n"),
            ("m2t", "es"),
        ] {
            assert!(audio_track_language_value(container, input).is_err());
        }
    }
}
