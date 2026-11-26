//! Procedural app emblem texture & lossless PNG channel codec for QN Bench.

#![allow(dead_code)]

use sha2::{Digest, Sha256};

const FRAME_MAGIC: [u8; 4] = *b"QNB1";

pub const LICENSE_RECORD: &str =
    "Copyright (c) 2026 Al Anderson | QN Bench | License: PolyForm-Noncommercial-1.0.0 (code) & CC-BY-NC-SA-4.0 (docs/media) | Educational & Non-Commercial Use Only | https://github.com/skcalanderson2/qn_bench";

/// Build a compact verified binary frame: `[MAGIC:4][LEN:2][SHA256_PREFIX:4][UTF8_PAYLOAD:LEN]`.
fn build_frame(payload: &str) -> Vec<u8> {
    let bytes = payload.as_bytes();
    let len = bytes.len().min(u16::MAX as usize);
    let digest = Sha256::digest(&bytes[..len]);
    let mut frame = Vec::with_capacity(10 + len);
    frame.extend_from_slice(&FRAME_MAGIC);
    frame.extend_from_slice(&(len as u16).to_be_bytes());
    frame.extend_from_slice(&digest[..4]);
    frame.extend_from_slice(&bytes[..len]);
    frame
}

/// Embed `payload` into the RGB least-significant bits of opaque pixels (`alpha == 255` for RGBA,
/// or all pixels for RGB). Returns `true` if capacity was sufficient.
pub fn encode_rgb_lsb(pixels: &mut [u8], channels: usize, payload: &str) -> bool {
    if channels != 3 && channels != 4 {
        return false;
    }
    let frame = build_frame(payload);
    let total_bits = frame.len() * 8;
    let mut bit_idx = 0usize;

    for px in pixels.chunks_exact_mut(channels) {
        if channels == 4 && px[3] != 255 {
            continue;
        }
        for ch in 0..3 {
            if bit_idx >= total_bits {
                return true;
            }
            let byte = frame[bit_idx / 8];
            let bit = (byte >> (7 - (bit_idx % 8))) & 1;
            px[ch] = (px[ch] & 0xFE) | bit;
            bit_idx += 1;
        }
        if bit_idx >= total_bits {
            return true;
        }
    }
    bit_idx >= total_bits
}

/// Extract and verify a previously encoded frame from opaque pixel RGB LSBs.
pub fn decode_rgb_lsb(pixels: &[u8], channels: usize) -> Option<String> {
    if channels != 3 && channels != 4 {
        return None;
    }
    let mut cur_byte = 0u8;
    let mut bit_in_byte = 0u8;
    let mut header = [0u8; 10];
    let mut header_pos = 0usize;
    let mut payload: Vec<u8> = Vec::new();
    let mut expected_len: Option<usize> = None;

    for px in pixels.chunks_exact(channels) {
        if channels == 4 && px[3] != 255 {
            continue;
        }
        for ch in 0..3 {
            cur_byte = (cur_byte << 1) | (px[ch] & 1);
            bit_in_byte += 1;
            if bit_in_byte == 8 {
                if header_pos < 10 {
                    header[header_pos] = cur_byte;
                    header_pos += 1;
                    if header_pos == 10 {
                        if header[0..4] != FRAME_MAGIC {
                            return None;
                        }
                        let len = u16::from_be_bytes([header[4], header[5]]) as usize;
                        if len == 0 || len > 4096 {
                            return None;
                        }
                        expected_len = Some(len);
                        payload.reserve(len);
                    }
                } else if let Some(len) = expected_len {
                    payload.push(cur_byte);
                    if payload.len() == len {
                        let digest = Sha256::digest(&payload);
                        if digest[..4] != header[6..10] {
                            return None;
                        }
                        return String::from_utf8(payload).ok();
                    }
                }
                cur_byte = 0;
                bit_in_byte = 0;
            }
        }
    }
    None
}

/// Decode a PNG byte buffer into `(width, height, color_type, raw_pixels)`.
pub fn decode_png_bytes(data: &[u8]) -> Result<(u32, u32, png::ColorType, Vec<u8>), String> {
    let decoder = png::Decoder::new(data);
    let mut reader = decoder.read_info().map_err(|e| format!("PNG read error: {e}"))?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("PNG frame error: {e}"))?;
    buf.truncate(info.buffer_size());
    Ok((info.width, info.height, info.color_type, buf))
}

/// Encode raw RGB or RGBA pixels into a lossless PNG byte buffer.
pub fn encode_png_bytes(w: u32, h: u32, color_type: png::ColorType, pixels: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w, h);
        enc.set_color(color_type);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc
            .write_header()
            .map_err(|e| format!("PNG header error: {e}"))?;
        writer
            .write_image_data(pixels)
            .map_err(|e| format!("PNG write error: {e}"))?;
    }
    Ok(out)
}

/// Apply `LICENSE_RECORD` to an existing RGB/RGBA PNG byte slice and re-encode losslessly.
pub fn encode_png_payload(data: &[u8], payload: &str) -> Result<Vec<u8>, String> {
    let (w, h, color_type, mut pixels) = decode_png_bytes(data)?;
    let channels = match color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        other => return Err(format!("Unsupported PNG color type: {other:?}")),
    };
    if !encode_rgb_lsb(&mut pixels, channels, payload) {
        return Err("Insufficient opaque pixel capacity in PNG.".to_string());
    }
    encode_png_bytes(w, h, color_type, &pixels)
}

/// Extract a verified payload from a PNG byte slice.
pub fn decode_png_payload(data: &[u8]) -> Option<String> {
    let (_, _, color_type, pixels) = decode_png_bytes(data).ok()?;
    let channels = match color_type {
        png::ColorType::Rgb => 3,
        png::ColorType::Rgba => 4,
        _ => return None,
    };
    decode_rgb_lsb(&pixels, channels)
}

/// Render a crisp 64x64 RGBA procedural QN Bench emblem carrying `LICENSE_RECORD`.
pub fn render_app_emblem_rgba() -> (usize, usize, Vec<u8>) {
    let size = 64usize;
    let mut px = vec![0u8; size * size * 4];
    let s = size as f32;
    let pad = 4.0f32;
    let x0 = pad;
    let y0 = pad;
    let x1 = s - pad;
    let y1 = s - pad;
    let rad = (x1 - x0) * 0.22;
    let cx = s * 0.5;
    let cy = s * 0.5;

    let sdf_rrect = |x: f32, y: f32, r0: f32, t0: f32, r1: f32, b1: f32, r: f32| -> f32 {
        let mcx = (r0 + r1) * 0.5;
        let mcy = (t0 + b1) * 0.5;
        let hx = (r1 - r0) * 0.5 - r;
        let hy = (b1 - t0) * 0.5 - r;
        let qx = (x - mcx).abs() - hx;
        let qy = (y - mcy).abs() - hy;
        qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
    };

    for y in 0..size {
        for x in 0..size {
            let xf = x as f32 + 0.5;
            let yf = y as f32 + 0.5;
            let d = sdf_rrect(xf, yf, x0, y0, x1, y1, rad);
            if d > 0.5 {
                continue;
            }
            let aa = (0.5 - d).clamp(0.0, 1.0);
            let mut r = 14.0f32;
            let mut g = 21.0f32;
            let mut b = 33.0f32;

            // Subtle optical breadboard grid lines
            if x % 12 == 0 || y % 12 == 0 {
                r += 8.0;
                g += 11.0;
                b += 16.0;
            }

            // Ambient 780nm red -> PPLN purple -> 1550nm cyan glow
            let d_red = (xf - 16.0).hypot(yf - cy);
            let d_pur = (xf - cx).hypot(yf - cy);
            let d_cya = (xf - 48.0).hypot(yf - cy);
            if d_red < 16.0 {
                let k = (1.0 - d_red / 16.0).powi(2) * 0.45;
                r += (244.0 - r) * k;
                g += (63.0 - g) * k;
                b += (94.0 - b) * k;
            }
            if d_pur < 18.0 {
                let k = (1.0 - d_pur / 18.0).powi(2) * 0.52;
                r += (167.0 - r) * k;
                g += (139.0 - g) * k;
                b += (250.0 - b) * k;
            }
            if d_cya < 16.0 {
                let k = (1.0 - d_cya / 16.0).powi(2) * 0.52;
                r += (56.0 - r) * k;
                g += (189.0 - g) * k;
                b += (248.0 - b) * k;
            }

            // Incoming 780nm wave (left) and outgoing 1550nm wave (right)
            if xf >= 9.0 && xf <= cx - 7.0 {
                let wy = cy + 3.2 * ((xf - 9.0) * 0.65).sin();
                if (yf - wy).abs() < 1.6 {
                    r = 244.0;
                    g = 72.0;
                    b = 104.0;
                }
            } else if xf >= cx + 7.0 && xf <= s - 9.0 {
                let wy = cy + 3.8 * ((xf - cx) * 0.34).sin();
                if (yf - wy).abs() < 1.7 {
                    r = 56.0;
                    g = 196.0;
                    b = 248.0;
                }
            }

            // Central PPLN crystal prism
            let dp = sdf_rrect(xf, yf, cx - 7.0, cy - 11.0, cx + 7.0, cy + 11.0, 2.5);
            if dp <= 0.0 {
                r = 46.0;
                g = 34.0;
                b = 88.0;
                if ((xf - (cx - 5.0)).round() as i32) % 3 == 0 {
                    r = 167.0;
                    g = 139.0;
                    b = 250.0;
                }
            } else if dp <= 1.2 {
                r = 216.0;
                g = 200.0;
                b = 255.0;
            }

            // Outer cyan rim
            if d.abs() <= 1.2 {
                r = 56.0;
                g = 189.0;
                b = 248.0;
            }

            let idx = (y * size + x) * 4;
            px[idx] = r.round().clamp(0.0, 255.0) as u8;
            px[idx + 1] = g.round().clamp(0.0, 255.0) as u8;
            px[idx + 2] = b.round().clamp(0.0, 255.0) as u8;
            px[idx + 3] = if d <= -0.5 { 255 } else { (aa * 255.0).round() as u8 };
        }
    }

    let ok = encode_rgb_lsb(&mut px, 4, LICENSE_RECORD);
    debug_assert!(ok, "emblem capacity must hold LICENSE_RECORD");
    (size, size, px)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emblem_and_png_record_round_trip() {
        let (w, h, rgba) = render_app_emblem_rgba();
        assert_eq!((w, h), (64, 64));
        let extracted = decode_rgb_lsb(&rgba, 4).expect("decode from emblem rgba");
        assert_eq!(extracted, LICENSE_RECORD);

        let png_bytes = encode_png_bytes(w as u32, h as u32, png::ColorType::Rgba, &rgba)
            .expect("encode png");
        let extracted_png = decode_png_payload(&png_bytes).expect("decode from png");
        assert_eq!(extracted_png, LICENSE_RECORD);

        // Verify RGB (3-channel) PNG round-trip as well
        let rgb = vec![42u8; 64 * 64 * 3];
        let raw_rgb_png =
            encode_png_bytes(64, 64, png::ColorType::Rgb, &rgb).expect("encode raw rgb png");
        let encoded_rgb_png =
            encode_png_payload(&raw_rgb_png, LICENSE_RECORD).expect("encode rgb payload");
        assert_eq!(
            decode_png_payload(&encoded_rgb_png).as_deref(),
            Some(LICENSE_RECORD)
        );
    }
}
