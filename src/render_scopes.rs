//! Bottom diagnostic & oscilloscope telemetry deck (Standard, HOM, QPU, and QKD/AES modes).

use crate::model::*;
use crate::qkd::QkdVerdict;
use crate::sim::BenchSim;
use egui::{Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

pub struct DeckAction {
    pub jump_level: Option<usize>,
    pub toggle_sandbox: bool,
    pub open_qkd_inspector: bool,
    pub open_hint: bool,
    pub open_lab_report: bool,
    pub open_video: bool,
}

fn draw_dashed_hline(p: &Painter, x1: f32, x2: f32, y: f32, color: Color32) {
    let mut x = x1;
    while x < x2 {
        p.line_segment([Pos2::new(x, y), Pos2::new((x + 5.0).min(x2), y)], Stroke::new(1.0_f32, color));
        x += 9.0;
    }
}

fn draw_scope_frame(p: &Painter, rect: Rect, title: &str, readout: &str, readout_col: Color32) {
    p.rect_filled(rect, CornerRadius::same(6), Color32::from_rgb(11, 15, 23));
    p.rect_stroke(rect, CornerRadius::same(6), Stroke::new(1.0_f32, Color32::from_rgb(38, 50, 68)), StrokeKind::Inside);

    let plot_top = rect.top() + 18.0;
    let plot_bot = rect.bottom() - 6.0;
    for frac in [0.25f32, 0.5, 0.75] {
        let gy = plot_top + (plot_bot - plot_top) * frac;
        p.line_segment([Pos2::new(rect.left() + 6.0, gy), Pos2::new(rect.right() - 6.0, gy)], Stroke::new(1.0_f32, Color32::from_rgb(20, 28, 40)));
        let gx = rect.left() + 6.0 + (rect.width() - 12.0) * frac;
        p.line_segment([Pos2::new(gx, plot_top), Pos2::new(gx, plot_bot)], Stroke::new(1.0_f32, Color32::from_rgb(20, 28, 40)));
    }
    p.text(Pos2::new(rect.left() + 7.0, rect.top() + 4.0), Align2::LEFT_TOP, title, FontId::proportional(10.5), TEXT_MUTED);
    if !readout.is_empty() {
        p.text(Pos2::new(rect.right() - 7.0, rect.top() + 4.0), Align2::RIGHT_TOP, readout, FontId::proportional(10.5), readout_col);
    }
}

pub fn draw_telemetry_deck(ui: &mut Ui, sim: &mut BenchSim, has_video: bool) -> DeckAction {
    let mut action = DeckAction {
        jump_level: None,
        toggle_sandbox: false,
        open_qkd_inspector: false,
        open_hint: false,
        open_lab_report: false,
        open_video: false,
    };
    let header_title = if sim.sandbox_mode {
        format!("SANDBOX: {}", sim.sandbox_bp.name)
    } else {
        sim.current_level().title.to_string()
    };
    let header_col = if sim.sandbox_mode { ACCENT_PURPLE } else { ACCENT_CYAN };
    let lvl_mode = sim.current_level().mode;
    let total_levels = sim.levels.len();

    // Row 1: Level title (left) + Speed / Pause / Clock / Status badge (right)
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Status badge
            let (st_fg, st_bg) = sim.sim_state.colors();
            egui::Frame::NONE
                .fill(st_bg)
                .stroke(Stroke::new(1.0_f32, st_fg))
                .corner_radius(CornerRadius::same(4))
                .inner_margin(egui::Margin::symmetric(8, 2))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(sim.sim_state.label()).strong().size(10.5).color(st_fg));
                });

            // Picosecond clock
            ui.label(
                egui::RichText::new(format!("T+{:04}ps", sim.tick * 5))
                    .monospace()
                    .size(11.0)
                    .color(TEXT_MUTED),
            );

            // Single-Step (+5ps) Frame Debugger button (always enabled; auto-fires paused if READY)
            let step_active = sim.sim_active && sim.paused;
            let step_btn = egui::Button::new(
                egui::RichText::new("STEP")
                    .strong()
                    .size(9.5)
                    .color(if step_active { BG_DARK } else { ACCENT_CYAN }),
            )
            .fill(if step_active { ACCENT_CYAN } else { Color32::from_rgb(18, 34, 48) })
            .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
            .corner_radius(CornerRadius::same(4))
            .min_size(Vec2::new(42.0, 20.0));
            if ui
                .add(step_btn)
                .on_hover_text("Step forward +5ps (auto-fires paused if Ready) [.]")
                .clicked()
            {
                sim.step_one_paused_frame();
                sim.pending_sfx.push(crate::sim::SfxEvent::Select);
            }

            // Play / Pause & Speed controls (ASCII-safe '>' and '||'; clicking '>' when READY fires)
            let is_running = sim.sim_active && !sim.paused;
            let pause_txt = if is_running { "||" } else { ">" };
            let pause_col = if sim.sim_active && sim.paused {
                ACCENT_AMBER
            } else if is_running {
                TEXT_WHITE
            } else {
                ACCENT_GREEN
            };
            let pause_bg = if sim.sim_active && sim.paused {
                Color32::from_rgb(52, 40, 18)
            } else if is_running {
                Color32::from_rgb(20, 28, 40)
            } else {
                Color32::from_rgb(16, 40, 32)
            };
            let pause_bd = if sim.sim_active && sim.paused {
                ACCENT_AMBER
            } else if is_running {
                GRID_LINE
            } else {
                ACCENT_GREEN
            };
            let pause_tip = if !sim.sim_active {
                "Fire laser / Start playback [SPACE]"
            } else if sim.paused {
                "Resume simulation [P]"
            } else {
                "Pause simulation [P]"
            };
            let pause_btn = egui::Button::new(egui::RichText::new(pause_txt).strong().size(10.5).color(pause_col))
                .fill(pause_bg)
                .stroke(Stroke::new(1.0_f32, pause_bd))
                .corner_radius(CornerRadius::same(4))
                .min_size(Vec2::new(26.0, 20.0));
            if ui.add(pause_btn).on_hover_text(pause_tip).clicked() {
                if sim.sim_active {
                    sim.paused = !sim.paused;
                    sim.pending_sfx.push(crate::sim::SfxEvent::Select);
                } else {
                    sim.fire_photons();
                }
            }

            let spd_label = if (sim.sim_speed - 0.5).abs() < 0.05 {
                "0.5x"
            } else if (sim.sim_speed - 2.0).abs() < 0.05 {
                "2x"
            } else {
                "1x"
            };
            let spd_btn = egui::Button::new(egui::RichText::new(spd_label).size(10.5).color(ACCENT_CYAN))
                .fill(Color32::from_rgb(20, 28, 40))
                .stroke(Stroke::new(1.0_f32, GRID_LINE))
                .min_size(Vec2::new(30.0, 20.0));
            if ui.add(spd_btn).on_hover_text("Cycle simulation speed (0.5x / 1x / 2x)").clicked() {
                sim.sim_speed = if (sim.sim_speed - 1.0).abs() < 0.05 {
                    2.0
                } else if (sim.sim_speed - 2.0).abs() < 0.05 {
                    0.5
                } else {
                    1.0
                };
            }

            // Quantum Lab Report + Stars pill
            let total_stars = sim.total_stars();
            let max_stars = (total_levels * 3) as u32;
            let rep_lbl = format!("REPORT {total_stars}/{max_stars}*");
            let rep_btn = egui::Button::new(egui::RichText::new(rep_lbl).strong().size(9.5).color(ACCENT_AMBER))
                .fill(Color32::from_rgb(32, 26, 18))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(135, 102, 36)))
                .corner_radius(CornerRadius::same(4))
                .min_size(Vec2::new(82.0, 20.0));
            if ui.add(rep_btn).on_hover_text("Open Quantum Lab Report & Star Ratings [G]").clicked() {
                action.open_lab_report = true;
            }

            // Title occupies remaining left space cleanly without overflow
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add(egui::Label::new(egui::RichText::new(header_title).strong().size(14.5).color(header_col)).truncate());
            });
        });
    });

    ui.add_space(3.0);

    // Row 2: Level pills 1..20 + SANDBOX (left) and UNDO / REDO / ALIGN / HINT (right)
    let narrow = ui.available_width() < 640.0;
    let pill_w = if narrow { 13.0 } else { 16.5 };
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for i in 0..total_levels {
            let is_cur = !sim.sandbox_mode && i == sim.level_idx;
            let is_done = sim.completed_levels.contains(&i);
            let stars = sim.level_stars.get(&i).copied().unwrap_or(if is_done { 1 } else { 0 });
            let bg = if is_cur { Color32::from_rgb(36, 56, 78) } else { Color32::from_rgb(18, 24, 34) };
            let bd = if is_cur {
                ACCENT_CYAN
            } else if stars >= 3 {
                ACCENT_AMBER
            } else if is_done {
                ACCENT_GREEN
            } else {
                GRID_LINE
            };
            let fg = if is_cur {
                TEXT_WHITE
            } else if stars >= 3 {
                ACCENT_AMBER
            } else if is_done {
                ACCENT_GREEN
            } else {
                TEXT_MUTED
            };
            let btn = egui::Button::new(egui::RichText::new(format!("{}", i + 1)).strong().size(if narrow { 8.5 } else { 9.5 }).color(fg))
                .fill(bg)
                .stroke(Stroke::new(1.0_f32, bd))
                .corner_radius(CornerRadius::same(4))
                .min_size(Vec2::new(pill_w, 20.0));
            let hover_txt = if is_done {
                format!("{}  ({}/3 Stars)", sim.levels[i].title, stars)
            } else {
                sim.levels[i].title.to_string()
            };
            if ui.add(btn).on_hover_text(hover_txt).clicked() {
                action.jump_level = Some(i);
            }
        }

        ui.add_space(2.0);
        let sb_on = sim.sandbox_mode;
        let sb_bg = if sb_on { Color32::from_rgb(56, 36, 92) } else { Color32::from_rgb(24, 22, 38) };
        let sb_bd = if sb_on { ACCENT_PURPLE } else { Color32::from_rgb(84, 68, 124) };
        let sb_fg = if sb_on { TEXT_WHITE } else { ACCENT_PURPLE };
        let sb_btn = egui::Button::new(egui::RichText::new("SANDBOX").strong().size(if narrow { 8.5 } else { 9.5 }).color(sb_fg))
            .fill(sb_bg)
            .stroke(Stroke::new(1.0_f32, sb_bd))
            .corner_radius(CornerRadius::same(4))
            .min_size(Vec2::new(if narrow { 46.0 } else { 54.0 }, 20.0));
        if ui.add(sb_btn).on_hover_text("Toggle Free-Build Sandbox Mode [S]").clicked() {
            action.toggle_sandbox = true;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 3.0;

            if !sim.sandbox_mode && has_video {
                let v_btn = egui::Button::new(egui::RichText::new("VIDEO").strong().size(9.0).color(TEXT_WHITE))
                    .fill(Color32::from_rgb(120, 24, 34))
                    .stroke(Stroke::new(1.0_f32, ACCENT_RED))
                    .corner_radius(CornerRadius::same(4))
                    .min_size(Vec2::new(if narrow { 34.0 } else { 40.0 }, 20.0));
                if ui.add(v_btn).on_hover_text("Watch Lab Video Guide on YouTube [V]").clicked() {
                    action.open_video = true;
                }
            }

            if !sim.sandbox_mode {
                let is_fault = sim.sim_state == SimState::Fault;
                let h_bg = if is_fault { Color32::from_rgb(50, 38, 16) } else { Color32::from_rgb(20, 28, 40) };
                let h_bd = if is_fault { ACCENT_AMBER } else { GRID_LINE };
                let h_fg = if is_fault { ACCENT_AMBER } else { TEXT_MUTED };
                let h_btn = egui::Button::new(egui::RichText::new("HINT").strong().size(9.0).color(h_fg))
                    .fill(h_bg)
                    .stroke(Stroke::new(1.0_f32, h_bd))
                    .corner_radius(CornerRadius::same(4))
                    .min_size(Vec2::new(if narrow { 30.0 } else { 36.0 }, 20.0));
                if ui.add(h_btn).on_hover_text("Reveal Progressive Level Hint [I]").clicked() {
                    action.open_hint = true;
                }
            }

            let al_on = sim.align_laser;
            let al_bg = if al_on { Color32::from_rgb(18, 42, 54) } else { Color32::from_rgb(20, 26, 36) };
            let al_bd = if al_on { ACCENT_CYAN } else { GRID_LINE };
            let al_fg = if al_on { ACCENT_CYAN } else { TEXT_MUTED };
            let al_btn = egui::Button::new(egui::RichText::new("ALIGN").strong().size(9.0).color(al_fg))
                .fill(al_bg)
                .stroke(Stroke::new(1.0_f32, al_bd))
                .corner_radius(CornerRadius::same(4))
                .min_size(Vec2::new(if narrow { 34.0 } else { 40.0 }, 20.0));
            if ui.add(al_btn).on_hover_text("Toggle Alignment Laser & Path-Length Timing [L]").clicked() {
                sim.align_laser = !sim.align_laser;
                sim.pending_sfx.push(crate::sim::SfxEvent::Select);
            }

            let can_redo = sim.can_redo();
            let r_btn = egui::Button::new(
                egui::RichText::new("REDO")
                    .strong()
                    .size(9.0)
                    .color(if can_redo { TEXT_WHITE } else { Color32::from_rgb(64, 76, 94) }),
            )
            .fill(Color32::from_rgb(20, 26, 36))
            .stroke(Stroke::new(1.0_f32, if can_redo { GRID_LINE } else { Color32::from_rgb(28, 34, 46) }))
            .corner_radius(CornerRadius::same(4))
            .min_size(Vec2::new(if narrow { 30.0 } else { 34.0 }, 20.0));
            if ui.add_enabled(can_redo, r_btn).on_hover_text("Redo [Ctrl/Cmd+Shift+Z]").clicked() {
                sim.redo();
            }

            let can_undo = sim.can_undo();
            let u_btn = egui::Button::new(
                egui::RichText::new("UNDO")
                    .strong()
                    .size(9.0)
                    .color(if can_undo { TEXT_WHITE } else { Color32::from_rgb(64, 76, 94) }),
            )
            .fill(Color32::from_rgb(20, 26, 36))
            .stroke(Stroke::new(1.0_f32, if can_undo { GRID_LINE } else { Color32::from_rgb(28, 34, 46) }))
            .corner_radius(CornerRadius::same(4))
            .min_size(Vec2::new(if narrow { 30.0 } else { 34.0 }, 20.0));
            if ui.add_enabled(can_undo, u_btn).on_hover_text("Undo [Ctrl/Cmd+Z]").clicked() {
                sim.undo();
            }
        });
    });

    ui.add_space(3.0);

    // Row 3: Full-width status log strip (zero button collisions!)
    egui::Frame::NONE
        .fill(Color32::from_rgb(16, 22, 32))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(32, 42, 58)))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                if let Some(stars) = sim.last_earned_stars {
                    let tag = match stars {
                        3 => "[*** 3/3 OPTIMAL]",
                        2 => "[** 2/3]",
                        _ => "[* 1/3]",
                    };
                    ui.label(egui::RichText::new(tag).strong().size(11.5).color(ACCENT_AMBER));
                }
                ui.add(
                    egui::Label::new(egui::RichText::new(&sim.status_msg).size(12.0).color(sim.status_color))
                        .truncate(),
                );
            });
        });

    ui.add_space(5.0);

    // Row 4: 3 Oscilloscope Panes (fluidly fit any width!)
    let scope_h = 98.0;
    let total_w = ui.available_width();
    let gap = 8.0;
    let pane_w = ((total_w - 2.0 * gap) / 3.0).max(110.0);

    if lvl_mode.is_qkd() {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            draw_qkd_panes(ui, sim, pane_w, scope_h, &mut action);
        });
    } else {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            draw_standard_panes(ui, sim, lvl_mode, pane_w, scope_h);
        });
    }

    action
}

fn draw_qkd_panes(ui: &mut Ui, sim: &BenchSim, pane_w: f32, scope_h: f32, action: &mut DeckAction) {
    let Some(qs) = &sim.qkd_session else { return };

    // Pane 1: QBER & Bell CHSH
    let (p1, _) = ui.allocate_exact_size(Vec2::new(pane_w, scope_h), Sense::hover());
    let p = ui.painter_at(p1);
    let qber_pct = qs.qber * 100.0;
    let (q_str, q_col) = if !qs.sifted_alice.is_empty() {
        let s = match qs.chsh_s {
            Some(sv) => format!("QBER:{qber_pct:.0}%  S:{sv:.2}"),
            None => format!("QBER:{qber_pct:.1}%"),
        };
        let ok = qs.qber <= crate::qkd::QkdSession::QBER_THRESHOLD && qs.chsh_s.map_or(true, |sv| sv > 2.0);
        (s, if ok { ACCENT_GREEN } else { ACCENT_RED })
    } else {
        let tag = if sim.current_level().mode == TargetMode::QkdE91 { "E91 BELL" } else { "BB84 QBER" };
        (tag.to_string(), TEXT_MUTED)
    };
    let compact = pane_w < 215.0;
    let q_title = if compact { "QBER" } else { "QBER / BELL CHSH" };
    draw_scope_frame(&p, p1, q_title, &q_str, q_col);
    let qx1 = p1.left() + 8.0;
    let qx2 = p1.right() - 8.0;
    let qy1 = p1.top() + 24.0;
    let qy2 = p1.bottom() - 8.0;
    let qh = qy2 - qy1;
    let qthresh_y = qy2 - (11.0 / 60.0) * qh;
    draw_dashed_hline(&p, qx1, qx2, qthresh_y, ACCENT_RED);
    p.text(Pos2::new(qx1 + 2.0, qthresh_y - 12.0), Align2::LEFT_TOP, "MAX 11%", FontId::proportional(10.0), Color32::from_rgb(210, 85, 105));
    if qs.qber_history.len() >= 2 {
        let max_n = qs.qber_history.last().map(|(n, _)| (*n).max(12)).unwrap_or(12) as f32;
        let pts: Vec<Pos2> = qs.qber_history.iter().map(|&(n, qp)| {
            Pos2::new(qx1 + (n as f32 / max_n) * (qx2 - qx1), qy2 - (qp.min(60.0) as f32 / 60.0) * qh)
        }).collect();
        let lcol = if qs.qber <= crate::qkd::QkdSession::QBER_THRESHOLD { ACCENT_GREEN } else { ACCENT_RED };
        if let Some(&last) = pts.last() {
            p.add(egui::Shape::line(pts, Stroke::new(2.0_f32, lcol)));
            p.circle_filled(last, 3.0, lcol);
        }
    }

    // Pane 2: Live Basis Sifting Stream
    let (p2, _) = ui.allocate_exact_size(Vec2::new(pane_w, scope_h), Sense::hover());
    let p = ui.painter_at(p2);
    let s_str = if !qs.rounds.is_empty() {
        if compact {
            format!("{}/{} SIFTED", qs.sifted_alice.len(), qs.rounds.len())
        } else {
            format!("{} SIFTED / {} RX", qs.sifted_alice.len(), qs.rounds.len())
        }
    } else {
        "AWAITING QUBITS".to_string()
    };
    let s_col = if qs.sifted_alice.len() >= crate::qkd::QkdSession::MIN_SIFTED_BITS {
        ACCENT_GREEN
    } else if !qs.rounds.is_empty() {
        ACCENT_CYAN
    } else {
        TEXT_MUTED
    };
    draw_scope_frame(&p, p2, if compact { "SIFTING" } else { "BASIS SIFTING" }, &s_str, s_col);
    p.text(Pos2::new(p2.left() + 8.0, p2.top() + 28.0), Align2::LEFT_TOP, "A:", FontId::proportional(10.5), TEXT_MUTED);
    p.text(Pos2::new(p2.left() + 8.0, p2.top() + 47.0), Align2::LEFT_TOP, "B:", FontId::proportional(10.5), TEXT_MUTED);
    p.text(Pos2::new(p2.left() + 8.0, p2.top() + 69.0), Align2::LEFT_TOP, "KEY:", FontId::proportional(10.5), TEXT_MUTED);

    let max_cols = (((p2.width() - 44.0) / 22.0).floor() as usize).clamp(5, 11);
    let start = qs.rounds.len().saturating_sub(max_cols);
    let mut col_x = p2.left() + 38.0;
    for rd in &qs.rounds[start..] {
        let cr = Rect::from_min_size(Pos2::new(col_x, p2.top() + 24.0), Vec2::new(20.0, 68.0));
        let (bg_c, bd_c) = if rd.matched {
            if rd.error {
                (Color32::from_rgb(48, 22, 28), ACCENT_RED)
            } else {
                (Color32::from_rgb(18, 46, 36), ACCENT_GREEN)
            }
        } else {
            (Color32::from_rgb(16, 22, 32), Color32::from_rgb(30, 38, 52))
        };
        p.rect_filled(cr, CornerRadius::same(3), bg_c);
        p.rect_stroke(cr, CornerRadius::same(3), Stroke::new(1.0_f32, bd_c), StrokeKind::Inside);
        let ac = if rd.a_basis == Pol::Plus { ACCENT_CYAN } else { ACCENT_PURPLE };
        let bc = if rd.b_basis == Pol::Plus { ACCENT_CYAN } else { ACCENT_PURPLE };
        p.text(Pos2::new(cr.center().x, p2.top() + 34.0), Align2::CENTER_CENTER, rd.a_basis.as_str(), FontId::proportional(11.0), ac);
        p.text(Pos2::new(cr.center().x, p2.top() + 53.0), Align2::CENTER_CENTER, rd.b_basis.as_str(), FontId::proportional(11.0), bc);
        if rd.matched {
            let kv = if rd.error { "!".to_string() } else { rd.b_bit.to_string() };
            let kc = if rd.error { ACCENT_RED } else { ACCENT_GREEN };
            p.text(Pos2::new(cr.center().x, p2.top() + 75.0), Align2::CENTER_CENTER, kv, FontId::proportional(11.0), kc);
        }
        col_x += 22.0;
    }

    // Pane 3: AES-256 Engine + Inspect Button
    let (p3, _) = ui.allocate_exact_size(Vec2::new(pane_w, scope_h), Sense::hover());
    let p = ui.painter_at(p3);
    let (a_str, a_col) = match qs.verdict {
        QkdVerdict::Secure => ("256-BIT LOCKED", ACCENT_GREEN),
        QkdVerdict::AbortEve => ("ABORTED!", ACCENT_RED),
        QkdVerdict::Running => ("DISTILLING...", ACCENT_AMBER),
        _ => ("STANDBY", TEXT_MUTED),
    };
    draw_scope_frame(&p, p3, if compact { "AES-256" } else { "AES-256 ENGINE" }, a_str, a_col);

    if let Some(ak) = qs.alice_key {
        let kh = crate::qkd::hex_upper(&ak);
        p.text(Pos2::new(p3.left() + 10.0, p3.top() + 28.0), Align2::LEFT_TOP, format!("KEY: {}...{}", &kh[..10], &kh[kh.len() - 8..]), FontId::monospace(10.5), ACCENT_GREEN);
        let ct_prev: String = qs.ciphertext_hex.chars().take(18).collect();
        p.text(Pos2::new(p3.left() + 10.0, p3.top() + 45.0), Align2::LEFT_TOP, format!("CTR: {ct_prev}..."), FontId::monospace(10.5), ACCENT_CYAN);
    } else if qs.verdict == QkdVerdict::AbortEve {
        p.text(Pos2::new(p3.left() + 10.0, p3.top() + 30.0), Align2::LEFT_TOP, "KEY DISCARDED — COMPROMISED", FontId::proportional(10.5), ACCENT_RED);
        p.text(Pos2::new(p3.left() + 10.0, p3.top() + 46.0), Align2::LEFT_TOP, "Fix PMD drift or bypass EVE.", FontId::proportional(10.5), TEXT_MUTED);
    } else {
        p.text(Pos2::new(p3.left() + 10.0, p3.top() + 30.0), Align2::LEFT_TOP, "Fire photons to sift & derive", FontId::proportional(10.5), TEXT_MUTED);
        p.text(Pos2::new(p3.left() + 10.0, p3.top() + 46.0), Align2::LEFT_TOP, "a real FIPS-197 AES-256 key.", FontId::proportional(10.5), TEXT_MUTED);
    }

    let btn_rect = Rect::from_min_max(Pos2::new(p3.left() + 10.0, p3.bottom() - 30.0), Pos2::new(p3.right() - 10.0, p3.bottom() - 8.0));
    let btn_resp = ui.interact(btn_rect, ui.id().with("qkd_inspect_btn"), Sense::click());
    let is_sec = qs.verdict == QkdVerdict::Secure;
    let ib_bg = if is_sec {
        if btn_resp.hovered() { Color32::from_rgb(30, 82, 62) } else { Color32::from_rgb(22, 62, 48) }
    } else if btn_resp.hovered() {
        Color32::from_rgb(34, 48, 68)
    } else {
        Color32::from_rgb(20, 28, 42)
    };
    let ib_bd = if is_sec { ACCENT_GREEN } else if btn_resp.hovered() { ACCENT_CYAN } else { GRID_LINE };
    p.rect_filled(btn_rect, CornerRadius::same(4), ib_bg);
    p.rect_stroke(btn_rect, CornerRadius::same(4), Stroke::new(1.0_f32, ib_bd), StrokeKind::Inside);
    p.text(btn_rect.center(), Align2::CENTER_CENTER, "INSPECT QKD & AES-256  [K]", FontId::proportional(10.5), if is_sec { ACCENT_GREEN } else { TEXT_WHITE });
    if btn_resp.clicked() {
        action.open_qkd_inspector = true;
    }
}

fn draw_standard_panes(ui: &mut Ui, sim: &BenchSim, mode: TargetMode, pane_w: f32, scope_h: f32) {
    let min_fid = if mode == TargetMode::Qpu { 90 } else { 60 };

    // Pane 1: Fidelity Oscilloscope Trace
    let (p1, _) = ui.allocate_exact_size(Vec2::new(pane_w, scope_h), Sense::hover());
    let p = ui.painter_at(p1);
    let (fid_str, fid_col) = if !sim.photons.is_empty() {
        let start = sim.photons.len().saturating_sub(2);
        let s = sim.photons[start..]
            .iter()
            .map(|ph| format!("{}:{}%", ph.pid, ph.fidelity))
            .collect::<Vec<_>>()
            .join(" ");
        let ok = sim.photons.iter().any(|ph| ph.fidelity >= min_fid);
        (s, if ok { ACCENT_GREEN } else { ACCENT_RED })
    } else {
        ("STANDBY".to_string(), TEXT_MUTED)
    };
    draw_scope_frame(&p, p1, "FIDELITY TRACE", &fid_str, fid_col);
    let px1 = p1.left() + 8.0;
    let px2 = p1.right() - 8.0;
    let py1 = p1.top() + 24.0;
    let py2 = p1.bottom() - 8.0;
    let ph_h = py2 - py1;
    let thresh_y = py2 - (min_fid as f32 / 100.0) * ph_h;
    draw_dashed_hline(&p, px1, px2, thresh_y, Color32::from_rgb(180, 70, 90));
    p.text(Pos2::new(px1 + 2.0, (thresh_y - 12.0).max(py1 - 2.0)), Align2::LEFT_TOP, format!("{min_fid}%"), FontId::proportional(10.0), Color32::from_rgb(180, 80, 100));

    for (idx, ph) in sim.photons.iter().enumerate() {
        if ph.fid_history.len() >= 2 {
            let max_t = 40u32.max(sim.tick).max(ph.fid_history.last().unwrap().0) as f32;
            let mut prev: Option<(Pos2, u32)> = None;
            for &(t_val, fid_val, wl) in &ph.fid_history {
                let pt = Pos2::new(px1 + (t_val as f32 / max_t) * (px2 - px1), py2 - (fid_val as f32 / 100.0) * ph_h);
                if let Some((pp, _)) = prev {
                    let col = if wl == 637 {
                        ACCENT_ORANGE
                    } else if wl == 780 {
                        ACCENT_RED
                    } else if ph.pid == "P" {
                        ACCENT_GREEN
                    } else if idx == 0 {
                        ACCENT_CYAN
                    } else {
                        ACCENT_PURPLE
                    };
                    p.line_segment([pp, pt], Stroke::new(2.0_f32, col));
                }
                prev = Some((pt, wl));
            }
            if let Some((last_pt, last_wl)) = prev {
                let lcol = if last_wl == 637 {
                    ACCENT_ORANGE
                } else if last_wl == 780 {
                    ACCENT_RED
                } else if ph.pid == "P" {
                    ACCENT_GREEN
                } else if idx == 0 {
                    ACCENT_CYAN
                } else {
                    ACCENT_PURPLE
                };
                p.circle_filled(last_pt, 3.0, lcol);
            }
        }
    }

    // Pane 2: Raman Noise + 8-segment LED Meter
    let (p2, _) = ui.allocate_exact_size(Vec2::new(pane_w, scope_h), Sense::hover());
    let p = ui.painter_at(p2);
    let max_noise = sim.photons.iter().map(|ph| ph.noise).max().unwrap_or(0);
    let (n_str, n_col) = if !sim.photons.is_empty() {
        let s = sim.photons.iter().map(|ph| format!("{}:{}", ph.pid, ph.noise)).collect::<Vec<_>>().join(" ");
        (s, if max_noise <= 10 { ACCENT_GREEN } else { ACCENT_AMBER })
    } else {
        ("CLEAN".to_string(), TEXT_MUTED)
    };
    draw_scope_frame(&p, p2, "RAMAN NOISE", &n_str, n_col);
    let nx1 = p2.left() + 8.0;
    let nx2 = p2.right() - 34.0;
    let ny1 = p2.top() + 24.0;
    let ny2 = p2.bottom() - 8.0;
    let nh = ny2 - ny1;
    let nthresh_y = ny2 - (10.0 / 50.0) * nh;
    draw_dashed_hline(&p, nx1, nx2, nthresh_y, ACCENT_AMBER);
    p.text(Pos2::new(nx1 + 2.0, nthresh_y - 12.0), Align2::LEFT_TOP, "MAX 10", FontId::proportional(10.0), Color32::from_rgb(185, 145, 50));

    for (idx, ph) in sim.photons.iter().enumerate() {
        if ph.noise_history.len() >= 2 {
            let max_t = 40u32.max(sim.tick).max(ph.noise_history.last().unwrap().0) as f32;
            let pts: Vec<Pos2> = ph.noise_history.iter().map(|&(t_val, n_val)| {
                Pos2::new(nx1 + (t_val as f32 / max_t) * (nx2 - nx1), ny2 - (n_val.min(50) as f32 / 50.0) * nh)
            }).collect();
            let col = if ph.noise > 10 { ACCENT_AMBER } else if idx == 0 { ACCENT_CYAN } else { ACCENT_PURPLE };
            if let Some(&last) = pts.last() {
                p.add(egui::Shape::line(pts, Stroke::new(2.0_f32, col)));
                p.circle_filled(last, 3.0, col);
            }
        }
    }

    let led_x = p2.right() - 22.0;
    let segs = 8usize;
    let seg_h = ((nh - (segs - 1) as f32 * 2.0) / segs as f32).max(3.0);
    let lit = if !sim.photons.is_empty() {
        ((max_noise.min(50) as f32 / 50.0) * segs as f32).round() as usize
    } else {
        0
    };
    for s_idx in 0..segs {
        let sy = ny2 - (s_idx + 1) as f32 * seg_h - s_idx as f32 * 2.0;
        let col = if s_idx < lit {
            if s_idx < 2 { ACCENT_GREEN } else if s_idx < 5 { ACCENT_AMBER } else { ACCENT_RED }
        } else {
            Color32::from_rgb(24, 32, 44)
        };
        p.rect_filled(Rect::from_min_size(Pos2::new(led_x, sy), Vec2::new(14.0, seg_h)), CornerRadius::same(2), col);
    }

    // Pane 3: HOM Coincidence Dip (BSM / Teleport) or Spectrum / Lock (SNSPD / QPU)
    let (p3, _) = ui.allocate_exact_size(Vec2::new(pane_w, scope_h), Sense::hover());
    let p = ui.painter_at(p3);
    if mode == TargetMode::Bsm || mode == TargetMode::Teleport {
        let (hom_str, hom_col) = if sim.pol_mismatch {
            ("POL MISMATCH".to_string(), ACCENT_RED)
        } else if mode == TargetMode::Teleport && sim.teleport_heralded {
            ("HERALDED |Psi>".to_string(), ACCENT_GREEN)
        } else if let (Some(signed), Some(delta)) = (sim.hom_signed_ps, sim.hom_delta_ps) {
            let sign = if signed > 0 { "+" } else { "" };
            (format!("dt:{sign}{signed}ps"), if delta <= 15 { ACCENT_GREEN } else { ACCENT_RED })
        } else if !sim.photons.is_empty() {
            ("SYNCING...".to_string(), ACCENT_AMBER)
        } else {
            ("WINDOW <=15ps".to_string(), TEXT_MUTED)
        };
        let p3_title = if mode == TargetMode::Teleport { "TELEPORT BSM" } else { "HOM INTERFERENCE" };
        draw_scope_frame(&p, p3, p3_title, &hom_str, hom_col);

        let hx1 = p3.left() + 10.0;
        let hx2 = p3.right() - 10.0;
        let hy1 = p3.top() + 24.0;
        let hy2 = p3.bottom() - 18.0;
        let hh = hy2 - hy1;
        let hmid = (hx1 + hx2) * 0.5;
        let win_half = (15.0 / 100.0) * ((hx2 - hx1) * 0.5);
        p.rect_filled(
            Rect::from_min_max(Pos2::new(hmid - win_half, hy1), Pos2::new(hmid + win_half, hy2)),
            0.0,
            Color32::from_rgba_unmultiplied(ACCENT_GREEN.r(), ACCENT_GREEN.g(), ACCENT_GREEN.b(), 38),
        );
        let dip_pts: Vec<Pos2> = (0..=40).map(|i| {
            let frac = i as f32 / 40.0;
            let dt = -100.0 + 200.0 * frac;
            let val = 1.0 - (-(dt / 22.0).powi(2)).exp();
            Pos2::new(hx1 + frac * (hx2 - hx1), hy2 - val * hh)
        }).collect();
        p.add(egui::Shape::line(dip_pts, Stroke::new(2.0_f32, Color32::from_rgb(70, 120, 160))));

        if let (Some(signed), Some(delta)) = (sim.hom_signed_ps, sim.hom_delta_ps) {
            let clamped = (signed as f32).clamp(-100.0, 100.0);
            let mx_pos = hmid + (clamped / 100.0) * ((hx2 - hx1) * 0.5);
            let val = 1.0 - (-(clamped / 22.0).powi(2)).exp();
            let my_pos = hy2 - val * hh;
            let mcol = if delta <= 15 && !sim.pol_mismatch { ACCENT_GREEN } else { ACCENT_RED };
            p.line_segment([Pos2::new(mx_pos, hy1), Pos2::new(mx_pos, hy2)], Stroke::new(2.0_f32, mcol));
            p.circle_filled(Pos2::new(mx_pos, my_pos), 4.0, mcol);
        }
        p.text(Pos2::new(hx1, p3.bottom() - 14.0), Align2::LEFT_TOP, "-100ps", FontId::proportional(10.0), TEXT_MUTED);
        p.text(Pos2::new(hmid, p3.bottom() - 14.0), Align2::CENTER_TOP, "0", FontId::proportional(10.0), ACCENT_GREEN);
        p.text(Pos2::new(hx2, p3.bottom() - 14.0), Align2::RIGHT_TOP, "+100ps", FontId::proportional(10.0), TEXT_MUTED);
    } else {
        let p0 = sim.photons.last();
        let is_qpu = mode == TargetMode::Qpu;
        let (wl_str, wl_col) = match p0 {
            Some(ph) => {
                let c = if ph.wavelength == 1550 { ACCENT_CYAN } else if ph.wavelength == 637 { ACCENT_ORANGE } else { ACCENT_RED };
                (format!("{}nm", ph.wavelength), c)
            }
            None => (if is_qpu { "QPU >=90%" } else { "SNSPD 1.5K" }.to_string(), TEXT_MUTED),
        };
        draw_scope_frame(&p, p3, "SPECTRUM / LOCK", &wl_str, wl_col);

        let sx1 = p3.left() + 10.0;
        let sx2 = p3.right() - 10.0;
        let sy1 = p3.top() + 24.0;
        let sy2 = p3.bottom() - 28.0;
        let sh = sy2 - sy1;
        let (center_frac, peak_col) = match p0.map(|ph| ph.wavelength) {
            None | Some(780) => (0.25f32, ACCENT_RED),
            Some(637) => (0.16, ACCENT_ORANGE),
            Some(_) => (0.75, ACCENT_CYAN),
        };
        let amp = if p0.is_some() { 0.92 } else { 0.20 };
        let spec_pts: Vec<Pos2> = (0..=40).map(|i| {
            let f = i as f32 / 40.0;
            let mut y_val = amp * (-((f - center_frac) / 0.065).powi(2)).exp();
            if let Some(ph) = p0 {
                if ph.noise > 0 {
                    y_val = (y_val + (ph.noise as f32 / 85.0) * (-((f - 0.56) / 0.14).powi(2)).exp()).min(1.0);
                }
            }
            Pos2::new(sx1 + f * (sx2 - sx1), sy2 - y_val * sh)
        }).collect();
        p.add(egui::Shape::line(spec_pts, Stroke::new(2.0_f32, peak_col)));

        let checks = [
            ("1550nm".to_string(), p0.map_or(false, |ph| ph.wavelength == 1550)),
            ("NOISE<=10".to_string(), p0.map_or(false, |ph| ph.noise <= 10)),
            (format!("FID>={min_fid}%"), p0.map_or(false, |ph| ph.fidelity >= min_fid)),
        ];
        let pw = (p3.width() - 24.0) / 3.0;
        for (i, (label, ok)) in checks.iter().enumerate() {
            let pr = Rect::from_min_size(Pos2::new(p3.left() + 8.0 + i as f32 * (pw + 4.0), p3.bottom() - 22.0), Vec2::new(pw, 16.0));
            let (bg, fg) = if p0.is_none() {
                (Color32::from_rgb(20, 26, 36), TEXT_MUTED)
            } else if *ok {
                (Color32::from_rgb(18, 48, 38), ACCENT_GREEN)
            } else {
                (Color32::from_rgb(52, 22, 28), ACCENT_RED)
            };
            p.rect_filled(pr, CornerRadius::same(3), bg);
            p.rect_stroke(pr, CornerRadius::same(3), Stroke::new(1.0_f32, fg), StrokeKind::Inside);
            p.text(pr.center(), Align2::CENTER_CENTER, label, FontId::proportional(10.0), fg);
        }
    }
}
