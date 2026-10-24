//! WebAssembly plugin of the `runaround` Typst package.
//!
//! Computes the contour of raster images: for each horizontal band of the
//! image, the leftmost and rightmost pixel that is opaque enough. The package
//! uses these extents to let text follow the shape of cut-out images.
//!
//! The plugin is pure: it only transforms the bytes it is given.

use std::io::Cursor;

/// An image reduced to its alpha channel.
pub struct Alpha {
    pub width: usize,
    pub height: usize,
    /// One byte per pixel, row-major.
    pub data: Vec<u8>,
}

/// Largest image accepted, in pixels (e.g. 8192 × 4096). Checked against the
/// header before any buffer is allocated, so a crafted header cannot exhaust
/// the plugin's memory.
pub const MAX_PIXELS: usize = 1 << 25;

/// Validates image dimensions from a header.
fn check_size(width: u32, height: u32) -> Result<(usize, usize), String> {
    let (w, h) = (width as usize, height as usize);
    match w.checked_mul(h) {
        Some(0) => Err("empty image".into()),
        Some(n) if n <= MAX_PIXELS => Ok((w, h)),
        _ => Err(format!("image too large ({width}×{height} pixels)")),
    }
}

/// Decodes PNG, GIF or WebP data into its alpha channel. Images without an
/// alpha channel are fully opaque.
pub fn decode(bytes: &[u8]) -> Result<Alpha, String> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        decode_png(bytes)
    } else if bytes.starts_with(b"GIF8") {
        decode_gif(bytes)
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        decode_webp(bytes)
    } else {
        Err("unsupported image format (expected PNG, GIF or WebP)".into())
    }
}

fn decode_png(bytes: &[u8]) -> Result<Alpha, String> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|e| format!("PNG: {e}"))?;
    check_size(reader.info().width, reader.info().height)?;
    let size = reader.output_buffer_size().ok_or("PNG: image too large")?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| format!("PNG: {e}"))?;
    let (width, height) = (info.width as usize, info.height as usize);
    let channels = info.color_type.samples();
    let has_alpha =
        matches!(info.color_type, png::ColorType::GrayscaleAlpha | png::ColorType::Rgba);
    let data = (0..height)
        .flat_map(|y| {
            let row = &buf[y * info.line_size..];
            (0..width).map(move |x| if has_alpha { row[x * channels + channels - 1] } else { 255 })
        })
        .collect();
    Ok(Alpha { width, height, data })
}

fn decode_gif(bytes: &[u8]) -> Result<Alpha, String> {
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = options.read_info(Cursor::new(bytes)).map_err(|e| format!("GIF: {e}"))?;
    let (width, height) = check_size(decoder.width().into(), decoder.height().into())?;
    let mut data = vec![0; width * height];
    if let Some(frame) = decoder.read_next_frame().map_err(|e| format!("GIF: {e}"))? {
        for fy in 0..frame.height as usize {
            for fx in 0..frame.width as usize {
                let (x, y) = (frame.left as usize + fx, frame.top as usize + fy);
                if x < width && y < height {
                    data[y * width + x] = frame.buffer[(fy * frame.width as usize + fx) * 4 + 3];
                }
            }
        }
    }
    Ok(Alpha { width, height, data })
}

fn decode_webp(bytes: &[u8]) -> Result<Alpha, String> {
    let mut decoder =
        image_webp::WebPDecoder::new(Cursor::new(bytes)).map_err(|e| format!("WebP: {e}"))?;
    let (width, height) = check_size(decoder.dimensions().0, decoder.dimensions().1)?;
    let size = decoder.output_buffer_size().ok_or("WebP: image too large")?;
    let mut buf = vec![0; size];
    decoder.read_image(&mut buf).map_err(|e| format!("WebP: {e}"))?;
    let data = if decoder.has_alpha() {
        buf.chunks_exact(4).map(|px| px[3]).collect()
    } else {
        vec![255; width * height]
    };
    Ok(Alpha { width, height, data })
}

/// For each of `rows` horizontal bands, the extent `(left, right)` of pixels
/// whose alpha exceeds `threshold`, as fractions of the width; `None` for
/// bands without such pixels.
pub fn extents(alpha: &Alpha, rows: usize, threshold: u8) -> Vec<Option<(f64, f64)>> {
    let (w, h) = (alpha.width, alpha.height);
    let mut out = Vec::with_capacity(rows);
    for r in 0..rows {
        let y0 = r * h / rows;
        let y1 = ((r + 1) * h / rows).max(y0 + 1).min(h);
        let mut left = usize::MAX;
        let mut right = 0;
        for y in y0..y1 {
            let row = &alpha.data[y * w..(y + 1) * w];
            if let Some(l) = row.iter().position(|&a| a > threshold) {
                let r = row.iter().rposition(|&a| a > threshold).unwrap();
                left = left.min(l);
                right = right.max(r + 1);
            }
        }
        out.push((left != usize::MAX).then(|| (left as f64 / w as f64, right as f64 / w as f64)));
    }
    out
}

/// Encodes the result as JSON: `{"width": .., "height": .., "rows": [[l, r] | null, ..]}`.
pub fn to_json(alpha: &Alpha, rows: &[Option<(f64, f64)>]) -> String {
    let mut s = format!("{{\"width\":{},\"height\":{},\"rows\":[", alpha.width, alpha.height);
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        match row {
            Some((l, r)) => s += &format!("[{l:.5},{r:.5}]"),
            None => s += "null",
        }
    }
    s += "]}";
    s
}

fn parse<T: std::str::FromStr>(bytes: &[u8], what: &str) -> Result<T, String> {
    std::str::from_utf8(bytes)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .ok_or_else(|| format!("invalid {what}"))
}

/// Contour of an image: see [`extents`] and [`to_json`]. Arguments are the
/// image bytes, the number of bands and the alpha threshold (0–255), the
/// latter two as decimal strings.
pub fn contour(image: &[u8], rows: &[u8], threshold: &[u8]) -> Result<Vec<u8>, String> {
    let rows: usize = parse(rows, "row count")?;
    let threshold: u8 = parse(threshold, "threshold")?;
    if rows == 0 || rows > 4096 {
        return Err("row count must be between 1 and 4096".into());
    }
    let alpha = decode(image)?;
    if alpha.width == 0 || alpha.height == 0 {
        return Err("empty image".into());
    }
    Ok(to_json(&alpha, &extents(&alpha, rows, threshold)).into_bytes())
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use wasm_minimal_protocol::*;

    initiate_protocol!();

    #[wasm_func]
    pub fn contour(image: &[u8], rows: &[u8], threshold: &[u8]) -> Result<Vec<u8>, String> {
        super::contour(image, rows, threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header().unwrap().write_image_data(rgba).unwrap();
        out
    }

    /// A disc of radius `r` centred in a `size`×`size` image.
    fn disc(size: u32, r: f64) -> Vec<u8> {
        let c = size as f64 / 2.0;
        (0..size * size)
            .flat_map(|i| {
                let (x, y) = ((i % size) as f64 + 0.5, (i / size) as f64 + 0.5);
                let inside = (x - c).powi(2) + (y - c).powi(2) <= r * r;
                [200, 0, 0, if inside { 255 } else { 0 }]
            })
            .collect()
    }

    #[test]
    fn disc_contour() {
        let alpha = decode(&png(100, 100, &disc(100, 40.0))).unwrap();
        let rows = extents(&alpha, 10, 127);
        assert_eq!(rows.len(), 10);
        // Bands above and below the disc are empty only if the disc misses
        // them entirely; here the disc spans 10..90, so all bands have pixels.
        let (l, r) = rows[5].unwrap();
        assert!((l - 0.1).abs() < 0.02 && (r - 0.9).abs() < 0.02, "{l} {r}");
        let (l0, r0) = rows[1].unwrap();
        assert!(l0 > l && r0 < r, "the disc is narrower near the top");
        // Symmetric.
        let (lt, rt) = rows[2].unwrap();
        let (lb, rb) = rows[7].unwrap();
        assert!((lt - lb).abs() < 0.02 && (rt - rb).abs() < 0.02);
    }

    #[test]
    fn transparent_rows_are_empty() {
        let mut data = vec![0u8; 4 * 4 * 4];
        // Only pixel (1, 2) is opaque.
        data[(2 * 4 + 1) * 4 + 3] = 255;
        let alpha = decode(&png(4, 4, &data)).unwrap();
        let rows = extents(&alpha, 4, 127);
        assert_eq!(rows, vec![None, None, Some((0.25, 0.5)), None]);
    }

    #[test]
    fn images_without_alpha_are_opaque() {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, 2, 2);
        encoder.set_color(png::ColorType::Rgb);
        encoder.write_header().unwrap().write_image_data(&[0; 12]).unwrap();
        let alpha = decode(&out).unwrap();
        assert_eq!(extents(&alpha, 2, 127), vec![Some((0.0, 1.0)), Some((0.0, 1.0))]);
    }

    #[test]
    fn more_rows_than_pixels() {
        let alpha = decode(&png(10, 10, &disc(10, 4.0))).unwrap();
        assert_eq!(extents(&alpha, 50, 127).len(), 50);
    }

    #[test]
    fn json_output() {
        let alpha = Alpha { width: 2, height: 2, data: vec![0, 255, 0, 0] };
        let json = to_json(&alpha, &extents(&alpha, 2, 127));
        assert_eq!(json, "{\"width\":2,\"height\":2,\"rows\":[[0.50000,1.00000],null]}");
    }

    #[test]
    fn errors() {
        assert!(contour(b"nope", b"10", b"127").is_err());
        assert!(contour(&png(1, 1, &[0, 0, 0, 255]), b"0", b"127").is_err());
        assert!(contour(&png(1, 1, &[0, 0, 0, 255]), b"x", b"127").is_err());
        assert!(contour(&png(1, 1, &[0, 0, 0, 255]), b"3", b"127").is_ok());
    }

    #[test]
    fn oversized_headers_are_rejected() {
        // A GIF whose screen claims 65535×65535 pixels, with one 1×1 frame.
        let mut gif = b"GIF89a".to_vec();
        gif.extend([0xff, 0xff, 0xff, 0xff, 0, 0, 0]);
        gif.extend([0x2c, 0, 0, 0, 0, 1, 0, 1, 0, 0, 2, 2, 0x44, 0x01, 0, 0x3b]);
        let err = decode(&gif).err().unwrap_or_default();
        assert!(err.contains("too large"), "{err}");
        assert!(check_size(8192, 4096).is_ok());
        assert!(check_size(8192, 4097).is_err());
        assert!(check_size(0, 10).is_err());
    }
}
