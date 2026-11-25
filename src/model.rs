//! Core domain types, grid constants, and color palette for QN Bench.

use egui::Color32;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tile {
    Placeable(Placeable),
    Wall,
    Fiber,
    LitFiber,
    PmdFiber,
    Atmos,
    Distill,
    Eve,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy)]
pub struct EmitterDef {
    pub gx: i32,
    pub gy: i32,
    pub dir: Dir,
    pub wavelength: u32,
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
        } else {
            vec![self.target]
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
    Update,
}
