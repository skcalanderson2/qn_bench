//! Pure-Rust procedural macOS .icns and Windows .ico icon generator for QN Bench.
//! Zero external image assets: renders a 1024x1024 RGBA optical-bench icon directly.

#[path = "../emblem.rs"]
mod emblem;

use std::f32::consts::PI;
use std::fs;
use std::path::Path;

struct Canvas {
    w: usize,
    h: usize,
    px: Vec<u8>, // RGBA8
}

impl Canvas {
    fn new(w: usize, h: usize) -> Self {
        Self { w, h, px: vec![0; w * h * 4] }
    }

    fn blend(&mut self, x: i32, y: i32, r: u8, g: u8, b: u8, a: u8) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 || a == 0 {
            return;
        }
        let i = (y as usize * self.w + x as usize) * 4;
        let sa = a as f32 / 255.0;
        let da = self.px[i + 3] as f32 / 255.0;
        let out_a = sa + da * (1.0 - sa);
        if out_a <= 0.0 {
            return;
        }
        let blend_c = |sc: u8, dc: u8| -> u8 {
            (((sc as f32 * sa) + (dc as f32 * da * (1.0 - sa))) / out_a)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        self.px[i] = blend_c(r, self.px[i]);
        self.px[i + 1] = blend_c(g, self.px[i + 1]);
        self.px[i + 2] = blend_c(b, self.px[i + 2]);
        self.px[i + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
    }

    fn fill_circle(&mut self, cx: f32, cy: f32, rad: f32, c: (u8, u8, u8, u8)) {
        let min_x = (cx - rad - 1.0).floor() as i32;
        let max_x = (cx + rad + 1.0).ceil() as i32;
        let min_y = (cy - rad - 1.0).floor() as i32;
        let max_y = (cy + rad + 1.0).ceil() as i32;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                if d <= rad + 0.5 {
                    let aa = (rad + 0.5 - d).clamp(0.0, 1.0);
                    self.blend(x, y, c.0, c.1, c.2, (c.3 as f32 * aa) as u8);
                }
            }
        }
    }

    fn draw_glow(&mut self, cx: f32, cy: f32, rad: f32, rgb: (u8, u8, u8), max_a: f32) {
        let min_x = (cx - rad).floor().max(0.0) as i32;
        let max_x = (cx + rad).ceil().min(self.w as f32 - 1.0) as i32;
        let min_y = (cy - rad).floor().max(0.0) as i32;
        let max_y = (cy + rad).ceil().min(self.h as f32 - 1.0) as i32;
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                if d < rad {
                    let t = 1.0 - d / rad;
                    let a = (max_a * t.powf(2.2)) as u8;
                    self.blend(x, y, rgb.0, rgb.1, rgb.2, a);
                }
            }
        }
    }

    fn sdf_rounded_rect(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> f32 {
        let cx = (x0 + x1) * 0.5;
        let cy = (y0 + y1) * 0.5;
        let hx = (x1 - x0) * 0.5 - r;
        let hy = (y1 - y0) * 0.5 - r;
        let qx = (px - cx).abs() - hx;
        let qy = (py - cy).abs() - hy;
        qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
    }

    fn fill_rounded_rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, r: f32, c: (u8, u8, u8, u8)) {
        for y in (y0.floor() as i32 - 1)..=(y1.ceil() as i32 + 1) {
            for x in (x0.floor() as i32 - 1)..=(x1.ceil() as i32 + 1) {
                let d = Self::sdf_rounded_rect(x as f32, y as f32, x0, y0, x1, y1, r);
                if d <= 0.5 {
                    let aa = (0.5 - d).clamp(0.0, 1.0);
                    self.blend(x, y, c.0, c.1, c.2, (c.3 as f32 * aa) as u8);
                }
            }
        }
    }

    fn stroke_rounded_rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, r: f32, thick: f32, c: (u8, u8, u8, u8)) {
        let half = thick * 0.5;
        for y in (y0.floor() as i32 - thick as i32)..=(y1.ceil() as i32 + thick as i32) {
            for x in (x0.floor() as i32 - thick as i32)..=(x1.ceil() as i32 + thick as i32) {
                let d = Self::sdf_rounded_rect(x as f32, y as f32, x0, y0, x1, y1, r).abs();
                if d <= half + 0.5 {
                    let aa = (half + 0.5 - d).clamp(0.0, 1.0);
                    self.blend(x, y, c.0, c.1, c.2, (c.3 as f32 * aa) as u8);
                }
            }
        }
    }

    fn draw_capsule(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, rad: f32, c: (u8, u8, u8, u8)) {
        let min_x = (x0.min(x1) - rad - 1.0).floor() as i32;
        let max_x = (x0.max(x1) + rad + 1.0).ceil() as i32;
        let min_y = (y0.min(y1) - rad - 1.0).floor() as i32;
        let max_y = (y0.max(y1) + rad + 1.0).ceil() as i32;
        let dx = x1 - x0;
        let dy = y1 - y0;
        let len2 = (dx * dx + dy * dy).max(1e-6);
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let t = (((x as f32 - x0) * dx + (y as f32 - y0) * dy) / len2).clamp(0.0, 1.0);
                let d = (x as f32 - (x0 + t * dx)).hypot(y as f32 - (y0 + t * dy));
                if d <= rad + 0.5 {
                    let aa = (rad + 0.5 - d).clamp(0.0, 1.0);
                    self.blend(x, y, c.0, c.1, c.2, (c.3 as f32 * aa) as u8);
                }
            }
        }
    }

    fn downscale(&self, target: usize) -> Canvas {
        if target == self.w && target == self.h {
            return Canvas { w: self.w, h: self.h, px: self.px.clone() };
        }
        let mut out = Canvas::new(target, target);
        let scale = self.w as f32 / target as f32;
        for ty in 0..target {
            let sy0 = (ty as f32 * scale).floor() as usize;
            let sy1 = (((ty + 1) as f32 * scale).ceil() as usize).min(self.h);
            for tx in 0..target {
                let sx0 = (tx as f32 * scale).floor() as usize;
                let sx1 = (((tx + 1) as f32 * scale).ceil() as usize).min(self.w);
                let mut acc = [0.0f32; 4];
                let mut n = 0.0f32;
                for sy in sy0..sy1 {
                    for sx in sx0..sx1 {
                        let i = (sy * self.w + sx) * 4;
                        let a = self.px[i + 3] as f32 / 255.0;
                        acc[0] += self.px[i] as f32 * a;
                        acc[1] += self.px[i + 1] as f32 * a;
                        acc[2] += self.px[i + 2] as f32 * a;
                        acc[3] += a;
                        n += 1.0;
                    }
                }
                let oi = (ty * target + tx) * 4;
                if n > 0.0 && acc[3] > 0.0 {
                    out.px[oi] = (acc[0] / acc[3]).round().clamp(0.0, 255.0) as u8;
                    out.px[oi + 1] = (acc[1] / acc[3]).round().clamp(0.0, 255.0) as u8;
                    out.px[oi + 2] = (acc[2] / acc[3]).round().clamp(0.0, 255.0) as u8;
                    out.px[oi + 3] = ((acc[3] / n) * 255.0).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
        out
    }

    fn encode_png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, self.w as u32, self.h as u32);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut writer = enc.write_header().expect("png header");
            writer.write_image_data(&self.px).expect("png data");
        }
        out
    }
}

fn draw_vector_glyph(c: &mut Canvas, ch: char, x: f32, y: f32, w: f32, h: f32, r: f32, col: (u8, u8, u8, u8)) {
    let segs: &[((f32, f32), (f32, f32))] = match ch {
        'Q' => &[
            ((0.2, 0.0), (0.8, 0.0)), ((0.8, 0.0), (1.0, 0.2)), ((1.0, 0.2), (1.0, 0.78)),
            ((1.0, 0.78), (0.78, 1.0)), ((0.78, 1.0), (0.2, 1.0)), ((0.2, 1.0), (0.0, 0.78)),
            ((0.0, 0.78), (0.0, 0.2)), ((0.0, 0.2), (0.2, 0.0)), ((0.58, 0.66), (1.02, 1.08)),
        ],
        'N' => &[((0.0, 1.0), (0.0, 0.0)), ((0.0, 0.0), (1.0, 1.0)), ((1.0, 1.0), (1.0, 0.0))],
        'B' => &[
            ((0.0, 0.0), (0.0, 1.0)), ((0.0, 0.0), (0.76, 0.0)), ((0.76, 0.0), (0.95, 0.22)),
            ((0.95, 0.22), (0.76, 0.48)), ((0.0, 0.48), (0.82, 0.48)), ((0.82, 0.48), (1.0, 0.74)),
            ((1.0, 0.74), (0.80, 1.0)), ((0.0, 1.0), (0.80, 1.0)),
        ],
        'E' => &[((0.0, 0.0), (0.0, 1.0)), ((0.0, 0.0), (0.95, 0.0)), ((0.0, 0.5), (0.78, 0.5)), ((0.0, 1.0), (0.95, 1.0))],
        'C' => &[
            ((0.95, 0.16), (0.75, 0.0)), ((0.75, 0.0), (0.22, 0.0)), ((0.22, 0.0), (0.0, 0.22)),
            ((0.0, 0.22), (0.0, 0.78)), ((0.0, 0.78), (0.22, 1.0)), ((0.22, 1.0), (0.75, 1.0)),
            ((0.75, 1.0), (0.95, 0.84)),
        ],
        'H' => &[((0.0, 0.0), (0.0, 1.0)), ((1.0, 0.0), (1.0, 1.0)), ((0.0, 0.5), (1.0, 0.5))],
        _ => &[],
    };
    for &((ax, ay), (bx, by)) in segs {
        c.draw_capsule(x + ax * w, y + ay * h, x + bx * w, y + by * h, r, col);
    }
}

fn render_master_icon() -> Canvas {
    let size = 1024usize;
    let mut card = Canvas::new(size, size);
    let pad = 88.0f32;
    let x0 = pad;
    let y0 = pad;
    let x1 = size as f32 - pad;
    let y1 = size as f32 - pad;
    let radius = (x1 - x0) * 0.225;

    card.fill_rounded_rect(x0, y0, x1, y1, radius, (13, 19, 30, 255));

    // Breadboard grid + tapped holes
    let step = 76;
    for gx in ((x0 as i32 + 56)..(x1 as i32 - 40)).step_by(step) {
        card.draw_capsule(gx as f32, y0 + 28.0, gx as f32, y1 - 28.0, 1.2, (24, 34, 50, 150));
    }
    for gy in ((y0 as i32 + 56)..(y1 as i32 - 40)).step_by(step) {
        card.draw_capsule(x0 + 28.0, gy as f32, x1 - 28.0, gy as f32, 1.2, (24, 34, 50, 150));
    }
    for gx in ((x0 as i32 + 94)..(x1 as i32 - 60)).step_by(step) {
        for gy in ((y0 as i32 + 94)..(y1 as i32 - 60)).step_by(step) {
            card.fill_circle(gx as f32, gy as f32, 5.0, (8, 12, 20, 220));
        }
    }

    let cx = 512.0f32;
    let cy = 420.0f32;

    // Ambient optical glows (780nm red -> PPLN purple -> 1550nm cyan)
    card.draw_glow(x0 + 170.0, cy, 210.0, (244, 63, 94), 120.0);
    card.draw_glow(cx, cy, 250.0, (167, 139, 250), 145.0);
    card.draw_glow(x1 - 170.0, cy, 230.0, (56, 189, 248), 150.0);

    // Incoming 780nm wave (left)
    let mut prev = None;
    for xi in ((x0 as i32 + 54)..(cx as i32 - 84)).step_by(4) {
        let xf = xi as f32;
        let yf = cy + 28.0 * ((xf - x0) * 0.062).sin();
        if let Some((px, py)) = prev {
            card.draw_capsule(px, py, xf, yf, 8.0, (244, 63, 94, 255));
            card.draw_capsule(px, py, xf, yf, 3.0, (255, 205, 216, 255));
        }
        prev = Some((xf, yf));
    }

    // Outgoing 1550nm C-band telecom wave (right)
    prev = None;
    for xi in ((cx as i32 + 84)..(x1 as i32 - 54)).step_by(4) {
        let xf = xi as f32;
        let yf = cy + 34.0 * ((xf - cx) * 0.031 + PI * 0.25).sin();
        if let Some((px, py)) = prev {
            card.draw_capsule(px, py, xf, yf, 9.0, (56, 189, 248, 255));
            card.draw_capsule(px, py, xf, yf, 3.5, (215, 246, 255, 255));
        }
        prev = Some((xf, yf));
    }

    // Central PPLN crystal prism
    card.fill_rounded_rect(cx - 88.0, cy - 116.0, cx + 88.0, cy + 116.0, 26.0, (38, 28, 72, 255));
    for px in ((cx as i32 - 60)..=(cx as i32 + 60)).step_by(24) {
        card.draw_capsule(px as f32, cy - 94.0, px as f32, cy + 94.0, 3.5, (167, 139, 250, 255));
    }
    card.stroke_rounded_rect(cx - 88.0, cy - 116.0, cx + 88.0, cy + 116.0, 26.0, 7.0, (216, 200, 255, 255));

    // Crisp vector "QN BENCH" wordmark at bottom
    let letters = ['Q', 'N', ' ', 'B', 'E', 'N', 'C', 'H'];
    let gw = 66.0f32;
    let gh = 92.0f32;
    let gap = 22.0f32;
    let total_w = 7.0 * gw + 0.55 * gw + 7.0 * gap;
    let mut lx = cx - total_w * 0.5;
    let ly = y1 - 210.0;
    for (idx, &ch) in letters.iter().enumerate() {
        if ch == ' ' {
            lx += gw * 0.55 + gap;
            continue;
        }
        let col = if idx < 2 { (56, 189, 248, 255) } else { (238, 244, 252, 255) };
        draw_vector_glyph(&mut card, ch, lx, ly, gw, gh, 7.5, col);
        lx += gw + gap;
    }

    // Mask cleanly to squircle and draw rim
    for y in 0..size {
        for x in 0..size {
            let d = Canvas::sdf_rounded_rect(x as f32, y as f32, x0, y0, x1, y1, radius);
            if d > 0.5 {
                let i = (y * size + x) * 4;
                card.px[i + 3] = 0;
            } else if d > -0.5 {
                let aa = (0.5 - d).clamp(0.0, 1.0);
                let i = (y * size + x) * 4;
                card.px[i + 3] = (card.px[i + 3] as f32 * aa) as u8;
            }
        }
    }
    card.stroke_rounded_rect(x0, y0, x1, y1, radius, 9.0, (56, 189, 248, 215));
    card
}

fn render_scaled_png(master: &Canvas, px: usize) -> Vec<u8> {
    let mut scaled = master.downscale(px);
    if px >= 64 {
        let _ = emblem::encode_rgb_lsb(&mut scaled.px, 4, emblem::LICENSE_RECORD);
    }
    scaled.encode_png()
}

fn write_icns(master: &Canvas, path: &Path) {
    let types: &[(&[u8; 4], usize)] = &[
        (b"icp4", 16),
        (b"icp5", 32),
        (b"icp6", 64),
        (b"ic07", 128),
        (b"ic08", 256),
        (b"ic09", 512),
        (b"ic10", 1024),
        (b"ic11", 32),
        (b"ic12", 64),
        (b"ic13", 256),
        (b"ic14", 512),
    ];
    let mut cache = std::collections::HashMap::new();
    let mut body = Vec::new();
    for &(tag, px) in types {
        let data = cache
            .entry(px)
            .or_insert_with(|| render_scaled_png(master, px));
        body.extend_from_slice(tag);
        body.extend_from_slice(&((8 + data.len()) as u32).to_be_bytes());
        body.extend_from_slice(data);
    }
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&((8 + body.len()) as u32).to_be_bytes());
    out.extend_from_slice(&body);
    fs::write(path, &out).expect("write icns");
    println!("Generated {} ({} bytes)", path.display(), out.len());
}

fn write_ico(master: &Canvas, path: &Path) {
    let sizes = [16usize, 24, 32, 48, 64, 128, 256];
    let pngs: Vec<(usize, Vec<u8>)> = sizes
        .iter()
        .map(|&px| (px, render_scaled_png(master, px)))
        .collect();
    let count = pngs.len() as u16;
    let mut out = Vec::new();
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&count.to_le_bytes());
    let mut offset = (6 + 16 * pngs.len()) as u32;
    for (px, data) in &pngs {
        let dim = if *px >= 256 { 0u8 } else { *px as u8 };
        out.push(dim);
        out.push(dim);
        out.push(0);
        out.push(0);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += data.len() as u32;
    }
    for (_, data) in &pngs {
        out.extend_from_slice(data);
    }
    fs::write(path, &out).expect("write ico");
    println!("Generated {} ({} bytes)", path.display(), out.len());
}

fn inspect_file(path: &Path) -> bool {
    let Ok(bytes) = fs::read(path) else {
        eprintln!("Could not read {}", path.display());
        return false;
    };
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        if let Some(rec) = emblem::decode_png_payload(&bytes) {
            println!("OK  {} => {}", path.display(), rec);
            return true;
        }
    } else if bytes.starts_with(b"icns") && bytes.len() >= 8 {
        let mut pos = 8usize;
        while pos + 8 <= bytes.len() {
            let chunk_len = u32::from_be_bytes([bytes[pos + 4], bytes[pos + 5], bytes[pos + 6], bytes[pos + 7]]) as usize;
            if chunk_len < 8 || pos + chunk_len > bytes.len() {
                break;
            }
            let chunk = &bytes[pos + 8..pos + chunk_len];
            if chunk.starts_with(b"\x89PNG\r\n\x1a\n") {
                if let Some(rec) = emblem::decode_png_payload(chunk) {
                    let tag = String::from_utf8_lossy(&bytes[pos..pos + 4]);
                    println!("OK  {} [{}] => {}", path.display(), tag, rec);
                    return true;
                }
            }
            pos += chunk_len;
        }
    } else if bytes.len() >= 6 && bytes[0..4] == [0, 0, 1, 0] {
        let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        for i in 0..count {
            let e = 6 + i * 16;
            if e + 16 > bytes.len() {
                break;
            }
            let sz = u32::from_le_bytes([bytes[e + 8], bytes[e + 9], bytes[e + 10], bytes[e + 11]]) as usize;
            let off = u32::from_le_bytes([bytes[e + 12], bytes[e + 13], bytes[e + 14], bytes[e + 15]]) as usize;
            if off + sz <= bytes.len() && bytes[off..off + sz].starts_with(b"\x89PNG\r\n\x1a\n") {
                if let Some(rec) = emblem::decode_png_payload(&bytes[off..off + sz]) {
                    println!("OK  {} [frame {}] => {}", path.display(), i, rec);
                    return true;
                }
            }
        }
    }
    eprintln!("MISS {}", path.display());
    false
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(pos) = args.iter().position(|a| a == "--apply-png") {
        for p in &args[pos + 1..] {
            let path = Path::new(p);
            let raw = fs::read(path).expect("read png");
            let updated = emblem::encode_png_payload(&raw, emblem::LICENSE_RECORD).expect("encode png");
            fs::write(path, &updated).expect("write png");
            println!("Updated {} ({} bytes)", path.display(), updated.len());
        }
        return;
    }
    if let Some(pos) = args.iter().position(|a| a == "--inspect") {
        let mut all_ok = true;
        for p in &args[pos + 1..] {
            if !inspect_file(Path::new(p)) {
                all_ok = false;
            }
        }
        if !all_ok {
            std::process::exit(1);
        }
        return;
    }
    if let Some(pos) = args.iter().position(|a| a == "--favicon") {
        let master = render_master_icon();
        let data = render_scaled_png(&master, 128);
        for p in &args[pos + 1..] {
            let path = Path::new(p);
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::write(path, &data).expect("write favicon");
            println!("Generated {} ({} bytes)", path.display(), data.len());
        }
        return;
    }

    let out_dir = Path::new("build");
    fs::create_dir_all(out_dir).expect("create build/");
    let master = render_master_icon();
    if args.iter().any(|a| a == "--all") {
        write_icns(&master, &out_dir.join("QN-Bench.icns"));
        write_ico(&master, &out_dir.join("QN-Bench.ico"));
    } else if args.iter().any(|a| a == "--ico") || cfg!(target_os = "windows") && !args.iter().any(|a| a == "--icns") {
        write_ico(&master, &out_dir.join("QN-Bench.ico"));
    } else {
        write_icns(&master, &out_dir.join("QN-Bench.icns"));
    }
}
