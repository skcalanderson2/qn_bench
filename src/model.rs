//! Core domain types, grid constants, and color palette for QN Bench.

use egui::Color32;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const TILE_SIZE: f32 = 80.0;
pub const GRID_COLS: i32 = 10;
pub const GRID_ROWS: i32 = 6;
pub const BOARD_W: f32 = GRID_COLS as f32 * TILE_SIZE;
pub const BOARD_H: f32 = GRID_ROWS as f32 * TILE_SIZE;

// --- Optical Bench Dark Palette ---
pub const BG_DARK: Color32 = Color32::from_rgb(12, 16, 23);
pub const GRID_BG: Color32 = Color32::from_rgb(20, 26, 36);
pub const GRID_LINE: Color32 = Color32::from_rgb(34, 44, 60);
pub const BREADBOARD_HOLE: Color32 = Color32::from_rgb(14, 18, 26);
pub const SIDEBAR_BG: Color32 = Color32::from_rgb(17, 22, 32);
pub const PANEL_BG: Color32 = Color32::from_rgb(26, 34, 48);
pub const TEXT_WHITE: Color32 = Color32::from_rgb(238, 244, 252);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(138, 154, 178);

pub const ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248); // 1550nm Telecom C-Band
pub const ACCENT_RED: Color32 = Color32::from_rgb(244, 63, 94); // 780nm Visible Light / Alert
pub const ACCENT_ORANGE: Color32 = Color32::from_rgb(251, 146, 60); // 637nm Diamond NV Emitter
pub const ACCENT_AMBER: Color32 = Color32::from_rgb(251, 191, 36); // Raman Noise / Delay / Warning
pub const ACCENT_GREEN: Color32 = Color32::from_rgb(52, 211, 153); // Success / Memory / Distill
pub const ACCENT_PURPLE: Color32 = Color32::from_rgb(167, 139, 250); // QFC / EPR / Nonlinear Crystal
pub const ACCENT_TEAL: Color32 = Color32::from_rgb(45, 212, 191); // Adaptive Optics / Atmos / QKD RX
pub const ACCENT_HWP: Color32 = Color32::from_rgb(244, 180, 90);
pub const ACCENT_MIRROR: Color32 = Color32::from_rgb(200, 218, 240);
pub const WALL_COLOR: Color32 = Color32::from_rgb(48, 58, 74);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Dir {
    Right,
    Down,
    Left,
    Up,
}

impl Dir {
    pub fn vec(self) -> (i32, i32) {
        match self {
            Dir::Right => (1, 0),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Up => (0, -1),
        }
    }

    pub fn rotate_cw(self) -> Dir {
        match self {
            Dir::Right => Dir::Down,
            Dir::Down => Dir::Left,
            Dir::Left => Dir::Up,
            Dir::Up => Dir::Right,
        }
    }

    pub fn arrow(self) -> &'static str {
        match self {
            Dir::Right => "→",
            Dir::Down => "↓",
            Dir::Left => "←",
            Dir::Up => "↑",
        }
    }

    pub fn reflect_slash(self) -> Dir {
        // (dx, dy) -> (-dy, -dx)
        match self {
            Dir::Right => Dir::Up,
            Dir::Down => Dir::Left,
            Dir::Up => Dir::Right,
            Dir::Left => Dir::Down,
        }
    }

    pub fn reflect_back(self) -> Dir {
        // (dx, dy) -> (dy, dx)
        match self {
            Dir::Right => Dir::Down,
            Dir::Down => Dir::Right,
            Dir::Up => Dir::Left,
            Dir::Left => Dir::Up,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pol {
    Plus,
    Cross,
}

impl Pol {
    pub fn flipped(self) -> Pol {
        match self {
            Pol::Plus => Pol::Cross,
            Pol::Cross => Pol::Plus,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Pol::Plus => "+",
            Pol::Cross => "x",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Placeable {
    Qfc,
    Vbg,
    Memory,
    Delay,
    Hwp,
    Ao,
    MirrorSlash,
    MirrorBack,
}

impl Placeable {
    pub const ALL: [Placeable; 8] = [
        Placeable::Qfc,
        Placeable::Vbg,
        Placeable::Memory,
        Placeable::Delay,
        Placeable::Hwp,
        Placeable::Ao,
        Placeable::MirrorSlash,
        Placeable::MirrorBack,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Placeable::Qfc => "PPLN QFC",
            Placeable::Vbg => "VBG Filter",
            Placeable::Memory => "Q-Memory",
            Placeable::Delay => "Delay Line",
            Placeable::Hwp => "Half-Wave Plate",
            Placeable::Ao => "Adaptive Optics",
            Placeable::MirrorSlash => "Mirror (/)",
            Placeable::MirrorBack => "Mirror (\\)",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Placeable::Qfc => "Shifts 780/637nm -> 1550nm (+noise)",
            Placeable::Vbg => "Strips Raman noise (-50)",
            Placeable::Memory => "Restores fidelity to 100%",
            Placeable::Delay => "Adds +100ps (1-tile) sync",
            Placeable::Hwp => "Rotates polarization (+ <-> x)",
            Placeable::Ao => "Stabilizes ATMOS loss (-25% -> -5%)",
            Placeable::MirrorSlash => "Reflects 90° (R->U, D->L)",
            Placeable::MirrorBack => "Reflects 90° (R->D, U->L)",
        }
    }

    pub fn color(self) -> Color32 {
        match self {
            Placeable::Qfc => ACCENT_PURPLE,
            Placeable::Vbg => ACCENT_CYAN,
            Placeable::Memory => ACCENT_GREEN,
            Placeable::Delay => ACCENT_AMBER,
            Placeable::Hwp => ACCENT_HWP,
            Placeable::Ao => ACCENT_TEAL,
            Placeable::MirrorSlash | Placeable::MirrorBack => ACCENT_MIRROR,
        }
    }

    pub fn doc_key(self) -> &'static str {
        match self {
            Placeable::Qfc => "QFC",
            Placeable::Vbg => "VBG",
            Placeable::Memory => "MEMORY",
            Placeable::Delay => "DELAY",
            Placeable::Hwp => "HWP",
            Placeable::Ao => "AO",
            Placeable::MirrorSlash => "MIRROR_SLASH",
            Placeable::MirrorBack => "MIRROR_BACK",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tile {
    Placeable(Placeable),
    Wall,
    Fiber,
    LitFiber,
    PmdFiber,
    Atmos,
    Distill,
    Repeater,
    Eve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetMode {
    Single,
    Bsm,
    Qpu,
    QkdBb84,
    QkdE91,
}

impl TargetMode {
    pub fn is_qkd(self) -> bool {
        matches!(self, TargetMode::QkdBb84 | TargetMode::QkdE91)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimState {
    Ready,
    Active,
    Locked,
    Fault,
}

impl SimState {
    pub fn label(self) -> &'static str {
        match self {
            SimState::Ready => "READY",
            SimState::Active => "ACTIVE",
            SimState::Locked => "LOCKED",
            SimState::Fault => "FAULT",
        }
    }

    pub fn colors(self) -> (Color32, Color32) {
        match self {
            SimState::Ready => (TEXT_MUTED, Color32::from_rgb(22, 28, 40)),
            SimState::Active => (ACCENT_AMBER, Color32::from_rgb(46, 36, 18)),
            SimState::Locked => (ACCENT_GREEN, Color32::from_rgb(18, 46, 36)),
            SimState::Fault => (ACCENT_RED, Color32::from_rgb(52, 20, 28)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmitterDef {
    pub gx: i32,
    pub gy: i32,
    pub dir: Dir,
    pub wavelength: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxTab {
    Optics,
    Channels,
    Sources,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxTool {
    Tile(Tile),
    EmitterIon,
    EmitterNv,
    EprSource,
    DetectorSnspd,
    DetectorBsm,
    DetectorQpu,
    DetectorBobBb84,
    DetectorAliceE91,
    DetectorBobE91,
}

impl SandboxTool {
    pub const OPTICS: [SandboxTool; 8] = [
        SandboxTool::Tile(Tile::Placeable(Placeable::Qfc)),
        SandboxTool::Tile(Tile::Placeable(Placeable::Vbg)),
        SandboxTool::Tile(Tile::Placeable(Placeable::Memory)),
        SandboxTool::Tile(Tile::Placeable(Placeable::Delay)),
        SandboxTool::Tile(Tile::Placeable(Placeable::Hwp)),
        SandboxTool::Tile(Tile::Placeable(Placeable::Ao)),
        SandboxTool::Tile(Tile::Placeable(Placeable::MirrorSlash)),
        SandboxTool::Tile(Tile::Placeable(Placeable::MirrorBack)),
    ];

    pub const CHANNELS: [SandboxTool; 8] = [
        SandboxTool::Tile(Tile::Fiber),
        SandboxTool::Tile(Tile::LitFiber),
        SandboxTool::Tile(Tile::PmdFiber),
        SandboxTool::Tile(Tile::Atmos),
        SandboxTool::Tile(Tile::Distill),
        SandboxTool::Tile(Tile::Repeater),
        SandboxTool::Tile(Tile::Eve),
        SandboxTool::Tile(Tile::Wall),
    ];

    pub const SOURCES: [SandboxTool; 9] = [
        SandboxTool::EmitterIon,
        SandboxTool::EmitterNv,
        SandboxTool::EprSource,
        SandboxTool::DetectorSnspd,
        SandboxTool::DetectorBsm,
        SandboxTool::DetectorQpu,
        SandboxTool::DetectorBobBb84,
        SandboxTool::DetectorAliceE91,
        SandboxTool::DetectorBobE91,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SandboxTool::Tile(Tile::Placeable(p)) => p.name(),
            SandboxTool::Tile(Tile::Fiber) => "1550nm Fiber",
            SandboxTool::Tile(Tile::LitFiber) => "Lit Fiber (WDM)",
            SandboxTool::Tile(Tile::PmdFiber) => "PMD Fiber",
            SandboxTool::Tile(Tile::Atmos) => "Atmosphere",
            SandboxTool::Tile(Tile::Distill) => "Distill Purifier",
            SandboxTool::Tile(Tile::Repeater) => "Quantum Repeater",
            SandboxTool::Tile(Tile::Eve) => "Eve Fiber Tap",
            SandboxTool::Tile(Tile::Wall) => "Cryostat Wall",
            SandboxTool::EmitterIon => "ION 780nm Emitter",
            SandboxTool::EmitterNv => "NV 637nm Emitter",
            SandboxTool::EprSource => "EPR Pair Source",
            SandboxTool::DetectorSnspd => "SNSPD 1.5K RX",
            SandboxTool::DetectorBsm => "BSM 50:50 RX",
            SandboxTool::DetectorQpu => "QPU >=90% RX",
            SandboxTool::DetectorBobBb84 => "BOB RX (BB84)",
            SandboxTool::DetectorAliceE91 => "ALICE RX (E91)",
            SandboxTool::DetectorBobE91 => "BOB RX (E91)",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            SandboxTool::Tile(Tile::Placeable(p)) => p.desc(),
            SandboxTool::Tile(Tile::Fiber) => "1550nm only (-10% fid/tile)",
            SandboxTool::Tile(Tile::LitFiber) => "1550nm (-10% fid, +15 noise)",
            SandboxTool::Tile(Tile::PmdFiber) => "1550nm (-10% fid, flips pol)",
            SandboxTool::Tile(Tile::Atmos) => "Free-space (-25% or -5% w/ AO)",
            SandboxTool::Tile(Tile::Distill) => "2x photons -> 1x 95% photon",
            SandboxTool::Tile(Tile::Repeater) => "Hop-1 BSM (<=15ps, pol=) -> Hop-2",
            SandboxTool::Tile(Tile::Eve) => "Intercept-resend QKD tap",
            SandboxTool::Tile(Tile::Wall) => "Opaque cryostat shield",
            SandboxTool::EmitterIon => "Fires 780nm photon (rotatable)",
            SandboxTool::EmitterNv => "Fires 637nm photon (rotatable)",
            SandboxTool::EprSource => "Fires entangled Bell pairs (E91)",
            SandboxTool::DetectorSnspd => "Target: 1550nm, N<=10, F>=60%",
            SandboxTool::DetectorBsm => "Target: 2 photons <=15ps, pol=",
            SandboxTool::DetectorQpu => "Target: 1550nm, N<=10, F>=90%",
            SandboxTool::DetectorBobBb84 => "Target: BB84 QKD + AES-256",
            SandboxTool::DetectorAliceE91 => "E91 Alice detector (+ BOB RX)",
            SandboxTool::DetectorBobE91 => "E91 Bob detector (+ ALICE RX)",
        }
    }

    pub fn color(self) -> Color32 {
        match self {
            SandboxTool::Tile(Tile::Placeable(p)) => p.color(),
            SandboxTool::Tile(Tile::Fiber) => ACCENT_CYAN,
            SandboxTool::Tile(Tile::LitFiber) => ACCENT_AMBER,
            SandboxTool::Tile(Tile::PmdFiber) => ACCENT_PURPLE,
            SandboxTool::Tile(Tile::Atmos) => ACCENT_TEAL,
            SandboxTool::Tile(Tile::Distill) => ACCENT_GREEN,
            SandboxTool::Tile(Tile::Repeater) => ACCENT_CYAN,
            SandboxTool::Tile(Tile::Eve) => ACCENT_RED,
            SandboxTool::Tile(Tile::Wall) => Color32::from_rgb(130, 148, 175),
            SandboxTool::EmitterIon => ACCENT_RED,
            SandboxTool::EmitterNv => ACCENT_ORANGE,
            SandboxTool::EprSource => ACCENT_PURPLE,
            SandboxTool::DetectorSnspd => ACCENT_CYAN,
            SandboxTool::DetectorBsm => ACCENT_AMBER,
            SandboxTool::DetectorQpu => ACCENT_PURPLE,
            SandboxTool::DetectorBobBb84 | SandboxTool::DetectorAliceE91 | SandboxTool::DetectorBobE91 => ACCENT_TEAL,
        }
    }

    pub fn doc_key(self) -> &'static str {
        match self {
            SandboxTool::Tile(Tile::Placeable(p)) => p.doc_key(),
            SandboxTool::Tile(Tile::Fiber) => "FIBER",
            SandboxTool::Tile(Tile::LitFiber) => "LIT_FIBER",
            SandboxTool::Tile(Tile::PmdFiber) => "PMD_FIBER",
            SandboxTool::Tile(Tile::Atmos) => "ATMOS",
            SandboxTool::Tile(Tile::Distill) => "DISTILL",
            SandboxTool::Tile(Tile::Repeater) => "REPEATER",
            SandboxTool::Tile(Tile::Eve) => "EVE",
            SandboxTool::Tile(Tile::Wall) => "WALL",
            SandboxTool::EmitterIon => "ION",
            SandboxTool::EmitterNv => "NV637",
            SandboxTool::EprSource => "EPR",
            SandboxTool::DetectorSnspd => "SNSPD",
            SandboxTool::DetectorBsm => "BSM",
            SandboxTool::DetectorQpu => "QPU",
            SandboxTool::DetectorBobBb84 => "QKD_RX",
            SandboxTool::DetectorAliceE91 | SandboxTool::DetectorBobE91 => "QKD_RX",
        }
    }

    pub fn is_rotatable(self) -> bool {
        matches!(self, SandboxTool::EmitterIon | SandboxTool::EmitterNv | SandboxTool::EprSource)
    }
}

/// Portable, human-readable JSON representation of a custom Sandbox optical bench.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SandboxBlueprint {
    #[serde(default = "default_bp_version")]
    pub version: u32,
    #[serde(default = "default_bp_name")]
    pub name: String,
    pub mode: TargetMode,
    #[serde(default)]
    pub epr_source: Option<(i32, i32)>,
    #[serde(default)]
    pub emitters: Vec<EmitterDef>,
    #[serde(default)]
    pub target: Option<(i32, i32)>,
    #[serde(default)]
    pub e91_alice: Option<(i32, i32)>,
    #[serde(default)]
    pub e91_bob: Option<(i32, i32)>,
    #[serde(default)]
    pub tiles: Vec<(i32, i32, Tile)>,
}

fn default_bp_version() -> u32 {
    1
}

fn default_bp_name() -> String {
    "Custom Optical Bench".to_string()
}

impl Default for SandboxBlueprint {
    fn default() -> Self {
        Self::preset_blank()
    }
}

impl SandboxBlueprint {
    /// Clean starter bench: 1 Ion 780nm emitter + 1 SNSPD detector, empty breadboard.
    pub fn preset_blank() -> Self {
        Self {
            version: 1,
            name: "Free-Build Starter".to_string(),
            mode: TargetMode::Single,
            epr_source: None,
            emitters: vec![EmitterDef { gx: 0, gy: 2, dir: Dir::Right, wavelength: 780 }],
            target: Some((9, 2)),
            e91_alice: None,
            e91_bob: None,
            tiles: vec![],
        }
    }

    /// Dual-arm HOM interferometer starter with asymmetric fiber arms.
    pub fn preset_hom_lab() -> Self {
        Self {
            version: 1,
            name: "HOM Interferometer Lab".to_string(),
            mode: TargetMode::Bsm,
            epr_source: None,
            emitters: vec![
                EmitterDef { gx: 0, gy: 1, dir: Dir::Right, wavelength: 780 },
                EmitterDef { gx: 0, gy: 4, dir: Dir::Right, wavelength: 637 },
            ],
            target: Some((9, 2)),
            e91_alice: None,
            e91_bob: None,
            tiles: vec![
                (4, 1, Tile::Fiber),
                (5, 1, Tile::PmdFiber),
                (4, 4, Tile::LitFiber),
                (5, 4, Tile::Fiber),
            ],
        }
    }

    /// Entangled E91 QKD + Eve Lab starter.
    pub fn preset_qkd_lab() -> Self {
        Self {
            version: 1,
            name: "E91 QKD & Eve Lab".to_string(),
            mode: TargetMode::QkdE91,
            epr_source: Some((4, 2)),
            emitters: vec![
                EmitterDef { gx: 4, gy: 2, dir: Dir::Left, wavelength: 780 },
                EmitterDef { gx: 4, gy: 2, dir: Dir::Right, wavelength: 780 },
            ],
            target: None,
            e91_alice: Some((0, 2)),
            e91_bob: Some((9, 2)),
            tiles: vec![
                (1, 2, Tile::Fiber),
                (7, 2, Tile::Eve),
                (8, 2, Tile::Fiber),
            ],
        }
    }

    /// Clone any Campaign level into an editable Sandbox blueprint!
    pub fn from_level(lvl: &LevelDef) -> Self {
        let mut tiles: Vec<(i32, i32, Tile)> = lvl.fixed.iter().map(|&((x, y), t)| (x, y, t)).collect();
        for &(x, y) in &lvl.walls {
            tiles.push((x, y, Tile::Wall));
        }
        Self {
            version: 1,
            name: format!("Sandbox: {}", lvl.title),
            mode: lvl.mode,
            epr_source: lvl.epr_source,
            emitters: lvl.emitters.clone(),
            target: if lvl.e91_targets.is_none() { Some(lvl.target) } else { None },
            e91_alice: lvl.e91_targets.map(|(a, _)| a),
            e91_bob: lvl.e91_targets.map(|(_, b)| b),
            tiles,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let in_bounds = |x: i32, y: i32| (0..GRID_COLS).contains(&x) && (0..GRID_ROWS).contains(&y);
        if let Some((x, y)) = self.epr_source {
            if !in_bounds(x, y) {
                return Err(format!("EPR source ({x},{y}) out of grid bounds."));
            }
        }
        for e in &self.emitters {
            if !in_bounds(e.gx, e.gy) {
                return Err(format!("Emitter ({},{}) out of grid bounds.", e.gx, e.gy));
            }
            if !matches!(e.wavelength, 637 | 780 | 1550) {
                return Err(format!("Unsupported emitter wavelength: {}nm.", e.wavelength));
            }
        }
        if let Some((x, y)) = self.target {
            if !in_bounds(x, y) {
                return Err(format!("Target ({x},{y}) out of grid bounds."));
            }
        }
        if let Some((x, y)) = self.e91_alice {
            if !in_bounds(x, y) {
                return Err(format!("Alice RX ({x},{y}) out of grid bounds."));
            }
        }
        if let Some((x, y)) = self.e91_bob {
            if !in_bounds(x, y) {
                return Err(format!("Bob RX ({x},{y}) out of grid bounds."));
            }
        }
        for &(x, y, _) in &self.tiles {
            if !in_bounds(x, y) {
                return Err(format!("Tile ({x},{y}) out of grid bounds."));
            }
        }
        Ok(())
    }

    pub fn to_level_def(&self) -> LevelDef {
        let e91_targets = match (self.mode == TargetMode::QkdE91, self.e91_alice, self.e91_bob) {
            (true, Some(a), Some(b)) => Some((a, b)),
            _ => None,
        };
        let target = self.target.unwrap_or((-1, -1));
        LevelDef {
            title: "Sandbox: Free-Build Optical Bench",
            goal: "Build & test any quantum network! L-Click place • R-Click remove • Q/E rotate emitters • SPACE fire.",
            briefing: "Free-Build Sandbox Mode: unlimited optics, channels, emitters, detectors, and QKD/AES-256 hardware. Export or import JSON blueprints to share custom experiments!",
            new_hardware: &[],
            mode: self.mode,
            qkd_rounds: 36,
            epr_source: self.epr_source,
            emitters: self.emitters.clone(),
            target,
            e91_targets,
            fixed: self.tiles.iter().map(|&(x, y, t)| ((x, y), t)).collect(),
            walls: Vec::new(),
            inventory: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LevelDef {
    pub title: &'static str,
    pub goal: &'static str,
    pub briefing: &'static str,
    pub new_hardware: &'static [&'static str],
    pub mode: TargetMode,
    pub qkd_rounds: usize,
    pub epr_source: Option<(i32, i32)>,
    pub emitters: Vec<EmitterDef>,
    pub target: (i32, i32),
    pub e91_targets: Option<((i32, i32), (i32, i32))>, // (Alice, Bob)
    pub fixed: Vec<((i32, i32), Tile)>,
    pub walls: Vec<(i32, i32)>,
    pub inventory: BTreeMap<Placeable, u32>,
}

impl LevelDef {
    pub fn target_coords(&self) -> Vec<(i32, i32)> {
        if let Some((a, b)) = self.e91_targets {
            vec![a, b]
        } else if self.target.0 >= 0 && self.target.1 >= 0 {
            vec![self.target]
        } else {
            vec![]
        }
    }

    pub fn is_emitter(&self, pos: (i32, i32)) -> bool {
        self.emitters.iter().any(|e| (e.gx, e.gy) == pos)
    }

    pub fn is_target(&self, pos: (i32, i32)) -> bool {
        self.target_coords().contains(&pos)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HardwareDoc {
    pub key: &'static str,
    pub title: &'static str,
    pub color: Color32,
    pub physics: &'static str,
    pub rule: &'static str,
    pub tip: &'static str,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    None,
    Level(usize),
    Comp(&'static str),
    QkdAes,
    SandboxShare,
    Update,
}
