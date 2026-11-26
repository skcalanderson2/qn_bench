//! Main eframe::App implementation: sidebar, fixed-timestep loop, keyboard shortcuts, and persistence.

use crate::audio::SoundEngine;
use crate::glossary::GlossaryState;
use crate::model::*;
use crate::render_bench::draw_optical_bench;
use crate::render_scopes::draw_telemetry_deck;
use crate::sim::{BenchSim, SfxEvent};
use crate::ui_modals::draw_modal_overlay;
#[cfg(target_arch = "wasm32")]
use crate::version::{APP_VERSION, DOWNLOAD_PAGE_URL};
use crate::video::{is_web_video_open, open_lab_video, VideoCatalog};
use egui::{Color32, CornerRadius, Key, RichText, Stroke, Vec2};
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use crate::updater::{UpdateManager, UpdateState};

const STORAGE_KEY: &str = "qn_bench_state_v2";
const STEP_DT: f64 = 1.0 / 60.0;

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedState {
    #[serde(default)]
    completed_levels: Vec<usize>,
    #[serde(default)]
    level_stars: std::collections::BTreeMap<usize, u8>,
    #[serde(default)]
    muted: bool,
    #[serde(default)]
    sandbox_bp: Option<SandboxBlueprint>,
    #[serde(default = "default_true")]
    align_laser: bool,
    #[serde(default)]
    colorblind_mode: bool,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            completed_levels: Vec::new(),
            level_stars: std::collections::BTreeMap::new(),
            muted: false,
            sandbox_bp: None,
            align_laser: true,
            colorblind_mode: false,
        }
    }
}

pub struct QnBenchApp {
    sim: BenchSim,
    sfx: SoundEngine,
    videos: VideoCatalog,
    glossary: GlossaryState,
    emblem_tex: egui::TextureHandle,
    overlay: Overlay,
    accum_dt: f64,
    #[cfg(target_arch = "wasm32")]
    last_hash: String,
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
                sim.level_stars.entry(idx).or_insert(1);
            }
        }
        for (idx, stars) in saved.level_stars {
            if idx < sim.levels.len() && stars > 0 {
                sim.completed_levels.insert(idx);
                sim.level_stars.insert(idx, stars.min(3));
            }
        }
        if let Some(bp) = saved.sandbox_bp {
            let _ = sim.load_sandbox_blueprint(bp);
        }
        sim.align_laser = saved.align_laser;
        sim.colorblind_mode = saved.colorblind_mode;
        sim.seen_briefings.insert(0);

        #[allow(unused_mut)]
        let mut initial_overlay = Overlay::Level(0);

        // On WebAssembly, automatically open shared Sandbox URLs (`#bp=<base64url>`)
        #[cfg(target_arch = "wasm32")]
        let last_hash = {
            let h = web_sys::window()
                .and_then(|w| w.location().hash().ok())
                .unwrap_or_default();
            if h.contains("#bp=") {
                if let Ok(bp) = SandboxBlueprint::from_share_str(&h) {
                    let bp_name = bp.name.clone();
                    if sim.load_sandbox_blueprint(bp).is_ok() {
                        sim.enter_sandbox();
                        sim.status_msg = format!("Loaded shared Sandbox blueprint: {bp_name}!");
                        sim.status_color = ACCENT_GREEN;
                        initial_overlay = Overlay::None;
                    }
                }
            }
            h
        };

        let sfx = SoundEngine::new(saved.muted);

        #[cfg(not(target_arch = "wasm32"))]
        let updater = {
            let u = UpdateManager::new();
            u.check_for_updates(true);
            u
        };

        let (ew, eh, ergba) = crate::emblem::render_app_emblem_rgba();
        let emblem_tex = cc.egui_ctx.load_texture(
            "qn_bench_emblem",
            egui::ColorImage::from_rgba_unmultiplied([ew, eh], &ergba),
            egui::TextureOptions::LINEAR,
        );

        Self {
            sim,
            sfx,
            videos: VideoCatalog::new(),
            glossary: GlossaryState::default(),
            emblem_tex,
            overlay: initial_overlay,
            accum_dt: 0.0,
            #[cfg(target_arch = "wasm32")]
            last_hash,
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
        if is_web_video_open() {
            return;
        }
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
                    self.sim.confirm_reset_progress = false;
                    self.sfx.play(SfxEvent::Select);
                }
                return;
            }
            if !text_focused {
                if self.overlay == Overlay::UiHelp {
                    if ctx.input(|i| i.key_pressed(Key::F1) || i.key_pressed(Key::Enter)) {
                        self.overlay = Overlay::None;
                        self.sfx.play(SfxEvent::Select);
                        return;
                    }
                    if ctx.input(|i| !i.modifiers.command && !i.modifiers.ctrl && i.key_pressed(Key::T)) {
                        self.overlay = Overlay::Glossary;
                        self.sfx.play(SfxEvent::Select);
                        return;
                    }
                } else if self.overlay == Overlay::Glossary {
                    if ctx.input(|i| i.key_pressed(Key::F1)) {
                        self.overlay = Overlay::UiHelp;
                        self.sfx.play(SfxEvent::Select);
                        return;
                    }
                } else if matches!(self.overlay, Overlay::Level(_) | Overlay::Comp(_)) {
                    if let Overlay::Level(idx) = self.overlay {
                        if ctx.input(|i| i.key_pressed(Key::V)) {
                            if let Some(vid) = self.videos.id_for_level(idx) {
                                open_lab_video(self.sim.levels[idx].title, vid);
                                self.sfx.play(SfxEvent::Select);
                                return;
                            }
                        }
                    }
                    if ctx.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space) || i.key_pressed(Key::H)) {
                        self.overlay = Overlay::None;
                        self.sfx.play(SfxEvent::Select);
                    }
                }
            }
            return;
        }

        if text_focused {
            return;
        }

        let (fire, reset, pause, step_dot, ui_help_f1, glossary_t, help, hint_i, video_v, align_l, report_g, erase_x, a11y_c, undo_k, redo_k, enter_next, qkd_k, update_u, mute, sandbox_s, rot_ccw, rot_cw, prev_l, next_l, num_jump) = ctx.input(|i| {
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
            let cmd = i.modifiers.command || i.modifiers.ctrl;
            let undo = cmd && !i.modifiers.shift && i.key_pressed(Key::Z);
            let redo = (cmd && i.modifiers.shift && i.key_pressed(Key::Z)) || (cmd && i.key_pressed(Key::Y));
            (
                i.key_pressed(Key::Space),
                !cmd && i.key_pressed(Key::R),
                i.key_pressed(Key::P),
                i.key_pressed(Key::Period),
                i.key_pressed(Key::F1) || (i.modifiers.shift && i.key_pressed(Key::Slash)),
                !cmd && i.key_pressed(Key::T),
                i.key_pressed(Key::H),
                i.key_pressed(Key::I),
                !cmd && i.key_pressed(Key::V),
                i.key_pressed(Key::L),
                !cmd && i.key_pressed(Key::G),
                !cmd && i.key_pressed(Key::X),
                !cmd && i.key_pressed(Key::C),
                undo,
                redo,
                i.key_pressed(Key::Enter),
                i.key_pressed(Key::K),
                i.key_pressed(Key::U),
                i.key_pressed(Key::M),
                !cmd && i.key_pressed(Key::S),
                !cmd && i.key_pressed(Key::Q),
                !cmd && i.key_pressed(Key::E),
                i.key_pressed(Key::OpenBracket) || i.key_pressed(Key::ArrowLeft),
                i.key_pressed(Key::CloseBracket) || i.key_pressed(Key::ArrowRight),
                jump,
            )
        });

        if undo_k {
            self.sim.undo();
        } else if redo_k {
            self.sim.redo();
        } else if ui_help_f1 {
            self.overlay = Overlay::UiHelp;
            self.sfx.play(SfxEvent::Select);
        } else if glossary_t {
            self.overlay = Overlay::Glossary;
            self.sfx.play(SfxEvent::Select);
        } else if step_dot {
            self.sim.step_one_paused_frame();
            self.sfx.play(SfxEvent::Select);
        } else if report_g {
            self.sim.sandbox_json_msg = None;
            self.sim.confirm_reset_progress = false;
            self.overlay = Overlay::LabReport;
            self.sfx.play(SfxEvent::Select);
        } else if video_v && !self.sim.sandbox_mode {
            if let Some(vid) = self.videos.id_for_level(self.sim.level_idx) {
                open_lab_video(self.sim.levels[self.sim.level_idx].title, vid);
                self.sfx.play(SfxEvent::Select);
            }
        } else if erase_x {
            self.sim.erase_mode = !self.sim.erase_mode;
            self.sfx.play(SfxEvent::Select);
        } else if a11y_c {
            self.sim.colorblind_mode = !self.sim.colorblind_mode;
            self.sfx.play(SfxEvent::Select);
        } else if align_l {
            self.sim.align_laser = !self.sim.align_laser;
            self.sfx.play(SfxEvent::Select);
        } else if hint_i && !self.sim.sandbox_mode {
            self.sim.hint_tier = self.sim.hint_tier.max(1);
            self.overlay = Overlay::Level(self.sim.level_idx);
            self.sfx.play(SfxEvent::Select);
        } else if enter_next && self.sim.level_complete && !self.sim.sandbox_mode {
            self.sfx.play(SfxEvent::Select);
            let next = (self.sim.level_idx + 1) % self.sim.levels.len();
            self.switch_level(next, None);
        } else if fire {
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
            level_stars: self.sim.level_stars.clone(),
            muted: self.sfx.muted,
            sandbox_bp: Some(self.sim.sandbox_bp.clone()),
            align_laser: self.sim.align_laser,
            colorblind_mode: self.sim.colorblind_mode,
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

        self.videos.poll_updates();
        self.handle_keyboard(ctx);

        // On WebAssembly, detect live URL hash changes (`#bp=...`) without requiring page reload
        #[cfg(target_arch = "wasm32")]
        if let Some(h) = web_sys::window().and_then(|w| w.location().hash().ok()) {
            if h != self.last_hash {
                self.last_hash = h.clone();
                if h.contains("#bp=") {
                    if let Ok(bp) = SandboxBlueprint::from_share_str(&h) {
                        let bp_name = bp.name.clone();
                        if self.sim.load_sandbox_blueprint(bp).is_ok() {
                            self.sim.enter_sandbox();
                            self.sim.status_msg = format!("Loaded shared Sandbox blueprint: {bp_name}!");
                            self.sim.status_color = ACCENT_GREEN;
                            self.overlay = Overlay::None;
                            self.sfx.play(SfxEvent::Place);
                        }
                    }
                }
            }
        }

        // Poll native updater periodically in the background and handle ready-to-quit relaunch
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.updater.maybe_periodic_check();
            let snap = self.updater.snapshot();
            if snap.ready_to_quit {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
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
                            let i_btn = egui::Button::new(RichText::new("BRIEFING").strong().size(10.5).color(ACCENT_CYAN))
                                .fill(Color32::from_rgb(22, 34, 50))
                                .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
                                .corner_radius(CornerRadius::same(5))
                                .min_size(Vec2::new(66.0, 24.0));
                            if ui
                                .add(i_btn)
                                .on_hover_text("Open Current Level Briefing & Hints [H]")
                                .clicked()
                            {
                                self.overlay = Overlay::Level(self.sim.level_idx);
                                self.sfx.play(SfxEvent::Select);
                            }
                        }
                    });
                });

                // Reference Guides Bar: UI HELP [F1] + GLOSSARY [T]
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let half_w = (ui.available_width() - 6.0) * 0.5;
                    let help_btn = egui::Button::new(
                        RichText::new("? UI HELP  [F1]")
                            .strong()
                            .size(10.5)
                            .color(ACCENT_CYAN),
                    )
                    .fill(Color32::from_rgb(18, 34, 48))
                    .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
                    .corner_radius(CornerRadius::same(5))
                    .min_size(Vec2::new(half_w, 24.0));
                    if ui
                        .add(help_btn)
                        .on_hover_text("Open QN Bench UI Help Guide & Controls Manual [F1 or ?]")
                        .clicked()
                    {
                        self.overlay = Overlay::UiHelp;
                        self.sfx.play(SfxEvent::Select);
                    }

                    let gl_btn = egui::Button::new(
                        RichText::new("GLOSSARY  [T]")
                            .strong()
                            .size(10.5)
                            .color(ACCENT_AMBER),
                    )
                    .fill(Color32::from_rgb(36, 28, 18))
                    .stroke(Stroke::new(1.0_f32, ACCENT_AMBER))
                    .corner_radius(CornerRadius::same(5))
                    .min_size(Vec2::new(half_w, 24.0));
                    if ui
                        .add(gl_btn)
                        .on_hover_text("Open Quantum Terminology, Abbreviations & Component Guide [T]")
                        .clicked()
                    {
                        self.overlay = Overlay::Glossary;
                        self.sfx.play(SfxEvent::Select);
                    }
                });

                // Touch & Accessibility Mode Bar: ERASE [X] + A11Y COLORBLIND [C]
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let half_w = (ui.available_width() - 6.0) * 0.5;
                    let er_on = self.sim.erase_mode;
                    let er_btn = egui::Button::new(
                        RichText::new(if er_on { "ERASE: ON [X]" } else { "ERASE TOOL [X]" })
                            .strong()
                            .size(10.5)
                            .color(if er_on { TEXT_WHITE } else { ACCENT_RED }),
                    )
                    .fill(if er_on { ACCENT_RED } else { Color32::from_rgb(38, 20, 26) })
                    .stroke(Stroke::new(1.0_f32, ACCENT_RED))
                    .corner_radius(CornerRadius::same(5))
                    .min_size(Vec2::new(half_w, 24.0));
                    if ui
                        .add(er_btn)
                        .on_hover_text("Toggle Touch/Tap Erase Mode [X] — Left-Click or tap any placed optic to remove it")
                        .clicked()
                    {
                        self.sim.erase_mode = !self.sim.erase_mode;
                        self.sfx.play(SfxEvent::Select);
                    }

                    let cb_on = self.sim.colorblind_mode;
                    let cb_btn = egui::Button::new(
                        RichText::new(if cb_on { "A11Y: ON [C]" } else { "A11Y PATTERNS [C]" })
                            .strong()
                            .size(10.5)
                            .color(if cb_on { BG_DARK } else { ACCENT_CYAN }),
                    )
                    .fill(if cb_on { ACCENT_CYAN } else { Color32::from_rgb(18, 30, 42) })
                    .stroke(Stroke::new(1.0_f32, if cb_on { ACCENT_CYAN } else { GRID_LINE }))
                    .corner_radius(CornerRadius::same(5))
                    .min_size(Vec2::new(half_w, 24.0));
                    if ui
                        .add(cb_btn)
                        .on_hover_text("Toggle Colorblind-Friendly / High-Contrast Wavelength Shapes & Badges [C]")
                        .clicked()
                    {
                        self.sim.colorblind_mode = !self.sim.colorblind_mode;
                        self.sfx.play(SfxEvent::Select);
                    }
                });
                ui.add_space(4.0);

                if self.sim.sandbox_mode {
                    ui.label(RichText::new("Tap: Place / Rotate  |  [X] Erase  |  R-Click: Erase").size(11.0).color(TEXT_MUTED));
                    ui.add_space(5.0);

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

                    // Reserve space for bottom-anchored playback controls (plus bright update pill only when visible)
                    #[cfg(not(target_arch = "wasm32"))]
                    let bottom_reserve = if self.updater.snapshot().state.should_show_button() { 202.0 } else { 166.0 };
                    #[cfg(target_arch = "wasm32")]
                    let bottom_reserve = 196.0;
                    let cards_max_h = (ui.available_height() - bottom_reserve).max(80.0);
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
                                let is_sel = !self.sim.erase_mode && self.sim.sandbox_tool == tool;
                                let bg = if is_sel { Color32::from_rgb(36, 48, 68) } else { PANEL_BG };
                                let bd = if is_sel { tool.color() } else { GRID_LINE };

                                let mut q_rect = egui::Rect::NOTHING;
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
                                                let q_btn = egui::Button::new(RichText::new("?").strong().size(11.5).color(ACCENT_CYAN))
                                                    .fill(Color32::from_rgb(20, 28, 42))
                                                    .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
                                                    .corner_radius(CornerRadius::same(12))
                                                    .min_size(Vec2::splat(24.0));
                                                let q_resp = ui.add(q_btn).on_hover_text("Open Hardware Physics & Tip");
                                                q_rect = q_resp.rect;
                                                if q_resp.clicked() && self.overlay == Overlay::None {
                                                    self.overlay = Overlay::Comp(tool.doc_key());
                                                    self.sfx.play(SfxEvent::Select);
                                                }
                                            });
                                        });
                                    })
                                    .response;

                                let select_rect = egui::Rect::from_min_max(
                                    frame_resp.rect.min,
                                    egui::Pos2::new((q_rect.left() - 4.0).max(frame_resp.rect.left()), frame_resp.rect.max.y),
                                );
                                let card_click = ui.interact(select_rect, ui.id().with(("sb_card", idx)), egui::Sense::click());
                                if card_click.clicked() && self.overlay == Overlay::None {
                                    self.sim.sandbox_tool = tool;
                                    self.sim.erase_mode = false;
                                    self.sfx.play(SfxEvent::Select);
                                }
                                ui.add_space(5.0);
                            }
                        });
                } else {
                    ui.label(RichText::new("Tap: Place  |  [X] Erase  |  R-Click: Remove").size(11.5).color(TEXT_MUTED));
                    ui.add_space(6.0);

                    // Component Cards (scrollable on smaller windows so bottom buttons never clip)
                    // Reserve space for bottom-anchored playback controls (plus bright update pill only when visible)
                    #[cfg(not(target_arch = "wasm32"))]
                    let bottom_reserve = if self.updater.snapshot().state.should_show_button() { 202.0 } else { 166.0 };
                    #[cfg(target_arch = "wasm32")]
                    let bottom_reserve = 196.0;
                    let cards_max_h = (ui.available_height() - bottom_reserve).max(80.0);
                    egui::ScrollArea::vertical()
                        .id_salt("comp_cards_scroll")
                        .max_height(cards_max_h)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            let active = self.sim.active_components();
                            for comp in active {
                                let count = self.sim.inventory.get(&comp).copied().unwrap_or(0);
                                let is_sel = !self.sim.erase_mode && self.sim.selected == Some(comp) && count > 0;
                                let bg = if is_sel { Color32::from_rgb(36, 48, 68) } else { PANEL_BG };
                                let bd = if is_sel { comp.color() } else if count > 0 { GRID_LINE } else { Color32::from_rgb(30, 36, 46) };

                                let mut q_rect = egui::Rect::NOTHING;
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
                                                let q_btn = egui::Button::new(RichText::new("?").strong().size(11.5).color(ACCENT_CYAN))
                                                    .fill(Color32::from_rgb(20, 28, 42))
                                                    .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
                                                    .corner_radius(CornerRadius::same(12))
                                                    .min_size(Vec2::splat(24.0));
                                                let q_resp = ui.add(q_btn).on_hover_text("Open Hardware Physics & Tip");
                                                q_rect = q_resp.rect;
                                                if q_resp.clicked() && self.overlay == Overlay::None {
                                                    self.overlay = Overlay::Comp(comp.doc_key());
                                                    self.sfx.play(SfxEvent::Select);
                                                }
                                            });
                                        });
                                    })
                                    .response;

                                let select_rect = egui::Rect::from_min_max(
                                    frame_resp.rect.min,
                                    egui::Pos2::new((q_rect.left() - 4.0).max(frame_resp.rect.left()), frame_resp.rect.max.y),
                                );
                                let card_click = ui.interact(select_rect, ui.id().with(("comp_card", comp)), egui::Sense::click());
                                if card_click.clicked() && count > 0 && self.overlay == Overlay::None {
                                    self.sim.selected = Some(comp);
                                    self.sim.erase_mode = false;
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

                    // Playback Secondary Row: PAUSE/RESUME [P] + RESET [R]
                    ui.allocate_ui_with_layout(
                        Vec2::new(row_w, 36.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let active = self.sim.sim_active;
                            let paused = self.sim.paused;
                            let p_lbl = if active && paused { "RESUME [P]" } else { "PAUSE [P]" };
                            let p_fg = if !active {
                                TEXT_MUTED
                            } else if paused {
                                BG_DARK
                            } else {
                                ACCENT_AMBER
                            };
                            let p_bg = if !active {
                                Color32::from_rgb(22, 28, 38)
                            } else if paused {
                                ACCENT_AMBER
                            } else {
                                Color32::from_rgb(42, 34, 18)
                            };
                            let p_bd = if active { ACCENT_AMBER } else { Color32::from_rgb(34, 42, 54) };
                            let p_btn = egui::Button::new(RichText::new(p_lbl).strong().size(12.0).color(p_fg))
                                .fill(p_bg)
                                .stroke(Stroke::new(1.0_f32, p_bd))
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(112.0, 36.0));
                            if ui
                                .add_enabled(active, p_btn)
                                .on_hover_text("Pause or resume active photon simulation [P]")
                                .clicked()
                            {
                                self.sim.paused = !self.sim.paused;
                                self.sfx.play(SfxEvent::Select);
                            }

                            let reset_lbl = if self.sim.sandbox_mode { "RESET BEAM  [R]" } else { "RESET LEVEL  [R]" };
                            let reset_btn = egui::Button::new(RichText::new(reset_lbl).strong().size(12.5).color(TEXT_WHITE))
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
                        },
                    );

                    // Playback Primary Row: FIRE LASER [SPACE] + STEP [.]
                    ui.allocate_ui_with_layout(
                        Vec2::new(row_w, 40.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            let step_w = 88.0;
                            let fire_w = (row_w - step_w - 6.0).max(120.0);
                            let fire_btn = egui::Button::new(RichText::new("FIRE LASER  [SPACE]").strong().size(13.5).color(TEXT_WHITE))
                                .fill(ACCENT_RED)
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(fire_w, 40.0));
                            if ui.add(fire_btn).clicked() {
                                self.sim.fire_photons();
                            }

                            let step_paused = self.sim.sim_active && self.sim.paused;
                            let step_btn = egui::Button::new(
                                RichText::new("STEP  [.]")
                                    .strong()
                                    .size(12.5)
                                    .color(if step_paused { BG_DARK } else { ACCENT_CYAN }),
                            )
                            .fill(if step_paused { ACCENT_CYAN } else { Color32::from_rgb(18, 34, 48) })
                            .stroke(Stroke::new(1.0_f32, ACCENT_CYAN))
                            .corner_radius(CornerRadius::same(6))
                            .min_size(Vec2::new(ui.available_width(), 40.0));
                            if ui
                                .add(step_btn)
                                .on_hover_text("Step forward +5ps per click (auto-fires paused if Ready) [.]")
                                .clicked()
                            {
                                self.sim.step_one_paused_frame();
                                self.sfx.play(SfxEvent::Select);
                            }
                        },
                    );

                    // Keybind legend
                    let hint = if self.sim.sandbox_mode {
                        "SPACE Fire • . Step • Q/E Aim • T Glossary"
                    } else {
                        "SPACE Fire • . Step • F1 Help • T Glossary"
                    };
                    ui.label(
                        RichText::new(hint)
                            .size(10.5)
                            .color(TEXT_MUTED),
                    );

                    // Bright Self-Update Button (Native: hidden unless an update is available or installing)
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let snap = self.updater.snapshot();
                        if snap.state.should_show_button() {
                            let v = snap
                                .release_info
                                .as_ref()
                                .map(|r| r.version.as_str())
                                .unwrap_or("");
                            let (u_bg, u_bd, u_fg, u_lbl) = match snap.state {
                                UpdateState::Available => (
                                    Color32::from_rgb(250, 204, 21), // Bright vivid amber-gold
                                    Color32::from_rgb(254, 249, 195),
                                    BG_DARK,
                                    format!("UPDATE AVAILABLE: v{v}  •  INSTALL"),
                                ),
                                _ => (
                                    ACCENT_CYAN,
                                    Color32::from_rgb(207, 250, 254),
                                    BG_DARK,
                                    if v.is_empty() {
                                        "UPDATING APP...".to_string()
                                    } else {
                                        format!("UPDATING TO v{v}...")
                                    },
                                ),
                            };
                            let u_btn = egui::Button::new(RichText::new(u_lbl).strong().size(11.0).color(u_fg))
                                .fill(u_bg)
                                .stroke(Stroke::new(1.5_f32, u_bd))
                                .corner_radius(CornerRadius::same(6))
                                .min_size(Vec2::new(ui.available_width(), 28.0));
                            if ui
                                .add(u_btn)
                                .on_hover_text("A new version of QN Bench is ready — click to view release notes & install [U]")
                                .clicked()
                            {
                                self.sfx.play(SfxEvent::Select);
                                self.overlay = Overlay::Update;
                            }
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
            .exact_height(206.0)
            .resizable(false)
            .frame(
                egui::Frame::NONE
                    .fill(PANEL_BG)
                    .stroke(Stroke::new(1.0_f32, GRID_LINE))
                    .inner_margin(egui::Margin::symmetric(12, 8)),
            )
            .show(ctx, |ui| {
                let has_video = !self.sim.sandbox_mode
                    && self.videos.id_for_level(self.sim.level_idx).is_some();
                let action = draw_telemetry_deck(ui, &mut self.sim, has_video);
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
                if action.open_hint && !self.sim.sandbox_mode {
                    self.sim.hint_tier = self.sim.hint_tier.max(1);
                    self.overlay = Overlay::Level(self.sim.level_idx);
                    self.sfx.play(SfxEvent::Select);
                }
                if action.open_lab_report {
                    self.sim.sandbox_json_msg = None;
                    self.sim.confirm_reset_progress = false;
                    self.overlay = Overlay::LabReport;
                    self.sfx.play(SfxEvent::Select);
                }
                if action.open_video && !self.sim.sandbox_mode {
                    if let Some(vid) = self.videos.id_for_level(self.sim.level_idx) {
                        open_lab_video(self.sim.levels[self.sim.level_idx].title, vid);
                        self.sfx.play(SfxEvent::Select);
                    }
                }
            });

        // Center Optical Bench Canvas
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(BG_DARK).inner_margin(egui::Margin::same(0)))
            .show(ctx, |ui| {
                let ov_open = self.overlay != Overlay::None;
                if let Some(doc_key) = draw_optical_bench(ui, &mut self.sim, ov_open) {
                    self.overlay = Overlay::Comp(doc_key);
                    self.sfx.play(SfxEvent::Select);
                }
            });

        // Modal Overlays
        draw_modal_overlay(
            ctx,
            &mut self.overlay,
            &mut self.sim,
            &self.videos,
            &mut self.glossary,
            &self.emblem_tex,
            #[cfg(not(target_arch = "wasm32"))]
            &self.updater,
        );

        self.flush_sfx();
        ctx.request_repaint();
    }
}
