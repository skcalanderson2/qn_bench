//! GPU-accelerated egui::Painter optical breadboard, hardware schematics, glow halos, and FX.

use crate::levels::hardware_doc;
use crate::model::*;
use crate::sim::BenchSim;
use egui::{Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use rand::Rng;

fn with_alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

fn draw_radial_glow(p: &Painter, center: Pos2, radius: f32, color: Color32, max_alpha: u8) {
    const RINGS: usize = 8;
    for i in (1..=RINGS).rev() {
        let frac = i as f32 / RINGS as f32;
        let r = radius * frac;
        let alpha = (max_alpha as f32 * (1.0 - frac * 0.85).powi(2) * 0.45) as u8;
        p.circle_filled(center, r, with_alpha(color, alpha));
    }
}

pub fn draw_tile_schematic(p: &Painter, rect: Rect, tile: Tile, frame: u64, alpha: u8) {
    let s = rect.width() / TILE_SIZE;
    let inner = rect.shrink(6.0 * s);
    let cx = rect.center().x;
    let cy = rect.center().y;
    let col = |c: Color32| with_alpha(c, ((c.a() as u32 * alpha as u32) / 255) as u8);
    let font_tile = FontId::proportional((10.0 * s).max(8.0));

    match tile {
        Tile::Wall => {
            p.rect_filled(inner, CornerRadius::same((5.0 * s) as u8), col(WALL_COLOR));
            p.rect_stroke(inner, CornerRadius::same((5.0 * s) as u8), Stroke::new(2.0 * s, col(Color32::from_rgb(75, 90, 112))), StrokeKind::Inside);
            for i in (14..66).step_by(12) {
                p.line_segment(
                    [Pos2::new(rect.left() + i as f32 * s, rect.top() + 10.0 * s),
                     Pos2::new(rect.left() + (i as f32 - 6.0) * s, rect.bottom() - 10.0 * s)],
                    Stroke::new(2.0 * s, col(Color32::from_rgb(36, 44, 58))),
                );
            }
            for (rx, ry) in [(12.0, 12.0), (68.0, 12.0), (12.0, 68.0), (68.0, 68.0)] {
                p.circle_filled(Pos2::new(rect.left() + rx * s, rect.top() + ry * s), 2.2 * s, col(Color32::from_rgb(95, 112, 138)));
            }
        }
        Tile::Fiber => {
            let band = Rect::from_min_max(Pos2::new(rect.left(), cy - 12.0 * s), Pos2::new(rect.right(), cy + 12.0 * s));
            p.rect_filled(band, 0.0, col(Color32::from_rgb(22, 34, 52)));
            p.line_segment([band.left_top(), band.right_top()], Stroke::new(2.0 * s, col(Color32::from_rgb(40, 80, 115))));
            p.line_segment([band.left_bottom(), band.right_bottom()], Stroke::new(2.0 * s, col(Color32::from_rgb(40, 80, 115))));
            p.line_segment([Pos2::new(rect.left(), cy), Pos2::new(rect.right(), cy)], Stroke::new(4.0 * s, col(ACCENT_CYAN)));
            p.line_segment([Pos2::new(rect.left(), cy), Pos2::new(rect.right(), cy)], Stroke::new(1.2 * s, col(Color32::from_rgb(190, 240, 255))));
            p.text(Pos2::new(cx, rect.bottom() - 11.0 * s), Align2::CENTER_CENTER, "1550 FIBER", font_tile, col(Color32::from_rgb(100, 165, 205)));
        }
        Tile::LitFiber => {
            let band = Rect::from_min_max(Pos2::new(rect.left(), cy - 12.0 * s), Pos2::new(rect.right(), cy + 12.0 * s));
            p.rect_filled(band, 0.0, col(Color32::from_rgb(42, 32, 22)));
            p.line_segment([band.left_top(), band.right_top()], Stroke::new(2.0 * s, col(Color32::from_rgb(140, 95, 35))));
            p.line_segment([band.left_bottom(), band.right_bottom()], Stroke::new(2.0 * s, col(Color32::from_rgb(140, 95, 35))));
            p.line_segment([Pos2::new(rect.left(), cy - 3.0 * s), Pos2::new(rect.right(), cy - 3.0 * s)], Stroke::new(3.0 * s, col(ACCENT_CYAN)));
            p.line_segment([Pos2::new(rect.left(), cy + 4.0 * s), Pos2::new(rect.right(), cy + 4.0 * s)], Stroke::new(2.0 * s, col(ACCENT_AMBER)));
            p.text(Pos2::new(cx, rect.bottom() - 11.0 * s), Align2::CENTER_CENTER, "LIT FIBER", font_tile, col(ACCENT_AMBER));
        }
        Tile::PmdFiber => {
            let band = Rect::from_min_max(Pos2::new(rect.left(), cy - 12.0 * s), Pos2::new(rect.right(), cy + 12.0 * s));
            p.rect_filled(band, 0.0, col(Color32::from_rgb(34, 24, 52)));
            p.line_segment([band.left_top(), band.right_top()], Stroke::new(2.0 * s, col(Color32::from_rgb(105, 70, 165))));
            p.line_segment([band.left_bottom(), band.right_bottom()], Stroke::new(2.0 * s, col(Color32::from_rgb(105, 70, 165))));
            let pts: Vec<Pos2> = (0..=80).step_by(4).map(|x| {
                Pos2::new(rect.left() + x as f32 * s, cy + (6.0 * (x as f32 * 0.22).sin()) * s)
            }).collect();
            p.add(egui::Shape::line(pts, Stroke::new(3.0 * s, col(ACCENT_PURPLE))));
            p.text(Pos2::new(cx, rect.bottom() - 11.0 * s), Align2::CENTER_CENTER, "PMD FIBER", font_tile, col(ACCENT_PURPLE));
        }
        Tile::CdFiber => {
            let cd_col = Color32::from_rgb(125, 211, 252);
            let band = Rect::from_min_max(Pos2::new(rect.left(), cy - 12.0 * s), Pos2::new(rect.right(), cy + 12.0 * s));
            p.rect_filled(band, 0.0, col(Color32::from_rgb(18, 34, 54)));
            p.line_segment([band.left_top(), band.right_top()], Stroke::new(2.0 * s, col(Color32::from_rgb(65, 130, 180))));
            p.line_segment([band.left_bottom(), band.right_bottom()], Stroke::new(2.0 * s, col(Color32::from_rgb(65, 130, 180))));
            // Chirped / broadening wavepacket envelope
            let pts_hi: Vec<Pos2> = (0..=80).step_by(4).map(|x| {
                let env = 2.0 + (x as f32 / 80.0) * 6.5;
                Pos2::new(rect.left() + x as f32 * s, cy - env * s)
            }).collect();
            let pts_lo: Vec<Pos2> = (0..=80).step_by(4).map(|x| {
                let env = 2.0 + (x as f32 / 80.0) * 6.5;
                Pos2::new(rect.left() + x as f32 * s, cy + env * s)
            }).collect();
            p.add(egui::Shape::line(pts_hi, Stroke::new(2.0 * s, col(cd_col))));
            p.add(egui::Shape::line(pts_lo, Stroke::new(2.0 * s, col(cd_col))));
            p.line_segment([Pos2::new(rect.left(), cy), Pos2::new(rect.right(), cy)], Stroke::new(1.5 * s, col(ACCENT_AMBER)));
            p.text(Pos2::new(cx, rect.bottom() - 11.0 * s), Align2::CENTER_CENTER, "CD FIBER", font_tile, col(cd_col));
        }
        Tile::Atmos => {
            p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(Color32::from_rgb(16, 36, 44)));
            p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(1.5 * s, col(Color32::from_rgb(55, 130, 145))), StrokeKind::Inside);
            for (row_off, amp) in [(-14.0f32, 4.0f32), (-2.0, 5.0), (10.0, 4.0)] {
                let pts: Vec<Pos2> = (12..=70).step_by(6).map(|x| {
                    let y = cy + (row_off + amp * (x as f32 * 0.16 + frame as f32 * 0.08 + row_off).sin()) * s;
                    Pos2::new(rect.left() + x as f32 * s, y)
                }).collect();
                p.add(egui::Shape::line(pts, Stroke::new(2.0 * s, col(ACCENT_TEAL))));
            }
            p.text(Pos2::new(cx, rect.top() + 62.0 * s), Align2::CENTER_CENTER, "ATMOS", font_tile, col(ACCENT_TEAL));
        }
        Tile::Eve => {
            p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(Color32::from_rgb(46, 18, 24)));
            p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(ACCENT_RED)), StrokeKind::Inside);
            p.line_segment([Pos2::new(rect.left(), cy - 6.0 * s), Pos2::new(rect.right(), cy - 6.0 * s)], Stroke::new(3.0 * s, col(Color32::from_rgb(120, 45, 58))));
            let top = vec![
                Pos2::new(cx - 18.0 * s, cy - 6.0 * s),
                Pos2::new(cx - 8.0 * s, cy - 16.0 * s),
                Pos2::new(cx, cy - 18.0 * s),
                Pos2::new(cx + 8.0 * s, cy - 16.0 * s),
                Pos2::new(cx + 18.0 * s, cy - 6.0 * s),
            ];
            let bot = vec![
                Pos2::new(cx - 18.0 * s, cy - 6.0 * s),
                Pos2::new(cx - 8.0 * s, cy + 4.0 * s),
                Pos2::new(cx, cy + 6.0 * s),
                Pos2::new(cx + 8.0 * s, cy + 4.0 * s),
                Pos2::new(cx + 18.0 * s, cy - 6.0 * s),
            ];
            p.add(egui::Shape::line(top, Stroke::new(2.0 * s, col(ACCENT_RED))));
            p.add(egui::Shape::line(bot, Stroke::new(2.0 * s, col(ACCENT_RED))));
            let pulse = (5.0 + 1.5 * (frame as f32 * 0.14).sin()) * s;
            p.circle_filled(Pos2::new(cx, cy - 6.0 * s), pulse, col(ACCENT_AMBER));
            p.circle_filled(Pos2::new(cx, cy - 6.0 * s), 2.2 * s, col(Color32::from_rgb(24, 10, 14)));
            p.text(Pos2::new(cx, rect.top() + 62.0 * s), Align2::CENTER_CENTER, "EVE TAP", font_tile, col(ACCENT_RED));
        }
        Tile::Distill => {
            p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(Color32::from_rgb(16, 44, 38)));
            p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(ACCENT_GREEN)), StrokeKind::Inside);
            let diamond = vec![
                Pos2::new(cx, rect.top() + 14.0 * s),
                Pos2::new(cx + 18.0 * s, rect.top() + 32.0 * s),
                Pos2::new(cx, rect.top() + 50.0 * s),
                Pos2::new(cx - 18.0 * s, rect.top() + 32.0 * s),
            ];
            p.add(egui::Shape::convex_polygon(diamond, col(Color32::from_rgb(28, 90, 72)), Stroke::new(2.0 * s, col(ACCENT_GREEN))));
            p.line_segment([Pos2::new(cx - 6.0 * s, rect.top() + 32.0 * s), Pos2::new(cx + 12.0 * s, rect.top() + 32.0 * s)], Stroke::new(2.0 * s, col(Color32::from_rgb(190, 255, 230))));
            p.text(Pos2::new(cx, rect.top() + 62.0 * s), Align2::CENTER_CENTER, "DISTILL", font_tile, col(ACCENT_GREEN));
        }
        Tile::Repeater => {
            p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(Color32::from_rgb(16, 36, 52)));
            p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(ACCENT_CYAN)), StrokeKind::Inside);
            // BSM cube on left half
            let cube = Rect::from_min_size(Pos2::new(cx - 20.0 * s, rect.top() + 18.0 * s), Vec2::new(22.0 * s, 26.0 * s));
            p.rect_filled(cube, CornerRadius::same((3.0 * s) as u8), col(Color32::from_rgb(28, 66, 94)));
            p.rect_stroke(cube, CornerRadius::same((3.0 * s) as u8), Stroke::new(1.6 * s, col(ACCENT_CYAN)), StrokeKind::Inside);
            p.line_segment([cube.left_bottom(), cube.right_top()], Stroke::new(1.6 * s, col(Color32::from_rgb(210, 248, 255))));
            // Classical feedforward herald pulse + rightward Hop-2 emitter chevron
            let pulse = (3.0 + 1.4 * (frame as f32 * 0.14).sin()) * s;
            p.circle_filled(Pos2::new(cx + 10.0 * s, rect.top() + 24.0 * s), pulse, col(ACCENT_AMBER));
            p.line_segment([Pos2::new(cx + 3.0 * s, rect.top() + 36.0 * s), Pos2::new(cx + 22.0 * s, rect.top() + 36.0 * s)], Stroke::new(2.2 * s, col(ACCENT_TEAL)));
            p.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(cx + 26.0 * s, rect.top() + 36.0 * s),
                    Pos2::new(cx + 18.0 * s, rect.top() + 31.0 * s),
                    Pos2::new(cx + 18.0 * s, rect.top() + 41.0 * s),
                ],
                col(ACCENT_TEAL),
                Stroke::NONE,
            ));
            p.text(Pos2::new(cx, rect.top() + 62.0 * s), Align2::CENTER_CENTER, "REPEATER", font_tile, col(ACCENT_CYAN));
        }
        Tile::TeleportBsm => {
            p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(Color32::from_rgb(44, 30, 18)));
            p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(ACCENT_AMBER)), StrokeKind::Inside);
            let cube = Rect::from_min_size(Pos2::new(cx - 15.0 * s, rect.top() + 16.0 * s), Vec2::new(30.0 * s, 30.0 * s));
            p.rect_filled(cube, CornerRadius::same((3.0 * s) as u8), col(Color32::from_rgb(76, 52, 26)));
            p.rect_stroke(cube, CornerRadius::same((3.0 * s) as u8), Stroke::new(1.8 * s, col(ACCENT_AMBER)), StrokeKind::Inside);
            p.line_segment([cube.left_bottom(), cube.right_top()], Stroke::new(2.0 * s, col(Color32::from_rgb(255, 240, 200))));
            let pulse = (3.0 + 1.5 * (frame as f32 * 0.14).sin()) * s;
            p.circle_filled(cube.center(), pulse, col(ACCENT_PURPLE));
            p.text(Pos2::new(cx, rect.top() + 62.0 * s), Align2::CENTER_CENTER, "TELE-BSM", font_tile, col(ACCENT_AMBER));
        }
        Tile::Placeable(Placeable::Qfc) => {
            p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(Color32::from_rgb(28, 24, 48)));
            p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(ACCENT_PURPLE)), StrokeKind::Inside);
            let crect = Rect::from_min_size(Pos2::new(rect.left() + 16.0 * s, rect.top() + 20.0 * s), Vec2::new(48.0 * s, 26.0 * s));
            p.rect_filled(crect, CornerRadius::same((3.0 * s) as u8), col(Color32::from_rgb(62, 46, 105)));
            for x in (20..62).step_by(6) {
                p.line_segment([Pos2::new(rect.left() + x as f32 * s, crect.top() + 1.0 * s), Pos2::new(rect.left() + x as f32 * s, crect.bottom() - 1.0 * s)], Stroke::new(2.0 * s, col(ACCENT_PURPLE)));
            }
            p.rect_stroke(crect, CornerRadius::same((3.0 * s) as u8), Stroke::new(1.0 * s, col(Color32::from_rgb(210, 195, 255))), StrokeKind::Inside);
            p.text(Pos2::new(cx, rect.top() + 62.0 * s), Align2::CENTER_CENTER, "PPLN QFC", font_tile, col(ACCENT_PURPLE));
        }
        Tile::Placeable(Placeable::Vbg) => {
            p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(Color32::from_rgb(18, 36, 48)));
            p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(ACCENT_CYAN)), StrokeKind::Inside);
            for x in (24..58).step_by(7) {
                p.line_segment(
                    [Pos2::new(rect.left() + x as f32 * s, rect.top() + 16.0 * s), Pos2::new(rect.left() + (x as f32 - 4.0) * s, rect.top() + 50.0 * s)],
                    Stroke::new(2.0 * s, col(ACCENT_CYAN)),
                );
            }
            p.text(Pos2::new(cx, rect.top() + 62.0 * s), Align2::CENTER_CENTER, "VBG FILTER", font_tile, col(ACCENT_CYAN));
        }
        Tile::Placeable(Placeable::Memory) => {
            p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(Color32::from_rgb(18, 40, 36)));
            p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(ACCENT_GREEN)), StrokeKind::Inside);
            let pts: Vec<Pos2> = (0..6).map(|i| {
                let a = i as f32 * std::f32::consts::FRAC_PI_3;
                Pos2::new(cx + 15.0 * a.cos() * s, cy - 6.0 * s + 15.0 * a.sin() * s)
            }).collect();
            p.add(egui::Shape::convex_polygon(pts, col(Color32::from_rgb(30, 80, 65)), Stroke::new(2.0 * s, col(ACCENT_GREEN))));
            let pulse = (4.0 + 2.0 * (frame as f32 * 0.1).sin()) * s;
            p.circle_filled(Pos2::new(cx, cy - 6.0 * s), pulse, col(Color32::from_rgb(180, 255, 225)));
            p.text(Pos2::new(cx, rect.top() + 62.0 * s), Align2::CENTER_CENTER, "Q-MEMORY", font_tile, col(ACCENT_GREEN));
        }
        Tile::Placeable(Placeable::Delay) => {
            p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(Color32::from_rgb(42, 34, 20)));
            p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(ACCENT_AMBER)), StrokeKind::Inside);
            for r in [16.0f32, 11.0, 6.0] {
                p.circle_stroke(Pos2::new(cx, cy - 6.0 * s), r * s, Stroke::new(2.0 * s, col(ACCENT_AMBER)));
            }
            p.text(Pos2::new(cx, rect.top() + 62.0 * s), Align2::CENTER_CENTER, "+100ps", font_tile, col(ACCENT_AMBER));
        }
        Tile::Placeable(Placeable::Hwp) => {
            p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(Color32::from_rgb(44, 34, 22)));
            p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(ACCENT_HWP)), StrokeKind::Inside);
            let bar = Rect::from_min_size(Pos2::new(cx - 7.0 * s, rect.top() + 16.0 * s), Vec2::new(14.0 * s, 34.0 * s));
            p.rect_filled(bar, CornerRadius::same((3.0 * s) as u8), col(Color32::from_rgb(92, 68, 34)));
            p.rect_stroke(bar, CornerRadius::same((3.0 * s) as u8), Stroke::new(1.0 * s, col(Color32::from_rgb(255, 220, 150))), StrokeKind::Inside);
            p.line_segment([Pos2::new(cx - 18.0 * s, rect.top() + 33.0 * s), Pos2::new(cx + 18.0 * s, rect.top() + 33.0 * s)], Stroke::new(1.2 * s, col(Color32::from_rgb(255, 220, 150))));
            p.line_segment([Pos2::new(cx - 12.0 * s, rect.top() + 21.0 * s), Pos2::new(cx + 12.0 * s, rect.top() + 45.0 * s)], Stroke::new(1.2 * s, col(Color32::from_rgb(255, 220, 150))));
            p.text(Pos2::new(cx, rect.top() + 62.0 * s), Align2::CENTER_CENTER, "HWP (+/x)", font_tile, col(ACCENT_HWP));
        }
        Tile::Placeable(Placeable::Ao) => {
            p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(Color32::from_rgb(16, 38, 42)));
            p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(ACCENT_TEAL)), StrokeKind::Inside);
            let arc: Vec<Pos2> = (0..7).map(|i| {
                Pos2::new(cx + (-18.0 + i as f32 * 6.0) * s, rect.top() + (22.0 + 4.0 * (i as f32 * 0.6).sin()) * s)
            }).collect();
            p.add(egui::Shape::line(arc, Stroke::new(3.0 * s, col(Color32::from_rgb(180, 255, 245)))));
            for ax in [-14.0f32, 0.0, 14.0] {
                p.line_segment([Pos2::new(cx + ax * s, rect.top() + 27.0 * s), Pos2::new(cx + ax * s, rect.top() + 44.0 * s)], Stroke::new(2.0 * s, col(ACCENT_TEAL)));
                p.rect_filled(Rect::from_min_size(Pos2::new(cx + (ax - 3.0) * s, rect.top() + 42.0 * s), Vec2::new(6.0 * s, 5.0 * s)), 1.0, col(Color32::from_rgb(40, 100, 105)));
            }
            p.text(Pos2::new(cx, rect.top() + 62.0 * s), Align2::CENTER_CENTER, "ADAPT OPT", font_tile, col(ACCENT_TEAL));
        }
        Tile::Placeable(Placeable::Dcf) => {
            let dcf_col = Color32::from_rgb(125, 211, 252);
            p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(Color32::from_rgb(16, 32, 48)));
            p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(dcf_col)), StrokeKind::Inside);
            // Chirped grating bars (narrowing spacing) showing pulse compression
            for &dx in &[-19.0f32, -10.0, -3.0, 2.0, 6.0, 10.0, 14.0, 18.0] {
                p.line_segment(
                    [Pos2::new(cx + dx * s, rect.top() + 18.0 * s), Pos2::new(cx + dx * s, rect.top() + 48.0 * s)],
                    Stroke::new(2.0 * s, col(dcf_col)),
                );
            }
            p.line_segment([Pos2::new(cx - 22.0 * s, rect.top() + 33.0 * s), Pos2::new(cx + 22.0 * s, rect.top() + 33.0 * s)], Stroke::new(1.5 * s, col(ACCENT_GREEN)));
            p.text(Pos2::new(cx, rect.top() + 62.0 * s), Align2::CENTER_CENTER, "DCF COMP", font_tile, col(dcf_col));
        }
        Tile::Placeable(Placeable::Wdm) => {
            let wdm_col = Color32::from_rgb(244, 114, 182);
            p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(Color32::from_rgb(38, 20, 42)));
            p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(wdm_col)), StrokeKind::Inside);
            // Dichroic (\) coated glass plate: transmits 1550nm cyan, reflects visible magenta/orange
            p.line_segment([Pos2::new(rect.left() + 16.0 * s, rect.top() + 16.0 * s), Pos2::new(rect.left() + 64.0 * s, rect.top() + 64.0 * s)], Stroke::new(6.0 * s, col(Color32::from_rgb(130, 52, 104))));
            p.line_segment([Pos2::new(rect.left() + 16.0 * s, rect.top() + 16.0 * s), Pos2::new(rect.left() + 64.0 * s, rect.top() + 64.0 * s)], Stroke::new(2.2 * s, col(wdm_col)));
            p.line_segment([Pos2::new(rect.left() + 12.0 * s, cy - 2.0 * s), Pos2::new(rect.right() - 12.0 * s, cy - 2.0 * s)], Stroke::new(1.8 * s, col(ACCENT_CYAN)));
            p.text(Pos2::new(cx, rect.top() + 62.0 * s), Align2::CENTER_CENTER, "WDM (\\)", font_tile, col(wdm_col));
        }
        Tile::Placeable(Placeable::MirrorSlash) => {
            p.rect_filled(inner, CornerRadius::same((6.0 * s) as u8), col(Color32::from_rgb(24, 30, 42)));
            p.rect_stroke(inner, CornerRadius::same((6.0 * s) as u8), Stroke::new(1.0 * s, col(Color32::from_rgb(55, 68, 90))), StrokeKind::Inside);
            p.line_segment([Pos2::new(rect.left() + 18.0 * s, rect.top() + 66.0 * s), Pos2::new(rect.left() + 66.0 * s, rect.top() + 18.0 * s)], Stroke::new(7.0 * s, col(Color32::from_rgb(95, 115, 140))));
            p.line_segment([Pos2::new(rect.left() + 16.0 * s, rect.top() + 64.0 * s), Pos2::new(rect.left() + 64.0 * s, rect.top() + 16.0 * s)], Stroke::new(3.0 * s, col(Color32::from_rgb(220, 245, 255))));
        }
        Tile::Placeable(Placeable::MirrorBack) => {
            p.rect_filled(inner, CornerRadius::same((6.0 * s) as u8), col(Color32::from_rgb(24, 30, 42)));
            p.rect_stroke(inner, CornerRadius::same((6.0 * s) as u8), Stroke::new(1.0 * s, col(Color32::from_rgb(55, 68, 90))), StrokeKind::Inside);
            p.line_segment([Pos2::new(rect.left() + 14.0 * s, rect.top() + 18.0 * s), Pos2::new(rect.left() + 62.0 * s, rect.top() + 66.0 * s)], Stroke::new(7.0 * s, col(Color32::from_rgb(95, 115, 140))));
            p.line_segment([Pos2::new(rect.left() + 16.0 * s, rect.top() + 16.0 * s), Pos2::new(rect.left() + 64.0 * s, rect.top() + 64.0 * s)], Stroke::new(3.0 * s, col(Color32::from_rgb(220, 245, 255))));
        }
    }
}

fn draw_epr_source(p: &Painter, rect: Rect, vertical: bool, frame: u64, alpha: u8) {
    let s = rect.width() / TILE_SIZE;
    let inner = rect.shrink(6.0 * s);
    let cx = inner.center().x;
    let cy = inner.center().y;
    let col = |c: Color32| with_alpha(c, ((c.a() as u32 * alpha as u32) / 255) as u8);
    p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(Color32::from_rgb(34, 22, 56)));
    p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(ACCENT_PURPLE)), StrokeKind::Inside);
    if alpha >= 200 {
        draw_radial_glow(p, Pos2::new(cx, cy - 4.0 * s), 26.0 * s, ACCENT_PURPLE, 120);
    }
    if vertical {
        p.add(egui::Shape::convex_polygon(
            vec![Pos2::new(cx, cy - 28.0 * s), Pos2::new(cx - 5.0 * s, cy - 21.0 * s), Pos2::new(cx + 5.0 * s, cy - 21.0 * s)],
            col(ACCENT_CYAN), Stroke::NONE,
        ));
        p.add(egui::Shape::convex_polygon(
            vec![Pos2::new(cx, cy + 20.0 * s), Pos2::new(cx - 5.0 * s, cy + 13.0 * s), Pos2::new(cx + 5.0 * s, cy + 13.0 * s)],
            col(ACCENT_CYAN), Stroke::NONE,
        ));
    } else {
        p.add(egui::Shape::convex_polygon(
            vec![Pos2::new(cx - 28.0 * s, cy - 4.0 * s), Pos2::new(cx - 21.0 * s, cy - 9.0 * s), Pos2::new(cx - 21.0 * s, cy + 1.0 * s)],
            col(ACCENT_CYAN), Stroke::NONE,
        ));
        p.add(egui::Shape::convex_polygon(
            vec![Pos2::new(cx + 28.0 * s, cy - 4.0 * s), Pos2::new(cx + 21.0 * s, cy - 9.0 * s), Pos2::new(cx + 21.0 * s, cy + 1.0 * s)],
            col(ACCENT_CYAN), Stroke::NONE,
        ));
    }
    let diamond = vec![
        Pos2::new(cx, cy - 20.0 * s),
        Pos2::new(cx + 15.0 * s, cy - 4.0 * s),
        Pos2::new(cx, cy + 12.0 * s),
        Pos2::new(cx - 15.0 * s, cy - 4.0 * s),
    ];
    p.add(egui::Shape::convex_polygon(diamond, col(Color32::from_rgb(62, 42, 110)), Stroke::new(2.0 * s, col(Color32::from_rgb(218, 198, 255)))));
    let pulse = (4.0 + 1.8 * (frame as f32 * 0.13).sin()) * s;
    p.circle_filled(Pos2::new(cx, cy - 4.0 * s), pulse, col(ACCENT_CYAN));
    p.text(Pos2::new(cx, inner.bottom() - 9.0 * s), Align2::CENTER_CENTER, "EPR PAIR", FontId::proportional((10.0 * s).max(8.0)), col(ACCENT_PURPLE));
}

fn draw_emitter(p: &Painter, rect: Rect, label: char, dir: Dir, wl: u32, pol: Pol, is_qkd_tx: bool, frame: u64, alpha: u8) {
    let s = rect.width() / TILE_SIZE;
    let inner = rect.shrink(6.0 * s);
    let cx = inner.center().x;
    let cy = inner.center().y;
    let col = |c: Color32| with_alpha(c, ((c.a() as u32 * alpha as u32) / 255) as u8);
    let is_nv = wl == 637;
    let is_psi = pol == Pol::Cross;
    let accent = if is_psi { ACCENT_PURPLE } else if is_nv { ACCENT_ORANGE } else { ACCENT_RED };
    let bg = if is_psi { Color32::from_rgb(38, 22, 56) } else if is_nv { Color32::from_rgb(48, 28, 16) } else { Color32::from_rgb(48, 20, 28) };
    p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(bg));
    p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(accent)), StrokeKind::Inside);
    let elec_col = if is_psi { Color32::from_rgb(140, 100, 195) } else if is_nv { Color32::from_rgb(175, 110, 65) } else { Color32::from_rgb(160, 80, 95) };
    for (dx, dy) in [(-14.0f32, -14.0f32), (14.0, -14.0), (-14.0, 10.0), (14.0, 10.0)] {
        p.circle_filled(Pos2::new(cx + dx * s, cy + dy * s), 4.0 * s, col(elec_col));
    }
    let (ddx, ddy) = dir.vec();
    let tip = Pos2::new(cx + ddx as f32 * 28.0 * s, cy - 2.0 * s + ddy as f32 * 24.0 * s);
    let (px, py) = (-ddy as f32, ddx as f32);
    let b1 = Pos2::new(tip.x + (-ddx as f32 * 6.0 + px * 4.0) * s, tip.y + (-ddy as f32 * 6.0 + py * 4.0) * s);
    let b2 = Pos2::new(tip.x + (-ddx as f32 * 6.0 - px * 4.0) * s, tip.y + (-ddy as f32 * 6.0 - py * 4.0) * s);
    p.add(egui::Shape::convex_polygon(vec![tip, b1, b2], col(accent), Stroke::NONE));
    if alpha >= 200 {
        draw_radial_glow(p, Pos2::new(cx, cy - 2.0 * s), 25.0 * s, accent, 140);
    }
    let pulse = (4.0 + 1.5 * (frame as f32 * 0.12).sin()) * s;
    let core = if is_psi { Color32::from_rgb(235, 215, 255) } else if is_nv { Color32::from_rgb(255, 225, 195) } else { Color32::from_rgb(255, 210, 220) };
    p.circle_filled(Pos2::new(cx, cy - 2.0 * s), pulse, col(core));
    let tag = if is_psi {
        "|Psi> 'x'".to_string()
    } else if is_qkd_tx {
        "QKD TX +".to_string()
    } else if is_nv {
        format!("NV {label} 637")
    } else {
        format!("ION {label} 780")
    };
    p.text(Pos2::new(cx, inner.bottom() - 9.0 * s), Align2::CENTER_CENTER, tag, FontId::proportional((10.0 * s).max(8.0)), col(accent));
}

fn draw_target(p: &Painter, rect: Rect, mode: TargetMode, label_override: Option<&str>, complete: bool, alpha: u8) {
    let s = rect.width() / TILE_SIZE;
    let inner = rect.shrink(6.0 * s);
    let cx = inner.center().x;
    let cy = inner.center().y;
    let col = |c: Color32| with_alpha(c, ((c.a() as u32 * alpha as u32) / 255) as u8);
    let is_qkd = mode.is_qkd();
    let border = if complete {
        ACCENT_GREEN
    } else if mode == TargetMode::Qpu || mode == TargetMode::Teleport {
        ACCENT_PURPLE
    } else if is_qkd {
        ACCENT_TEAL
    } else {
        ACCENT_CYAN
    };
    let bg = if mode == TargetMode::Qpu || mode == TargetMode::Teleport {
        Color32::from_rgb(24, 24, 48)
    } else if is_qkd {
        Color32::from_rgb(14, 40, 38)
    } else {
        Color32::from_rgb(16, 38, 48)
    };
    p.rect_filled(inner, CornerRadius::same((8.0 * s) as u8), col(bg));
    p.rect_stroke(inner, CornerRadius::same((8.0 * s) as u8), Stroke::new(2.0 * s, col(border)), StrokeKind::Inside);
    let font_tile = FontId::proportional((10.0 * s).max(8.0));

    if is_qkd {
        let pbs = Rect::from_min_size(Pos2::new(cx - 15.0 * s, cy - 22.0 * s), Vec2::new(30.0 * s, 30.0 * s));
        p.rect_filled(pbs, CornerRadius::same((4.0 * s) as u8), col(Color32::from_rgb(24, 68, 66)));
        p.rect_stroke(pbs, CornerRadius::same((4.0 * s) as u8), Stroke::new(2.0 * s, col(border)), StrokeKind::Inside);
        let c = pbs.center();
        p.line_segment([Pos2::new(c.x - 8.0 * s, c.y), Pos2::new(c.x + 8.0 * s, c.y)], Stroke::new(2.0 * s, col(Color32::from_rgb(200, 255, 245))));
        p.line_segment([Pos2::new(c.x, c.y - 8.0 * s), Pos2::new(c.x, c.y + 8.0 * s)], Stroke::new(2.0 * s, col(Color32::from_rgb(200, 255, 245))));
        p.line_segment([Pos2::new(c.x - 6.0 * s, c.y - 6.0 * s), Pos2::new(c.x + 6.0 * s, c.y + 6.0 * s)], Stroke::new(1.2 * s, col(ACCENT_PURPLE)));
        p.line_segment([Pos2::new(c.x - 6.0 * s, c.y + 6.0 * s), Pos2::new(c.x + 6.0 * s, c.y - 6.0 * s)], Stroke::new(1.2 * s, col(ACCENT_PURPLE)));
        p.text(Pos2::new(cx, inner.bottom() - 9.0 * s), Align2::CENTER_CENTER, label_override.unwrap_or("BOB RX"), font_tile, col(border));
    } else if mode == TargetMode::Teleport {
        let rx = Rect::from_min_size(Pos2::new(cx - 15.0 * s, cy - 22.0 * s), Vec2::new(30.0 * s, 30.0 * s));
        p.rect_filled(rx, CornerRadius::same((5.0 * s) as u8), col(Color32::from_rgb(50, 32, 86)));
        p.rect_stroke(rx, CornerRadius::same((5.0 * s) as u8), Stroke::new(2.0 * s, col(border)), StrokeKind::Inside);
        let c = rx.center();
        p.line_segment([Pos2::new(c.x - 8.0 * s, c.y - 8.0 * s), Pos2::new(c.x + 8.0 * s, c.y + 8.0 * s)], Stroke::new(2.4 * s, col(Color32::from_rgb(225, 200, 255))));
        p.line_segment([Pos2::new(c.x - 8.0 * s, c.y + 8.0 * s), Pos2::new(c.x + 8.0 * s, c.y - 8.0 * s)], Stroke::new(2.4 * s, col(Color32::from_rgb(225, 200, 255))));
        p.text(Pos2::new(cx, inner.bottom() - 9.0 * s), Align2::CENTER_CENTER, "TELEPORT RX", font_tile, col(border));
    } else if mode == TargetMode::Bsm {
        let cube = Rect::from_min_size(Pos2::new(cx - 16.0 * s, cy - 22.0 * s), Vec2::new(32.0 * s, 32.0 * s));
        p.rect_filled(cube, 0.0, col(Color32::from_rgb(35, 85, 115)));
        p.rect_stroke(cube, 0.0, Stroke::new(2.0 * s, col(ACCENT_CYAN)), StrokeKind::Inside);
        p.line_segment([cube.left_bottom(), cube.right_top()], Stroke::new(2.0 * s, col(Color32::from_rgb(200, 245, 255))));
        p.text(Pos2::new(cx, inner.bottom() - 9.0 * s), Align2::CENTER_CENTER, "BSM 50:50", font_tile, col(ACCENT_AMBER));
    } else if mode == TargetMode::Qpu {
        let chip = Rect::from_min_size(Pos2::new(cx - 14.0 * s, cy - 20.0 * s), Vec2::new(28.0 * s, 28.0 * s));
        p.rect_filled(chip, CornerRadius::same((4.0 * s) as u8), col(Color32::from_rgb(48, 36, 86)));
        p.rect_stroke(chip, CornerRadius::same((4.0 * s) as u8), Stroke::new(2.0 * s, col(border)), StrokeKind::Inside);
        for dx in [-6.0f32, 6.0] {
            for dy in [-6.0f32, 6.0] {
                p.circle_filled(Pos2::new(chip.center().x + dx * s, chip.center().y + dy * s), 3.0 * s, col(Color32::from_rgb(220, 200, 255)));
            }
        }
        p.text(Pos2::new(cx, inner.bottom() - 9.0 * s), Align2::CENTER_CENTER, "QPU >=90%", font_tile, col(ACCENT_PURPLE));
    } else {
        let pts = vec![
            Pos2::new(cx - 16.0 * s, cy - 18.0 * s),
            Pos2::new(cx + 16.0 * s, cy - 18.0 * s),
            Pos2::new(cx + 16.0 * s, cy - 11.0 * s),
            Pos2::new(cx - 16.0 * s, cy - 11.0 * s),
            Pos2::new(cx - 16.0 * s, cy - 4.0 * s),
            Pos2::new(cx + 16.0 * s, cy - 4.0 * s),
            Pos2::new(cx + 16.0 * s, cy + 3.0 * s),
            Pos2::new(cx - 16.0 * s, cy + 3.0 * s),
        ];
        p.add(egui::Shape::line(pts, Stroke::new(2.0 * s, col(border))));
        p.text(Pos2::new(cx, inner.bottom() - 9.0 * s), Align2::CENTER_CENTER, "SNSPD 1.5K", font_tile, col(ACCENT_CYAN));
    }
}

fn draw_dashed_segment(p: &Painter, a: Pos2, b: Pos2, color: Color32, width: f32, dash: f32, gap: f32, phase: f32) {
    let delta = b - a;
    let len = delta.length();
    if len < 0.5 {
        return;
    }
    let dir = delta / len;
    let period = dash + gap;
    let mut dist = (phase % period) - period;
    while dist < len {
        let s0 = dist.max(0.0);
        let s1 = (dist + dash).min(len);
        if s1 > s0 {
            p.line_segment([a + dir * s0, a + dir * s1], Stroke::new(width, color));
        }
        dist += period;
    }
}

fn doc_key_at(sim: &BenchSim, pos: (i32, i32)) -> Option<&'static str> {
    let lvl = sim.current_level();
    if lvl.epr_source == Some(pos) {
        return Some("EPR");
    }
    if let Some(em) = lvl.emitters.iter().find(|e| (e.gx, e.gy) == pos) {
        if lvl.mode == TargetMode::QkdBb84 {
            return Some("QKD_TX");
        }
        return Some(if em.wavelength == 637 { "NV637" } else { "ION" });
    }
    let is_e91_rx = sim.sandbox_mode
        && lvl.mode == TargetMode::QkdE91
        && (sim.sandbox_bp.e91_alice == Some(pos) || sim.sandbox_bp.e91_bob == Some(pos));
    if lvl.is_target(pos) || is_e91_rx {
        return Some(match lvl.mode {
            TargetMode::Single => "SNSPD",
            TargetMode::Bsm => "BSM",
            TargetMode::Qpu => "QPU",
            TargetMode::QkdBb84 | TargetMode::QkdE91 => "QKD_RX",
            TargetMode::Teleport => "TELEPORT_RX",
        });
    }
    sim.grid.get(&pos).map(|t| match t {
        Tile::Placeable(p) => p.doc_key(),
        Tile::Wall => "WALL",
        Tile::Fiber => "FIBER",
        Tile::LitFiber => "LIT_FIBER",
        Tile::PmdFiber => "PMD_FIBER",
        Tile::CdFiber => "CD_FIBER",
        Tile::Atmos => "ATMOS",
        Tile::Distill => "DISTILL",
        Tile::Repeater => "REPEATER",
        Tile::TeleportBsm => "TELEPORT_BSM",
        Tile::Eve => "EVE",
    })
}

pub fn draw_optical_bench(ui: &mut Ui, sim: &mut BenchSim, overlay_open: bool) -> Option<&'static str> {
    let mut inspect_doc: Option<&'static str> = None;
    let avail = ui.available_size();
    let hud_reserve = 26.0;
    let grid_avail_h = (avail.y - hud_reserve).max(160.0);
    let scale = (avail.x / BOARD_W).min(grid_avail_h / BOARD_H).max(0.35);
    let board_size = Vec2::new(BOARD_W * scale, BOARD_H * scale);

    let (outer_rect, _) = ui.allocate_exact_size(avail, Sense::hover());
    let origin = Pos2::new(
        outer_rect.left() + ((avail.x - board_size.x) * 0.5).max(0.0),
        outer_rect.top() + hud_reserve + ((grid_avail_h - board_size.y) * 0.5).max(0.0),
    );
    let board_rect = Rect::from_min_size(origin, board_size);
    let resp = ui.interact(board_rect, ui.id().with("optical_bench"), Sense::click());
    let p_outer = ui.painter_at(outer_rect);
    let p = ui.painter_at(board_rect);

    let tile_rect = |gx: i32, gy: i32| -> Rect {
        Rect::from_min_size(
            Pos2::new(origin.x + gx as f32 * TILE_SIZE * scale, origin.y + gy as f32 * TILE_SIZE * scale),
            Vec2::splat(TILE_SIZE * scale),
        )
    };
    let to_screen = |wx: f32, wy: f32| -> Pos2 {
        Pos2::new(origin.x + wx * scale, origin.y + wy * scale)
    };

    // 1. Machined optical breadboard grid
    p.rect_filled(board_rect, 0.0, GRID_BG);
    for r in 0..GRID_ROWS {
        for c in 0..GRID_COLS {
            let tr = tile_rect(c, r);
            p.rect_stroke(tr, 0.0, Stroke::new(1.0_f32, GRID_LINE), StrokeKind::Inside);
            for hx in [20.0f32, 60.0] {
                for hy in [20.0f32, 60.0] {
                    p.circle_filled(Pos2::new(tr.left() + hx * scale, tr.top() + hy * scale), 2.0 * scale, BREADBOARD_HOLE);
                }
            }
        }
    }

    // 2. Fixed & user-placed components
    for (&(gx, gy), &tile) in &sim.grid {
        let tr = tile_rect(gx, gy);
        draw_tile_schematic(&p, tr, tile, sim.frame, 255);
        if sim.colorblind_mode {
            let badge = match tile {
                Tile::Fiber => Some("1550"),
                Tile::LitFiber => Some("+N15"),
                Tile::PmdFiber => Some("PMD"),
                Tile::CdFiber => Some("CD"),
                Tile::Atmos => Some("ATM"),
                Tile::Eve => Some("EVE!"),
                _ => None,
            };
            if let Some(tag) = badge {
                let br = Rect::from_min_size(
                    Pos2::new(tr.left() + 4.0 * scale, tr.top() + 4.0 * scale),
                    Vec2::new((tag.len() as f32 * 5.8 + 6.0) * scale, 13.0 * scale),
                );
                p.rect_filled(br, CornerRadius::same(2), Color32::from_rgba_unmultiplied(10, 14, 22, 225));
                p.rect_stroke(br, CornerRadius::same(2), Stroke::new(1.0_f32, TEXT_WHITE), StrokeKind::Inside);
                p.text(br.center(), Align2::CENTER_CENTER, tag, FontId::monospace((8.5 * scale).max(7.5)), TEXT_WHITE);
            }
        }
    }

    // 2b. Live Alignment Laser Preview (subtle animated guide beam + timing readout when idle)
    let align_beams = if sim.align_laser && !sim.sim_active {
        let beams = sim.trace_alignment_beams();
        let phase = (sim.frame as f32 * 0.7) % 16.0;
        for beam in &beams {
            let mut prev_wl: Option<u32> = None;
            for seg in &beam.segments {
                let c1 = tile_rect(seg.x1, seg.y1).center();
                let c2 = tile_rect(seg.x2, seg.y2).center();
                let base_col = if beam.label == "H" {
                    ACCENT_TEAL
                } else if beam.label == "P" {
                    ACCENT_GREEN
                } else if seg.wavelength == 637 {
                    ACCENT_ORANGE
                } else if seg.wavelength == 780 {
                    ACCENT_RED
                } else {
                    ACCENT_CYAN
                };
                // Colorblind mode uses distinct dash cadences per wavelength + inline wavelength pills
                let (dash_len, gap_len, width_mult) = if sim.colorblind_mode {
                    match seg.wavelength {
                        780 => (4.0, 4.0, 2.8),
                        637 => (6.0, 9.0, 2.8),
                        _ => (14.0, 5.0, 2.6),
                    }
                } else {
                    (8.0, 8.0, 2.2)
                };
                draw_dashed_segment(
                    &p,
                    c1,
                    c2,
                    with_alpha(base_col, if sim.colorblind_mode { 195 } else { 130 }),
                    (width_mult * scale).max(1.2),
                    dash_len * scale,
                    gap_len * scale,
                    phase * scale,
                );
                if sim.colorblind_mode && prev_wl != Some(seg.wavelength) {
                    prev_wl = Some(seg.wavelength);
                    let mid = Pos2::new((c1.x + c2.x) * 0.5, (c1.y + c2.y) * 0.5 - 10.0 * scale);
                    let wtag = format!("{}", seg.wavelength);
                    let tr_b = Rect::from_center_size(mid, Vec2::new(28.0 * scale, 12.0 * scale));
                    p.rect_filled(tr_b, CornerRadius::same(2), Color32::from_rgba_unmultiplied(10, 14, 22, 230));
                    p.rect_stroke(tr_b, CornerRadius::same(2), Stroke::new(1.0_f32, base_col), StrokeKind::Inside);
                    p.text(mid, Align2::CENTER_CENTER, wtag, FontId::monospace((8.5 * scale).max(7.5)), TEXT_WHITE);
                }
            }
            if beam.blocked {
                let end_c = tile_rect(beam.end_pos.0, beam.end_pos.1).center();
                let d = 10.0 * scale;
                let pulse_a = (155.0 + 75.0 * (sim.frame as f32 * 0.14).sin()) as u8;
                let hcol = with_alpha(ACCENT_RED, pulse_a);
                p.line_segment([Pos2::new(end_c.x - d, end_c.y - d), Pos2::new(end_c.x + d, end_c.y + d)], Stroke::new(2.5 * scale, hcol));
                p.line_segment([Pos2::new(end_c.x - d, end_c.y + d), Pos2::new(end_c.x + d, end_c.y - d)], Stroke::new(2.5 * scale, hcol));
            }
        }
        Some(beams)
    } else {
        None
    };

    // 3. Pointer interaction + hover ghost preview + precision reticle
    let hover_tile = if !overlay_open {
        resp.hover_pos().map(|pos| {
            let gx = ((pos.x - origin.x) / (TILE_SIZE * scale)).floor() as i32;
            let gy = ((pos.y - origin.y) / (TILE_SIZE * scale)).floor() as i32;
            (gx.clamp(0, GRID_COLS - 1), gy.clamp(0, GRID_ROWS - 1))
        })
    } else {
        None
    };

    if let Some((hgx, hgy)) = hover_tile {
        let shift_held = ui.input(|i| i.modifiers.shift);
        if (resp.clicked_by(egui::PointerButton::Primary) && shift_held)
            || resp.clicked_by(egui::PointerButton::Middle)
        {
            inspect_doc = doc_key_at(sim, (hgx, hgy));
        } else if resp.clicked_by(egui::PointerButton::Primary) {
            sim.bench_primary_click(hgx, hgy);
        } else if resp.clicked_by(egui::PointerButton::Secondary) {
            sim.remove_component(hgx, hgy);
        }

        let tr = tile_rect(hgx, hgy);
        let draw_reticle = |p: &Painter, reticle_col: Color32| {
            let m = 3.0 * scale;
            let arm = 10.0 * scale;
            for (cx0, cy0, dx, dy) in [
                (tr.left() + m, tr.top() + m, arm, arm),
                (tr.right() - m, tr.top() + m, -arm, arm),
                (tr.left() + m, tr.bottom() - m, arm, -arm),
                (tr.right() - m, tr.bottom() - m, -arm, -arm),
            ] {
                p.line_segment([Pos2::new(cx0, cy0), Pos2::new(cx0 + dx, cy0)], Stroke::new(1.2_f32, reticle_col));
                p.line_segment([Pos2::new(cx0, cy0), Pos2::new(cx0, cy0 + dy)], Stroke::new(1.2_f32, reticle_col));
            }
            p.text(
                Pos2::new(tr.left() + 6.0 * scale, tr.top() + 5.0 * scale),
                Align2::LEFT_TOP,
                format!("{hgx},{hgy}"),
                FontId::proportional((10.0 * scale).max(8.0)),
                Color32::from_rgb(90, 130, 160),
            );
        };

        if sim.erase_mode {
            let can_erase = sim.sandbox_mode || sim.user_placed.contains(&(hgx, hgy));
            let ecol = if can_erase { ACCENT_RED } else { Color32::from_rgb(110, 68, 76) };
            draw_reticle(&p, ecol);
            if can_erase {
                let d = 14.0 * scale;
                let c = tr.center();
                p.rect_filled(tr.shrink(4.0 * scale), CornerRadius::same(4), Color32::from_rgba_unmultiplied(244, 63, 94, 42));
                p.line_segment([Pos2::new(c.x - d, c.y - d), Pos2::new(c.x + d, c.y + d)], Stroke::new(2.5 * scale, ACCENT_RED));
                p.line_segment([Pos2::new(c.x - d, c.y + d), Pos2::new(c.x + d, c.y - d)], Stroke::new(2.5 * scale, ACCENT_RED));
            }
        } else if sim.sandbox_mode {
            match sim.sandbox_tool {
                SandboxTool::Tile(t) => draw_tile_schematic(&p, tr, t, sim.frame, 115),
                SandboxTool::EmitterIon => draw_emitter(&p, tr, '+', sim.sandbox_dir, 780, Pol::Plus, false, sim.frame, 120),
                SandboxTool::EmitterNv => draw_emitter(&p, tr, '+', sim.sandbox_dir, 637, Pol::Plus, false, sim.frame, 120),
                SandboxTool::EprSource => draw_epr_source(&p, tr, matches!(sim.sandbox_dir, Dir::Up | Dir::Down), sim.frame, 120),
                SandboxTool::DetectorSnspd => draw_target(&p, tr, TargetMode::Single, None, false, 120),
                SandboxTool::DetectorBsm => draw_target(&p, tr, TargetMode::Bsm, None, false, 120),
                SandboxTool::DetectorQpu => draw_target(&p, tr, TargetMode::Qpu, None, false, 120),
                SandboxTool::DetectorBobBb84 => draw_target(&p, tr, TargetMode::QkdBb84, Some("BOB RX"), false, 120),
                SandboxTool::DetectorAliceE91 => draw_target(&p, tr, TargetMode::QkdE91, Some("ALICE RX"), false, 120),
                SandboxTool::DetectorBobE91 => draw_target(&p, tr, TargetMode::QkdE91, Some("BOB RX"), false, 120),
            }
            draw_reticle(&p, ACCENT_PURPLE);
        } else {
            let lvl = sim.current_level();
            if !sim.grid.contains_key(&(hgx, hgy)) && !lvl.is_emitter((hgx, hgy)) && !lvl.is_target((hgx, hgy)) {
                if let Some(sel) = sim.selected {
                    if sim.inventory.get(&sel).copied().unwrap_or(0) > 0 {
                        draw_tile_schematic(&p, tr, Tile::Placeable(sel), sim.frame, 115);
                        draw_reticle(&p, ACCENT_CYAN);
                    }
                }
            }
        }
    }

    // 4. Emitters & Target(s)
    let lvl = sim.current_level();
    if let Some((sx, sy)) = lvl.epr_source {
        let vert = lvl
            .emitters
            .iter()
            .find(|e| (e.gx, e.gy) == (sx, sy))
            .map(|e| matches!(e.dir, Dir::Up | Dir::Down))
            .unwrap_or(false);
        draw_epr_source(&p, tile_rect(sx, sy), vert, sim.frame, 255);
        for (i, e) in lvl.emitters.iter().enumerate() {
            if (e.gx, e.gy) != (sx, sy) {
                let lbl = (b'A' + i as u8) as char;
                draw_emitter(&p, tile_rect(e.gx, e.gy), lbl, e.dir, e.wavelength, e.pol, false, sim.frame, 255);
            }
        }
    } else {
        for (i, e) in lvl.emitters.iter().enumerate() {
            let lbl = (b'A' + i as u8) as char;
            draw_emitter(&p, tile_rect(e.gx, e.gy), lbl, e.dir, e.wavelength, e.pol, lvl.mode == TargetMode::QkdBb84, sim.frame, 255);
        }
    }
    if let Some((a_pos, b_pos)) = lvl.e91_targets {
        draw_target(&p, tile_rect(a_pos.0, a_pos.1), lvl.mode, Some("ALICE RX"), sim.level_complete, 255);
        draw_target(&p, tile_rect(b_pos.0, b_pos.1), lvl.mode, Some("BOB RX"), sim.level_complete, 255);
    } else if sim.sandbox_mode && lvl.mode == TargetMode::QkdE91 {
        if let Some((ax, ay)) = sim.sandbox_bp.e91_alice {
            draw_target(&p, tile_rect(ax, ay), TargetMode::QkdE91, Some("ALICE RX"), sim.level_complete, 255);
        }
        if let Some((bx, by)) = sim.sandbox_bp.e91_bob {
            draw_target(&p, tile_rect(bx, by), TargetMode::QkdE91, Some("BOB RX"), sim.level_complete, 255);
        }
    } else if lvl.target.0 >= 0 && lvl.target.1 >= 0 {
        let lbl = if lvl.mode == TargetMode::QkdBb84 { Some("BOB RX") } else { None };
        draw_target(&p, tile_rect(lvl.target.0, lvl.target.1), lvl.mode, lbl, sim.level_complete, 255);
    }

    // 5. Photons & tapered laser trails
    let mut rng = rand::thread_rng();
    for ph in &sim.photons {
        if ph.trail.len() > 1 {
            let n = ph.trail.len() as f32;
            for i in 1..ph.trail.len() {
                let (x1, y1, wl) = ph.trail[i - 1];
                let (x2, y2, _) = ph.trail[i];
                let col = if wl == 637 {
                    ACCENT_ORANGE
                } else if wl == 780 {
                    ACCENT_RED
                } else if ph.pid == "P" {
                    ACCENT_GREEN
                } else if ph.pid == "H" {
                    ACCENT_TEAL
                } else {
                    ACCENT_CYAN
                };
                let width = (5.0 * (i as f32 / n) * scale).max(1.0);
                p.line_segment([to_screen(x1, y1), to_screen(x2, y2)], Stroke::new(width, col));
            }
        }

        if ph.alive {
            let center = to_screen(ph.px, ph.py);
            let pcol = if ph.wavelength == 637 {
                ACCENT_ORANGE
            } else if ph.wavelength == 780 {
                ACCENT_RED
            } else if ph.pid == "P" {
                ACCENT_GREEN
            } else if ph.pid == "H" {
                ACCENT_TEAL
            } else {
                ACCENT_CYAN
            };
            draw_radial_glow(&p, center, 24.0 * scale, pcol, 150);

            if ph.noise > 10 {
                for _ in 0..3 {
                    let jx = center.x + rng.gen_range(-12.0..12.0) * scale;
                    let jy = center.y + rng.gen_range(-12.0..12.0) * scale;
                    p.circle_filled(Pos2::new(jx, jy), 2.2 * scale, ACCENT_AMBER);
                }
            }

            if ph.pol == Pol::Cross {
                let d = 9.0 * scale;
                p.line_segment([Pos2::new(center.x - d, center.y - d), Pos2::new(center.x + d, center.y + d)], Stroke::new(2.0 * scale, ACCENT_PURPLE));
                p.line_segment([Pos2::new(center.x - d, center.y + d), Pos2::new(center.x + d, center.y - d)], Stroke::new(2.0 * scale, ACCENT_PURPLE));
            }
            if ph.dispersion > 0 {
                p.circle_stroke(center, (11.0 + ph.dispersion.min(3) as f32 * 2.0) * scale, Stroke::new(1.5 * scale, Color32::from_rgb(125, 211, 252)));
            }

            p.circle_filled(center, 7.0 * scale, pcol);
            p.circle_filled(center, 3.0 * scale, TEXT_WHITE);

            if sim.colorblind_mode {
                let r = 12.0 * scale;
                match ph.wavelength {
                    780 => {
                        // Upward triangle marker for 780nm
                        let tri = vec![
                            Pos2::new(center.x, center.y - r),
                            Pos2::new(center.x + r * 0.9, center.y + r * 0.75),
                            Pos2::new(center.x - r * 0.9, center.y + r * 0.75),
                        ];
                        p.add(egui::Shape::closed_line(tri, Stroke::new(2.0 * scale, TEXT_WHITE)));
                    }
                    637 => {
                        // Diamond marker for 637nm
                        let dia = vec![
                            Pos2::new(center.x, center.y - r),
                            Pos2::new(center.x + r, center.y),
                            Pos2::new(center.x, center.y + r),
                            Pos2::new(center.x - r, center.y),
                        ];
                        p.add(egui::Shape::closed_line(dia, Stroke::new(2.0 * scale, TEXT_WHITE)));
                    }
                    _ => {
                        // Crisp high-contrast circle ring for 1550nm
                        p.circle_stroke(center, r, Stroke::new(2.0 * scale, TEXT_WHITE));
                    }
                }
                let badge_pos = Pos2::new(center.x, center.y - (r + 8.0 * scale));
                let badge_r = Rect::from_center_size(badge_pos, Vec2::new(30.0 * scale, 12.0 * scale));
                p.rect_filled(badge_r, CornerRadius::same(2), Color32::from_rgba_unmultiplied(10, 14, 22, 235));
                p.rect_stroke(badge_r, CornerRadius::same(2), Stroke::new(1.0_f32, TEXT_WHITE), StrokeKind::Inside);
                p.text(badge_pos, Align2::CENTER_CENTER, format!("{}", ph.wavelength), FontId::monospace((8.5 * scale).max(7.5)), TEXT_WHITE);
            }
        }
    }

    // 6. Shockwaves & Particles
    for sw in &sim.shockwaves {
        let prog = 1.0 - (sw.life as f32 / sw.max_life as f32);
        let r = (8.0 + prog * sw.max_r) * scale;
        let alpha = (200.0 * (1.0 - prog)).clamp(0.0, 255.0) as u8;
        p.circle_stroke(to_screen(sw.x, sw.y), r, Stroke::new(2.0 * scale, with_alpha(sw.color, alpha)));
    }
    for pt in &sim.particles {
        let alpha = (255.0 * (pt.life as f32 / pt.max_life as f32)).clamp(0.0, 255.0) as u8;
        p.circle_filled(to_screen(pt.x, pt.y), pt.size * scale, with_alpha(pt.color, alpha));
    }

    // 6b. Compact Alignment Laser Path-Length & Sync HUD pill (top-left of bench)
    if let Some(beams) = &align_beams {
        let primary: Vec<&AlignmentBeam> = beams.iter().filter(|b| b.label != "H" && b.label != "P").collect();
        if !primary.is_empty() {
            let mut parts: Vec<String> = primary
                .iter()
                .map(|b| {
                    let st = if b.blocked {
                        " BLOCKED"
                    } else if b.hit_target {
                        " OK"
                    } else {
                        ""
                    };
                    format!("{}:{}ps('{}'){}", b.label, b.total_ps(), b.final_pol.as_str(), st)
                })
                .collect();
            let mut pill_col = ACCENT_CYAN;
            if primary.len() == 2 && !sim.current_level().mode.is_qkd() {
                let dt = (primary[0].total_ps() as i32 - primary[1].total_ps() as i32).abs();
                let pol_ok = primary[0].final_pol == primary[1].final_pol;
                if dt == 0 && pol_ok && !primary[0].blocked && !primary[1].blocked {
                    parts.push("SYNC: 0ps OK".to_string());
                    pill_col = ACCENT_GREEN;
                } else {
                    parts.push(format!("Δt:{dt}ps{}", if pol_ok { "" } else { " POL!" }));
                    pill_col = ACCENT_AMBER;
                }
            }
            let hud_txt = format!("ALIGN  {}", parts.join("  •  "));
            let hud_font = FontId::monospace((10.5 * scale).clamp(9.0, 11.5));
            let galley = p_outer.layout_no_wrap(hud_txt, hud_font, pill_col);
            let pad = Vec2::new(10.0, 3.0);
            let box_sz = galley.size() + pad * 2.0;
            let hud_rect = Rect::from_min_size(
                Pos2::new(
                    board_rect.center().x - box_sz.x * 0.5,
                    (board_rect.top() - box_sz.y - 4.0).max(outer_rect.top() + 3.0),
                ),
                box_sz,
            );
            p_outer.rect_filled(hud_rect, CornerRadius::same(4), Color32::from_rgba_unmultiplied(12, 18, 28, 235));
            p_outer.rect_stroke(hud_rect, CornerRadius::same(4), Stroke::new(1.0_f32, with_alpha(pill_col, 160)), StrokeKind::Inside);
            p_outer.galley(hud_rect.left_top() + pad, galley, pill_col);
        }
    }

    // 7. Interactive hover tooltip for photons (when active/paused) or hardware tiles
    if !overlay_open {
        if let Some(pos) = resp.hover_pos() {
            let hovered_photon = sim.photons.iter().find(|ph| {
                ph.alive && to_screen(ph.px, ph.py).distance(pos) < 18.0 * scale
            });
            if let Some(ph) = hovered_photon {
                egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), egui::Id::new("photon_tip"), |ui| {
                    ui.label(egui::RichText::new(format!("Photon {} ({}nm)", ph.pid, ph.wavelength)).strong().color(ACCENT_CYAN));
                    let cd = if ph.dispersion > 0 { format!("  |  CD: {} (Needs DCF!)", ph.dispersion) } else { String::new() };
                    ui.label(format!("Fidelity: {}%  |  Noise: {}  |  Pol: '{}'{}", ph.fidelity, ph.noise, ph.pol.as_str(), cd));
                });
            } else if let Some((hgx, hgy)) = hover_tile {
                if let Some(key) = doc_key_at(sim, (hgx, hgy)) {
                    if let Some(doc) = hardware_doc(key) {
                        egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), egui::Id::new("tile_hw_tip"), |ui| {
                            ui.set_max_width(300.0);
                            ui.label(egui::RichText::new(doc.title).strong().size(12.0).color(doc.color));
                            ui.label(egui::RichText::new(doc.rule).size(11.0).color(TEXT_WHITE));
                            let sub = if sim.sandbox_mode {
                                "R-Click: Erase • Shift+Click: Full Spec"
                            } else if sim.user_placed.contains(&(hgx, hgy)) {
                                "R-Click: Remove • Shift+Click: Full Spec"
                            } else {
                                "Shift+Click: Open Hardware Spec"
                            };
                            ui.label(egui::RichText::new(sub).size(10.0).color(TEXT_MUTED));
                        });
                    }
                }
            }
        }
    }

    inspect_doc
}
