//! Modal overlays: Level Briefing, Hardware Specification (?), Interactive QKD & AES-256 Inspector,
//! and Native Software Update modal.

use crate::levels::hardware_doc;
use crate::model::*;
use crate::qkd::{hex_upper, QkdVerdict};
use crate::render_bench::draw_tile_schematic;
use crate::sim::{BenchSim, SfxEvent};
use egui::{Color32, CornerRadius, RichText, Stroke, Ui, Vec2};

#[cfg(not(target_arch = "wasm32"))]
use crate::updater::{UpdateManager, UpdateState};
#[cfg(not(target_arch = "wasm32"))]
use crate::version::APP_VERSION;

fn tile_for_doc_key(key: &str) -> Option<Tile> {
    match key {
        "QFC" => Some(Tile::Placeable(Placeable::Qfc)),
        "VBG" => Some(Tile::Placeable(Placeable::Vbg)),
        "MEMORY" => Some(Tile::Placeable(Placeable::Memory)),
        "DELAY" => Some(Tile::Placeable(Placeable::Delay)),
        "HWP" => Some(Tile::Placeable(Placeable::Hwp)),
        "AO" => Some(Tile::Placeable(Placeable::Ao)),
        "DCF" => Some(Tile::Placeable(Placeable::Dcf)),
        "WDM" => Some(Tile::Placeable(Placeable::Wdm)),
        "MIRROR_SLASH" => Some(Tile::Placeable(Placeable::MirrorSlash)),
        "MIRROR_BACK" => Some(Tile::Placeable(Placeable::MirrorBack)),
        "FIBER" => Some(Tile::Fiber),
        "LIT_FIBER" => Some(Tile::LitFiber),
        "PMD_FIBER" => Some(Tile::PmdFiber),
        "CD_FIBER" => Some(Tile::CdFiber),
        "ATMOS" => Some(Tile::Atmos),
        "DISTILL" => Some(Tile::Distill),
        "REPEATER" => Some(Tile::Repeater),
        "TELEPORT_BSM" => Some(Tile::TeleportBsm),
        "EVE" => Some(Tile::Eve),
        "WALL" => Some(Tile::Wall),
        _ => None,
    }
}

pub fn draw_modal_overlay(
    ctx: &egui::Context,
    overlay: &mut Overlay,
    sim: &mut BenchSim,
    #[cfg(not(target_arch = "wasm32"))] updater: &UpdateManager,
) {
    if *overlay == Overlay::None {
        return;
    }

    // Dim backdrop scrim
    let screen = ctx.screen_rect();
    egui::Area::new(egui::Id::new("modal_backdrop"))
        .fixed_pos(screen.left_top())
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
            ui.painter().rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(6, 10, 16, 195));
            if resp.clicked() {
                let can_dismiss = match overlay {
                    #[cfg(not(target_arch = "wasm32"))]
                    Overlay::Update => !updater.snapshot().state.is_busy(),
                    _ => true,
                };
                if can_dismiss {
                    *overlay = Overlay::None;
                    sim.pending_sfx.push(SfxEvent::Select);
                }
            }
        });

    let cur = overlay.clone();
    match cur {
        Overlay::None => {}
        Overlay::Level(idx) => {
            let lvl_title = sim.levels[idx].title;
            let lvl_briefing = sim.levels[idx].briefing;
            let lvl_hint_1 = sim.levels[idx].hint_1;
            let lvl_hint_2 = sim.levels[idx].hint_2;
            let lvl_new_hw = sim.levels[idx].new_hardware;
            let total_levels = sim.levels.len();
            let mut close = false;
            egui::Window::new("experiment_briefing")
                .order(egui::Order::Foreground)
                .title_bar(false)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .min_width(660.0f32.min(screen.width() - 32.0))
                .max_width(660.0f32.min(screen.width() - 32.0))
                .frame(modal_frame(ACCENT_CYAN))
                .show(ctx, |ui| {
                    ui.label(RichText::new(format!("EXPERIMENT BRIEFING  •  LEVEL {} OF {}", idx + 1, total_levels)).strong().size(11.0).color(ACCENT_CYAN));
                    ui.label(RichText::new(lvl_title).strong().size(18.0).color(TEXT_WHITE));
                    ui.separator();
                    ui.add_space(4.0);
                    ui.label(RichText::new(lvl_briefing).size(13.5).color(TEXT_WHITE));

                    if !lvl_new_hw.is_empty() {
                        ui.add_space(10.0);
                        ui.label(RichText::new("NEW HARDWARE INTRODUCED IN THIS LEVEL").strong().size(11.0).color(ACCENT_AMBER));
                        ui.add_space(4.0);
                        for &hk in lvl_new_hw {
                            if let Some(doc) = hardware_doc(hk) {
                                egui::Frame::NONE
                                    .fill(Color32::from_rgb(13, 19, 30))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(38, 50, 68)))
                                    .corner_radius(CornerRadius::same(7))
                                    .inner_margin(egui::Margin::symmetric(12, 8))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.label(RichText::new(doc.title).strong().size(13.5).color(doc.color));
                                        ui.label(RichText::new(format!("{} {}", doc.rule, doc.physics)).size(12.0).color(TEXT_MUTED));
                                    });
                                ui.add_space(6.0);
                            }
                        }
                    }

                    // Progressive 2-Tier Opt-In Hint System
                    ui.add_space(8.0);
                    if sim.hint_tier == 0 {
                        ui.horizontal(|ui| {
                            let h1_btn = egui::Button::new(RichText::new("? NEED A HINT (PHYSICS NUDGE)").strong().size(11.5).color(ACCENT_AMBER))
                                .fill(Color32::from_rgb(32, 26, 16))
                                .stroke(Stroke::new(1.0_f32, ACCENT_AMBER))
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(230.0, 28.0));
                            if ui.add(h1_btn).clicked() {
                                sim.hint_tier = 1;
                                sim.pending_sfx.push(SfxEvent::Select);
                            }
                            ui.label(RichText::new("Optional — spoilers hidden until clicked").size(11.0).color(TEXT_MUTED));
                        });
                    } else {
                        egui::Frame::NONE
                            .fill(Color32::from_rgb(28, 24, 16))
                            .stroke(Stroke::new(1.0_f32, ACCENT_AMBER))
                            .corner_radius(CornerRadius::same(7))
                            .inner_margin(egui::Margin::symmetric(12, 8))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.label(RichText::new("HINT 1 • PHYSICS NUDGE").strong().size(10.5).color(ACCENT_AMBER));
                                ui.label(RichText::new(lvl_hint_1).size(12.5).color(TEXT_WHITE));
                            });
                        ui.add_space(6.0);
                        if sim.hint_tier == 1 {
                            let h2_btn = egui::Button::new(RichText::new("REVEAL TACTICAL PLACEMENT HINT").strong().size(11.5).color(ACCENT_CYAN))
                                .fill(Color32::from_rgb(18, 30, 44))
                                .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(240.0, 26.0));
                            if ui.add(h2_btn).clicked() {
                                sim.hint_tier = 2;
                                sim.pending_sfx.push(SfxEvent::Select);
                            }
                        } else {
                            egui::Frame::NONE
                                .fill(Color32::from_rgb(14, 28, 40))
                                .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
                                .corner_radius(CornerRadius::same(7))
                                .inner_margin(egui::Margin::symmetric(12, 8))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.label(RichText::new("HINT 2 • TACTICAL PLACEMENT").strong().size(10.5).color(ACCENT_CYAN));
                                    ui.label(RichText::new(lvl_hint_2).size(12.5).color(TEXT_WHITE));
                                });
                        }
                    }

                    ui.add_space(12.0);
                    ui.vertical_centered(|ui| {
                        let btn = egui::Button::new(RichText::new("BEGIN EXPERIMENT  [ENTER]").strong().size(14.0).color(BG_DARK))
                            .fill(ACCENT_CYAN)
                            .corner_radius(CornerRadius::same(7))
                            .min_size(Vec2::new(260.0, 36.0));
                        if ui.add(btn).clicked() {
                            close = true;
                        }
                    });
                });
            if close {
                *overlay = Overlay::None;
                sim.pending_sfx.push(SfxEvent::Select);
            }
        }
        Overlay::Comp(key) => {
            if let Some(doc) = hardware_doc(key) {
                let mut close = false;
                egui::Window::new("hardware_spec")
                    .order(egui::Order::Foreground)
                    .title_bar(false)
                    .resizable(false)
                    .collapsible(false)
                    .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                    .min_width(560.0f32.min(screen.width() - 32.0))
                    .max_width(560.0f32.min(screen.width() - 32.0))
                    .frame(modal_frame(doc.color))
                    .show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            if let Some(tile) = tile_for_doc_key(key) {
                                let (r, _) = ui.allocate_exact_size(Vec2::splat(72.0), egui::Sense::hover());
                                draw_tile_schematic(ui.painter(), r, tile, sim.frame, 255);
                            }
                            ui.vertical(|ui| {
                                ui.label(RichText::new("HARDWARE SPECIFICATION").strong().size(11.0).color(doc.color));
                                ui.label(RichText::new(doc.title).strong().size(17.0).color(TEXT_WHITE));
                            });
                        });
                        ui.separator();
                        ui.add_space(6.0);
                        spec_section(ui, "QUANTUM PHYSICS", ACCENT_CYAN, doc.physics, TEXT_WHITE);
                        spec_section(ui, "BENCH MECHANIC", ACCENT_AMBER, doc.rule, TEXT_WHITE);
                        spec_section(ui, "LAB TIP", ACCENT_GREEN, doc.tip, TEXT_MUTED);
                        ui.add_space(8.0);
                        ui.vertical_centered(|ui| {
                            let btn = egui::Button::new(RichText::new("CLOSE  [ESC]").strong().size(13.5).color(TEXT_WHITE))
                                .fill(PANEL_BG)
                                .stroke(Stroke::new(1.0_f32, doc.color))
                                .corner_radius(CornerRadius::same(7))
                                .min_size(Vec2::new(190.0, 34.0));
                            if ui.add(btn).clicked() {
                                close = true;
                            }
                        });
                    });
                if close {
                    *overlay = Overlay::None;
                    sim.pending_sfx.push(SfxEvent::Select);
                }
            }
        }
        Overlay::QkdAes => {
            let mut close = false;
            let mut reset_msg = false;
            let accent = match sim.qkd_session.as_ref().map(|q| q.verdict) {
                Some(QkdVerdict::Secure) => ACCENT_GREEN,
                Some(QkdVerdict::AbortEve) => ACCENT_RED,
                _ => ACCENT_CYAN,
            };
            egui::Window::new("qkd_aes_inspector")
                .order(egui::Order::Foreground)
                .title_bar(false)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .min_width(740.0f32.min(screen.width() - 28.0))
                .max_width(740.0f32.min(screen.width() - 28.0))
                .frame(modal_frame(accent))
                .show(ctx, |ui| {
                    let proto_name = match sim.qkd_session.as_ref().map(|q| q.protocol) {
                        Some(TargetMode::QkdE91) => "E91 / BBM92 ENTANGLED QKD  •  FIPS-197 AES-256-CTR",
                        _ => "BB84 PREPARE-AND-MEASURE QKD  •  FIPS-197 AES-256-CTR",
                    };
                    ui.label(RichText::new(proto_name).strong().size(11.0).color(accent));
                    ui.label(RichText::new("Quantum Key Distribution & AES-256 Inspector").strong().size(18.0).color(TEXT_WHITE));
                    ui.separator();
                    ui.add_space(6.0);

                    if let Some(qs) = &mut sim.qkd_session {
                        // 4 stat boxes
                        ui.columns(4, |cols| {
                            stat_box(&mut cols[0], "QUBITS / PAIRS", &format!("{} sent / {} rx", qs.total_emitted, qs.rounds.len()), ACCENT_CYAN);
                            let sb_col = if qs.sifted_alice.len() >= crate::qkd::QkdSession::MIN_SIFTED_BITS { ACCENT_GREEN } else { TEXT_MUTED };
                            stat_box(&mut cols[1], "SIFTED BITS", &format!("{} bits (min {})", qs.sifted_alice.len(), crate::qkd::QkdSession::MIN_SIFTED_BITS), sb_col);
                            let qb_col = if !qs.sifted_alice.is_empty() {
                                if qs.qber <= crate::qkd::QkdSession::QBER_THRESHOLD { ACCENT_GREEN } else { ACCENT_RED }
                            } else { TEXT_MUTED };
                            stat_box(&mut cols[2], "QBER (MAX 11%)", &format!("{:.1}%", qs.qber * 100.0), qb_col);
                            let (s_val, s_col) = match qs.chsh_s {
                                Some(s) => (format!("S = {s:.2}"), if s > 2.0 { ACCENT_GREEN } else { ACCENT_RED }),
                                None => ("BB84 (N/A)".to_string(), TEXT_MUTED),
                            };
                            stat_box(&mut cols[3], "BELL CHSH (S > 2)", &s_val, s_col);
                        });

                        ui.add_space(10.0);
                        ui.label(RichText::new("1. SIFTED RAW KEY BITS (MATCHING '+' / 'x' BASES)").strong().size(11.0).color(ACCENT_AMBER));
                        let raw_bits: String = qs.sifted_alice.iter().map(|&b| if b != 0 { '1' } else { '0' }).collect();
                        code_box(ui, if raw_bits.is_empty() { "(Fire photons to generate and sift qubits)" } else { &raw_bits }, if raw_bits.is_empty() { TEXT_MUTED } else { ACCENT_CYAN }, Color32::from_rgb(36, 48, 66));

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("2. PRIVACY AMPLIFICATION (SHA-256 -> 256-BIT AES KEY)").strong().size(11.0).color(if qs.alice_key.is_some() { ACCENT_GREEN } else { ACCENT_AMBER }));
                            if let Some(ak) = qs.alice_key {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("Copy Key").clicked() {
                                        ui.ctx().copy_text(hex_upper(&ak));
                                    }
                                });
                            }
                        });
                        if let (Some(ak), Some(bk)) = (qs.alice_key, qs.bob_key) {
                            let ka = hex_upper(&ak);
                            let kb = hex_upper(&bk);
                            egui::Frame::NONE
                                .fill(Color32::from_rgb(11, 16, 25))
                                .stroke(Stroke::new(1.0_f32, ACCENT_GREEN))
                                .corner_radius(CornerRadius::same(5))
                                .inner_margin(egui::Margin::symmetric(10, 6))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.label(RichText::new(format!("ALICE: {ka}")).monospace().size(11.5).color(ACCENT_GREEN));
                                    ui.label(RichText::new(format!("BOB:   {kb}")).monospace().size(11.5).color(if ka == kb { ACCENT_GREEN } else { ACCENT_RED }));
                                });
                        } else if qs.verdict == QkdVerdict::AbortEve {
                            code_box(ui, "KEY ABORTED: Quantum security threshold violated (QBER > 11% or CHSH S <= 2.0).", ACCENT_RED, ACCENT_RED);
                        } else {
                            code_box(ui, "No AES-256 key derived yet. Solve the optical link and fire photons first.", TEXT_MUTED, Color32::from_rgb(36, 48, 66));
                        }

                        ui.add_space(8.0);
                        ui.label(RichText::new("3. ALICE PLAINTEXT (EDIT LIVE TO ENCRYPT WITH AES-256-CTR)").strong().size(11.0).color(ACCENT_CYAN));
                        let te = egui::TextEdit::singleline(&mut sim.qkd_plaintext)
                            .font(egui::FontId::monospace(12.5))
                            .char_limit(64)
                            .desired_width(f32::INFINITY);
                        if ui.add(te).changed() {
                            qs.plaintext = sim.qkd_plaintext.clone();
                            qs.re_encrypt();
                        }

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("4. CLASSICAL CHANNEL CIPHERTEXT (AES-256-CTR HEX)").strong().size(11.0).color(ACCENT_PURPLE));
                            if !qs.ciphertext_hex.is_empty() {
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("Copy Hex").clicked() {
                                        ui.ctx().copy_text(qs.ciphertext_hex.clone());
                                    }
                                });
                            }
                        });
                        let ct_disp = if qs.ciphertext_hex.is_empty() {
                            "(Locked until QKD establishes a secure 256-bit key)".to_string()
                        } else {
                            qs.ciphertext_hex.clone()
                        };
                        code_box(ui, &ct_disp, if qs.ciphertext_hex.is_empty() { TEXT_MUTED } else { ACCENT_PURPLE }, Color32::from_rgb(48, 40, 74));

                        ui.add_space(8.0);
                        ui.label(RichText::new("5. BOB DECRYPTED MESSAGE (USING BOB'S INDEPENDENTLY DERIVED AES-256 KEY)").strong().size(11.0).color(ACCENT_GREEN));
                        let dec_disp = if qs.decrypted_text.is_empty() {
                            "(Waiting for valid AES-256 key at Bob RX)".to_string()
                        } else {
                            qs.decrypted_text.clone()
                        };
                        code_box(ui, &dec_disp, if qs.decrypted_text.is_empty() { TEXT_MUTED } else { ACCENT_GREEN }, if qs.decrypted_text.is_empty() { Color32::from_rgb(36, 48, 66) } else { ACCENT_GREEN });
                    }

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let rst = egui::Button::new(RichText::new("RESET MESSAGE").size(12.5).color(TEXT_WHITE))
                            .fill(PANEL_BG)
                            .stroke(Stroke::new(1.0_f32, GRID_LINE))
                            .min_size(Vec2::new(145.0, 32.0));
                        if ui.add(rst).clicked() {
                            reset_msg = true;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let cls = egui::Button::new(RichText::new("CLOSE  [ESC]").strong().size(12.5).color(TEXT_WHITE))
                                .fill(PANEL_BG)
                                .stroke(Stroke::new(1.0_f32, accent))
                                .min_size(Vec2::new(130.0, 32.0));
                            if ui.add(cls).clicked() {
                                close = true;
                            }
                        });
                    });
                });

            if reset_msg {
                sim.qkd_plaintext = "QUANTUM LINK SECURED BY 1550nm QKD".to_string();
                if let Some(qs) = &mut sim.qkd_session {
                    qs.plaintext = sim.qkd_plaintext.clone();
                    qs.re_encrypt();
                }
                sim.pending_sfx.push(SfxEvent::Select);
            }
            if close {
                *overlay = Overlay::None;
                sim.pending_sfx.push(SfxEvent::Select);
            }
        }
        Overlay::SandboxShare => {
            let mut close = false;
            let cur_campaign_lvl = sim.levels[sim.level_idx].clone();
            let cur_campaign_num = sim.level_idx + 1;
            egui::Window::new("sandbox_blueprints")
                .order(egui::Order::Foreground)
                .title_bar(false)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .min_width(720.0f32.min(screen.width() - 32.0))
                .max_width(720.0f32.min(screen.width() - 32.0))
                .frame(modal_frame(ACCENT_PURPLE))
                .show(ctx, |ui| {
                    ui.label(RichText::new("FREE-BUILD SANDBOX  •  STARTER LABS & JSON BLUEPRINTS").strong().size(11.0).color(ACCENT_PURPLE));
                    ui.label(RichText::new("Load a Starter Optical Bench or Share / Import JSON").strong().size(17.5).color(TEXT_WHITE));
                    ui.separator();
                    ui.add_space(6.0);

                    // Bench Name input
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("BENCH NAME:").strong().size(11.0).color(TEXT_MUTED));
                        let name_resp = ui.add(
                            egui::TextEdit::singleline(&mut sim.sandbox_bp.name)
                                .desired_width(280.0)
                                .char_limit(48),
                        );
                        if name_resp.changed() {
                            sim.sandbox_json_buf = serde_json::to_string_pretty(&sim.sandbox_bp).unwrap_or_default();
                        }
                    });
                    ui.add_space(8.0);

                    // Starter Presets Row
                    ui.label(RichText::new("QUICK-LOAD STARTER TEMPLATES").strong().size(11.0).color(ACCENT_CYAN));
                    ui.add_space(3.0);
                    ui.horizontal_wrapped(|ui| {
                        let presets: [(&str, Color32, SandboxBlueprint); 4] = [
                            ("Starter (ION + SNSPD)", ACCENT_CYAN, SandboxBlueprint::preset_blank()),
                            ("HOM Interferometer Lab", ACCENT_AMBER, SandboxBlueprint::preset_hom_lab()),
                            ("E91 QKD & Eve Lab", ACCENT_PURPLE, SandboxBlueprint::preset_qkd_lab()),
                            (&format!("Clone Level {cur_campaign_num}"), ACCENT_GREEN, SandboxBlueprint::from_level(&cur_campaign_lvl)),
                        ];
                        for (lbl, col, bp) in presets {
                            let btn = egui::Button::new(RichText::new(lbl).strong().size(11.5).color(col))
                                .fill(Color32::from_rgb(20, 28, 42))
                                .stroke(Stroke::new(1.0_f32, col))
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(150.0, 30.0));
                            if ui.add(btn).clicked() {
                                let _ = sim.load_sandbox_blueprint(bp);
                                sim.refresh_sandbox_json();
                                sim.sandbox_json_msg = Some(("Template loaded onto optical bench!".to_string(), ACCENT_GREEN));
                                sim.pending_sfx.push(SfxEvent::Select);
                            }
                        }
                    });

                    ui.add_space(10.0);
                    ui.label(RichText::new("PORTABLE JSON OR SHARE LINK (COPY LINK TO PLAY IN BROWSER, OR PASTE JSON / URL BELOW)").strong().size(11.0).color(ACCENT_AMBER));
                    ui.add_space(3.0);

                    egui::ScrollArea::vertical().max_height(185.0).show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut sim.sandbox_json_buf)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(9),
                        );
                    });

                    if let Some((msg, col)) = &sim.sandbox_json_msg {
                        ui.add_space(4.0);
                        ui.label(RichText::new(msg).size(12.0).color(*col));
                    }

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let link_btn = egui::Button::new(RichText::new("COPY SHARE LINK").strong().size(12.0).color(BG_DARK))
                            .fill(ACCENT_PURPLE)
                            .min_size(Vec2::new(132.0, 34.0));
                        if ui.add(link_btn).on_hover_text("Copy a playable browser URL (#bp=...) for this custom optical bench").clicked() {
                            match sim.sandbox_bp.to_share_url() {
                                Ok(url) => {
                                    ctx.copy_text(url);
                                    sim.sandbox_json_msg = Some(("Copied clickable WebAssembly Share Link to clipboard!".to_string(), ACCENT_PURPLE));
                                    sim.pending_sfx.push(SfxEvent::Select);
                                }
                                Err(err) => {
                                    sim.sandbox_json_msg = Some((format!("Encode error: {err}"), ACCENT_RED));
                                    sim.pending_sfx.push(SfxEvent::Fail);
                                }
                            }
                        }

                        let copy_btn = egui::Button::new(RichText::new("COPY JSON").strong().size(12.0).color(ACCENT_CYAN))
                            .fill(PANEL_BG)
                            .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
                            .min_size(Vec2::new(98.0, 34.0));
                        if ui.add(copy_btn).clicked() {
                            sim.sandbox_json_buf = serde_json::to_string_pretty(&sim.sandbox_bp).unwrap_or_default();
                            ctx.copy_text(sim.sandbox_json_buf.clone());
                            sim.sandbox_json_msg = Some(("Copied blueprint JSON to clipboard!".to_string(), ACCENT_CYAN));
                            sim.pending_sfx.push(SfxEvent::Select);
                        }

                        let load_btn = egui::Button::new(RichText::new("IMPORT JSON / URL").strong().size(12.0).color(BG_DARK))
                            .fill(ACCENT_GREEN)
                            .min_size(Vec2::new(142.0, 34.0));
                        if ui.add(load_btn).clicked() {
                            match SandboxBlueprint::from_share_str(&sim.sandbox_json_buf) {
                                Ok(bp) => match sim.load_sandbox_blueprint(bp) {
                                    Ok(()) => {
                                        sim.refresh_sandbox_json();
                                        sim.sandbox_json_msg = Some(("Blueprint imported successfully!".to_string(), ACCENT_GREEN));
                                        sim.pending_sfx.push(SfxEvent::Place);
                                    }
                                    Err(err) => {
                                        sim.sandbox_json_msg = Some((format!("Invalid blueprint: {err}"), ACCENT_RED));
                                        sim.pending_sfx.push(SfxEvent::Fail);
                                    }
                                },
                                Err(err) => {
                                    sim.sandbox_json_msg = Some((err, ACCENT_RED));
                                    sim.pending_sfx.push(SfxEvent::Fail);
                                }
                            }
                        }

                        let clr_btn = egui::Button::new(RichText::new("BLANK").strong().size(12.0).color(ACCENT_RED))
                            .fill(Color32::from_rgb(44, 20, 26))
                            .stroke(Stroke::new(1.0_f32, ACCENT_RED))
                            .min_size(Vec2::new(72.0, 34.0));
                        if ui.add(clr_btn).clicked() {
                            sim.clear_sandbox();
                            sim.refresh_sandbox_json();
                            sim.sandbox_json_msg = Some(("Cleared breadboard to empty grid.".to_string(), ACCENT_AMBER));
                            sim.pending_sfx.push(SfxEvent::Remove);
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let done_btn = egui::Button::new(RichText::new("DONE").strong().size(12.0).color(TEXT_WHITE))
                                .fill(PANEL_BG)
                                .stroke(Stroke::new(1.0_f32, ACCENT_PURPLE))
                                .min_size(Vec2::new(82.0, 34.0));
                            if ui.add(done_btn).clicked() {
                                close = true;
                            }
                        });
                    });
                });
            if close {
                *overlay = Overlay::None;
                sim.pending_sfx.push(SfxEvent::Select);
            }
        }
        Overlay::LabReport => {
            let mut close = false;
            let total = sim.levels.len();
            let done = sim.completed_levels.len();
            let stars = sim.total_stars();
            let max_stars = (total * 3) as u32;
            egui::Window::new("quantum_lab_report")
                .order(egui::Order::Foreground)
                .title_bar(false)
                .resizable(false)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .min_width(700.0f32.min(screen.width() - 32.0))
                .max_width(700.0f32.min(screen.width() - 32.0))
                .frame(modal_frame(ACCENT_AMBER))
                .show(ctx, |ui| {
                    ui.label(RichText::new("CLASSROOM TELEMETRY  •  MASTERY & EFFICIENCY RATINGS").strong().size(11.0).color(ACCENT_AMBER));
                    ui.label(RichText::new("Quantum Networking Lab Report").strong().size(18.0).color(TEXT_WHITE));
                    ui.separator();
                    ui.add_space(6.0);

                    ui.columns(3, |cols| {
                        stat_box(&mut cols[0], "CAMPAIGN PROGRESS", &format!("{done} / {total} Levels"), if done == total { ACCENT_GREEN } else { ACCENT_CYAN });
                        stat_box(&mut cols[1], "EFFICIENCY STARS", &format!("{stars} / {max_stars} Stars"), ACCENT_AMBER);
                        let pct = if max_stars > 0 { (stars as f32 / max_stars as f32) * 100.0 } else { 0.0 };
                        stat_box(&mut cols[2], "MASTERY SCORE", &format!("{pct:.0}%"), if pct >= 85.0 { ACCENT_GREEN } else { ACCENT_PURPLE });
                    });
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Rating Criteria:  1* Solved  •  2* Optimal Optics (<= Par) OR Zero Hints  •  3* Optimal Optics AND Zero Hints")
                            .size(11.0)
                            .color(TEXT_MUTED),
                    );
                    ui.add_space(6.0);

                    egui::Frame::NONE
                        .fill(Color32::from_rgb(11, 16, 25))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(36, 48, 66)))
                        .corner_radius(CornerRadius::same(7))
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            egui::ScrollArea::vertical()
                                .max_height(265.0)
                                .auto_shrink([false, true])
                                .show(ui, |ui| {
                                    for (i, lvl) in sim.levels.iter().enumerate() {
                                        let is_done = sim.completed_levels.contains(&i);
                                        let s = sim.level_stars.get(&i).copied().unwrap_or(if is_done { 1 } else { 0 });
                                        let par = crate::levels::level_par_components(i);
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!("{:02}", i + 1))
                                                    .monospace()
                                                    .strong()
                                                    .size(11.5)
                                                    .color(if is_done { ACCENT_GREEN } else { TEXT_MUTED }),
                                            );
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(lvl.title)
                                                        .size(12.0)
                                                        .color(if is_done { TEXT_WHITE } else { TEXT_MUTED }),
                                                )
                                                .truncate(),
                                            );
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                draw_star_trio(ui, s);
                                                ui.add_space(8.0);
                                                let st_lbl = if is_done { "LOCKED" } else { "PENDING" };
                                                let st_col = if is_done { ACCENT_GREEN } else { Color32::from_rgb(80, 94, 114) };
                                                ui.label(RichText::new(st_lbl).strong().size(10.0).color(st_col));
                                                ui.add_space(8.0);
                                                ui.label(RichText::new(format!("Par: {par}")).monospace().size(10.5).color(TEXT_MUTED));
                                            });
                                        });
                                        if i + 1 < total {
                                            ui.separator();
                                        }
                                    }
                                });
                        });

                    if let Some((msg, col)) = &sim.sandbox_json_msg {
                        ui.add_space(6.0);
                        ui.label(RichText::new(msg).size(12.0).color(*col));
                    }

                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        let copy_btn = egui::Button::new(RichText::new("COPY MARKDOWN REPORT").strong().size(12.5).color(BG_DARK))
                            .fill(ACCENT_AMBER)
                            .min_size(Vec2::new(210.0, 34.0));
                        if ui.add(copy_btn).clicked() {
                            let md = sim.build_lab_report_markdown();
                            ctx.copy_text(md);
                            sim.sandbox_json_msg = Some(("Copied Markdown Quantum Lab Report to clipboard!".to_string(), ACCENT_GREEN));
                            sim.pending_sfx.push(SfxEvent::Select);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let cls = egui::Button::new(RichText::new("CLOSE  [ESC]").strong().size(12.5).color(TEXT_WHITE))
                                .fill(PANEL_BG)
                                .stroke(Stroke::new(1.0_f32, ACCENT_AMBER))
                                .min_size(Vec2::new(125.0, 34.0));
                            if ui.add(cls).clicked() {
                                close = true;
                            }
                        });
                    });
                });
            if close {
                *overlay = Overlay::None;
                sim.sandbox_json_msg = None;
                sim.pending_sfx.push(SfxEvent::Select);
            }
        }
        Overlay::Update => {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let snap = updater.snapshot();
                let mut close = false;
                let accent = match snap.state {
                    UpdateState::Available => ACCENT_GREEN,
                    UpdateState::Error => ACCENT_RED,
                    _ => ACCENT_CYAN,
                };
                egui::Window::new("software_update")
                    .order(egui::Order::Foreground)
                    .title_bar(false)
                    .resizable(false)
                    .collapsible(false)
                    .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                    .min_width(560.0f32.min(screen.width() - 32.0))
                    .max_width(560.0f32.min(screen.width() - 32.0))
                    .frame(modal_frame(accent))
                    .show(ctx, |ui| {
                        ui.label(RichText::new(format!("SOFTWARE UPDATE  •  INSTALLED v{APP_VERSION}")).strong().size(11.0).color(accent));
                        let ver = snap.release_info.as_ref().map(|r| r.version.as_str()).unwrap_or("");
                        let headline = match snap.state {
                            UpdateState::Available => format!("QN Bench v{ver} is Ready to Install"),
                            UpdateState::Checking => "Checking GitHub Releases...".to_string(),
                            UpdateState::UpToDate => format!("QN Bench v{APP_VERSION} is Up to Date"),
                            UpdateState::Downloading => format!("Downloading QN Bench v{ver}..."),
                            UpdateState::Verifying => "Verifying Signature & Checksum...".to_string(),
                            UpdateState::Installing => "Installing Update & Relaunching...".to_string(),
                            UpdateState::Error => "Update Check / Install Notice".to_string(),
                            UpdateState::Idle => "Software Updates".to_string(),
                        };
                        ui.label(RichText::new(headline).strong().size(17.5).color(TEXT_WHITE));
                        ui.separator();
                        ui.add_space(6.0);

                        let st_col = match snap.state {
                            UpdateState::Error => ACCENT_RED,
                            UpdateState::Available | UpdateState::UpToDate => ACCENT_GREEN,
                            _ => TEXT_WHITE,
                        };
                        ui.label(RichText::new(&snap.status_text).size(13.0).color(st_col));

                        if snap.state == UpdateState::Available {
                            if let Some(info) = &snap.release_info {
                                ui.add_space(10.0);
                                ui.label(RichText::new("RELEASE NOTES").strong().size(11.0).color(ACCENT_AMBER));
                                egui::Frame::NONE
                                    .fill(Color32::from_rgb(13, 19, 30))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(38, 50, 68)))
                                    .corner_radius(CornerRadius::same(7))
                                    .inner_margin(egui::Margin::same(10))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        egui::ScrollArea::vertical()
                                            .max_height(210.0)
                                            .auto_shrink([false, true])
                                            .show(ui, |ui| {
                                                render_markdown_notes(ui, &info.notes);
                                            });
                                    });
                            }
                        }

                        if snap.state.is_busy() && snap.state != UpdateState::Checking {
                            ui.add_space(10.0);
                            ui.add(egui::ProgressBar::new(snap.progress).show_percentage());
                        }

                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            match snap.state {
                                UpdateState::Available => {
                                    let can_inst = snap.can_self_install && snap.release_info.as_ref().and_then(|r| r.zip_url.as_ref()).is_some();
                                    if can_inst {
                                        let b = egui::Button::new(RichText::new("INSTALL & RELAUNCH").strong().size(13.0).color(BG_DARK))
                                            .fill(ACCENT_GREEN)
                                            .min_size(Vec2::new(185.0, 34.0));
                                        if ui.add(b).clicked() {
                                            updater.start_install();
                                            sim.pending_sfx.push(SfxEvent::Select);
                                        }
                                    }
                                    let bp = egui::Button::new(RichText::new("RELEASE PAGE").strong().size(13.0).color(TEXT_WHITE))
                                        .fill(PANEL_BG)
                                        .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
                                        .min_size(Vec2::new(140.0, 34.0));
                                    if ui.add(bp).clicked() {
                                        updater.open_release_page();
                                        sim.pending_sfx.push(SfxEvent::Select);
                                    }
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let bl = egui::Button::new(RichText::new("LATER").size(13.0).color(TEXT_MUTED))
                                            .fill(PANEL_BG)
                                            .stroke(Stroke::new(1.0_f32, GRID_LINE))
                                            .min_size(Vec2::new(90.0, 34.0));
                                        if ui.add(bl).clicked() {
                                            close = true;
                                        }
                                    });
                                }
                                UpdateState::Downloading => {
                                    let bc = egui::Button::new(RichText::new("CANCEL DOWNLOAD").strong().size(13.0).color(TEXT_WHITE))
                                        .fill(Color32::from_rgb(48, 22, 28))
                                        .stroke(Stroke::new(1.0_f32, ACCENT_RED))
                                        .min_size(Vec2::new(170.0, 34.0));
                                    if ui.add(bc).clicked() {
                                        updater.cancel_download();
                                        sim.pending_sfx.push(SfxEvent::Select);
                                    }
                                }
                                UpdateState::Verifying | UpdateState::Installing => {
                                    ui.label(RichText::new("Please wait — do not close the app while installing...").size(12.5).color(TEXT_MUTED));
                                }
                                _ => {
                                    let br = egui::Button::new(RichText::new("CHECK AGAIN").strong().size(13.0).color(TEXT_WHITE))
                                        .fill(PANEL_BG)
                                        .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
                                        .min_size(Vec2::new(140.0, 34.0));
                                    if ui.add(br).clicked() {
                                        updater.check_for_updates(false);
                                        sim.pending_sfx.push(SfxEvent::Select);
                                    }
                                    let bp = egui::Button::new(RichText::new("DOWNLOAD PAGE").strong().size(13.0).color(TEXT_WHITE))
                                        .fill(PANEL_BG)
                                        .stroke(Stroke::new(1.0_f32, GRID_LINE))
                                        .min_size(Vec2::new(150.0, 34.0));
                                    if ui.add(bp).clicked() {
                                        updater.open_release_page();
                                        sim.pending_sfx.push(SfxEvent::Select);
                                    }
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let bc = egui::Button::new(RichText::new("CLOSE").size(13.0).color(TEXT_MUTED))
                                            .fill(PANEL_BG)
                                            .stroke(Stroke::new(1.0_f32, GRID_LINE))
                                            .min_size(Vec2::new(90.0, 34.0));
                                        if ui.add(bc).clicked() {
                                            close = true;
                                        }
                                    });
                                }
                            }
                        });
                    });
                if close {
                    *overlay = Overlay::None;
                    sim.pending_sfx.push(SfxEvent::Select);
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                *overlay = Overlay::None;
            }
        }
    }
}

fn modal_frame(border: Color32) -> egui::Frame {
    egui::Frame::NONE
        .fill(Color32::from_rgb(16, 22, 34))
        .stroke(Stroke::new(2.0_f32, border))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(egui::Margin::same(22))
}

fn spec_section(ui: &mut Ui, heading: &str, col: Color32, body: &str, body_col: Color32) {
    ui.label(RichText::new(heading).strong().size(11.0).color(col));
    ui.label(RichText::new(body).size(13.0).color(body_col));
    ui.add_space(10.0);
}

fn draw_star_trio(ui: &mut Ui, earned: u8) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(52.0, 16.0), egui::Sense::hover());
    let p = ui.painter();
    for idx in 0..3u8 {
        let cx = rect.left() + 8.0 + idx as f32 * 18.0;
        let cy = rect.center().y;
        let filled = idx < earned;
        let col = if filled { ACCENT_AMBER } else { Color32::from_rgb(42, 52, 70) };
        let r_out = 6.5f32;
        let r_in = 2.8f32;
        let mut pts = Vec::with_capacity(10);
        for k in 0..10 {
            let angle = -std::f32::consts::FRAC_PI_2 + k as f32 * std::f32::consts::PI / 5.0;
            let r = if k % 2 == 0 { r_out } else { r_in };
            pts.push(egui::Pos2::new(cx + r * angle.cos(), cy + r * angle.sin()));
        }
        p.add(egui::Shape::closed_line(pts.clone(), Stroke::new(1.2_f32, col)));
        if filled {
            // Paint 5 triangular star petals from center for crisp concave fill
            let center = egui::Pos2::new(cx, cy);
            for k in 0..10 {
                let a = pts[k];
                let b = pts[(k + 1) % 10];
                p.add(egui::Shape::convex_polygon(vec![center, a, b], col, Stroke::NONE));
            }
        }
    }
}

fn stat_box(ui: &mut Ui, label: &str, val: &str, col: Color32) {
    egui::Frame::NONE
        .fill(Color32::from_rgb(11, 16, 25))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(36, 48, 66)))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(label).strong().size(10.0).color(TEXT_MUTED));
            ui.label(RichText::new(val).strong().size(13.0).color(col));
        });
}

fn code_box(ui: &mut Ui, text: &str, fg: Color32, border: Color32) {
    egui::Frame::NONE
        .fill(Color32::from_rgb(11, 16, 25))
        .stroke(Stroke::new(1.0_f32, border))
        .corner_radius(CornerRadius::same(5))
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(egui::Label::new(RichText::new(text).monospace().size(11.5).color(fg)).wrap());
        });
}

#[cfg(not(target_arch = "wasm32"))]
fn sanitize_release_glyphs(s: &str) -> String {
    s.replace('→', "->")
        .replace('←', "<-")
        .replace('↶', "Undo")
        .replace('↷', "Redo")
        .replace('✓', "OK")
        .replace('↑', "^")
        .replace('↓', "v")
}

#[cfg(not(target_arch = "wasm32"))]
fn append_inline_markdown(job: &mut egui::text::LayoutJob, text: &str, base_size: f32, base_col: Color32) {
    let normal_fmt = egui::TextFormat {
        font_id: egui::FontId::proportional(base_size),
        color: base_col,
        ..Default::default()
    };
    let bold_fmt = egui::TextFormat {
        font_id: egui::FontId::proportional(base_size + 0.3),
        color: TEXT_WHITE,
        ..Default::default()
    };
    let italic_fmt = egui::TextFormat {
        font_id: egui::FontId::proportional(base_size),
        color: Color32::from_rgb(172, 188, 212),
        italics: true,
        ..Default::default()
    };
    let code_fmt = egui::TextFormat {
        font_id: egui::FontId::monospace((base_size - 0.8).max(10.0)),
        color: ACCENT_AMBER,
        background: Color32::from_rgb(24, 34, 48),
        ..Default::default()
    };
    let link_fmt = egui::TextFormat {
        font_id: egui::FontId::proportional(base_size),
        color: ACCENT_CYAN,
        underline: Stroke::new(1.0_f32, ACCENT_CYAN),
        ..Default::default()
    };

    let bytes = text.as_bytes();
    let mut i = 0;
    let mut plain_start = 0;
    let flush_plain = |job: &mut egui::text::LayoutJob, start: usize, end: usize| {
        if end > start {
            job.append(&text[start..end], 0.0, normal_fmt.clone());
        }
    };

    while i < bytes.len() {
        // Bold: **text**
        if bytes[i..].starts_with(b"**") {
            if let Some(rel_end) = text[i + 2..].find("**") {
                flush_plain(job, plain_start, i);
                job.append(&text[i + 2..i + 2 + rel_end], 0.0, bold_fmt.clone());
                i += 2 + rel_end + 2;
                plain_start = i;
                continue;
            }
        }
        // Inline code: `code`
        if bytes[i] == b'`' {
            if let Some(rel_end) = text[i + 1..].find('`') {
                flush_plain(job, plain_start, i);
                job.append(&text[i + 1..i + 1 + rel_end], 0.0, code_fmt.clone());
                i += 1 + rel_end + 1;
                plain_start = i;
                continue;
            }
        }
        // Markdown link: [label](url)
        if bytes[i] == b'[' {
            if let Some(close_br) = text[i + 1..].find("](") {
                let after_mid = i + 1 + close_br + 2;
                if let Some(close_par) = text[after_mid..].find(')') {
                    flush_plain(job, plain_start, i);
                    let label = &text[i + 1..i + 1 + close_br];
                    job.append(label, 0.0, link_fmt.clone());
                    i = after_mid + close_par + 1;
                    plain_start = i;
                    continue;
                }
            }
        }
        // Bare URL: https://...
        if text[i..].starts_with("https://") || text[i..].starts_with("http://") {
            let end = text[i..]
                .find(|c: char| c.is_whitespace() || c == ')' || c == ']')
                .map(|r| i + r)
                .unwrap_or(text.len());
            flush_plain(job, plain_start, i);
            job.append(&text[i..end], 0.0, link_fmt.clone());
            i = end;
            plain_start = i;
            continue;
        }
        // Italic: *text* (single asterisk, not **)
        if bytes[i] == b'*' && !bytes[i..].starts_with(b"**") {
            if let Some(rel_end) = text[i + 1..].find('*') {
                if rel_end > 0 {
                    flush_plain(job, plain_start, i);
                    job.append(&text[i + 1..i + 1 + rel_end], 0.0, italic_fmt.clone());
                    i += 1 + rel_end + 1;
                    plain_start = i;
                    continue;
                }
            }
        }
        let ch_len = text[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        i += ch_len;
    }
    flush_plain(job, plain_start, text.len());
}

#[cfg(not(target_arch = "wasm32"))]
fn render_markdown_notes(ui: &mut Ui, markdown: &str) {
    let clean = sanitize_release_glyphs(markdown);
    let mut first_block = true;
    for raw_line in clean.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            ui.add_space(4.0);
            continue;
        }
        if let Some(h) = line.strip_prefix("### ") {
            if !first_block {
                ui.add_space(6.0);
            }
            ui.label(RichText::new(h.trim()).strong().size(13.5).color(ACCENT_CYAN));
            ui.add_space(2.0);
        } else if let Some(h) = line.strip_prefix("## ") {
            if !first_block {
                ui.add_space(7.0);
            }
            ui.label(RichText::new(h.trim()).strong().size(14.5).color(TEXT_WHITE));
            ui.add_space(2.0);
        } else if let Some(h) = line.strip_prefix("# ") {
            if !first_block {
                ui.add_space(8.0);
            }
            ui.label(RichText::new(h.trim()).strong().size(15.5).color(ACCENT_CYAN));
            ui.add_space(3.0);
        } else if let Some(item) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(RichText::new("•").strong().size(12.5).color(ACCENT_CYAN));
                let mut job = egui::text::LayoutJob::default();
                append_inline_markdown(&mut job, item.trim(), 12.2, TEXT_MUTED);
                ui.add(egui::Label::new(job).wrap());
            });
            ui.add_space(2.0);
        } else if line == "---" || line == "***" {
            ui.separator();
        } else {
            let mut job = egui::text::LayoutJob::default();
            append_inline_markdown(&mut job, line, 12.2, TEXT_MUTED);
            ui.add(egui::Label::new(job).wrap());
            ui.add_space(2.0);
        }
        first_block = false;
    }
}
