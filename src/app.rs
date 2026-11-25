//! Main eframe::App implementation: sidebar, fixed-timestep loop, keyboard shortcuts, and persistence.

use crate::audio::SoundEngine;
use crate::model::*;
use crate::render_bench::draw_optical_bench;
use crate::render_scopes::draw_telemetry_deck;
use crate::sim::{BenchSim, SfxEvent};
use crate::ui_modals::draw_modal_overlay;
use crate::version::APP_VERSION;
#[cfg(target_arch = "wasm32")]
use crate::version::DOWNLOAD_PAGE_URL;
use egui::{Color32, CornerRadius, Key, RichText, Stroke, Vec2};
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use crate::updater::{UpdateManager, UpdateState};

const STORAGE_KEY: &str = "qn_bench_state_v2";
const STEP_DT: f64 = 1.0 / 60.0;

#[derive(Debug, Default, Serialize, Deserialize)]
struct PersistedState {
    completed_levels: Vec<usize>,
    muted: bool,
    #[serde(default)]
    sandbox_bp: Option<SandboxBlueprint>,
}

pub struct QnBenchApp {
    sim: BenchSim,
    sfx: SoundEngine,
    overlay: Overlay,
    accum_dt: f64,
    #[cfg(not(target_arch = "wasm32"))]
    updater: UpdateManager,
}

impl QnBenchApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Configure dark machined optical-bench visuals
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG_DARK;
        visuals.window_fill = Color32::from_rgb(16, 22, 34);
        visuals.extreme_bg_color = Color32::from_rgb(11, 15, 23);
        cc.egui_ctx.set_visuals(visuals);

        let saved: PersistedState = cc
            .storage
            .and_then(|s| eframe::get_value(s, STORAGE_KEY))
            .unwrap_or_default();

        let mut sim = BenchSim::new();
        for idx in saved.completed_levels {
            if idx < sim.levels.len() {
                sim.completed_levels.insert(idx);
            }
        }
        if let Some(bp) = saved.sandbox_bp {
            let _ = sim.load_sandbox_blueprint(bp);
        }
        sim.seen_briefings.insert(0);

        let sfx = SoundEngine::new(saved.muted);

        #[cfg(not(target_arch = "wasm32"))]
        let updater = {
            let u = UpdateManager::new();
            u.check_for_updates(true);
            u
        };

        Self {
            sim,
            sfx,
            overlay: Overlay::Level(0),
            accum_dt: 0.0,
            #[cfg(not(target_arch = "wasm32"))]
            updater,
        }
    }

    fn switch_level(&mut self, idx: usize, force_briefing: Option<bool>) {
        let target = idx % self.sim.levels.len();
        self.sim.exit_sandbox(target);
        let show = force_briefing.unwrap_or(!self.sim.seen_briefings.contains(&target));
        if show {
            self.overlay = Overlay::Level(target);
            self.sim.seen_briefings.insert(target);
        } else {
            self.overlay = Overlay::None;
        }
    }

    fn toggle_sandbox(&mut self) {
        self.sfx.play(SfxEvent::Select);
        if self.sim.sandbox_mode {
            let cur = self.sim.level_idx;
            self.switch_level(cur, Some(false));
        } else {
            self.sim.enter_sandbox();
            self.overlay = Overlay::None;
        }
    }

    fn flush_sfx(&mut self) {
        for ev in self.sim.pending_sfx.drain(..) {
            self.sfx.play(ev);
        }
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        // Never intercept typing when a TextEdit (e.g. AES-256 plaintext) has focus
        let text_focused = ctx.wants_keyboard_input();

        if self.overlay != Overlay::None {
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                let can_close = match self.overlay {
                    #[cfg(not(target_arch = "wasm32"))]
                    Overlay::Update => !self.updater.snapshot().state.is_busy(),
                    _ => true,
                };
                if can_close {
                    self.overlay = Overlay::None;
                    self.sfx.play(SfxEvent::Select);
                }
                return;
            }
            if !text_focused && matches!(self.overlay, Overlay::Level(_) | Overlay::Comp(_)) {
                if ctx.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space) || i.key_pressed(Key::H)) {
                    self.overlay = Overlay::None;
                    self.sfx.play(SfxEvent::Select);
                }
            }
            return;
        }

        if text_focused {
            return;
        }

        let (fire, reset, pause, help, qkd_k, update_u, mute, sandbox_s, rot_ccw, rot_cw, prev_l, next_l, num_jump) = ctx.input(|i| {
            let mut jump = None;
            let num_keys = [
                Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5,
                Key::Num6, Key::Num7, Key::Num8, Key::Num9,
            ];
            for (idx, k) in num_keys.iter().enumerate() {
                if i.key_pressed(*k) {
                    jump = Some(idx);
                }
            }
            if i.key_pressed(Key::Num0) {
                jump = Some(9);
            }
            if i.key_pressed(Key::Minus) {
                jump = Some(10);
            }
            (
                i.key_pressed(Key::Space),
                i.key_pressed(Key::R),
                i.key_pressed(Key::P),
                i.key_pressed(Key::H),
                i.key_pressed(Key::K),
                i.key_pressed(Key::U),
                i.key_pressed(Key::M),
                i.key_pressed(Key::S),
                i.key_pressed(Key::Q),
                i.key_pressed(Key::E),
                i.key_pressed(Key::OpenBracket) || i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::CloseBracket) || i.key_pressed(Key::ArrowRight),
                jump,
            )
        });

        if fire {
            self.sim.fire_photons();
        } else if reset {
            self.sfx.play(SfxEvent::Select);
            if self.sim.sandbox_mode {
                self.sim.sync_sandbox_from_bp();
            } else {
                let cur = self.sim.level_idx;
                self.switch_level(cur, Some(false));
            }
        } else if pause && self.sim.sim_active {
            self.sim.paused = !self.sim.paused;
            self.sfx.play(SfxEvent::Select);
        } else if sandbox_s {
            self.toggle_sandbox();
        } else if rot_ccw && self.sim.sandbox_mode {
            self.sim.rotate_sandbox_dir(false);
            self.sfx.play(SfxEvent::Select);
        } else if rot_cw && self.sim.sandbox_mode {
            self.sim.rotate_sandbox_dir(true);
            self.sfx.play(SfxEvent::Select);
        } else if help {
            if self.sim.sandbox_mode {
                self.sim.refresh_sandbox_json();
                self.overlay = Overlay::SandboxShare;
            } else {
                self.overlay = Overlay::Level(self.sim.level_idx);
            }
            self.sfx.play(SfxEvent::Select);
        } else if qkd_k && self.sim.is_qkd_level() {
            self.overlay = Overlay::QkdAes;
            self.sfx.play(SfxEvent::Select);
        } else if mute {
            self.sfx.toggle_mute();
        } else if prev_l {
            self.sfx.play(SfxEvent::Select);
            let n = self.sim.levels.len();
            let prev = (self.sim.level_idx + n - 1) % n;
            self.switch_level(prev, None);
        } else if next_l {
            self.sfx.play(SfxEvent::Select);
            let next = (self.sim.level_idx + 1) % self.sim.levels.len();
            self.switch_level(next, None);
        } else if let Some(idx) = num_jump {
            if idx < self.sim.levels.len() {
                self.sfx.play(SfxEvent::Select);
                self.switch_level(idx, None);
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        if update_u {
            self.sfx.play(SfxEvent::Select);
            self.updater.mark_seen();
            let st = self.updater.snapshot().state;
            if matches!(st, UpdateState::Idle | UpdateState::UpToDate | UpdateState::Error) {
                self.updater.check_for_updates(false);
            }
            self.overlay = Overlay::Update;
        }
        #[cfg(target_arch = "wasm32")]
        let _ = update_u;
    }
}

impl eframe::App for QnBenchApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let mut completed: Vec<usize> = self.sim.completed_levels.iter().copied().collect();
        completed.sort_unstable();
        let state = PersistedState {
            completed_levels: completed,
            muted: self.sfx.muted,
            sandbox_bp: Some(self.sim.sandbox_bp.clone()),
        };
        eframe::set_value(storage, STORAGE_KEY, &state);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Fixed 60 Hz physics accumulator scaled by simulation speed (consistent across 60/120/144Hz monitors!)
        let dt = ctx.input(|i| i.unstable_dt as f64).min(0.1);
        let eff_scale = if self.sim.sim_active && !self.sim.paused {
            self.sim.sim_speed as f64
        } else {
            1.0
        };
        self.accum_dt += dt * eff_scale;
        let mut steps = 0;
        while self.accum_dt >= STEP_DT && steps < 6 {
            self.sim.step_frame();
            self.accum_dt -= STEP_DT;
            steps += 1;
        }

        self.handle_keyboard(ctx);

        // Poll native updater for unseen update prompt or ready-to-quit relaunch
        #[cfg(not(target_arch = "wasm32"))]
        {
            let snap = self.updater.snapshot();
            if snap.ready_to_quit {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            if snap.has_unseen_update && self.overlay == Overlay::None && !self.sim.sim_active {
                self.updater.mark_seen();
                self.overlay = Overlay::Update;
            }
        }

        // Right Sidebar: Hardware Palette & Actions
        egui::SidePanel::right("sidebar")
            .exact_width(336.0)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(SIDEBAR_BG)
                    .stroke(Stroke::new(1.0_f32, GRID_LINE))
                    .inner_margin(egui::Margin::same(14)),
            )
            .show(ctx, |ui| {
                // Header row: COMPONENTS (or SANDBOX LAB) + ? INFO / LABS + SFX toggle
                ui.horizontal(|ui| {
                    if self.sim.sandbox_mode {
                        ui.label(RichText::new("SANDBOX LAB").strong().size(16.0).color(ACCENT_PURPLE));
                    } else {
                        ui.label(RichText::new("COMPONENTS").strong().size(16.0).color(TEXT_WHITE));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let sfx_on = self.sfx.is_active();
                        let m_bg = if sfx_on { Color32::from_rgb(24, 46, 56) } else { Color32::from_rgb(36, 26, 32) };
                        let m_bd = if sfx_on { ACCENT_CYAN } else { TEXT_MUTED };
                        let m_btn = egui::Button::new(
                            RichText::new(if sfx_on { "SFX: ON" } else { "SFX: OFF" })
                                .strong()
                                .size(10.5)
                                .color(if sfx_on { ACCENT_CYAN } else { TEXT_MUTED }),
                        )
                        .fill(m_bg)
                        .stroke(Stroke::new(1.0_f32, m_bd))
                        .corner_radius(CornerRadius::same(5))
                        .min_size(Vec2::new(66.0, 24.0));
                        if ui.add(m_btn).clicked() {
                            self.sfx.toggle_mute();
                        }

                        if self.sim.sandbox_mode {
                            let b_btn = egui::Button::new(RichText::new("LABS / JSON").strong().size(10.5).color(ACCENT_PURPLE))
                                .fill(Color32::from_rgb(32, 24, 52))
                                .stroke(Stroke::new(1.0_f32, ACCENT_PURPLE))
                                .corner_radius(CornerRadius::same(5))
                                .min_size(Vec2::new(82.0, 24.0));
                            if ui.add(b_btn).clicked() {
                                self.sim.refresh_sandbox_json();
                                self.overlay = Overlay::SandboxShare;
                                self.sfx.play(SfxEvent::Select);
                            }
                        } else {
                            let i_btn = egui::Button::new(RichText::new("? INFO").strong().size(10.5).color(ACCENT_CYAN))
                                .fill(Color32::from_rgb(22, 34, 50))
                                .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
                                .corner_radius(CornerRadius::same(5))
                                .min_size(Vec2::new(58.0, 24.0));
                            if ui.add(i_btn).clicked() {
                                self.overlay = Overlay::Level(self.sim.level_idx);
                                self.sfx.play(SfxEvent::Select);
                            }
                        }
                    });
                });

                if self.sim.sandbox_mode {
                    ui.label(RichText::new("L-Click: Place / Rotate  |  R-Click: Erase").size(11.5).color(TEXT_MUTED));
                    ui.add_space(6.0);

                    // Sandbox Category Tabs: OPTICS | CHANNELS | SOURCES
                    ui.horizontal(|ui| {
                        let tab_w = (ui.available_width() - 12.0) / 3.0;
                        for (tab, label) in [
                            (SandboxTab::Optics, "OPTICS"),
                            (SandboxTab::Channels, "CHANNELS"),
                            (SandboxTab::Sources, "SOURCES"),
                        ] {
                            let active = self.sim.sandbox_tab == tab;
                            let btn = egui::Button::new(
                                RichText::new(label)
                                    .strong()
                                    .size(10.5)
                                    .color(if active { TEXT_WHITE } else { TEXT_MUTED }),
                            )
                            .fill(if active { Color32::from_rgb(46, 34, 74) } else { PANEL_BG })
                            .stroke(Stroke::new(1.0_f32, if active { ACCENT_PURPLE } else { GRID_LINE }))
                            .corner_radius(CornerRadius::same(5))
                            .min_size(Vec2::new(tab_w, 25.0));
                            if ui.add(btn).clicked() {
                                self.sim.sandbox_tab = tab;
                                self.sfx.play(SfxEvent::Select);
                            }
                        }
                    });
                    ui.add_space(6.0);

                    // Rotatable Emitter Direction Strip when an emitter/EPR tool is active
                    if self.sim.sandbox_tool.is_rotatable() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("AIM [Q/E]:").strong().size(10.5).color(ACCENT_AMBER));
                            for d in [Dir::Right, Dir::Down, Dir::Left, Dir::Up] {
                                let sel = self.sim.sandbox_dir == d;
                                let b = egui::Button::new(
                                    RichText::new(d.arrow())
                                        .strong()
                                        .size(12.0)
                                        .color(if sel { BG_DARK } else { TEXT_WHITE }),
                                )
                                .fill(if sel { ACCENT_AMBER } else { PANEL_BG })
                                .stroke(Stroke::new(1.0_f32, if sel { ACCENT_AMBER } else { GRID_LINE }))
                                .corner_radius(CornerRadius::same(4))
                                .min_size(Vec2::new(28.0, 22.0));
                                if ui.add(b).clicked() {
                                    self.sim.sandbox_dir = d;
                                    self.sfx.play(SfxEvent::Select);
                                }
                            }
                        });
                        ui.add_space(6.0);
                    }

                    let cards_max_h = (ui.available_height() - 196.0).max(80.0);
                    let tools: &[SandboxTool] = match self.sim.sandbox_tab {
                        SandboxTab::Optics => &SandboxTool::OPTICS,
                        SandboxTab::Channels => &SandboxTool::CHANNELS,
                        SandboxTab::Sources => &SandboxTool::SOURCES,
                    };
                    egui::ScrollArea::vertical()
                        .id_salt(("sandbox_cards_scroll", self.sim.sandbox_tab as u8))
                        .max_height(cards_max_h)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for (idx, &tool) in tools.iter().enumerate() {
                                let is_sel = self.sim.sandbox_tool == tool;
                                let bg = if is_sel { Color32::from_rgb(36, 48, 68) } else { PANEL_BG };
                                let bd = if is_sel { tool.color() } else { GRID_LINE };

                                let frame_resp = egui::Frame::NONE
                                    .fill(bg)
                                    .stroke(Stroke::new(if is_sel { 2.0_f32 } else { 1.0_f32 }, bd))
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(egui::Margin::symmetric(10, 6))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(RichText::new(format!("{}  (∞)", tool.name())).strong().size(13.5).color(tool.color()));
                                                ui.label(RichText::new(tool.desc()).size(11.0).color(TEXT_MUTED));
                                            });
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                                let q_btn = egui::Button::new(RichText::new("?").strong().size(11.0).color(ACCENT_CYAN))
                                                    .fill(Color32::from_rgb(20, 28, 42))
                                                    .stroke(Stroke::new(1.0_f32, GRID_LINE))
                                                    .corner_radius(CornerRadius::same(11))
                                                    .min_size(Vec2::splat(22.0));
                                                if ui.add(q_btn).on_hover_text("Hardware Physics & Tip").clicked() {
                                                    self.overlay = Overlay::Comp(tool.doc_key());
                                                    self.sfx.play(SfxEvent::Select);
                                                }
                                            });
                                        });
                                    })
                                    .response;

                                let card_click = ui.interact(frame_resp.rect, ui.id().with(("sb_card", idx)), egui::Sense::click());
                                if card_click.clicked() && self.overlay == Overlay::None {
                                    self.sim.sandbox_tool = tool;
                                    self.sfx.play(SfxEvent::Select);
                                }
                                ui.add_space(5.0);
                            }
                        });
                } else {
                    ui.label(RichText::new("L-Click: Place  |  R-Click: Remove").size(12.0).color(TEXT_MUTED));
                    ui.add_space(10.0);

                    // Component Cards (scrollable on smaller windows so bottom buttons never clip)
                    let cards_max_h = (ui.available_height() - 196.0).max(80.0);
                    egui::ScrollArea::vertical()
                        .id_salt("comp_cards_scroll")
                        .max_height(cards_max_h)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            let active = self.sim.active_components();
                            for comp in active {
                                let count = self.sim.inventory.get(&comp).copied().unwrap_or(0);
                                let is_sel = self.sim.selected == Some(comp) && count > 0;
                                let bg = if is_sel { Color32::from_rgb(36, 48, 68) } else { PANEL_BG };
                                let bd = if is_sel { comp.color() } else if count > 0 { GRID_LINE } else { Color32::from_rgb(30, 36, 46) };

                                let frame_resp = egui::Frame::NONE
                                    .fill(bg)
                                    .stroke(Stroke::new(if is_sel { 2.0_f32 } else { 1.0_f32 }, bd))
                                    .corner_radius(CornerRadius::same(6))
                                    .inner_margin(egui::Margin::symmetric(10, 7))
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                let tcol = if count > 0 { comp.color() } else { TEXT_MUTED };
                                                ui.label(RichText::new(format!("{} (x{count})", comp.name())).strong().size(14.0).color(tcol));
                                                ui.label(RichText::new(comp.desc()).size(11.5).color(TEXT_MUTED));
                                            });
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                                let q_btn = egui::Button::new(RichText::new("?").strong().size(11.0).color(ACCENT_CYAN))
                                                    .fill(Color32::from_rgb(20, 28, 42))
                                                    .stroke(Stroke::new(1.0_f32, GRID_LINE))
                                                    .corner_radius(CornerRadius::same(11))
                                                    .min_size(Vec2::splat(22.0));
                                                if ui.add(q_btn).on_hover_text("Hardware Physics & Tip").clicked() {
                                                    self.overlay = Overlay::Comp(comp.doc_key());
                                                    self.sfx.play(SfxEvent::Select);
                                                }
                                            });
                                        });
                                    })
                                    .response;

                                let card_click = ui.interact(frame_resp.rect, ui.id().with(("comp_card", comp)), egui::Sense::click());
                                if card_click.clicked() && count > 0 && self.overlay == Overlay::None {
                                    self.sim.selected = Some(comp);
                                    self.sfx.play(SfxEvent::Select);
                                }
                                ui.add_space(6.0);
                            }
                        });
                }

                // Bottom-anchored actions inside sidebar
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;

                    // Bottom Action Row (declare full 40px height upfront so bottom_up never clips)
                    let row_w = ui.available_width();
                    ui.allocate_ui_with_layout(
                        Vec2::new(row_w, 40.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            if self.sim.sandbox_mode {
                                let clr_btn = egui::Button::new(RichText::new("CLEAR BENCH").strong().size(12.5).color(ACCENT_RED))
                                    .fill(Color32::from_rgb(42, 20, 28))
                                    .stroke(Stroke::new(1.0_f32, ACCENT_RED))
                                    .corner_radius(CornerRadius::same(6))
                                    .min_size(Vec2::new(112.0, 40.0));
                                if ui.add(clr_btn).clicked() {
                                    self.sim.clear_sandbox();
                                    self.sfx.play(SfxEvent::Remove);
                                }

                                let share_btn = egui::Button::new(RichText::new("SHARE / LOAD LAB").strong().size(12.5).color(BG_DARK))
                                    .fill(ACCENT_PURPLE)
                                    .corner_radius(CornerRadius::same(6))
                                    .min_size(Vec2::new(ui.available_width(), 40.0));
                                if ui.add(share_btn).clicked() {
                                    self.sim.refresh_sandbox_json();
                                    self.overlay = Overlay::SandboxShare;
                                    self.sfx.play(SfxEvent::Select);
                                }
                            } else {
                                let can_prev = self.sim.level_idx > 0;
                                let prev_btn = egui::Button::new(
                                    RichText::new("<- PREV")
                                        .strong()
                                        .size(13.5)
                                        .color(if can_prev { TEXT_WHITE } else { TEXT_MUTED }),
                                )
                                .fill(if can_prev { PANEL_BG } else { Color32::from_rgb(24, 30, 42) })
                                .stroke(Stroke::new(1.0_f32, if can_prev { GRID_LINE } else { Color32::from_rgb(34, 40, 52) }))
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(92.0, 40.0));
                                if ui.add_enabled(can_prev, prev_btn).clicked() {
                                    self.sfx.play(SfxEvent::Select);
                                    let p = self.sim.level_idx - 1;
                                    self.switch_level(p, None);
                                }

                                let done = self.sim.level_complete;
                                let next_lbl = if done { "NEXT LEVEL ->" } else { "SOLVE TO ADVANCE" };
                                let next_btn = egui::Button::new(
                                    RichText::new(next_lbl)
                                        .strong()
                                        .size(13.5)
                                        .color(if done { BG_DARK } else { TEXT_MUTED }),
                                )
                                .fill(if done { ACCENT_GREEN } else { Color32::from_rgb(30, 38, 50) })
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(ui.available_width(), 40.0));
                                if ui.add_enabled(done, next_btn).clicked() {
                                    self.sfx.play(SfxEvent::Select);
                                    let n = (self.sim.level_idx + 1) % self.sim.levels.len();
                                    self.switch_level(n, None);
                                }
                            }
                        },
                    );

                    // Reset Level / Stop Beam button
                    let reset_lbl = if self.sim.sandbox_mode { "RESET BEAM  [R]" } else { "RESET LEVEL  [R]" };
                    let reset_btn = egui::Button::new(RichText::new(reset_lbl).strong().size(13.5).color(TEXT_WHITE))
                        .fill(PANEL_BG)
                        .stroke(Stroke::new(1.0_f32, GRID_LINE))
                        .corner_radius(CornerRadius::same(6))
                        .min_size(Vec2::new(ui.available_width(), 36.0));
                    if ui.add(reset_btn).clicked() {
                        self.sfx.play(SfxEvent::Select);
                        if self.sim.sandbox_mode {
                            self.sim.sync_sandbox_from_bp();
                        } else {
                            let cur = self.sim.level_idx;
                            self.switch_level(cur, Some(false));
                        }
                    }

                    // Fire Laser button
                    let fire_btn = egui::Button::new(RichText::new("FIRE LASER  [SPACE]").strong().size(14.5).color(TEXT_WHITE))
                        .fill(ACCENT_RED)
                        .corner_radius(CornerRadius::same(6))
                        .min_size(Vec2::new(ui.available_width(), 40.0));
                    if ui.add(fire_btn).clicked() {
                        self.sim.fire_photons();
                    }

                    // Keybind legend
                    let hint = if self.sim.sandbox_mode {
                        "SPACE Fire • Q/E Aim • S Exit Sandbox • P Pause"
                    } else {
                        "SPACE Fire • S Sandbox • P Pause • R Reset • H Info"
                    };
                    ui.label(
                        RichText::new(hint)
                            .size(10.5)
                            .color(TEXT_MUTED),
                    );

                    // Version / Self-Update Pill (Native) or Web Edition Badge (WASM)
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let snap = self.updater.snapshot();
                        let (u_bg, u_bd, u_fg, u_lbl) = match snap.state {
                            UpdateState::Available => {
                                let v = snap.release_info.as_ref().map(|r| r.version.as_str()).unwrap_or("");
                                (Color32::from_rgb(16, 48, 38), ACCENT_GREEN, ACCENT_GREEN, format!("UPDATE AVAILABLE: v{v} • CLICK"))
                            }
                            UpdateState::Checking => (Color32::from_rgb(48, 38, 18), ACCENT_AMBER, ACCENT_AMBER, "CHECKING UPDATES...".to_string()),
                            UpdateState::Downloading | UpdateState::Verifying | UpdateState::Installing => {
                                (Color32::from_rgb(48, 38, 18), ACCENT_AMBER, ACCENT_AMBER, "UPDATING APP...".to_string())
                            }
                            _ => (Color32::from_rgb(20, 27, 39), GRID_LINE, TEXT_MUTED, format!("v{APP_VERSION}  •  CHECK FOR UPDATES")),
                        };
                        let u_btn = egui::Button::new(RichText::new(u_lbl).strong().size(10.5).color(u_fg))
                            .fill(u_bg)
                            .stroke(Stroke::new(1.0_f32, u_bd))
                            .corner_radius(CornerRadius::same(5))
                            .min_size(Vec2::new(ui.available_width(), 25.0));
                        if ui.add(u_btn).clicked() {
                            self.sfx.play(SfxEvent::Select);
                            self.updater.mark_seen();
                            if matches!(snap.state, UpdateState::Idle | UpdateState::UpToDate | UpdateState::Error) {
                                self.updater.check_for_updates(false);
                            }
                            self.overlay = Overlay::Update;
                        }
                    }

                    #[cfg(target_arch = "wasm32")]
                    {
                        let w_btn = egui::Button::new(
                            RichText::new(format!("v{APP_VERSION} WEB  •  DESKTOP DOWNLOADS"))
                                .strong()
                                .size(10.5)
                                .color(ACCENT_CYAN),
                        )
                        .fill(Color32::from_rgb(20, 27, 39))
                        .stroke(Stroke::new(1.0_f32, GRID_LINE))
                        .corner_radius(CornerRadius::same(5))
                        .min_size(Vec2::new(ui.available_width(), 25.0));
                        if ui.add(w_btn).clicked() {
                            ctx.open_url(egui::OpenUrl::new_tab(DOWNLOAD_PAGE_URL));
                        }
                    }
                });
            });

        // Bottom Telemetry & Oscilloscope Panel
        egui::TopBottomPanel::bottom("telemetry_deck")
            .exact_height(190.0)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(PANEL_BG)
                    .stroke(Stroke::new(1.0_f32, GRID_LINE))
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ctx, |ui| {
                let action = draw_telemetry_deck(ui, &mut self.sim);
                if let Some(target_idx) = action.jump_level {
                    self.sfx.play(SfxEvent::Select);
                    self.switch_level(target_idx, None);
                }
                if action.toggle_sandbox {
                    self.toggle_sandbox();
                }
                if action.open_qkd_inspector {
                    self.sfx.play(SfxEvent::Select);
                    self.overlay = Overlay::QkdAes;
                }
            });

        // Center Optical Bench Canvas
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(BG_DARK).inner_margin(egui::Margin::same(0)))
            .show(ctx, |ui| {
                let ov_open = self.overlay != Overlay::None;
                draw_optical_bench(ui, &mut self.sim, ov_open);
            });

        // Modal Overlays
        draw_modal_overlay(
            ctx,
            &mut self.overlay,
            &mut self.sim,
            #[cfg(not(target_arch = "wasm32"))]
            &self.updater,
        );

        self.flush_sfx();
        ctx.request_repaint();
    }
}
