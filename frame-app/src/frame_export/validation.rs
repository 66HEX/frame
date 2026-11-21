use frame_core::error::ConversionError;

pub(super) fn validate_png(bytes: &[u8], decoded_limit: u64) -> Result<(), ConversionError> {
    let mut input = std::io::Cursor::new(bytes);
    let mut decoder = png::Decoder::new_with_limits(
        &mut input,
        png::Limits {
            bytes: usize::try_from(decoded_limit).unwrap_or(usize::MAX),
        },
    );
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info().map_err(|error| invalid_png(&error))?;
    if reader.info().animation_control.is_some() {
        return Err(ConversionError::Worker(
            "Source frame output must contain one still PNG".into(),
        ));
    }
    let (color, depth) = reader.output_color_type();
    let sample_bytes = if depth == png::BitDepth::Sixteen {
        2
    } else {
        1
    };
    let pixel_bytes = u64::from(reader.info().width)
        .checked_mul(u64::from(reader.info().height))
        .and_then(|pixels| pixels.checked_mul(u64::try_from(color.samples()).ok()?))
        .and_then(|samples| samples.checked_mul(sample_bytes))
        .filter(|bytes| *bytes != 0 && *bytes <= decoded_limit)
        .ok_or_else(|| {
            ConversionError::Worker(format!(
                "Source frame exceeds the decoded image limit of {decoded_limit} bytes"
            ))
        })?;
    let size = usize::try_from(pixel_bytes).map_err(|_| {
        ConversionError::Worker("Source frame is too large for this platform".into())
    })?;
    if reader.output_buffer_size() != Some(size) {
        return Err(ConversionError::Worker(
            "Source frame has invalid PNG dimensions".into(),
        ));
    }
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(size).map_err(|error| {
        ConversionError::Worker(format!(
            "Could not allocate source frame validation buffer: {error}"
        ))
    })?;
    pixels.resize(size, 0);
    reader
        .next_frame(&mut pixels)
        .map_err(|error| invalid_png(&error))?;
    drop(pixels);
    reader.finish().map_err(|error| invalid_png(&error))?;
    drop(reader);
    if input.position() != u64::try_from(bytes.len()).unwrap_or(u64::MAX) {
        return Err(ConversionError::Worker(
            "Source frame PNG has unexpected trailing data".into(),
        ));
    }
    Ok(())
}

fn invalid_png(error: &png::DecodingError) -> ConversionError {
    ConversionError::Worker(format!(
        "Source frame did not produce a complete PNG within the image limits: {error}"
    ))
}
