use super::*;

fn timing(origin: Option<i64>, base: &str, start: Option<i64>) -> SourceTiming {
    SourceTiming {
        format_start_microseconds: origin,
        format_start_invalid: false,
        first_video_stream_index: Some(0),
        streams: vec![SourceStreamTiming {
            index: 0,
            time_base: TimeBase::parse(base),
            start_pts: start,
        }],
    }
}

fn clock(timing: &SourceTiming) -> ResolvedSourceClock {
    resolve_source_clock(Some(timing), Some("matroska,webm"), Some(0), &[0])
}

#[test]
fn signed_origin_rounding_uses_exact_integer_half_ties() {
    for (origin, base, seek, expected) in [
        (0, "1/1000", 123_456, "round(123/(1000*TB))"),
        (0, "1/1000", -123_500, "round(-31/(250*TB))"),
        (6, "1/90000", -123_456, "round(-463/(3750*TB))"),
        (6, "1/90000", 123_456, "round(11111/(90000*TB))"),
        (-501_000, "1/1000", 123_500, "round(123/(1000*TB))"),
        (-501_000, "1/1000", -123_500, "round(-31/(250*TB))"),
    ] {
        assert_eq!(
            clock(&timing(Some(origin), base, Some(0)))
                .shift_expression(seek)
                .unwrap()
                .as_deref(),
            Some(expected)
        );
    }
}

#[test]
fn transport_clock_uses_only_enabled_streams_and_exact_format_tokens() {
    let mut source = timing(Some(1_400_000), "1/90000", Some(145_920));
    source.streams.push(SourceStreamTiming {
        index: 7,
        time_base: TimeBase::parse("1/90000"),
        start_pts: Some(126_000),
    });
    // Audio and PGS can both occupy index 7: the clock depends on actual maps,
    // not the stream type. Separate sidecar inputs never enter this list.
    let resolve = |format, selected: &[u32]| {
        resolve_source_clock(Some(&source), Some(format), Some(0), selected)
    };
    for format in [
        "dhav",
        "dvdvideo",
        "live_flv",
        "hls",
        "m4v",
        "mpeg",
        "mpegts",
        "mpegtsraw",
        "ogg",
        "rcwt",
        "ty",
        "mpegts,other",
    ] {
        assert_eq!(
            resolve(format, &[0]).shift_expression(123_456).unwrap(),
            None,
            "{format}"
        );
        assert_eq!(
            resolve(format, &[7, 0])
                .shift_expression(123_456)
                .unwrap()
                .as_deref(),
            Some("round(11111/(90000*TB))"),
            "{format}"
        );
    }
    for format in ["flv", "not_mpegts", "mpegts_other", "ty+"] {
        assert!(
            resolve(format, &[0])
                .shift_expression(123_456)
                .unwrap()
                .is_some(),
            "{format}"
        );
    }
    assert_eq!(
        resolve("mpegts", &[0])
            .shift_expression(350_001)
            .unwrap()
            .as_deref(),
        Some("round(193/(1500*TB))")
    );
}

#[test]
fn absent_timing_invalid_origin_and_nopts_do_not_invent_exact_metadata() {
    assert_eq!(
        resolve_source_clock(None, Some("mkv"), Some(0), &[0]),
        ResolvedSourceClock::Unknown
    );
    let mut source = timing(None, "1/1000", None);
    assert!(matches!(
        clock(&source),
        ResolvedSourceClock::Known {
            origin_microseconds: 0,
            ..
        }
    ));
    source.format_start_invalid = true;
    assert_eq!(clock(&source), ResolvedSourceClock::Unknown);
    source.format_start_invalid = false;
    source.format_start_microseconds = Some(i64::MIN);
    assert_eq!(clock(&source), clock(&timing(None, "1/1000", None)));
    source.format_start_microseconds = Some(0);
    source.streams[0].start_pts = Some(i64::MIN);
    assert_eq!(
        resolve_source_clock(Some(&source), Some("mpegts"), Some(0), &[0]),
        ResolvedSourceClock::Unknown
    );
    source.streams[0].start_pts = Some(1000);
    source.streams.push(SourceStreamTiming {
        index: 1,
        time_base: None,
        start_pts: Some(0),
    });
    assert_eq!(
        resolve_source_clock(Some(&source), Some("mpegts"), Some(0), &[0, 1]),
        ResolvedSourceClock::Unknown
    );
    source.streams[1].start_pts = Some(i64::MIN);
    assert!(matches!(
        resolve_source_clock(Some(&source), Some("mpegts"), Some(0), &[0, 1]),
        ResolvedSourceClock::Known { .. }
    ));
}

#[test]
fn serde_preserves_unknown_clock_and_never_turns_invalid_timebase_into_nopts() {
    let old: crate::types::ProbeMetadata =
        serde_json::from_str(r#"{"audioTracks":[],"subtitleTracks":[]}"#).unwrap();
    assert!(old.source_timing.is_none());
    let mut source = timing(Some(0), "1/1000", Some(0));
    let serialized = serde_json::to_value(&source).unwrap();
    assert_eq!(
        serde_json::from_value::<SourceTiming>(serialized.clone()).unwrap(),
        source
    );
    for invalid in [
        serde_json::json!({"numerator":0,"denominator":1}),
        serde_json::json!({"numerator":1,"denominator":0}),
        serde_json::json!({"numerator":1,"denominator":4_294_967_295_u64}),
        serde_json::json!("broken"),
    ] {
        let mut value = serialized.clone();
        value["streams"][0]["timeBase"] = invalid;
        source = serde_json::from_value(value).unwrap();
        assert_eq!(source.streams[0].start_pts, Some(0));
        assert_eq!(source.streams[0].time_base, None);
        assert_eq!(clock(&source), ResolvedSourceClock::Unknown);
    }
}

#[test]
fn known_overflow_is_an_error_only_when_the_clock_is_used() {
    let near_limit = clock(&timing(Some(i64::MAX), "1/1000000", Some(0)));
    assert!(near_limit.shift_expression(1).is_err());
    assert_eq!(near_limit.shift_expression(0).unwrap(), None);
    // A large absolute origin with a small difference needs no large expression.
    assert_eq!(
        near_limit.shift_expression(-1).unwrap().as_deref(),
        Some("round(-1/(1000000*TB))")
    );
    let microseconds = clock(&timing(Some(0), "1/1000000", Some(0)));
    assert!(
        microseconds
            .shift_expression(9_000_000_000_000_000_000)
            .is_err()
    );
    assert!(
        microseconds
            .shift_expression_for_filter((1_i64 << 51) + 1, true)
            .is_err()
    );
    assert!(
        microseconds
            .shift_expression_for_filter(1_i64 << 51, true)
            .is_ok()
    );
    assert!(microseconds.shift_expression(1_i64 << 52).is_ok());
    assert!(microseconds.shift_expression((1_i64 << 52) + 1).is_err());
    // Both shifts damaged tiny PTS 0..3 in the independent FFmpeg oracle.
    for shift in [(1_i64 << 53) - 1, 1_i64 << 53] {
        assert!(microseconds.shift_expression(shift).is_err());
        assert!(microseconds.shift_expression(-shift).is_err());
    }
    assert_eq!(
        ResolvedSourceClock::Invalid.shift_expression(0).unwrap(),
        None
    );
    assert!(ResolvedSourceClock::Invalid.shift_expression(1).is_err());
    assert_eq!(
        ResolvedSourceClock::Unknown
            .shift_expression(-123_456)
            .unwrap()
            .as_deref(),
        Some("round(-0.123456/TB)")
    );
}

#[test]
fn probe_records_raw_stream_ticks_and_distinguishes_missing_and_invalid_origin() {
    for (origin, invalid, expected) in [
        (serde_json::Value::Null, false, None),
        (serde_json::json!("N/A"), false, None),
        (serde_json::json!("broken"), true, None),
        (serde_json::json!("-0.501000"), false, Some(-501_000)),
    ] {
        let payload = serde_json::json!({"format":{"format_name":"mpegts","start_time":origin}, "streams":[
            {"index":4,"codec_type":"video","codec_name":"h264","time_base":"1/90000","start_pts":145_920},
            {"index":1,"codec_type":"audio","codec_name":"aac","time_base":"0/0","start_pts":126_000},
            {"index":9,"codec_type":"subtitle","codec_name":"hdmv_pgs_subtitle","time_base":"1/90000","start_pts":i64::MIN}
        ]});
        let metadata =
            crate::probe::parse_ffprobe_stdout("source.m2ts", payload.to_string()).unwrap();
        let timing = metadata.source_timing.unwrap();
        assert_eq!(timing.first_video_stream_index, Some(4));
        assert_eq!(timing.format_start_microseconds, expected);
        assert_eq!(timing.format_start_invalid, invalid);
        assert_eq!(timing.streams[1].time_base, None);
        assert_eq!(timing.streams[1].start_pts, Some(126_000));
        assert_eq!(timing.streams[2].start_pts, None);
    }
}
