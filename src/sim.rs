//! Decoupled optical-bench simulation engine, particles, shockwaves, and full physics test suite.

use crate::levels::build_levels;
use crate::model::*;
use crate::qkd::{QkdSession, QkdVerdict};
use egui::Color32;
use rand::Rng;
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SfxEvent {
    Place,
    Remove,
    Select,
    Fire,
    Qfc,
    Vbg,
    Memory,
    Delay,
    Mirror,
    SnspdOk,
    BsmOk,
    Fail,
    Hwp,
    Distill,
    Ao,
    Eve,
    AesLock,
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub color: Color32,
    pub life: i32,
    pub max_life: i32,
    pub size: f32,
}

impl Particle {
    pub fn new(x: f32, y: f32, color: Color32, speed: f32) -> Self {
        let mut rng = rand::thread_rng();
        let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
        let spd: f32 = rng.gen_range(0.5..speed.max(0.6));
        Self {
            x,
            y,
            vx: angle.cos() * spd,
            vy: angle.sin() * spd,
            color,
            life: 28,
            max_life: 28,
            size: 3.0,
        }
    }

    pub fn update(&mut self) {
        self.x += self.vx;
        self.y += self.vy;
        self.vx *= 0.94;
        self.vy *= 0.94;
        self.life -= 1;
    }
}

#[derive(Debug, Clone)]
pub struct Shockwave {
    pub x: f32,
    pub y: f32,
    pub color: Color32,
    pub max_r: f32,
    pub life: i32,
    pub max_life: i32,
}

impl Shockwave {
    pub fn new(x: f32, y: f32, color: Color32) -> Self {
        Self { x, y, color, max_r: 45.0, life: 24, max_life: 24 }
    }

    pub fn update(&mut self) {
        self.life -= 1;
    }
}

#[derive(Debug, Clone)]
pub struct Photon {
    pub pid: String,
    pub px: f32,
    pub py: f32,
    pub dir: Dir,
    pub wavelength: u32,
    pub noise: i32,
    pub fidelity: i32,
    pub pol: Pol,
    pub pmd_flips: u32,
    pub eve_tapped: bool,
    pub ao_locked: bool,
    pub alive: bool,
    pub arrived: bool,
    pub arrival_tick: Option<u32>,
    pub distill_tick: Option<u32>,
    pub distill_pos: Option<(i32, i32)>,
    pub delay_timer: u32,
    pub speed: f32,
    pub qkd_pair_id: Option<usize>,
    pub qkd_role: char,
    pub qkd_bit: u8,
    pub last_tile: (i32, i32),
    pub trail: Vec<(f32, f32, u32)>,
    pub fid_history: Vec<(u32, i32, u32)>,
    pub noise_history: Vec<(u32, i32)>,
}

impl Photon {
    pub fn new(pid: impl Into<String>, gx: i32, gy: i32, dir: Dir, wavelength: u32) -> Self {
        Self {
            pid: pid.into(),
            px: gx as f32 * TILE_SIZE + TILE_SIZE * 0.5,
            py: gy as f32 * TILE_SIZE + TILE_SIZE * 0.5,
            dir,
            wavelength,
            noise: 0,
            fidelity: 100,
            pol: Pol::Plus,
            pmd_flips: 0,
            eve_tapped: false,
            ao_locked: false,
            alive: true,
            arrived: false,
            arrival_tick: None,
            distill_tick: None,
            distill_pos: None,
            delay_timer: 0,
            speed: 4.0,
            qkd_pair_id: None,
            qkd_role: 'B',
            qkd_bit: 0,
            last_tile: (gx, gy),
            trail: Vec::new(),
            fid_history: vec![(0, 100, wavelength)],
            noise_history: vec![(0, 0)],
        }
    }

    pub fn record_telemetry(&mut self, tick: u32) {
        self.fid_history.push((tick, self.fidelity, self.wavelength));
        self.noise_history.push((tick, self.noise));
        if self.fid_history.len() > 160 {
            self.fid_history.remove(0);
        }
        if self.noise_history.len() > 160 {
            self.noise_history.remove(0);
        }
    }
}

pub struct BenchSim {
    pub levels: Vec<LevelDef>,
    pub level_idx: usize,
    pub grid: HashMap<(i32, i32), Tile>,
    pub user_placed: HashSet<(i32, i32)>,
    pub inventory: BTreeMap<Placeable, u32>,
    pub selected: Option<Placeable>,
    pub photons: Vec<Photon>,
    pub particles: Vec<Particle>,
    pub shockwaves: Vec<Shockwave>,
    pub completed_levels: HashSet<usize>,
    pub seen_briefings: HashSet<usize>,
    pub tick: u32,
    pub frame: u64,
    pub hom_delta_ps: Option<i32>,
    pub hom_signed_ps: Option<i32>,
    pub pol_mismatch: bool,
    pub distill_fired: bool,
    pub sim_state: SimState,
    pub status_msg: String,
    pub status_color: Color32,
    pub level_complete: bool,
    pub sim_active: bool,
    pub paused: bool,
    pub sim_speed: f32,
    pub qkd_session: Option<QkdSession>,
    pub qkd_burst_remaining: usize,
    pub qkd_burst_idx: usize,
    pub qkd_last_drop_msg: Option<String>,
    pub qkd_plaintext: String,
    pub pending_sfx: Vec<SfxEvent>,
}

impl BenchSim {
    pub fn new() -> Self {
        let levels = build_levels();
        let mut s = Self {
            levels,
            level_idx: 0,
            grid: HashMap::new(),
            user_placed: HashSet::new(),
            inventory: BTreeMap::new(),
            selected: None,
            photons: Vec::new(),
            particles: Vec::new(),
            shockwaves: Vec::new(),
            completed_levels: HashSet::new(),
            seen_briefings: HashSet::new(),
            tick: 0,
            frame: 0,
            hom_delta_ps: None,
            hom_signed_ps: None,
            pol_mismatch: false,
            distill_fired: false,
            sim_state: SimState::Ready,
            status_msg: String::new(),
            status_color: TEXT_WHITE,
            level_complete: false,
            sim_active: false,
            paused: false,
            sim_speed: 1.0,
            qkd_session: None,
            qkd_burst_remaining: 0,
            qkd_burst_idx: 0,
            qkd_last_drop_msg: None,
            qkd_plaintext: "QUANTUM LINK SECURED BY 1550nm QKD".to_string(),
            pending_sfx: Vec::new(),
        };
        s.load_level(0);
        s
    }

    pub fn current_level(&self) -> &LevelDef {
        &self.levels[self.level_idx]
    }

    pub fn is_qkd_level(&self) -> bool {
        self.current_level().mode.is_qkd()
    }

    pub fn active_components(&self) -> Vec<Placeable> {
        let lvl = self.current_level();
        Placeable::ALL
            .iter()
            .copied()
            .filter(|k| lvl.inventory.get(k).copied().unwrap_or(0) > 0)
            .collect()
    }

    pub fn spawn_fx(&mut self, kind: &str, x: f32, y: f32, color: Color32, count: usize) {
        let spd = if kind == "burst" { 3.2 } else { 2.0 };
        for _ in 0..count {
            self.particles.push(Particle::new(x, y, color, spd));
        }
        if kind == "ring" {
            self.shockwaves.push(Shockwave::new(x, y, color));
        }
    }

    pub fn load_level(&mut self, idx: usize) {
        self.level_idx = idx % self.levels.len();
        let lvl = self.levels[self.level_idx].clone();
        self.grid.clear();
        self.user_placed.clear();
        for (pos, tile) in &lvl.fixed {
            self.grid.insert(*pos, *tile);
        }
        for pos in &lvl.walls {
            self.grid.insert(*pos, Tile::Wall);
        }

        self.inventory.clear();
        for k in Placeable::ALL {
            self.inventory.insert(k, lvl.inventory.get(&k).copied().unwrap_or(0));
        }
        self.selected = self
            .active_components()
            .into_iter()
            .find(|k| self.inventory.get(k).copied().unwrap_or(0) > 0);

        self.photons.clear();
        self.particles.clear();
        self.shockwaves.clear();
        self.tick = 0;
        self.hom_delta_ps = None;
        self.hom_signed_ps = None;
        self.pol_mismatch = false;
        self.distill_fired = false;
        self.qkd_burst_remaining = 0;
        self.qkd_burst_idx = 0;
        self.qkd_last_drop_msg = None;
        self.qkd_session = if lvl.mode.is_qkd() {
            Some(QkdSession::new(lvl.mode, &self.qkd_plaintext))
        } else {
            None
        };
        self.sim_state = SimState::Ready;
        self.status_msg = lvl.goal.to_string();
        self.status_color = TEXT_WHITE;
        self.level_complete = false;
        self.sim_active = false;
        self.paused = false;
    }

    pub fn place_component(&mut self, gx: i32, gy: i32) -> bool {
        let pos = (gx, gy);
        if gx < 0 || gx >= GRID_COLS || gy < 0 || gy >= GRID_ROWS {
            return false;
        }
        let lvl = self.current_level();
        if lvl.is_emitter(pos) || lvl.is_target(pos) || self.grid.contains_key(&pos) {
            return false;
        }
        let Some(comp) = self.selected else { return false };
        let count = self.inventory.get(&comp).copied().unwrap_or(0);
        if count == 0 {
            return false;
        }
        self.grid.insert(pos, Tile::Placeable(comp));
        self.user_placed.insert(pos);
        self.inventory.insert(comp, count - 1);
        if count - 1 == 0 {
            if let Some(next_c) = self
                .active_components()
                .into_iter()
                .find(|k| self.inventory.get(k).copied().unwrap_or(0) > 0)
            {
                self.selected = Some(next_c);
            }
        }
        self.reset_sim_keep_grid();
        self.pending_sfx.push(SfxEvent::Place);
        true
    }

    pub fn remove_component(&mut self, gx: i32, gy: i32) -> bool {
        let pos = (gx, gy);
        if !self.user_placed.remove(&pos) {
            return false;
        }
        if let Some(Tile::Placeable(comp)) = self.grid.remove(&pos) {
            *self.inventory.entry(comp).or_insert(0) += 1;
            self.selected = Some(comp);
        }
        self.reset_sim_keep_grid();
        self.pending_sfx.push(SfxEvent::Remove);
        true
    }

    fn reset_sim_keep_grid(&mut self) {
        self.photons.clear();
        self.sim_active = false;
        self.paused = false;
        self.sim_state = SimState::Ready;
        self.qkd_burst_remaining = 0;
        if let Some(qs) = &mut self.qkd_session {
            qs.reset();
        }
        self.hom_delta_ps = None;
        self.hom_signed_ps = None;
        self.pol_mismatch = false;
    }

    pub fn fire_photons(&mut self) {
        let lvl = self.levels[self.level_idx].clone();
        self.tick = 0;
        self.hom_delta_ps = None;
        self.hom_signed_ps = None;
        self.pol_mismatch = false;
        self.distill_fired = false;
        self.qkd_last_drop_msg = None;
        self.photons.clear();
        self.paused = false;

        if lvl.mode.is_qkd() {
            let mut qs = QkdSession::new(lvl.mode, &self.qkd_plaintext);
            qs.verdict = QkdVerdict::Running;
            self.qkd_session = Some(qs);
            self.qkd_burst_remaining = lvl.qkd_rounds;
            self.qkd_burst_idx = 0;
            self.spawn_qkd_round();
            self.pending_sfx.push(SfxEvent::Fire);
            self.sim_active = true;
            self.sim_state = SimState::Active;
            self.level_complete = false;
            let proto = if lvl.mode == TargetMode::QkdBb84 {
                "BB84 polarized qubits"
            } else {
                "E91 entangled Bell pairs"
            };
            self.status_msg = format!(
                "Streaming {} {proto} for QKD sifting & AES-256...",
                lvl.qkd_rounds
            );
            self.status_color = ACCENT_CYAN;
            return;
        }

        let mut wl_tags = Vec::new();
        for (i, e) in lvl.emitters.iter().enumerate() {
            let pid = ((b'A' + i as u8) as char).to_string();
            self.photons.push(Photon::new(pid, e.gx, e.gy, e.dir, e.wavelength));
            let fx_col = if e.wavelength == 637 { ACCENT_ORANGE } else { ACCENT_RED };
            self.spawn_fx(
                "ring",
                e.gx as f32 * TILE_SIZE + 40.0,
                e.gy as f32 * TILE_SIZE + 40.0,
                fx_col,
                8,
            );
            wl_tags.push(format!("{}nm", e.wavelength));
        }
        self.pending_sfx.push(SfxEvent::Fire);
        self.sim_active = true;
        self.sim_state = SimState::Active;
        self.level_complete = false;
        self.status_msg = format!(
            "Firing {} single photon(s) from Quantum Emitter(s)...",
            wl_tags.join("/")
        );
        self.status_color = ACCENT_RED;
    }

    fn spawn_qkd_round(&mut self) {
        let lvl = self.levels[self.level_idx].clone();
        let pair_id = self.qkd_burst_idx;
        self.qkd_burst_idx += 1;
        self.qkd_burst_remaining = self.qkd_burst_remaining.saturating_sub(1);
        let (basis, bit) = self
            .qkd_session
            .as_mut()
            .expect("qkd_session active")
            .prepare_round(pair_id);

        if lvl.mode == TargetMode::QkdBb84 {
            let e = lvl.emitters[0];
            let mut p = Photon::new(format!("Q{pair_id}"), e.gx, e.gy, e.dir, e.wavelength);
            p.pol = basis;
            p.speed = 6.0;
            p.qkd_pair_id = Some(pair_id);
            p.qkd_role = 'B';
            p.qkd_bit = bit;
            self.photons.push(p);
            if pair_id % 6 == 0 {
                self.spawn_fx(
                    "spark",
                    e.gx as f32 * TILE_SIZE + 40.0,
                    e.gy as f32 * TILE_SIZE + 40.0,
                    ACCENT_RED,
                    4,
                );
            }
        } else {
            for (idx, e) in lvl.emitters.iter().enumerate() {
                let role = if idx == 0 { 'A' } else { 'B' };
                let mut p = Photon::new(format!("{role}{pair_id}"), e.gx, e.gy, e.dir, e.wavelength);
                p.pol = basis;
                p.speed = 6.0;
                p.qkd_pair_id = Some(pair_id);
                p.qkd_role = role;
                p.qkd_bit = bit;
                self.photons.push(p);
            }
            if pair_id % 6 == 0 {
                if let Some((sx, sy)) = lvl.epr_source {
                    self.spawn_fx(
                        "ring",
                        sx as f32 * TILE_SIZE + 40.0,
                        sy as f32 * TILE_SIZE + 40.0,
                        ACCENT_PURPLE,
                        5,
                    );
                }
            }
        }
    }

    pub fn step_frame(&mut self) {
        self.frame += 1;

        if self.sim_active && !self.paused {
            self.tick += 1;
            let lvl = self.levels[self.level_idx].clone();
            let is_qkd = lvl.mode.is_qkd();

            if is_qkd && self.qkd_burst_remaining > 0 && self.tick % 4 == 0 {
                self.spawn_qkd_round();
            }

            let count = self.photons.len();
            for i in 0..count {
                if !self.photons[i].alive {
                    continue;
                }
                let p_target = match (lvl.e91_targets, self.photons[i].qkd_role) {
                    (Some((a, _)), 'A') => a,
                    (Some((_, b)), _) => b,
                    (None, _) => lvl.target,
                };
                let msg = self.update_photon(i, p_target, lvl.mode);
                if let Some(m) = msg {
                    if is_qkd {
                        if m.starts_with("FAILURE") && self.qkd_last_drop_msg.is_none() {
                            self.qkd_last_drop_msg = Some(m);
                        } else if m.starts_with("ALERT") {
                            self.status_msg = m;
                            self.status_color = ACCENT_RED;
                        }
                    } else {
                        self.status_color = if m.starts_with("FAILURE") { ACCENT_RED } else { ACCENT_AMBER };
                        self.status_msg = m;
                    }
                }
            }

            if is_qkd && self.photons.len() > 24 {
                self.photons.retain(|p| p.alive);
            }

            self.process_distillation();
            self.evaluate_simulation();
        }

        for pt in &mut self.particles {
            pt.update();
        }
        self.particles.retain(|pt| pt.life > 0);
        for sw in &mut self.shockwaves {
            sw.update();
        }
        self.shockwaves.retain(|sw| sw.life > 0);
    }

    fn emit_sfx(&mut self, ev: SfxEvent, is_qkd: bool, qkd_pair_id: Option<usize>) {
        if is_qkd {
            if let Some(pid) = qkd_pair_id {
                if pid < 2 || (ev == SfxEvent::Eve && pid % 8 == 0) {
                    self.pending_sfx.push(ev);
                }
            }
        } else {
            self.pending_sfx.push(ev);
        }
    }

    fn update_photon(&mut self, idx: usize, target_pos: (i32, i32), mode: TargetMode) -> Option<String> {
        let tick = self.tick;
        let is_qkd = mode.is_qkd();

        if tick % 2 == 0 {
            self.photons[idx].record_telemetry(tick);
        }

        if self.photons[idx].delay_timer > 0 {
            self.photons[idx].delay_timer -= 1;
            if self.photons[idx].delay_timer % 4 == 0 {
                let (px, py) = (self.photons[idx].px, self.photons[idx].py);
                self.spawn_fx("spark", px, py, ACCENT_AMBER, 3);
            }
            return None;
        }

        let (dx, dy) = self.photons[idx].dir.vec();
        let spd = self.photons[idx].speed;
        self.photons[idx].px += dx as f32 * spd;
        self.photons[idx].py += dy as f32 * spd;
        let (px, py, wl) = (self.photons[idx].px, self.photons[idx].py, self.photons[idx].wavelength);
        self.photons[idx].trail.push((px, py, wl));
        if self.photons[idx].trail.len() > 36 {
            self.photons[idx].trail.remove(0);
        }

        if px < 0.0 || px >= BOARD_W || py < 0.0 || py >= BOARD_H {
            self.photons[idx].alive = false;
            self.photons[idx].record_telemetry(tick);
            let pair_id = self.photons[idx].qkd_pair_id;
            let pid = self.photons[idx].pid.clone();
            if is_qkd {
                if let Some(qs) = &mut self.qkd_session {
                    qs.record_drop();
                }
            }
            self.emit_sfx(SfxEvent::Fail, is_qkd, pair_id);
            return Some(format!("FAILURE: Photon {pid} exited the optical bench."));
        }

        let cx = (px / TILE_SIZE).floor() as i32;
        let cy = (py / TILE_SIZE).floor() as i32;
        let center_x = cx as f32 * TILE_SIZE + TILE_SIZE * 0.5;
        let center_y = cy as f32 * TILE_SIZE + TILE_SIZE * 0.5;

        if (cx, cy) != self.photons[idx].last_tile
            && (px - center_x).abs() < spd
            && (py - center_y).abs() < spd
        {
            self.photons[idx].px = center_x;
            self.photons[idx].py = center_y;
            self.photons[idx].last_tile = (cx, cy);
            let res = self.interact_photon(idx, cx, cy, target_pos, mode);
            self.photons[idx].record_telemetry(tick);
            return res;
        }
        None
    }

    fn interact_photon(
        &mut self,
        idx: usize,
        cx: i32,
        cy: i32,
        target_pos: (i32, i32),
        mode: TargetMode,
    ) -> Option<String> {
        let is_qkd = mode.is_qkd();
        let (px, py) = (self.photons[idx].px, self.photons[idx].py);
        let pair_id = self.photons[idx].qkd_pair_id;
        let pid = self.photons[idx].pid.clone();

        if (cx, cy) == target_pos {
            self.photons[idx].alive = false;
            let min_fid = if mode == TargetMode::Qpu { 90 } else { 60 };
            let wl = self.photons[idx].wavelength;
            let noise = self.photons[idx].noise;
            let fid = self.photons[idx].fidelity;

            if wl != 1550 {
                self.spawn_fx("burst", px, py, ACCENT_RED, if is_qkd { 8 } else { 18 });
                self.emit_sfx(SfxEvent::Fail, is_qkd, pair_id);
                if let Some(qs) = &mut self.qkd_session {
                    qs.record_drop();
                }
                return Some(format!(
                    "FAILURE: Detector tuned for 1550nm, Photon {pid} arrived at {wl}nm!"
                ));
            }
            if noise > 10 {
                self.spawn_fx("burst", px, py, ACCENT_AMBER, if is_qkd { 8 } else { 20 });
                self.emit_sfx(SfxEvent::Fail, is_qkd, pair_id);
                if let Some(qs) = &mut self.qkd_session {
                    qs.record_drop();
                }
                return Some(format!(
                    "FAILURE: Detector latched by Photon {pid} Raman noise ({noise})! Add VBG Filter."
                ));
            }
            if fid < min_fid {
                self.spawn_fx("burst", px, py, ACCENT_RED, if is_qkd { 8 } else { 18 });
                self.emit_sfx(SfxEvent::Fail, is_qkd, pair_id);
                if let Some(qs) = &mut self.qkd_session {
                    qs.record_drop();
                }
                let hint = if mode == TargetMode::Qpu {
                    "Purify at DISTILL node."
                } else {
                    "Use Q-Memory or AO."
                };
                return Some(format!(
                    "FAILURE: Photon {pid} fidelity too low ({fid}% < {min_fid}%). {hint}"
                ));
            }

            self.photons[idx].arrived = true;
            self.photons[idx].arrival_tick = Some(self.tick);
            if is_qkd {
                let role = self.photons[idx].qkd_role;
                let pol = self.photons[idx].pol;
                let bit = self.photons[idx].qkd_bit;
                let pmd_flipped = self.photons[idx].pmd_flips % 2 == 1;
                let eve_tapped = self.photons[idx].eve_tapped;
                if let (Some(qs), Some(pid_num)) = (&mut self.qkd_session, pair_id) {
                    qs.measure_photon(role, pid_num, pol, bit, pmd_flipped, eve_tapped);
                }
                self.spawn_fx("spark", px, py, if eve_tapped { ACCENT_RED } else { ACCENT_GREEN }, 4);
            } else {
                self.spawn_fx("ring", px, py, ACCENT_CYAN, 10);
            }
            return None;
        }

        let tile = *self.grid.get(&(cx, cy))?;
        match tile {
            Tile::Wall => {
                self.photons[idx].alive = false;
                self.spawn_fx("burst", px, py, ACCENT_RED, if is_qkd { 8 } else { 16 });
                self.emit_sfx(SfxEvent::Fail, is_qkd, pair_id);
                if let Some(qs) = &mut self.qkd_session {
                    qs.record_drop();
                }
                Some(format!("FAILURE: Photon {pid} collided with cryostat shielding."))
            }
            Tile::Fiber | Tile::LitFiber | Tile::PmdFiber => {
                let wl = self.photons[idx].wavelength;
                if wl != 1550 {
                    self.photons[idx].alive = false;
                    self.spawn_fx("burst", px, py, ACCENT_RED, if is_qkd { 8 } else { 18 });
                    self.emit_sfx(SfxEvent::Fail, is_qkd, pair_id);
                    if let Some(qs) = &mut self.qkd_session {
                        qs.record_drop();
                    }
                    return Some(format!(
                        "FAILURE: {wl}nm Photon {pid} absorbed by telecom fiber! Insert PPLN QFC first."
                    ));
                }
                self.photons[idx].fidelity = (self.photons[idx].fidelity - 10).max(0);
                if tile == Tile::LitFiber {
                    self.photons[idx].noise += 15;
                    self.spawn_fx("spark", px, py, ACCENT_AMBER, if is_qkd { 3 } else { 5 });
                } else if tile == Tile::PmdFiber {
                    self.photons[idx].pol = self.photons[idx].pol.flipped();
                    self.photons[idx].pmd_flips += 1;
                    self.spawn_fx("spark", px, py, ACCENT_PURPLE, if is_qkd { 3 } else { 6 });
                }
                if self.photons[idx].fidelity <= 0 {
                    self.photons[idx].alive = false;
                    self.spawn_fx("burst", px, py, ACCENT_CYAN, if is_qkd { 8 } else { 14 });
                    self.emit_sfx(SfxEvent::Fail, is_qkd, pair_id);
                    if let Some(qs) = &mut self.qkd_session {
                        qs.record_drop();
                    }
                    return Some(format!("FAILURE: Photon {pid} decohered completely in fiber."));
                }
                if tile == Tile::PmdFiber && !is_qkd {
                    return Some(format!(
                        "PMD Fiber: Photon {pid} polarization drifted to '{}'.",
                        self.photons[idx].pol.as_str()
                    ));
                }
                None
            }
            Tile::Eve => {
                self.photons[idx].eve_tapped = true;
                let pol = self.photons[idx].pol;
                let bit = self.photons[idx].qkd_bit;
                let pmd_flipped = self.photons[idx].pmd_flips % 2 == 1;
                if let (Some(qs), Some(pid_num)) = (&mut self.qkd_session, pair_id) {
                    let (eb, ebit) = qs.eve_intercept(pid_num, pol, bit, pmd_flipped);
                    self.photons[idx].pol = eb;
                    self.photons[idx].qkd_bit = ebit;
                }
                self.spawn_fx("spark", px, py, ACCENT_RED, 6);
                self.emit_sfx(SfxEvent::Eve, is_qkd, pair_id);
                Some("ALERT: EVE intercepted & re-emitted photon (quantum state collapsed -> QBER spike)!".to_string())
            }
            Tile::Atmos => {
                let ao = self.photons[idx].ao_locked;
                let loss = if ao { 5 } else { 25 };
                self.photons[idx].fidelity = (self.photons[idx].fidelity - loss).max(0);
                self.spawn_fx("spark", px, py, if ao { ACCENT_TEAL } else { ACCENT_RED }, 5);
                if self.photons[idx].fidelity <= 0 {
                    self.photons[idx].alive = false;
                    self.spawn_fx("burst", px, py, ACCENT_RED, 14);
                    self.emit_sfx(SfxEvent::Fail, is_qkd, pair_id);
                    return Some(format!(
                        "FAILURE: Photon {pid} lost to atmospheric turbulence! Add Adaptive Optics."
                    ));
                }
                None
            }
            Tile::Placeable(Placeable::Qfc) => {
                let old_wl = self.photons[idx].wavelength;
                if old_wl != 1550 {
                    let added_noise = if old_wl == 637 { 70 } else { 40 };
                    self.photons[idx].wavelength = 1550;
                    self.photons[idx].noise += added_noise;
                    self.spawn_fx(
                        if is_qkd { "spark" } else { "ring" },
                        px,
                        py,
                        ACCENT_PURPLE,
                        if is_qkd { 4 } else { 14 },
                    );
                    self.emit_sfx(SfxEvent::Qfc, is_qkd, pair_id);
                    if !is_qkd {
                        return Some(format!(
                            "QFC: Photon {pid} shifted {old_wl}nm -> 1550nm (+{added_noise} noise)."
                        ));
                    }
                }
                None
            }
            Tile::Placeable(Placeable::Vbg) => {
                if self.photons[idx].noise > 0 {
                    self.photons[idx].noise = (self.photons[idx].noise - 50).max(0);
                    let n = self.photons[idx].noise;
                    self.spawn_fx(
                        if is_qkd { "spark" } else { "burst" },
                        px,
                        py,
                        ACCENT_AMBER,
                        if is_qkd { 4 } else { 12 },
                    );
                    self.emit_sfx(SfxEvent::Vbg, is_qkd, pair_id);
                    if !is_qkd {
                        return Some(format!(
                            "VBG: Stripped Raman noise from Photon {pid} (now {n})."
                        ));
                    }
                }
                None
            }
            Tile::Placeable(Placeable::Memory) => {
                self.photons[idx].fidelity = 100;
                self.spawn_fx("ring", px, py, ACCENT_GREEN, 14);
                self.emit_sfx(SfxEvent::Memory, is_qkd, pair_id);
                if !is_qkd {
                    return Some(format!(
                        "Q-Memory: Photon {pid} buffered & fidelity restored to 100%."
                    ));
                }
                None
            }
            Tile::Placeable(Placeable::Delay) => {
                self.photons[idx].delay_timer = 20;
                self.spawn_fx("ring", px, py, ACCENT_AMBER, 8);
                self.emit_sfx(SfxEvent::Delay, is_qkd, pair_id);
                if !is_qkd {
                    return Some(format!("Delay Line: Holding Photon {pid} for +100ps sync."));
                }
                None
            }
            Tile::Placeable(Placeable::Hwp) => {
                self.photons[idx].pol = self.photons[idx].pol.flipped();
                self.photons[idx].pmd_flips += 1;
                let pol_s = self.photons[idx].pol.as_str();
                self.spawn_fx(
                    if is_qkd { "spark" } else { "ring" },
                    px,
                    py,
                    ACCENT_AMBER,
                    if is_qkd { 4 } else { 10 },
                );
                self.emit_sfx(SfxEvent::Hwp, is_qkd, pair_id);
                if !is_qkd {
                    return Some(format!("HWP: Rotated Photon {pid} polarization to '{pol_s}'."));
                }
                None
            }
            Tile::Placeable(Placeable::Ao) => {
                self.photons[idx].ao_locked = true;
                self.spawn_fx("ring", px, py, ACCENT_TEAL, 12);
                self.emit_sfx(SfxEvent::Ao, is_qkd, pair_id);
                Some(format!(
                    "Adaptive Optics: Wavefront locked for Photon {pid} (ATMOS loss reduced to -5%)."
                ))
            }
            Tile::Distill => {
                self.photons[idx].alive = false;
                let wl = self.photons[idx].wavelength;
                let noise = self.photons[idx].noise;
                let fid = self.photons[idx].fidelity;
                if wl != 1550 {
                    self.spawn_fx("burst", px, py, ACCENT_RED, 16);
                    self.emit_sfx(SfxEvent::Fail, is_qkd, pair_id);
                    return Some(format!(
                        "FAILURE: DISTILL requires 1550nm, Photon {pid} arrived at {wl}nm!"
                    ));
                }
                if noise > 10 {
                    self.spawn_fx("burst", px, py, ACCENT_AMBER, 16);
                    self.emit_sfx(SfxEvent::Fail, is_qkd, pair_id);
                    return Some(format!(
                        "FAILURE: DISTILL latched by Photon {pid} Raman noise ({noise})!"
                    ));
                }
                if fid < 60 {
                    self.spawn_fx("burst", px, py, ACCENT_RED, 16);
                    self.emit_sfx(SfxEvent::Fail, is_qkd, pair_id);
                    return Some(format!(
                        "FAILURE: Photon {pid} fidelity ({fid}%) too low for distillation."
                    ));
                }
                self.photons[idx].distill_tick = Some(self.tick);
                self.photons[idx].distill_pos = Some((cx, cy));
                self.spawn_fx("ring", px, py, ACCENT_GREEN, 10);
                Some(format!("Distill: Photon {pid} buffered at Purifier ({fid}%)."))
            }
            Tile::Placeable(Placeable::MirrorSlash) => {
                self.photons[idx].dir = self.photons[idx].dir.reflect_slash();
                self.spawn_fx("spark", px, py, TEXT_WHITE, if is_qkd { 3 } else { 6 });
                self.emit_sfx(SfxEvent::Mirror, is_qkd, pair_id);
                None
            }
            Tile::Placeable(Placeable::MirrorBack) => {
                self.photons[idx].dir = self.photons[idx].dir.reflect_back();
                self.spawn_fx("spark", px, py, TEXT_WHITE, if is_qkd { 3 } else { 6 });
                self.emit_sfx(SfxEvent::Mirror, is_qkd, pair_id);
                None
            }
        }
    }

    fn process_distillation(&mut self) {
        if self.distill_fired {
            return;
        }
        let buffered: Vec<(u32, (i32, i32))> = self
            .photons
            .iter()
            .filter_map(|p| Some((p.distill_tick?, p.distill_pos?)))
            .collect();
        if buffered.len() >= 2 {
            self.distill_fired = true;
            let (t1, (dx, dy)) = buffered[0];
            let (t2, _) = buffered[1];
            let delta_frames = t1.abs_diff(t2);
            let delta_ps = delta_frames * 5;
            if delta_frames <= 3 {
                let mut purified = Photon::new("P", dx, dy, Dir::Right, 1550);
                purified.fidelity = 95;
                purified.fid_history = vec![(self.tick, 95, 1550)];
                purified.noise_history = vec![(self.tick, 0)];
                self.photons.push(purified);
                self.spawn_fx(
                    "ring",
                    dx as f32 * TILE_SIZE + 40.0,
                    dy as f32 * TILE_SIZE + 40.0,
                    ACCENT_GREEN,
                    20,
                );
                self.pending_sfx.push(SfxEvent::Distill);
                self.status_msg = format!(
                    "DISTILL: 2 photons purified (dt:{delta_ps}ps) -> 95% fidelity photon P emitted!"
                );
                self.status_color = ACCENT_GREEN;
            } else {
                self.spawn_fx(
                    "burst",
                    dx as f32 * TILE_SIZE + 40.0,
                    dy as f32 * TILE_SIZE + 40.0,
                    ACCENT_AMBER,
                    16,
                );
                self.pending_sfx.push(SfxEvent::Fail);
                self.status_msg = format!(
                    "FAILURE: Distillation timing mismatch ({delta_ps}ps > 15ps). Use Delay Line."
                );
                self.status_color = ACCENT_RED;
            }
        }
    }

    fn evaluate_simulation(&mut self) {
        let lvl = self.levels[self.level_idx].clone();
        if lvl.mode.is_qkd() {
            if self.qkd_burst_remaining > 0 || self.photons.iter().any(|p| p.alive) {
                return;
            }
            self.sim_active = false;
            let (ok, msg, verdict) = {
                let qs = self.qkd_session.as_mut().expect("qkd_session active");
                let (ok, msg) = qs.finalize();
                (ok, msg, qs.verdict)
            };
            if ok {
                self.status_msg = msg;
                self.status_color = ACCENT_GREEN;
                self.sim_state = SimState::Locked;
                self.level_complete = true;
                self.completed_levels.insert(self.level_idx);
                self.pending_sfx.push(SfxEvent::AesLock);
                for (tx, ty) in lvl.target_coords() {
                    self.spawn_fx(
                        "ring",
                        tx as f32 * TILE_SIZE + 40.0,
                        ty as f32 * TILE_SIZE + 40.0,
                        ACCENT_GREEN,
                        20,
                    );
                }
            } else {
                if verdict == QkdVerdict::AbortNoKey {
                    if let Some(drop_msg) = &self.qkd_last_drop_msg {
                        self.status_msg = drop_msg.clone();
                    } else {
                        self.status_msg = msg;
                    }
                } else {
                    self.status_msg = msg;
                }
                self.status_color = ACCENT_RED;
                self.sim_state = SimState::Fault;
                self.pending_sfx.push(SfxEvent::Fail);
                for (tx, ty) in lvl.target_coords() {
                    self.spawn_fx(
                        "burst",
                        tx as f32 * TILE_SIZE + 40.0,
                        ty as f32 * TILE_SIZE + 40.0,
                        ACCENT_RED,
                        14,
                    );
                }
            }
            return;
        }

        if self.photons.iter().any(|p| p.alive) {
            return;
        }
        self.sim_active = false;
        let (tx, ty) = lvl.target;
        let tcx = tx as f32 * TILE_SIZE + 40.0;
        let tcy = ty as f32 * TILE_SIZE + 40.0;

        if lvl.mode == TargetMode::Qpu {
            let arrived_fid = self.photons.iter().rev().find(|p| p.arrived).map(|p| p.fidelity);
            let Some(fid) = arrived_fid else {
                self.sim_state = SimState::Fault;
                return;
            };
            self.status_msg = format!("SUCCESS! Fault-tolerant QPU locked | 1550nm | Fidelity: {fid}%");
            self.status_color = ACCENT_GREEN;
            self.sim_state = SimState::Locked;
            self.level_complete = true;
            self.completed_levels.insert(self.level_idx);
            self.pending_sfx.push(SfxEvent::BsmOk);
            self.spawn_fx("ring", tcx, tcy, ACCENT_GREEN, 26);
            return;
        }

        if !self.photons.iter().all(|p| p.arrived) {
            self.sim_state = SimState::Fault;
            return;
        }

        match lvl.mode {
            TargetMode::Single => {
                let p = &self.photons[0];
                self.status_msg = format!(
                    "SUCCESS! 1550nm detected | Noise: {} | Fidelity: {}%",
                    p.noise, p.fidelity
                );
                self.status_color = ACCENT_GREEN;
                self.sim_state = SimState::Locked;
                self.level_complete = true;
                self.completed_levels.insert(self.level_idx);
                self.pending_sfx.push(SfxEvent::SnspdOk);
                self.spawn_fx("ring", tcx, tcy, ACCENT_GREEN, 20);
            }
            TargetMode::Bsm => {
                let p_a = &self.photons[0];
                let p_b = &self.photons[1];
                let t_a = p_a.arrival_tick.unwrap_or(0) as i32;
                let t_b = p_b.arrival_tick.unwrap_or(0) as i32;
                let delta_frames = (t_a - t_b).abs();
                let delta_ps = delta_frames * 5;
                self.hom_delta_ps = Some(delta_ps);
                self.hom_signed_ps = Some((t_a - t_b) * 5);
                if delta_frames > 3 {
                    let early = if t_a < t_b { "A" } else { "B" };
                    self.status_msg = format!(
                        "FAILURE: HOM timing mismatch! Photon {early} arrived {delta_ps}ps early. Use Delay Line."
                    );
                    self.status_color = ACCENT_RED;
                    self.sim_state = SimState::Fault;
                    self.pending_sfx.push(SfxEvent::Fail);
                    self.spawn_fx("burst", tcx, tcy, ACCENT_AMBER, 16);
                } else if p_a.pol != p_b.pol {
                    self.pol_mismatch = true;
                    self.status_msg = format!(
                        "FAILURE: Polarization mismatch (A:'{}' vs B:'{}')! Add Half-Wave Plate (HWP).",
                        p_a.pol.as_str(),
                        p_b.pol.as_str()
                    );
                    self.status_color = ACCENT_RED;
                    self.sim_state = SimState::Fault;
                    self.pending_sfx.push(SfxEvent::Fail);
                    self.spawn_fx("burst", tcx, tcy, ACCENT_PURPLE, 18);
                } else {
                    let pol_s = p_a.pol.as_str();
                    self.status_msg = format!(
                        "SUCCESS! HOM Interference (dt: {delta_ps}ps, pol: '{pol_s}') — Entanglement Swapped!"
                    );
                    self.status_color = ACCENT_GREEN;
                    self.sim_state = SimState::Locked;
                    self.level_complete = true;
                    self.completed_levels.insert(self.level_idx);
                    self.pending_sfx.push(SfxEvent::BsmOk);
                    self.spawn_fx("ring", tcx, tcy, ACCENT_GREEN, 28);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qkd::{aes256_ctr_crypt, aes256_encrypt_block, hex_upper};

    fn run_level(idx: usize, placements: &[(Placeable, i32, i32)], seed: Option<u64>) -> BenchSim {
        let mut sim = BenchSim::new();
        sim.load_level(idx);
        for &(comp, gx, gy) in placements {
            sim.selected = Some(comp);
            assert!(sim.place_component(gx, gy), "failed placing {comp:?} at ({gx},{gy}) on L{}", idx + 1);
        }
        sim.fire_photons();
        if let (Some(qs), Some(s)) = (&mut sim.qkd_session, seed) {
            qs.reseed(s);
        }
        for _ in 0..800 {
            sim.step_frame();
            if !sim.sim_active {
                break;
            }
        }
        sim
    }

    #[test]
    fn nist_fips197_aes256_test_vector() {
        let key: [u8; 32] = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
        ];
        let pt: [u8; 16] = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff,
        ];
        let ct = aes256_encrypt_block(&key, &pt);
        assert_eq!(hex_upper(&ct), "8EA2B7CA516745BFEAFC49904B496089");

        let nonce = [0x42u8; 16];
        let msg = b"QN Bench 1550nm Rust AES-256-CTR";
        let enc = aes256_ctr_crypt(&key, &nonce, msg);
        assert_eq!(aes256_ctr_crypt(&key, &nonce, &enc), msg);
    }

    #[test]
    fn all_14_levels_solvable_and_physics_enforced() {
        use Placeable::*;

        // L1: fail without QFC, solve with QFC + VBG
        assert_eq!(run_level(0, &[], None).sim_state, SimState::Fault);
        assert_eq!(run_level(0, &[(Qfc, 2, 2), (Vbg, 3, 2)], None).sim_state, SimState::Locked);

        // L2: Cryostat Obstacles
        assert_eq!(
            run_level(1, &[(Qfc, 1, 1), (Vbg, 2, 1), (MirrorBack, 3, 1), (MirrorBack, 3, 4)], None).sim_state,
            SimState::Locked
        );

        // L3: Long-Haul Attenuation
        assert_eq!(run_level(2, &[(Qfc, 1, 2), (Vbg, 8, 2)], None).sim_state, SimState::Fault);
        assert_eq!(
            run_level(2, &[(Qfc, 1, 2), (Memory, 5, 2), (Vbg, 8, 2)], None).sim_state,
            SimState::Locked
        );

        // L4: Entanglement Swapping (HOM)
        assert_eq!(
            run_level(3, &[(Qfc, 1, 1), (Vbg, 2, 1), (MirrorBack, 8, 1), (Qfc, 1, 4), (Vbg, 2, 4), (MirrorSlash, 8, 4)], None).sim_state,
            SimState::Fault
        );
        assert_eq!(
            run_level(3, &[(Qfc, 1, 1), (Vbg, 2, 1), (Delay, 3, 1), (MirrorBack, 8, 1), (Qfc, 1, 4), (Vbg, 2, 4), (MirrorSlash, 8, 4)], None).sim_state,
            SimState::Locked
        );

        // L5: The Repeater Node
        assert_eq!(
            run_level(4, &[(Qfc, 1, 1), (Memory, 5, 1), (Vbg, 8, 1), (MirrorBack, 9, 1), (Qfc, 1, 4), (Vbg, 2, 4), (Delay, 3, 4), (MirrorSlash, 9, 4)], None).sim_state,
            SimState::Locked
        );

        // L6: Cross-Axis Multiplexing (shared VBG at (3,2); Arm A=11 tiles, Arm B=10 tiles -> Delay on Arm B)
        assert_eq!(
            run_level(5, &[(Qfc, 1, 2), (Qfc, 3, 1), (Vbg, 3, 2), (MirrorBack, 3, 5), (Delay, 7, 5), (MirrorBack, 8, 2)], None).sim_state,
            SimState::Locked
        );

        // L7: Classical Coexistence (Lit Fiber)
        assert_eq!(
            run_level(6, &[(Qfc, 1, 2), (Vbg, 6, 2), (Vbg, 7, 2)], None).sim_state,
            SimState::Locked
        );

        // L8: Polarization Drift (PMD)
        assert_eq!(
            run_level(7, &[(Qfc, 1, 1), (Vbg, 2, 1), (Hwp, 6, 1), (Delay, 7, 1), (MirrorBack, 8, 1), (Qfc, 1, 4), (Vbg, 2, 4), (MirrorSlash, 8, 4)], None).sim_state,
            SimState::Locked
        );

        // L9: Entanglement Distillation
        assert_eq!(
            run_level(8, &[(Qfc, 1, 1), (Vbg, 2, 1), (Delay, 3, 1), (MirrorBack, 6, 1), (Qfc, 1, 4), (Vbg, 2, 4), (MirrorSlash, 6, 4)], None).sim_state,
            SimState::Locked
        );

        // L10: Hybrid Quantum Network (780nm + 637nm NV)
        assert_eq!(
            run_level(9, &[(Qfc, 1, 1), (Vbg, 2, 1), (Delay, 3, 1), (MirrorBack, 9, 1), (Qfc, 1, 4), (Vbg, 2, 4), (Vbg, 3, 4), (MirrorSlash, 9, 4)], None).sim_state,
            SimState::Locked
        );

        // L11: Satellite Downlink (ATMOS + AO)
        assert_eq!(
            run_level(10, &[(Qfc, 1, 1), (Vbg, 2, 1), (Ao, 3, 1), (MirrorBack, 7, 1), (MirrorBack, 7, 4)], None).sim_state,
            SimState::Locked
        );

        // L12: BB84 QKD & AES-256 (aborts without HWP, locks with HWP)
        let l12_fail = run_level(11, &[(Qfc, 1, 2), (Vbg, 2, 2)], Some(123));
        assert_eq!(l12_fail.sim_state, SimState::Fault);
        let l12_ok = run_level(11, &[(Qfc, 1, 2), (Vbg, 2, 2), (Hwp, 6, 2)], Some(123));
        assert_eq!(l12_ok.sim_state, SimState::Locked);
        assert_eq!(l12_ok.qkd_session.as_ref().unwrap().decrypted_text, l12_ok.qkd_plaintext);

        // L13: Entangled QKD (E91)
        let l13 = run_level(12, &[(Qfc, 3, 2), (Vbg, 2, 2), (Qfc, 5, 2), (Vbg, 6, 2), (Hwp, 8, 2)], Some(456));
        assert_eq!(l13.sim_state, SimState::Locked);
        assert!(l13.qkd_session.as_ref().unwrap().chsh_s.unwrap() > 2.8);

        // L14: Eavesdropper & AES Link (aborts straight through EVE, locks via mirror bypass)
        let l14_eve = run_level(13, &[(Qfc, 3, 2), (Vbg, 2, 2), (Qfc, 5, 2), (Vbg, 6, 2)], Some(789));
        assert_eq!(l14_eve.sim_state, SimState::Fault);
        assert!(l14_eve.qkd_session.as_ref().unwrap().eve_intercepts > 0);

        let l14_ok = run_level(
            13,
            &[(Qfc, 3, 2), (Vbg, 2, 2), (Qfc, 5, 2), (MirrorBack, 6, 2), (Vbg, 6, 3), (MirrorBack, 6, 4), (MirrorSlash, 9, 4)],
            Some(789),
        );
        assert_eq!(l14_ok.sim_state, SimState::Locked);
        assert!(l14_ok.qkd_session.as_ref().unwrap().chsh_s.unwrap() > 2.8);
    }
}
