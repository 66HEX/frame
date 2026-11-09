use super::{
    AlacStreamFacts, HashedData, ManifestPacket, ObservationError, PacketTimestamp,
    ParsedAlacSource, SourcePacketFrame,
};

#[derive(Clone, Copy, Eq, PartialEq)]
struct SampleKey {
    payload: HashedData,
    samples: u32,
    explicit_zero_skip: bool,
}

/// Both passes are linear even when silence makes every payload identical.
pub(super) fn match_source_samples(
    packets: &[&ManifestPacket],
    source: &ParsedAlacSource,
) -> Result<Vec<u32>, ObservationError> {
    let payloads: Vec<_> = packets.iter().map(|packet| packet.payload).collect();
    let (matches, first) = occurrences(
        &payloads,
        source.pairs.iter().map(|pair| Some(pair.packet.payload)),
    );
    let first = first.ok_or(ObservationError::Invalid("source sequence coverage"))?;
    let reference: Vec<_> = source.pairs[first..first + packets.len()]
        .iter()
        .map(|pair| sample_key(pair, &source.stream))
        .collect::<Option<_>>()
        .ok_or(ObservationError::Invalid("selected source sample facts"))?;
    for (packet, key) in packets.iter().zip(&reference) {
        if !(packet.side_data.is_empty()
            || key.explicit_zero_skip && zero_skip_side_data(&packet.side_data))
        {
            return Err(ObservationError::Invalid("copied packet side data"));
        }
    }
    let (equivalent, _) = occurrences(
        &reference,
        source
            .pairs
            .iter()
            .map(|pair| sample_key(pair, &source.stream)),
    );
    if equivalent != matches {
        return Err(ObservationError::Invalid("ambiguous source sample facts"));
    }
    Ok(reference.iter().map(|key| key.samples).collect())
}

fn sample_key(pair: &SourcePacketFrame, stream: &AlacStreamFacts) -> Option<SampleKey> {
    let packet = &pair.packet;
    let frame = &pair.frame;
    if !matches!(packet.pts, PacketTimestamp::Known(value) if value != i64::MIN)
        || packet.pts != packet.dts
        || packet.pts != frame.pts
        || packet.duration.is_some_and(|value| value < 0)
        || !packet.flags.key
        || packet.flags.corrupt
        || packet.flags.discard
        || pair.effective_skip != 0
        || pair.configuration_changed
        || packet
            .trimming
            .is_some_and(|value| value.skip != 0 || value.discard != 0)
        || !packet.other_side_data.is_empty()
        || !frame.side_data_types.is_empty()
        || frame.physical_samples == 0
        || frame.sample_format != stream.sample_format
        || frame.channels != stream.channels
        || frame.channel_layout != stream.channel_layout
    {
        return None;
    }
    Some(SampleKey {
        payload: packet.payload,
        samples: frame.physical_samples,
        explicit_zero_skip: packet.trimming.is_some(),
    })
}

fn zero_skip_side_data(data: &[HashedData]) -> bool {
    // AV_PKT_DATA_SKIP_SAMPLES consists of two LE u32 counts and two reason
    // bytes. Accept its all-zero hash only when the source identifies its type.
    const ZERO_SKIP_SHA256: [u8; 32] = [
        1, 212, 72, 175, 217, 40, 6, 84, 88, 207, 103, 11, 96, 245, 165, 148, 215, 53, 175, 1, 114,
        200, 214, 127, 34, 168, 22, 128, 19, 38, 129, 202,
    ];
    matches!(data, [value] if value.size == 10 && value.digest.bytes() == &ZERO_SKIP_SHA256)
}

fn occurrences<T: Copy + Eq>(
    pattern: &[T],
    values: impl Iterator<Item = Option<T>>,
) -> (usize, Option<usize>) {
    if pattern.is_empty() {
        return (0, None);
    }
    let mut prefix = vec![0; pattern.len()];
    let mut matched = 0;
    for index in 1..pattern.len() {
        while matched > 0 && pattern[matched] != pattern[index] {
            matched = prefix[matched - 1];
        }
        if pattern[matched] == pattern[index] {
            matched += 1;
        }
        prefix[index] = matched;
    }
    matched = 0;
    let mut count = 0;
    let mut first = None;
    for (index, value) in values.enumerate() {
        while matched > 0 && Some(pattern[matched]) != value {
            matched = prefix[matched - 1];
        }
        if Some(pattern[matched]) == value {
            matched += 1;
        }
        if matched == pattern.len() {
            count += 1;
            first.get_or_insert(index + 1 - matched);
            matched = prefix[matched - 1];
        }
    }
    (count, first)
}

#[cfg(test)]
mod tests {
    use super::occurrences;

    #[test]
    fn overlapping_repeated_matches_equal_naive_full_window_oracle() {
        for pattern_len in 1..=4 {
            for pattern_bits in 0_u32..(1 << pattern_len) {
                let pattern: Vec<_> = (0..pattern_len)
                    .map(|bit| (pattern_bits >> bit) & 1)
                    .collect();
                for source_len in 0..=8 {
                    for source_bits in 0_u32..(1 << source_len) {
                        let source: Vec<_> = (0..source_len)
                            .map(|bit| Some((source_bits >> bit) & 1))
                            .collect();
                        let expected: Vec<_> = source
                            .windows(pattern_len)
                            .enumerate()
                            .filter_map(|(index, window)| {
                                window
                                    .iter()
                                    .copied()
                                    .eq(pattern.iter().copied().map(Some))
                                    .then_some(index)
                            })
                            .collect();
                        assert_eq!(
                            occurrences(&pattern, source.into_iter()),
                            (expected.len(), expected.first().copied())
                        );
                    }
                }
            }
        }
        // A failed fact is a barrier, never a wildcard spanning two safe seams.
        assert_eq!(
            occurrences(&[1, 1], [Some(1), None, Some(1), Some(1)].into_iter()),
            (1, Some(2))
        );
    }
}
