/// Whether this explicit image-output choice uses sixteen-bit composition.
///
/// The caller separately gates overlay availability and Copy/source preview.
/// No source pixel-format probe is used: the selected frame may change format.
#[must_use]
pub fn high_depth_image_overlay(container: &str, pixel_format: &str, opacity: f64) -> bool {
    (container.eq_ignore_ascii_case("png") || container.eq_ignore_ascii_case("tiff"))
        && matches!(
            pixel_format.trim(),
            "rgb48le" | "rgb48be" | "rgba64le" | "rgba64be"
        )
        && opacity_thousandths(opacity).is_some_and(|value| value > 0)
}

pub(super) fn opacity_thousandths(opacity: f64) -> Option<u16> {
    let opacity = opacity.clamp(0.0, 1.0);
    if opacity == 0.0 {
        return Some(0);
    }
    // Preserve the existing formatted three-decimal opacity, including its
    // rounding, then use integer arithmetic for every alpha sample. Multiplying
    // the unformatted float by 1000 would introduce a different boundary.
    format!("{opacity:.3}")
        .replace('.', "")
        .parse::<u16>()
        .ok()
        .filter(|value| *value <= 1000)
}

pub(super) fn build_high_depth_overlay(
    base_chain: String,
    x: f64,
    y: f64,
    width: f64,
    opacity: u16,
    output: &str,
) -> String {
    let mut graph = vec![
        base_chain,
        "[base]format=rgba64le,format=gbrap16le,split=6[ov16_main][ov16_ref][ov16_canvas0][ov16_canvas1][ov16_canvas2][ov16_props]".into(),
        "[1:v:0]format=rgba64le,format=gbrap16le,extractplanes=r+g+b+a[ov16_pre_r][ov16_pre_g][ov16_pre_b][ov16_pre_a]".into(),
        "[ov16_ref]split=4[ov16_ref_r][ov16_ref_g][ov16_ref_b][ov16_ref_a]".into(),
    ];
    scale_planes(&mut graph, width, opacity);
    encode_bytes(&mut graph);
    place_bytes(&mut graph, x, y);
    decode_bytes(&mut graph);
    composite(&mut graph);
    graph.push(format!(
        "[ov16_props][ov16_pixels]blend=all_mode=normal:all_opacity=0,format=rgba64le[{output}]"
    ));
    graph.join(";")
}

fn scale_planes(graph: &mut Vec<String>, width: f64, opacity: u16) {
    // Four gray scalers preserve sixteen-bit constant samples that RGB swscale
    // can alter. All branches evaluate the same current-frame dimensions.
    let scale = format!(
        "scale=w='max(1,min(rw,min(floor(rw*{width:.6}),floor(rh*iw/ih))))':h='max(1,min(rh,round(ow*ih/iw)))'"
    );
    for channel in ['r', 'g', 'b', 'a'] {
        let input = if channel == 'a' {
            graph.push(format!(
                "[ov16_pre_a]geq=lum_expr='floor((lum(X,Y)*{opacity}+500)/1000)':interpolation=nearest[ov16_opacity_a]"
            ));
            "ov16_opacity_a".to_string()
        } else {
            format!("ov16_pre_{channel}")
        };
        graph.push(format!(
            "[{input}][ov16_ref_{channel}]{scale},format=gray16le[ov16_o{channel}]"
        ));
    }
}

fn encode_bytes(graph: &mut Vec<String>) {
    // overlay supports only lower-depth placement. Encode each 16-bit channel
    // as two opaque 8-bit bytes, place those bytes, then reconstruct exactly.
    for channel in ['r', 'g', 'b', 'a'] {
        graph.push(format!(
            "[ov16_o{channel}]split[ov16_hi{channel}][ov16_lo{channel}]"
        ));
        for (part, expression) in [
            ("hi", "floor(lum(X,Y)/256)*257"),
            ("lo", "mod(lum(X,Y),256)*257"),
        ] {
            graph.push(format!(
                "[ov16_{part}{channel}]format=gray16le,geq=lum_expr='{expression}':interpolation=nearest,format=gray16le,format=gray[ov16_{channel}{part}]"
            ));
        }
    }
    graph.extend([
        "[ov16_ahi]split[ov16_ah][ov16_zero]".into(),
        "[ov16_zero]lut=0[ov16_z]".into(),
        "[ov16_ghi][ov16_bhi][ov16_rhi]mergeplanes=map0s=0:map1s=1:map2s=2:format=gbrp,format=rgb24[ov16_byte0]".into(),
        "[ov16_glo][ov16_blo][ov16_rlo]mergeplanes=map0s=0:map1s=1:map2s=2:format=gbrp,format=rgb24[ov16_byte1]".into(),
        "[ov16_ah][ov16_alo][ov16_z]mergeplanes=map0s=0:map1s=1:map2s=2:format=gbrp,format=rgb24[ov16_byte2]".into(),
    ]);
}

fn place_bytes(graph: &mut Vec<String>, x: f64, y: f64) {
    for index in 0..3 {
        graph.push(format!(
            "[ov16_canvas{index}]format=rgb24,lutrgb=r=0:g=0:b=0[ov16_blank{index}]"
        ));
        graph.push(format!(
            "[ov16_blank{index}][ov16_byte{index}]overlay=x='min(max(main_w*{x:.6}-overlay_w/2,0),main_w-overlay_w)':y='min(max(main_h*{y:.6}-overlay_h/2,0),main_h-overlay_h)':format=rgb[ov16_placed{index}]"
        ));
    }
}

fn decode_bytes(graph: &mut Vec<String>) {
    graph.extend([
        "[ov16_placed0]extractplanes=r+g+b[ov16_prhi][ov16_pghi][ov16_pbhi]".into(),
        "[ov16_placed1]extractplanes=r+g+b[ov16_prlo][ov16_pglo][ov16_pblo]".into(),
        "[ov16_placed2]extractplanes=g+b[ov16_pahi][ov16_palo]".into(),
    ]);
    for channel in ['r', 'g', 'b', 'a'] {
        for part in ["hi", "lo"] {
            graph.push(format!(
                "[ov16_p{channel}{part}]format=gray16le[ov16_full{channel}{part}]"
            ));
        }
        graph.push(format!(
            "[ov16_full{channel}hi][ov16_full{channel}lo]blend=all_expr='round((A*256+B)/257)'[ov16_full{channel}]"
        ));
    }
    graph.push("[ov16_fullg][ov16_fullb][ov16_fullr][ov16_fulla]mergeplanes=map0s=0:map1s=1:map2s=2:map3s=3:format=gbrap16le[ov16_placed]".into());
}

fn composite(graph: &mut Vec<String>) {
    // These Y/U/V/A planes are temporary scalar registers, not a color-space
    // conversion: background color/alpha and source color/alpha, respectively.
    const COLOR: &str = "if(eq(alpha(X,Y),0),lum(X,Y),round((cr(X,Y)*alpha(X,Y)*65535+lum(X,Y)*cb(X,Y)*(65535-alpha(X,Y)))/(alpha(X,Y)*65535+cb(X,Y)*(65535-alpha(X,Y)))))";
    graph.extend([
        "[ov16_main]extractplanes=r+g+b+a[ov16_br][ov16_bg][ov16_bb][ov16_ba]".into(),
        "[ov16_ba]split=4[ov16_bar][ov16_bag][ov16_bab][ov16_bao]".into(),
        "[ov16_placed]extractplanes=r+g+b+a[ov16_sr][ov16_sg][ov16_sb][ov16_sa]".into(),
        "[ov16_sa]split=4[ov16_sar][ov16_sag][ov16_sab][ov16_sao]".into(),
    ]);
    for channel in ['r', 'g', 'b'] {
        graph.push(format!(
            "[ov16_b{channel}][ov16_ba{channel}][ov16_s{channel}][ov16_sa{channel}]mergeplanes=map0s=0:map1s=1:map2s=2:map3s=3:format=yuva444p16le,geq=lum_expr='{COLOR}':cb_expr=0:cr_expr=0:alpha_expr=0:interpolation=nearest,extractplanes=y[ov16_out{channel}]"
        ));
    }
    graph.extend([
        "[ov16_sao][ov16_bao]blend=all_expr='round(A+B*(65535-A)/65535)'[ov16_outa]".into(),
        "[ov16_outg][ov16_outb][ov16_outr][ov16_outa]mergeplanes=map0s=0:map1s=1:map2s=2:map3s=3:format=gbrap16le,format=rgba64le[ov16_pixels]".into(),
    ]);
}
