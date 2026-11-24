import pygame
import sys
import math
import random
from audio import SoundEngine
from updater import UpdateManager
from version import APP_VERSION

# --- Configuration ---
TILE_SIZE = 80
GRID_COLS = 10
GRID_ROWS = 6
SIDEBAR_W = 340
BOTTOM_H = 190

SCREEN_W = GRID_COLS * TILE_SIZE + SIDEBAR_W
SCREEN_H = GRID_ROWS * TILE_SIZE + BOTTOM_H
FPS = 60

# --- Colors ---
BG_DARK = (12, 16, 23)
GRID_BG = (20, 26, 36)
GRID_LINE = (34, 44, 60)
BREADBOARD_HOLE = (14, 18, 26)
SIDEBAR_BG = (17, 22, 32)
PANEL_BG = (26, 34, 48)
TEXT_WHITE = (238, 244, 252)
TEXT_MUTED = (138, 154, 178)

ACCENT_CYAN = (56, 189, 248)    # 1550nm Telecom C-Band
ACCENT_RED = (244, 63, 94)      # 780nm Visible Light
ACCENT_ORANGE = (251, 146, 60)  # 637nm Diamond NV Emitter
ACCENT_AMBER = (251, 191, 36)   # Raman Noise / Delay / Warning
ACCENT_GREEN = (52, 211, 153)   # Success / Memory / Distill
ACCENT_PURPLE = (167, 139, 250) # QFC / Nonlinear Crystal
ACCENT_TEAL = (45, 212, 191)    # Adaptive Optics / Atmos
WALL_COLOR = (48, 58, 74)

RIGHT = (1, 0)
DOWN = (0, 1)
LEFT = (-1, 0)
UP = (0, -1)


def unpack_emitter(e):
    return (e[0], e[1], e[2], e[3] if len(e) > 3 else 780)


# --- Component Definitions ---
COMPONENTS = {
    "QFC": {"name": "PPLN QFC", "desc": "Shifts 780/637nm -> 1550nm (+noise)", "color": ACCENT_PURPLE},
    "VBG": {"name": "VBG Filter", "desc": "Strips Raman noise (-50)", "color": ACCENT_CYAN},
    "MEMORY": {"name": "Q-Memory", "desc": "Restores fidelity to 100%", "color": ACCENT_GREEN},
    "DELAY": {"name": "Delay Line", "desc": "Adds +100ps (1-tile) sync", "color": ACCENT_AMBER},
    "HWP": {"name": "Half-Wave Plate", "desc": "Rotates polarization (+ <-> x)", "color": (244, 180, 90)},
    "AO": {"name": "Adaptive Optics", "desc": "Stabilizes ATMOS loss (-25% -> -5%)", "color": ACCENT_TEAL},
    "MIRROR_SLASH": {"name": "Mirror (/)", "desc": "Reflects 90° (R->U, D->L)", "color": (200, 218, 240)},
    "MIRROR_BACK": {"name": "Mirror (\\)", "desc": "Reflects 90° (R->D, U->L)", "color": (200, 218, 240)},
}

# --- Hardware Encyclopedia (used by '?' popups & level briefings) ---
HARDWARE_DOCS = {
    "QFC": {
        "title": "PPLN Quantum Frequency Converter (QFC)",
        "color": ACCENT_PURPLE,
        "physics": "Periodically Poled Lithium Niobate (PPLN) waveguide using Difference Frequency Generation with a classical pump laser to shift visible qubit photons into the low-loss 1550nm telecom C-band.",
        "rule": "Shifts 780nm -> 1550nm (+40 Raman noise) or 637nm -> 1550nm (+70 Raman noise).",
        "tip": "Always place BEFORE telecom fiber, and pair with VBG Filter(s) to strip pump noise.",
    },
    "VBG": {
        "title": "Volume Bragg Grating (VBG) Filter",
        "color": ACCENT_CYAN,
        "physics": "An ultra-narrowband holographic optical filter that transmits single 1550nm quantum photons while rejecting broadband Raman scattering noise from classical pump or data lasers.",
        "rule": "Strips -50 Raman noise (clamped at 0). Detectors latch if noise > 10.",
        "tip": "Can filter multiple perpendicular beams if they cross the same tile!",
    },
    "MEMORY": {
        "title": "Quantum Memory (Diamond NV / Ion Buffer)",
        "color": ACCENT_GREEN,
        "physics": "Stores a flying photonic qubit in a long-lived solid-state or atomic spin state, acting as a quantum repeater buffer to overcome fiber attenuation.",
        "rule": "Restores photon fidelity back to 100% without adding noise.",
        "tip": "Place mid-link on long fiber runs so fidelity stays >= 60% at the detector.",
    },
    "DELAY": {
        "title": "Optical Fiber Delay Line (+100ps)",
        "color": ACCENT_AMBER,
        "physics": "A calibrated coiled fiber spool that slows one optical path so two independent photons arrive at a Beam Splitter simultaneously for Hong-Ou-Mandel interference.",
        "rule": "Holds a photon for +100ps (20 frames) — equal to exactly 1 tile of travel.",
        "tip": "Place on the SHORTER arm of a BSM or DISTILL puzzle to match path lengths.",
    },
    "HWP": {
        "title": "Half-Wave Plate (HWP)",
        "color": (244, 180, 90),
        "physics": "A birefringent quartz crystal retarder that introduces a 180-degree phase shift between orthogonal axes, rotating linear photon polarization without optical loss.",
        "rule": "Rotates polarization between '+' (rectilinear) and 'x' (diagonal).",
        "tip": "Use after PMD Fiber so both photons share the same polarization at the BSM.",
    },
    "AO": {
        "title": "Adaptive Optics (Deformable Mirror)",
        "color": ACCENT_TEAL,
        "physics": "A closed-loop wavefront sensor and micro-actuator mirror that pre-compensates atmospheric phase distortion in real time for satellite-to-ground quantum links.",
        "rule": "Locks wavefront so ATMOS turbulence loss drops from -25%/tile to -5%/tile.",
        "tip": "Must be placed BEFORE the photon enters turbulent ATMOS tiles.",
    },
    "MIRROR_SLASH": {
        "title": "45° Dielectric Mirror (/)",
        "color": (200, 218, 240),
        "physics": "High-reflectivity (>99.99%) multi-layer dielectric mirror used on free-space optical benches to steer single photons cleanly around obstacles.",
        "rule": "Reflects 90° without loss: Right->Up, Down->Left, Up->Right, Left->Down.",
        "tip": "Combine with Mirror (\\) to route around cryostat shielding walls.",
    },
    "MIRROR_BACK": {
        "title": "45° Dielectric Mirror (\\)",
        "color": (200, 218, 240),
        "physics": "High-reflectivity (>99.99%) multi-layer dielectric mirror oriented along the back-diagonal to steer photons 90° with zero decoherence.",
        "rule": "Reflects 90° without loss: Right->Down, Down->Right, Up->Left, Left->Up.",
        "tip": "Remember: turning adds tile distance — count tiles carefully on BSM levels!",
    },
    "FIBER": {
        "title": "1550nm Telecom Fiber (Dark Fiber)",
        "color": ACCENT_CYAN,
        "physics": "Standard single-mode silica fiber has its minimum attenuation window at 1550nm (C-band), while visible wavelengths like 780nm or 637nm are rapidly absorbed.",
        "rule": "Destroys non-1550nm photons; transmits 1550nm at -10% fidelity per tile.",
        "tip": "Convert photons to 1550nm with a QFC before they enter fiber.",
    },
    "LIT_FIBER": {
        "title": "Lit Telecom Fiber (WDM Coexistence)",
        "color": ACCENT_AMBER,
        "physics": "Real telecom fibers carry bright classical data/sync channels alongside quantum signals, generating spontaneous Raman scattering noise along the fiber.",
        "rule": "Transmits 1550nm (-10% fidelity/tile) AND injects +15 Raman noise per tile.",
        "tip": "Place a VBG Filter AFTER the lit fiber to strip the accumulated coexistence noise.",
    },
    "PMD_FIBER": {
        "title": "Birefringent Fiber (PMD)",
        "color": ACCENT_PURPLE,
        "physics": "Thermal and mechanical stress in buried fiber creates Polarization Mode Dispersion (PMD), randomly rotating photon polarization and breaking HOM interference.",
        "rule": "Transmits 1550nm (-10% fidelity) AND flips polarization '+' <-> 'x'.",
        "tip": "Compensate with a Half-Wave Plate (HWP) before reaching the BSM.",
    },
    "ATMOS": {
        "title": "Atmospheric Turbulence (Free-Space)",
        "color": ACCENT_TEAL,
        "physics": "Air-density eddies distort optical wavefronts during satellite-to-ground quantum downlinks, causing beam wander and severe fidelity loss.",
        "rule": "Degrades fidelity by -25% per tile (-5% per tile if wavefront-locked by AO).",
        "tip": "Place Adaptive Optics (AO) upstream of the ATMOS corridor.",
    },
    "DISTILL": {
        "title": "Entanglement Distillation Node",
        "color": ACCENT_GREEN,
        "physics": "Performs local quantum gates and measurement on two noisy entangled pairs to sacrifice one pair and purify the other to fault-tolerant fidelity.",
        "rule": "Consumes two 1550nm photons (>=60% fid, noise<=10, dt<=15ps) -> emits one 95% photon.",
        "tip": "Sync both incoming arms to arrive within <= 15ps just like a BSM.",
    },
    "SNSPD": {
        "title": "SNSPD Cryogenic Detector (1.5K)",
        "color": ACCENT_CYAN,
        "physics": "Superconducting Nanowire Single-Photon Detector cooled to 1.5 Kelvin; detects single 1550nm photons with >90% efficiency but latches if flooded by Raman noise.",
        "rule": "Requires 1550nm wavelength, Raman noise <= 10, and fidelity >= 60%.",
        "tip": "Watch the 3 lock LEDs in the bottom-right telemetry panel.",
    },
    "BSM": {
        "title": "50:50 Bell-State Measurement (BSM)",
        "color": ACCENT_AMBER,
        "physics": "Interferes two indistinguishable photons on a 50:50 beam splitter (Hong-Ou-Mandel effect) to swap entanglement between distant quantum nodes.",
        "rule": "Both 1550nm photons must arrive within <= 15ps (3 frames) with matching polarization.",
        "tip": "Check the HOM Coincidence Dip plot at bottom-right to see your timing offset.",
    },
    "QPU": {
        "title": "Fault-Tolerant QPU Target (>=90%)",
        "color": ACCENT_PURPLE,
        "physics": "Error-corrected quantum processor node requiring ultra-high link fidelity (>=90%) to run distributed quantum gates without exceeding fault-tolerance thresholds.",
        "rule": "Requires 1550nm wavelength, noise <= 10, and fidelity >= 90%.",
        "tip": "Raw fiber links drop below 90% — route through the DISTILL purifier first.",
    },
    "NV637": {
        "title": "Diamond NV Center Emitter (637nm)",
        "color": ACCENT_ORANGE,
        "physics": "A Nitrogen-Vacancy defect in diamond emitting visible 637nm red-orange photons. Converting 637nm to 1550nm requires a larger frequency shift than 780nm.",
        "rule": "Emits 637nm photon; QFC shifts 637nm -> 1550nm with +70 Raman noise (needs 2 VBGs).",
        "tip": "Give the NV arm two VBG Filters while the 780nm Ion arm only needs one.",
    },
}

# --- Level Definitions ---
LEVELS = [
    {
        "title": "Level 1: The Wavelength Barrier",
        "goal": "Convert 780nm to 1550nm before fiber, and filter Raman noise before SNSPD.",
        "briefing": "Trapped-ion qubits emit visible 780nm photons, which are instantly absorbed by silica telecom fiber. Use a PPLN Frequency Converter (QFC) to shift the photon into the 1550nm C-band, then strip the converter's Raman pump noise with a VBG Filter before it hits the cryogenic SNSPD detector.",
        "new_hardware": ["QFC", "VBG", "FIBER", "SNSPD"],
        "mode": "SINGLE",
        "emitters": [(0, 2, RIGHT)],
        "target": (9, 2),
        "fixed": {(4, 2): "FIBER", (5, 2): "FIBER", (6, 2): "FIBER"},
        "walls": [],
        "inventory": {"QFC": 1, "VBG": 1, "MEMORY": 0, "DELAY": 0, "MIRROR_SLASH": 0, "MIRROR_BACK": 0},
    },
    {
        "title": "Level 2: Cryostat Obstacles",
        "goal": "Route around the cryostat shielding using mirrors while keeping noise <= 10.",
        "briefing": "Optical benches are crowded with vacuum chambers and cryostat shielding. Use 45° dielectric mirrors to steer your beam around the walls and into the telecom fiber link without losing fidelity.",
        "new_hardware": ["MIRROR_BACK", "MIRROR_SLASH"],
        "mode": "SINGLE",
        "emitters": [(0, 1, RIGHT)],
        "target": (9, 4),
        "fixed": {(4, 4): "FIBER", (5, 4): "FIBER", (6, 4): "FIBER"},
        "walls": [(4, 0), (4, 1), (4, 2), (4, 3), (7, 2), (7, 3)],
        "inventory": {"QFC": 1, "VBG": 1, "MEMORY": 0, "DELAY": 0, "MIRROR_SLASH": 2, "MIRROR_BACK": 2},
    },
    {
        "title": "Level 3: Long-Haul Attenuation",
        "goal": "Long fiber degrades fidelity (-10/tile). Buffer in Q-Memory so fidelity >= 60%.",
        "briefing": "Even at 1550nm, long telecom fibers attenuate quantum signals (-10% fidelity per tile). Five fiber tiles will drop your photon to 50%, below the SNSPD's 60% threshold. Place a Quantum Memory mid-link to buffer and restore fidelity back to 100%.",
        "new_hardware": ["MEMORY"],
        "mode": "SINGLE",
        "emitters": [(0, 2, RIGHT)],
        "target": (9, 2),
        "fixed": {(2, 2): "FIBER", (3, 2): "FIBER", (4, 2): "FIBER", (6, 2): "FIBER", (7, 2): "FIBER"},
        "walls": [(5, 0), (5, 1), (5, 3), (5, 4), (5, 5)],
        "inventory": {"QFC": 1, "VBG": 1, "MEMORY": 1, "DELAY": 0, "MIRROR_SLASH": 0, "MIRROR_BACK": 0},
    },
    {
        "title": "Level 4: Entanglement Swapping (HOM)",
        "goal": "Sync both photons to arrive at the 50:50 BSM simultaneously using the Delay Line.",
        "briefing": "To swap entanglement between two remote ions, their photons must interfere at a 50:50 Bell-State Measurement (BSM) beam splitter within <= 15ps (Hong-Ou-Mandel effect). Because one path is 1 tile shorter, insert an Optical Delay Line (+100ps = 1 tile) on the shorter arm.",
        "new_hardware": ["DELAY", "BSM"],
        "mode": "BSM",
        "emitters": [(0, 1, RIGHT), (0, 4, RIGHT)],
        "target": (8, 2),
        "fixed": {(4, 1): "FIBER", (5, 1): "FIBER", (4, 4): "FIBER", (5, 4): "FIBER"},
        "walls": [(3, 2), (4, 2), (5, 2), (6, 2)],
        "inventory": {"QFC": 2, "VBG": 2, "MEMORY": 0, "DELAY": 1, "MIRROR_SLASH": 1, "MIRROR_BACK": 1},
    },
    {
        "title": "Level 5: The Repeater Node",
        "goal": "Buffer the long-haul fiber arm with Q-Memory and sync the short arm with Delay Line.",
        "briefing": "Real quantum repeaters must solve attenuation and synchronization simultaneously. The upper link spans 5 fiber tiles (requiring Q-Memory), while the lower link is 1 tile shorter (requiring a Delay Line to hit 0ps at the BSM).",
        "new_hardware": [],
        "mode": "BSM",
        "emitters": [(0, 1, RIGHT), (0, 4, RIGHT)],
        "target": (9, 3),
        "fixed": {
            (2, 1): "FIBER", (3, 1): "FIBER", (4, 1): "FIBER",
            (6, 1): "FIBER", (7, 1): "FIBER",
            (4, 4): "FIBER", (5, 4): "FIBER",
        },
        "walls": [(1, 2), (2, 2), (3, 2), (4, 2), (5, 2), (6, 2), (7, 2)],
        "inventory": {"QFC": 2, "VBG": 2, "MEMORY": 1, "DELAY": 1, "MIRROR_SLASH": 1, "MIRROR_BACK": 1},
    },
    {
        "title": "Level 6: Cross-Axis Multiplexing",
        "goal": "Only 1 VBG Filter available! Route both perpendicular beams through the same VBG and sync at BSM.",
        "briefing": "Optical filters are scarce in this cryostat: you only have ONE VBG Filter for TWO emitters! Because perpendicular free-space beams can cross the same filter tile at different times, place the shared VBG at their intersection before syncing at the BSM.",
        "new_hardware": [],
        "mode": "BSM",
        "emitters": [(0, 2, RIGHT), (3, 0, DOWN)],
        "target": (8, 5),
        "fixed": {
            (2, 2): "FIBER", (4, 2): "FIBER", (5, 2): "FIBER",
            (4, 5): "FIBER", (5, 5): "FIBER", (6, 5): "FIBER",
        },
        "walls": [(5, 3), (6, 3), (5, 4), (6, 4)],
        "inventory": {"QFC": 2, "VBG": 1, "MEMORY": 0, "DELAY": 1, "MIRROR_SLASH": 0, "MIRROR_BACK": 2},
    },
    {
        "title": "Level 7: Classical Coexistence (WDM)",
        "goal": "Lit telecom fiber injects +15 Raman noise/tile. Filter noise AFTER the lit fiber before SNSPD.",
        "briefing": "In deployed metropolitan networks, quantum photons share 'lit' fibers with bright classical internet traffic. Spontaneous Raman scattering adds +15 noise per lit fiber tile, so you must filter noise AFTER the fiber rather than only right after the QFC.",
        "new_hardware": ["LIT_FIBER"],
        "mode": "SINGLE",
        "emitters": [(0, 2, RIGHT)],
        "target": (9, 2),
        "fixed": {(3, 2): "LIT_FIBER", (4, 2): "LIT_FIBER", (5, 2): "LIT_FIBER"},
        "walls": [],
        "inventory": {"QFC": 1, "VBG": 2},
    },
    {
        "title": "Level 8: Polarization Drift (PMD)",
        "goal": "PMD fiber rotates polarization (+ -> x). Restore indistinguishability with HWP and sync at BSM.",
        "briefing": "Temperature and stress in buried fiber cause Polarization Mode Dispersion (PMD), rotating a photon from '+' to 'x'. HOM interference at a BSM fails if photons have mismatched polarizations — use a Half-Wave Plate (HWP) to rotate it back and align timing with the Delay Line.",
        "new_hardware": ["PMD_FIBER", "HWP"],
        "mode": "BSM",
        "emitters": [(0, 1, RIGHT), (0, 4, RIGHT)],
        "target": (8, 2),
        "fixed": {
            (3, 1): "PMD_FIBER", (4, 1): "FIBER", (5, 1): "FIBER",
            (4, 4): "FIBER", (5, 4): "FIBER",
        },
        "walls": [(3, 2), (4, 2), (5, 2), (6, 2)],
        "inventory": {"QFC": 2, "VBG": 2, "HWP": 1, "DELAY": 1, "MIRROR_SLASH": 1, "MIRROR_BACK": 1},
    },
    {
        "title": "Level 9: Entanglement Distillation",
        "goal": "QPU requires >=90% fidelity. Sync both photons into the DISTILL node to purify a 95% photon.",
        "briefing": "Fault-tolerant Quantum Processing Units (QPUs) demand >= 90% fidelity, which raw fiber links cannot achieve alone. Synchronize both 80% photons into the Entanglement Distillation (DISTILL) node within <= 15ps to purify a single 95% photon aimed straight into the QPU.",
        "new_hardware": ["DISTILL", "QPU"],
        "mode": "QPU",
        "emitters": [(0, 1, RIGHT), (0, 4, RIGHT)],
        "target": (9, 2),
        "fixed": {
            (4, 1): "FIBER", (5, 1): "FIBER",
            (4, 4): "FIBER", (5, 4): "FIBER",
            (6, 2): "DISTILL",
        },
        "walls": [(7, 0), (7, 1), (7, 3), (7, 4), (7, 5)],
        "inventory": {"QFC": 2, "VBG": 2, "DELAY": 1, "MIRROR_SLASH": 1, "MIRROR_BACK": 1},
    },
    {
        "title": "Level 10: Hybrid Quantum Network",
        "goal": "Transduce 780nm Ion & 637nm Diamond NV (+70 noise in QFC) into 1550nm and sync at BSM.",
        "briefing": "Future quantum internets connect heterogeneous hardware: here, a 780nm Trapped-Ion node links with a 637nm Diamond NV Center. Shifting 637nm to 1550nm injects +70 Raman noise (requiring TWO VBG Filters on the NV arm) so both photons meet identically at the BSM.",
        "new_hardware": ["NV637"],
        "mode": "BSM",
        "emitters": [(0, 1, RIGHT, 780), (0, 4, RIGHT, 637)],
        "target": (9, 2),
        "fixed": {
            (4, 1): "FIBER", (5, 1): "FIBER", (6, 1): "FIBER",
            (5, 4): "FIBER", (6, 4): "FIBER",
        },
        "walls": [(3, 2), (4, 2), (5, 2), (6, 2), (7, 2)],
        "inventory": {"QFC": 2, "VBG": 3, "DELAY": 1, "MIRROR_SLASH": 1, "MIRROR_BACK": 1},
    },
    {
        "title": "Level 11: Satellite Downlink (ATMOS)",
        "goal": "Stabilize atmospheric turbulence (-25%/tile) with Adaptive Optics before coupling to ground fiber.",
        "briefing": "Intercontinental quantum links beam photons from satellites through free space to avoid hundreds of kilometers of fiber loss. Place Adaptive Optics (AO) upstream of the turbulent ATMOS corridor to cut atmospheric loss from -25% to -5% per tile before coupling into the ground station fiber.",
        "new_hardware": ["ATMOS", "AO"],
        "mode": "SINGLE",
        "emitters": [(0, 1, RIGHT, 780)],
        "target": (9, 4),
        "fixed": {
            (4, 1): "ATMOS", (5, 1): "ATMOS", (6, 1): "ATMOS",
            (8, 4): "FIBER",
        },
        "walls": [(3, 2), (4, 2), (5, 2), (6, 2), (3, 3), (3, 4), (3, 5)],
        "inventory": {"QFC": 1, "VBG": 1, "AO": 1, "MIRROR_BACK": 2},
    },
]


# --- Visual Helpers ---
def make_glow_surface(radius, color, max_alpha=120):
    surf = pygame.Surface((radius * 2, radius * 2), pygame.SRCALPHA)
    for r in range(radius, 0, -2):
        alpha = int(max_alpha * ((1 - r / radius) ** 2))
        pygame.draw.circle(surf, (*color, alpha), (radius, radius), r)
    return surf


class Particle:
    def __init__(self, x, y, color, speed=2.5, life=28, size=3):
        angle = random.uniform(0, math.tau)
        spd = random.uniform(0.5, speed)
        self.x = x
        self.y = y
        self.vx = math.cos(angle) * spd
        self.vy = math.sin(angle) * spd
        self.color = color
        self.life = life
        self.max_life = life
        self.size = size

    def update(self):
        self.x += self.vx
        self.y += self.vy
        self.vx *= 0.94
        self.vy *= 0.94
        self.life -= 1

    def draw(self, screen):
        if self.life <= 0:
            return
        alpha = int(255 * (self.life / self.max_life))
        s = pygame.Surface((self.size * 2, self.size * 2), pygame.SRCALPHA)
        pygame.draw.circle(s, (*self.color, alpha), (self.size, self.size), self.size)
        screen.blit(s, (self.x - self.size, self.y - self.size))


class Shockwave:
    def __init__(self, x, y, color, max_r=45, life=24):
        self.x = x
        self.y = y
        self.color = color
        self.max_r = max_r
        self.life = life
        self.max_life = life

    def update(self):
        self.life -= 1

    def draw(self, screen):
        if self.life <= 0:
            return
        prog = 1 - (self.life / self.max_life)
        r = int(8 + prog * self.max_r)
        alpha = int(200 * (1 - prog))
        s = pygame.Surface((r * 2 + 4, r * 2 + 4), pygame.SRCALPHA)
        pygame.draw.circle(s, (*self.color, alpha), (r + 2, r + 2), r, 2)
        screen.blit(s, (self.x - r - 2, self.y - r - 2))


class Photon:
    def __init__(self, pid, gx, gy, direction, wavelength=780, fidelity=100, noise=0, pol="+"):
        self.pid = pid
        self.px = gx * TILE_SIZE + TILE_SIZE // 2
        self.py = gy * TILE_SIZE + TILE_SIZE // 2
        self.dir = direction
        self.wavelength = wavelength
        self.noise = noise
        self.fidelity = fidelity
        self.pol = pol
        self.ao_locked = False
        self.alive = True
        self.arrived = False
        self.arrival_tick = None
        self.distill_tick = None
        self.distill_pos = None
        self.delay_timer = 0
        self.speed = 4.0
        self.last_tile = (gx, gy)
        self.trail = []
        self.fid_history = [(0, fidelity, wavelength)]
        self.noise_history = [(0, noise)]

    def record_telemetry(self, current_tick):
        self.fid_history.append((current_tick, self.fidelity, self.wavelength))
        self.noise_history.append((current_tick, self.noise))
        if len(self.fid_history) > 160:
            self.fid_history.pop(0)
        if len(self.noise_history) > 160:
            self.noise_history.pop(0)

    def update(self, grid, target_pos, current_tick, fx_cb, sfx_cb, target_mode="SINGLE"):
        if not self.alive:
            return None

        if current_tick % 2 == 0:
            self.record_telemetry(current_tick)

        if self.delay_timer > 0:
            self.delay_timer -= 1
            if self.delay_timer % 4 == 0:
                fx_cb("spark", self.px, self.py, ACCENT_AMBER, 3)
            return None

        self.px += self.dir[0] * self.speed
        self.py += self.dir[1] * self.speed
        self.trail.append((self.px, self.py, self.wavelength))
        if len(self.trail) > 36:
            self.trail.pop(0)

        if not (0 <= self.px < GRID_COLS * TILE_SIZE and 0 <= self.py < GRID_ROWS * TILE_SIZE):
            self.alive = False
            self.record_telemetry(current_tick)
            sfx_cb("fail")
            return f"FAILURE: Photon {self.pid} exited the optical bench."

        cx = int(self.px // TILE_SIZE)
        cy = int(self.py // TILE_SIZE)
        center_x = cx * TILE_SIZE + TILE_SIZE // 2
        center_y = cy * TILE_SIZE + TILE_SIZE // 2

        if (cx, cy) != self.last_tile and abs(self.px - center_x) < self.speed and abs(self.py - center_y) < self.speed:
            self.px, self.py = center_x, center_y
            self.last_tile = (cx, cy)
            res = self.interact(cx, cy, grid, target_pos, current_tick, fx_cb, sfx_cb, target_mode)
            self.record_telemetry(current_tick)
            return res
        return None

    def interact(self, cx, cy, grid, target_pos, current_tick, fx_cb, sfx_cb, target_mode="SINGLE"):
        if (cx, cy) == target_pos:
            self.alive = False
            min_fid = 90 if target_mode == "QPU" else 60
            if self.wavelength != 1550:
                fx_cb("burst", self.px, self.py, ACCENT_RED, 18)
                sfx_cb("fail")
                return f"FAILURE: Detector tuned for 1550nm, Photon {self.pid} arrived at {self.wavelength}nm!"
            if self.noise > 10:
                fx_cb("burst", self.px, self.py, ACCENT_AMBER, 20)
                sfx_cb("fail")
                return f"FAILURE: Detector latched by Photon {self.pid} Raman noise ({self.noise})! Add VBG Filter."
            if self.fidelity < min_fid:
                fx_cb("burst", self.px, self.py, ACCENT_RED, 18)
                sfx_cb("fail")
                hint = "Purify at DISTILL node." if target_mode == "QPU" else "Use Q-Memory or AO."
                return f"FAILURE: Photon {self.pid} fidelity too low ({self.fidelity}% < {min_fid}%). {hint}"
            self.arrived = True
            self.arrival_tick = current_tick
            fx_cb("ring", self.px, self.py, ACCENT_CYAN, 10)
            return None

        tile = grid.get((cx, cy))
        if not tile:
            return None

        if tile == "WALL":
            self.alive = False
            fx_cb("burst", self.px, self.py, ACCENT_RED, 16)
            sfx_cb("fail")
            return f"FAILURE: Photon {self.pid} collided with cryostat shielding."

        elif tile in ("FIBER", "LIT_FIBER", "PMD_FIBER"):
            if self.wavelength != 1550:
                self.alive = False
                fx_cb("burst", self.px, self.py, ACCENT_RED, 18)
                sfx_cb("fail")
                return f"FAILURE: {self.wavelength}nm Photon {self.pid} absorbed by telecom fiber! Insert PPLN QFC first."
            self.fidelity = max(0, self.fidelity - 10)
            if tile == "LIT_FIBER":
                self.noise += 15
                fx_cb("spark", self.px, self.py, ACCENT_AMBER, 5)
            elif tile == "PMD_FIBER":
                self.pol = "x" if self.pol == "+" else "+"
                fx_cb("spark", self.px, self.py, ACCENT_PURPLE, 6)
            if self.fidelity <= 0:
                self.alive = False
                fx_cb("burst", self.px, self.py, ACCENT_CYAN, 14)
                sfx_cb("fail")
                return f"FAILURE: Photon {self.pid} decohered completely in fiber."
            if tile == "PMD_FIBER":
                return f"PMD Fiber: Photon {self.pid} polarization drifted to '{self.pol}'."

        elif tile == "ATMOS":
            loss = 5 if self.ao_locked else 25
            self.fidelity = max(0, self.fidelity - loss)
            fx_cb("spark", self.px, self.py, ACCENT_TEAL if self.ao_locked else ACCENT_RED, 5)
            if self.fidelity <= 0:
                self.alive = False
                fx_cb("burst", self.px, self.py, ACCENT_RED, 14)
                sfx_cb("fail")
                return f"FAILURE: Photon {self.pid} lost to atmospheric turbulence! Add Adaptive Optics."

        elif tile == "QFC":
            if self.wavelength != 1550:
                added_noise = 70 if self.wavelength == 637 else 40
                old_wl = self.wavelength
                self.wavelength = 1550
                self.noise += added_noise
                fx_cb("ring", self.px, self.py, ACCENT_PURPLE, 14)
                sfx_cb("qfc")
                return f"QFC: Photon {self.pid} shifted {old_wl}nm -> 1550nm (+{added_noise} noise)."

        elif tile == "VBG":
            if self.noise > 0:
                self.noise = max(0, self.noise - 50)
                fx_cb("burst", self.px, self.py, ACCENT_AMBER, 12)
                sfx_cb("vbg")
                return f"VBG: Stripped Raman noise from Photon {self.pid} (now {self.noise})."

        elif tile == "MEMORY":
            self.fidelity = 100
            fx_cb("ring", self.px, self.py, ACCENT_GREEN, 14)
            sfx_cb("memory")
            return f"Q-Memory: Photon {self.pid} buffered & fidelity restored to 100%."

        elif tile == "DELAY":
            self.delay_timer = 20
            fx_cb("ring", self.px, self.py, ACCENT_AMBER, 8)
            sfx_cb("delay")
            return f"Delay Line: Holding Photon {self.pid} for +100ps sync."

        elif tile == "HWP":
            self.pol = "x" if self.pol == "+" else "+"
            fx_cb("ring", self.px, self.py, ACCENT_AMBER, 10)
            sfx_cb("hwp")
            return f"HWP: Rotated Photon {self.pid} polarization to '{self.pol}'."

        elif tile == "AO":
            self.ao_locked = True
            fx_cb("ring", self.px, self.py, ACCENT_TEAL, 12)
            sfx_cb("ao")
            return f"Adaptive Optics: Wavefront locked for Photon {self.pid} (ATMOS loss reduced to -5%)."

        elif tile == "DISTILL":
            self.alive = False
            if self.wavelength != 1550:
                fx_cb("burst", self.px, self.py, ACCENT_RED, 16)
                sfx_cb("fail")
                return f"FAILURE: DISTILL requires 1550nm, Photon {self.pid} arrived at {self.wavelength}nm!"
            if self.noise > 10:
                fx_cb("burst", self.px, self.py, ACCENT_AMBER, 16)
                sfx_cb("fail")
                return f"FAILURE: DISTILL latched by Photon {self.pid} Raman noise ({self.noise})!"
            if self.fidelity < 60:
                fx_cb("burst", self.px, self.py, ACCENT_RED, 16)
                sfx_cb("fail")
                return f"FAILURE: Photon {self.pid} fidelity ({self.fidelity}%) too low for distillation."
            self.distill_tick = current_tick
            self.distill_pos = (cx, cy)
            fx_cb("ring", self.px, self.py, ACCENT_GREEN, 10)
            return f"Distill: Photon {self.pid} buffered at Purifier ({self.fidelity}%)."

        elif tile == "MIRROR_SLASH":
            dx, dy = self.dir
            self.dir = (-dy, -dx)
            fx_cb("spark", self.px, self.py, TEXT_WHITE, 6)
            sfx_cb("mirror")

        elif tile == "MIRROR_BACK":
            dx, dy = self.dir
            self.dir = (dy, dx)
            fx_cb("spark", self.px, self.py, TEXT_WHITE, 6)
            sfx_cb("mirror")

        return None


class Game:
    def __init__(self):
        pygame.mixer.pre_init(44100, -16, 1, 512)
        pygame.init()
        self.screen = pygame.display.set_mode((SCREEN_W, SCREEN_H))
        pygame.display.set_caption(f"1550nm v{APP_VERSION} — Quantum Networking Optical Bench")
        self.clock = pygame.time.Clock()
        ui_font = "helveticaneue,arial,helvetica,avenirnext,dejavusans"
        mono_font = "menlo,sfmono,monaco,dejavusansmono,monospace"
        self.font_tile = pygame.font.SysFont(ui_font, 10, bold=True)
        self.font_xs = pygame.font.SysFont(ui_font, 11, bold=True)
        self.font_sm = pygame.font.SysFont(ui_font, 13)
        self.font_md = pygame.font.SysFont(ui_font, 15, bold=True)
        self.font_lg = pygame.font.SysFont(ui_font, 16, bold=True)
        self.font_mono = pygame.font.SysFont(mono_font, 11, bold=True)
        self.sfx = SoundEngine()

        # Pre-rendered glow caches
        self.glows = {
            "red": make_glow_surface(26, ACCENT_RED, 130),
            "orange": make_glow_surface(26, ACCENT_ORANGE, 130),
            "cyan": make_glow_surface(26, ACCENT_CYAN, 130),
            "amber": make_glow_surface(28, ACCENT_AMBER, 110),
            "green": make_glow_surface(28, ACCENT_GREEN, 120),
            "purple": make_glow_surface(26, ACCENT_PURPLE, 110),
        }

        self.particles = []
        self.shockwaves = []
        self.completed_levels = set()
        self.seen_briefings = set()
        self.overlay = None
        self.frame = 0
        self.tick = 0
        self.level_idx = 0
        self.update_btn_rects = {}
        self.updater = UpdateManager()
        self.updater.check_for_updates(silent=True)
        self.load_level(0)

    def spawn_fx(self, kind, x, y, color, count=10):
        for _ in range(count):
            self.particles.append(Particle(x, y, color, speed=3.2 if kind == "burst" else 2.0))
        if kind == "ring":
            self.shockwaves.append(Shockwave(x, y, color))

    def active_components(self):
        lvl = LEVELS[self.level_idx]
        return [k for k in COMPONENTS if lvl["inventory"].get(k, 0) > 0]

    def load_level(self, idx, show_briefing=None):
        self.level_idx = idx % len(LEVELS)
        lvl = LEVELS[self.level_idx]
        self.grid = {}
        self.user_placed = set()
        for pos, comp in lvl["fixed"].items():
            self.grid[pos] = comp
        for pos in lvl["walls"]:
            self.grid[pos] = "WALL"

        self.inventory = {k: lvl["inventory"].get(k, 0) for k in COMPONENTS}
        self.selected = next((k for k in self.active_components() if self.inventory[k] > 0), None)
        self.photons = []
        self.particles.clear()
        self.shockwaves.clear()
        self.tick = 0
        self.hom_delta_ps = None
        self.hom_signed_ps = None
        self.pol_mismatch = False
        self.distill_fired = False
        self.sim_state = "READY"
        self.status_msg = lvl["goal"]
        self.status_color = TEXT_WHITE
        self.level_complete = False
        self.sim_active = False

        if show_briefing is True or (show_briefing is None and self.level_idx not in self.seen_briefings):
            self.overlay = ("LEVEL", self.level_idx)
            self.seen_briefings.add(self.level_idx)
        else:
            self.overlay = None

    def fire_photons(self):
        lvl = LEVELS[self.level_idx]
        self.tick = 0
        self.hom_delta_ps = None
        self.hom_signed_ps = None
        self.pol_mismatch = False
        self.distill_fired = False
        self.photons = []
        for i, e in enumerate(lvl["emitters"]):
            ex, ey, edir, wl = unpack_emitter(e)
            self.photons.append(Photon(chr(65 + i), ex, ey, edir, wavelength=wl))
            fx_col = ACCENT_ORANGE if wl == 637 else ACCENT_RED
            self.spawn_fx("ring", ex * TILE_SIZE + 40, ey * TILE_SIZE + 40, fx_col, 8)
        self.sfx.play("fire")
        self.sim_active = True
        self.sim_state = "ACTIVE"
        self.level_complete = False
        wls = "/".join(str(unpack_emitter(e)[3]) + "nm" for e in lvl["emitters"])
        self.status_msg = f"Firing {wls} single photon(s) from Quantum Emitter(s)..."
        self.status_color = ACCENT_RED

    def level_pill_rect(self, idx, b_y):
        s_x = GRID_COLS * TILE_SIZE
        n = len(LEVELS)
        pill_w, gap = 21, 3
        total_w = n * pill_w + (n - 1) * gap
        start_x = (s_x - 166) - total_w
        return pygame.Rect(start_x + idx * (pill_w + gap), b_y + 10, pill_w, 22)

    def handle_update_overlay_click(self, mx, my):
        snap = self.updater.snapshot()
        state = snap["state"]
        rects = self.update_btn_rects

        if rects.get("install") and rects["install"].collidepoint(mx, my):
            self.sfx.play("select")
            self.updater.start_install()
            return
        if rects.get("open_page") and rects["open_page"].collidepoint(mx, my):
            self.sfx.play("select")
            self.updater.open_release_page()
            return
        if rects.get("recheck") and rects["recheck"].collidepoint(mx, my):
            self.sfx.play("select")
            self.updater.check_for_updates(silent=False)
            return
        if rects.get("cancel") and rects["cancel"].collidepoint(mx, my):
            self.sfx.play("select")
            self.updater.cancel_download()
            return
        if rects.get("close") and rects["close"].collidepoint(mx, my):
            if state not in ("DOWNLOADING", "VERIFYING", "INSTALLING"):
                self.sfx.play("select")
                self.overlay = None
            return
        # Allow clicking outside card to close only when not busy
        card = rects.get("card")
        if card and not card.collidepoint(mx, my) and state not in ("DOWNLOADING", "VERIFYING", "INSTALLING"):
            self.sfx.play("select")
            self.overlay = None

    def handle_click(self, pos, button):
        mx, my = pos

        if self.overlay is not None:
            if self.overlay[0] == "UPDATE":
                self.handle_update_overlay_click(mx, my)
                return
            self.overlay = None
            self.sfx.play("select")
            return

        if mx >= GRID_COLS * TILE_SIZE:
            s_x = GRID_COLS * TILE_SIZE
            info_rect = pygame.Rect(s_x + SIDEBAR_W - 154, 14, 62, 26)
            if info_rect.collidepoint(mx, my):
                self.overlay = ("LEVEL", self.level_idx)
                self.sfx.play("select")
                return

            mute_rect = pygame.Rect(s_x + SIDEBAR_W - 86, 14, 70, 26)
            if mute_rect.collidepoint(mx, my):
                self.sfx.toggle_mute()
                if not self.sfx.muted:
                    self.sfx.play("select")
                return

            y = 70
            for key in self.active_components():
                rect = pygame.Rect(s_x + 16, y, SIDEBAR_W - 32, 54)
                help_rect = pygame.Rect(rect.right - 28, rect.y + 6, 22, 22)
                if help_rect.collidepoint(mx, my):
                    self.overlay = ("COMP", key)
                    self.sfx.play("select")
                    return
                if rect.collidepoint(mx, my) and self.inventory.get(key, 0) > 0:
                    self.selected = key
                    self.sfx.play("select")
                    return
                y += 60

            update_rect = pygame.Rect(s_x + 16, SCREEN_H - 224, SIDEBAR_W - 32, 26)
            if update_rect.collidepoint(mx, my):
                self.sfx.play("select")
                self.updater.mark_seen()
                snap = self.updater.snapshot()
                if snap["state"] in ("IDLE", "UP_TO_DATE", "ERROR"):
                    self.updater.check_for_updates(silent=False)
                self.overlay = ("UPDATE", None)
                return

            fire_rect = pygame.Rect(s_x + 16, SCREEN_H - 166, SIDEBAR_W - 32, 42)
            reset_rect = pygame.Rect(s_x + 16, SCREEN_H - 116, SIDEBAR_W - 32, 38)
            prev_rect = pygame.Rect(s_x + 16, SCREEN_H - 68, 96, 44)
            next_rect = pygame.Rect(s_x + 120, SCREEN_H - 68, SIDEBAR_W - 136, 44)

            if fire_rect.collidepoint(mx, my):
                self.fire_photons()
            elif reset_rect.collidepoint(mx, my):
                self.sfx.play("select")
                self.load_level(self.level_idx, show_briefing=False)
            elif prev_rect.collidepoint(mx, my) and self.level_idx > 0:
                self.sfx.play("select")
                self.load_level(self.level_idx - 1)
            elif next_rect.collidepoint(mx, my) and self.level_complete:
                self.sfx.play("select")
                self.load_level(self.level_idx + 1)
            return

        # Bottom panel clickable level pills
        b_y = GRID_ROWS * TILE_SIZE
        if my >= b_y:
            for i in range(len(LEVELS)):
                pill = self.level_pill_rect(i, b_y)
                if pill.collidepoint(mx, my):
                    self.sfx.play("select")
                    self.load_level(i)
                    return
            return

        if my < GRID_ROWS * TILE_SIZE:
            gx, gy = mx // TILE_SIZE, my // TILE_SIZE
            lvl = LEVELS[self.level_idx]
            emitter_coords = {(unpack_emitter(e)[0], unpack_emitter(e)[1]) for e in lvl["emitters"]}
            if (gx, gy) in emitter_coords or (gx, gy) == lvl["target"]:
                return

            if button == 1:
                if (gx, gy) not in self.grid and self.selected and self.inventory[self.selected] > 0:
                    self.grid[(gx, gy)] = self.selected
                    self.user_placed.add((gx, gy))
                    self.inventory[self.selected] -= 1
                    if self.inventory[self.selected] == 0:
                        self.selected = next((k for k in self.active_components() if self.inventory[k] > 0), self.selected)
                    self.photons = []
                    self.sim_active = False
                    self.sim_state = "READY"
                    self.hom_delta_ps = None
                    self.hom_signed_ps = None
                    self.pol_mismatch = False
                    self.sfx.play("place")
            elif button == 3:
                if (gx, gy) in self.user_placed:
                    comp = self.grid.pop((gx, gy))
                    self.user_placed.remove((gx, gy))
                    self.inventory[comp] += 1
                    self.selected = comp
                    self.photons = []
                    self.sim_active = False
                    self.sim_state = "READY"
                    self.hom_delta_ps = None
                    self.hom_signed_ps = None
                    self.pol_mismatch = False
                    self.sfx.play("remove")

    def process_distillation(self):
        if self.distill_fired:
            return
        buffered = [p for p in self.photons if p.distill_tick is not None]
        if len(buffered) >= 2:
            self.distill_fired = True
            p1, p2 = buffered[0], buffered[1]
            delta_frames = abs(p1.distill_tick - p2.distill_tick)
            delta_ps = delta_frames * 5
            dx, dy = p1.distill_pos
            if delta_frames <= 3:
                purified = Photon("P", dx, dy, RIGHT, wavelength=1550, fidelity=95, noise=0, pol="+")
                purified.fid_history = [(self.tick, 95, 1550)]
                purified.noise_history = [(self.tick, 0)]
                self.photons.append(purified)
                self.spawn_fx("ring", dx * TILE_SIZE + 40, dy * TILE_SIZE + 40, ACCENT_GREEN, 20)
                self.sfx.play("distill")
                self.status_msg = f"DISTILL: 2 photons purified (dt:{delta_ps}ps) -> 95% fidelity photon P emitted!"
                self.status_color = ACCENT_GREEN
            else:
                self.spawn_fx("burst", dx * TILE_SIZE + 40, dy * TILE_SIZE + 40, ACCENT_AMBER, 16)
                self.sfx.play("fail")
                self.status_msg = f"FAILURE: Distillation timing mismatch ({delta_ps}ps > 15ps). Use Delay Line."
                self.status_color = ACCENT_RED

    def evaluate_simulation(self):
        lvl = LEVELS[self.level_idx]
        if any(p.alive for p in self.photons):
            return

        self.sim_active = False
        tx, ty = lvl["target"]
        tcx, tcy = tx * TILE_SIZE + 40, ty * TILE_SIZE + 40

        if lvl["mode"] == "QPU":
            arrived = [p for p in self.photons if p.arrived]
            if not arrived:
                self.sim_state = "FAULT"
                return
            p = arrived[-1]
            self.status_msg = f"SUCCESS! Fault-tolerant QPU locked | 1550nm | Fidelity: {p.fidelity}%"
            self.status_color = ACCENT_GREEN
            self.sim_state = "LOCKED"
            self.level_complete = True
            self.completed_levels.add(self.level_idx)
            self.sfx.play("bsm_ok")
            self.spawn_fx("ring", tcx, tcy, ACCENT_GREEN, 26)
            return

        if not all(p.arrived for p in self.photons):
            self.sim_state = "FAULT"
            return

        if lvl["mode"] == "SINGLE":
            p = self.photons[0]
            self.status_msg = f"SUCCESS! 1550nm detected | Noise: {p.noise} | Fidelity: {p.fidelity}%"
            self.status_color = ACCENT_GREEN
            self.sim_state = "LOCKED"
            self.level_complete = True
            self.completed_levels.add(self.level_idx)
            self.sfx.play("snspd_ok")
            self.spawn_fx("ring", tcx, tcy, ACCENT_GREEN, 20)
        elif lvl["mode"] == "BSM":
            p_a, p_b = self.photons[0], self.photons[1]
            t_a, t_b = p_a.arrival_tick, p_b.arrival_tick
            delta_frames = abs(t_a - t_b)
            delta_ps = delta_frames * 5
            self.hom_delta_ps = delta_ps
            self.hom_signed_ps = (t_a - t_b) * 5
            if delta_frames > 3:
                early = "A" if t_a < t_b else "B"
                self.status_msg = f"FAILURE: HOM timing mismatch! Photon {early} arrived {delta_ps}ps early. Use Delay Line."
                self.status_color = ACCENT_RED
                self.sim_state = "FAULT"
                self.sfx.play("fail")
                self.spawn_fx("burst", tcx, tcy, ACCENT_AMBER, 16)
            elif p_a.pol != p_b.pol:
                self.pol_mismatch = True
                self.status_msg = f"FAILURE: Polarization mismatch (A:'{p_a.pol}' vs B:'{p_b.pol}')! Add Half-Wave Plate (HWP)."
                self.status_color = ACCENT_RED
                self.sim_state = "FAULT"
                self.sfx.play("fail")
                self.spawn_fx("burst", tcx, tcy, ACCENT_PURPLE, 18)
            else:
                self.status_msg = f"SUCCESS! HOM Interference (dt: {delta_ps}ps, pol: '{p_a.pol}') — Entanglement Swapped!"
                self.status_color = ACCENT_GREEN
                self.sim_state = "LOCKED"
                self.level_complete = True
                self.completed_levels.add(self.level_idx)
                self.sfx.play("bsm_ok")
                self.spawn_fx("ring", tcx, tcy, ACCENT_GREEN, 28)

    # --- Procedural Hardware Renderers ---
    def draw_component(self, surf, gx, gy, comp, alpha=255):
        tile_surf = pygame.Surface((TILE_SIZE, TILE_SIZE), pygame.SRCALPHA)
        rect = pygame.Rect(6, 6, TILE_SIZE - 12, TILE_SIZE - 12)
        cx, cy = TILE_SIZE // 2, TILE_SIZE // 2

        if comp == "WALL":
            pygame.draw.rect(tile_surf, WALL_COLOR, rect, border_radius=5)
            pygame.draw.rect(tile_surf, (75, 90, 112), rect, 2, border_radius=5)
            for i in range(14, TILE_SIZE - 14, 12):
                pygame.draw.line(tile_surf, (36, 44, 58), (i, 10), (i - 6, TILE_SIZE - 10), 2)
            for rx, ry in [(12, 12), (68, 12), (12, 68), (68, 68)]:
                pygame.draw.circle(tile_surf, (95, 112, 138), (rx, ry), 2)

        elif comp == "FIBER":
            # Fiber cladding + glowing cyan core
            pygame.draw.rect(tile_surf, (22, 34, 52), (0, cy - 12, TILE_SIZE, 24))
            pygame.draw.line(tile_surf, (40, 80, 115), (0, cy - 12), (TILE_SIZE, cy - 12), 2)
            pygame.draw.line(tile_surf, (40, 80, 115), (0, cy + 12), (TILE_SIZE, cy + 12), 2)
            pygame.draw.line(tile_surf, ACCENT_CYAN, (0, cy), (TILE_SIZE, cy), 4)
            pygame.draw.line(tile_surf, (190, 240, 255), (0, cy), (TILE_SIZE, cy), 1)
            lbl = self.font_tile.render("1550 FIBER", True, (100, 165, 205))
            tile_surf.blit(lbl, (cx - lbl.get_width() // 2, TILE_SIZE - 18))

        elif comp == "LIT_FIBER":
            # Lit coexistence fiber carrying bright classical channel (+15 Raman noise)
            pygame.draw.rect(tile_surf, (42, 32, 22), (0, cy - 12, TILE_SIZE, 24))
            pygame.draw.line(tile_surf, (140, 95, 35), (0, cy - 12), (TILE_SIZE, cy - 12), 2)
            pygame.draw.line(tile_surf, (140, 95, 35), (0, cy + 12), (TILE_SIZE, cy + 12), 2)
            pygame.draw.line(tile_surf, ACCENT_CYAN, (0, cy - 3), (TILE_SIZE, cy - 3), 3)
            pygame.draw.line(tile_surf, ACCENT_AMBER, (0, cy + 4), (TILE_SIZE, cy + 4), 2)
            lbl = self.font_tile.render("LIT FIBER", True, ACCENT_AMBER)
            tile_surf.blit(lbl, (cx - lbl.get_width() // 2, TILE_SIZE - 18))

        elif comp == "PMD_FIBER":
            # Birefringent PMD fiber rotating polarization (+ <-> x)
            pygame.draw.rect(tile_surf, (34, 24, 52), (0, cy - 12, TILE_SIZE, 24))
            pygame.draw.line(tile_surf, (105, 70, 165), (0, cy - 12), (TILE_SIZE, cy - 12), 2)
            pygame.draw.line(tile_surf, (105, 70, 165), (0, cy + 12), (TILE_SIZE, cy + 12), 2)
            wave_pts = [(x, cy + int(6 * math.sin(x * 0.22))) for x in range(0, TILE_SIZE + 1, 4)]
            pygame.draw.lines(tile_surf, ACCENT_PURPLE, False, wave_pts, 3)
            lbl = self.font_tile.render("PMD FIBER", True, ACCENT_PURPLE)
            tile_surf.blit(lbl, (cx - lbl.get_width() // 2, TILE_SIZE - 18))

        elif comp == "ATMOS":
            # Turbulent free-space atmospheric downlink corridor
            pygame.draw.rect(tile_surf, (16, 36, 44), rect, border_radius=8)
            pygame.draw.rect(tile_surf, (55, 130, 145), rect, 1, border_radius=8)
            for row_off, amp in ((-14, 4), (-2, 5), (10, 4)):
                pts = [
                    (x, cy + row_off + int(amp * math.sin(x * 0.16 + self.frame * 0.08 + row_off)))
                    for x in range(12, TILE_SIZE - 10, 6)
                ]
                pygame.draw.lines(tile_surf, ACCENT_TEAL, False, pts, 2)
            lbl = self.font_tile.render("ATMOS", True, ACCENT_TEAL)
            tile_surf.blit(lbl, (cx - lbl.get_width() // 2, 55))

        elif comp == "DISTILL":
            # Entanglement Distillation / Purification Node (2 -> 1 at 95%)
            pygame.draw.rect(tile_surf, (16, 44, 38), rect, border_radius=8)
            pygame.draw.rect(tile_surf, ACCENT_GREEN, rect, 2, border_radius=8)
            diamond = [(cx, 14), (cx + 18, 32), (cx, 50), (cx - 18, 32)]
            pygame.draw.polygon(tile_surf, (28, 90, 72), diamond)
            pygame.draw.polygon(tile_surf, ACCENT_GREEN, diamond, 2)
            pygame.draw.line(tile_surf, (190, 255, 230), (cx - 6, 32), (cx + 12, 32), 2)
            lbl = self.font_tile.render("DISTILL", True, ACCENT_GREEN)
            tile_surf.blit(lbl, (cx - lbl.get_width() // 2, 55))

        elif comp == "HWP":
            # Half-Wave Plate birefringent crystal
            pygame.draw.rect(tile_surf, (44, 34, 22), rect, border_radius=8)
            pygame.draw.rect(tile_surf, (244, 180, 90), rect, 2, border_radius=8)
            pygame.draw.rect(tile_surf, (92, 68, 34), (cx - 7, 16, 14, 34), border_radius=3)
            pygame.draw.rect(tile_surf, (255, 220, 150), (cx - 7, 16, 14, 34), 1, border_radius=3)
            pygame.draw.line(tile_surf, (255, 220, 150), (cx - 18, 33), (cx + 18, 33), 1)
            pygame.draw.line(tile_surf, (255, 220, 150), (cx - 12, 21), (cx + 12, 45), 1)
            lbl = self.font_tile.render("HWP (+/x)", True, (244, 180, 90))
            tile_surf.blit(lbl, (cx - lbl.get_width() // 2, 55))

        elif comp == "AO":
            # Adaptive Optics deformable mirror + wavefront sensor
            pygame.draw.rect(tile_surf, (16, 38, 42), rect, border_radius=8)
            pygame.draw.rect(tile_surf, ACCENT_TEAL, rect, 2, border_radius=8)
            arc_pts = [(cx - 18 + i * 6, 22 + int(4 * math.sin(i * 0.6))) for i in range(7)]
            pygame.draw.lines(tile_surf, (180, 255, 245), False, arc_pts, 3)
            for ax in (cx - 14, cx, cx + 14):
                pygame.draw.line(tile_surf, ACCENT_TEAL, (ax, 27), (ax, 44), 2)
                pygame.draw.rect(tile_surf, (40, 100, 105), (ax - 3, 42, 6, 5))
            lbl = self.font_tile.render("ADAPT OPT", True, ACCENT_TEAL)
            tile_surf.blit(lbl, (cx - lbl.get_width() // 2, 55))

        elif comp == "QFC":
            pygame.draw.rect(tile_surf, (28, 24, 48), rect, border_radius=8)
            pygame.draw.rect(tile_surf, ACCENT_PURPLE, rect, 2, border_radius=8)
            # PPLN Crystal block with periodic poling stripes
            c_rect = pygame.Rect(16, 22, 48, 26)
            pygame.draw.rect(tile_surf, (62, 46, 105), c_rect, border_radius=3)
            for x in range(20, 62, 6):
                pygame.draw.line(tile_surf, ACCENT_PURPLE, (x, 23), (x, 47), 2)
            pygame.draw.rect(tile_surf, (210, 195, 255), c_rect, 1, border_radius=3)
            lbl = self.font_tile.render("PPLN QFC", True, ACCENT_PURPLE)
            tile_surf.blit(lbl, (cx - lbl.get_width() // 2, 55))

        elif comp == "VBG":
            pygame.draw.rect(tile_surf, (18, 36, 48), rect, border_radius=8)
            pygame.draw.rect(tile_surf, ACCENT_CYAN, rect, 2, border_radius=8)
            # Volume Bragg Grating diffraction planes
            for x in range(24, 58, 7):
                pygame.draw.line(tile_surf, ACCENT_CYAN, (x, 16), (x - 4, 50), 2)
            lbl = self.font_tile.render("VBG FILTER", True, ACCENT_CYAN)
            tile_surf.blit(lbl, (cx - lbl.get_width() // 2, 55))

        elif comp == "MEMORY":
            pygame.draw.rect(tile_surf, (18, 40, 36), rect, border_radius=8)
            pygame.draw.rect(tile_surf, ACCENT_GREEN, rect, 2, border_radius=8)
            # Diamond NV lattice hexagon + pulsing spin core
            pts = [(cx + int(15 * math.cos(a)), cy - 6 + int(15 * math.sin(a))) for a in [i * math.pi / 3 for i in range(6)]]
            pygame.draw.polygon(tile_surf, (30, 80, 65), pts)
            pygame.draw.polygon(tile_surf, ACCENT_GREEN, pts, 2)
            pulse = 4 + int(2 * math.sin(self.frame * 0.1))
            pygame.draw.circle(tile_surf, (180, 255, 225), (cx, cy - 6), pulse)
            lbl = self.font_tile.render("Q-MEMORY", True, ACCENT_GREEN)
            tile_surf.blit(lbl, (cx - lbl.get_width() // 2, 55))

        elif comp == "DELAY":
            pygame.draw.rect(tile_surf, (42, 34, 20), rect, border_radius=8)
            pygame.draw.rect(tile_surf, ACCENT_AMBER, rect, 2, border_radius=8)
            # Coiled optical fiber loops
            for r in (16, 11, 6):
                pygame.draw.circle(tile_surf, ACCENT_AMBER, (cx, cy - 6), r, 2)
            lbl = self.font_tile.render("+100ps", True, ACCENT_AMBER)
            tile_surf.blit(lbl, (cx - lbl.get_width() // 2, 55))

        elif comp in ("MIRROR_SLASH", "MIRROR_BACK"):
            pygame.draw.rect(tile_surf, (24, 30, 42), rect, border_radius=6)
            pygame.draw.rect(tile_surf, (55, 68, 90), rect, 1, border_radius=6)
            if comp == "MIRROR_SLASH":
                pygame.draw.line(tile_surf, (95, 115, 140), (18, 66), (66, 18), 7)
                pygame.draw.line(tile_surf, (220, 245, 255), (16, 64), (64, 16), 3)
            else:
                pygame.draw.line(tile_surf, (95, 115, 140), (14, 18), (62, 66), 7)
                pygame.draw.line(tile_surf, (220, 245, 255), (16, 16), (64, 64), 3)

        if alpha < 255:
            tile_surf.set_alpha(alpha)
        surf.blit(tile_surf, (gx * TILE_SIZE, gy * TILE_SIZE))

    def draw_emitter(self, gx, gy, label, direction=RIGHT, wavelength=780):
        rect = pygame.Rect(gx * TILE_SIZE + 6, gy * TILE_SIZE + 6, TILE_SIZE - 12, TILE_SIZE - 12)
        cx, cy = rect.centerx, rect.centery
        is_nv = (wavelength == 637)
        accent = ACCENT_ORANGE if is_nv else ACCENT_RED
        bg = (48, 28, 16) if is_nv else (48, 20, 28)
        pygame.draw.rect(self.screen, bg, rect, border_radius=8)
        pygame.draw.rect(self.screen, accent, rect, 2, border_radius=8)
        # Paul Trap electrodes or NV diamond corners
        for dx, dy in [(-14, -14), (14, -14), (-14, 10), (14, 10)]:
            pygame.draw.circle(self.screen, (175, 110, 65) if is_nv else (160, 80, 95), (cx + dx, cy + dy), 4)
        # Subtle emission direction chevron
        ddx, ddy = direction
        tip = (cx + ddx * 28, cy - 2 + ddy * 24)
        perp = (-ddy, ddx)
        b1 = (tip[0] - ddx * 6 + perp[0] * 4, tip[1] - ddy * 6 + perp[1] * 4)
        b2 = (tip[0] - ddx * 6 - perp[0] * 4, tip[1] - ddy * 6 - perp[1] * 4)
        pygame.draw.polygon(self.screen, accent, [tip, b1, b2])
        # Glowing emitter core
        self.screen.blit(self.glows["orange" if is_nv else "red"], (cx - 26, cy - 28))
        pulse = 4 + int(1.5 * math.sin(self.frame * 0.12))
        pygame.draw.circle(self.screen, (255, 225, 195) if is_nv else (255, 210, 220), (cx, cy - 2), pulse)
        tag = f"NV {label} 637" if is_nv else f"ION {label} 780"
        lbl = self.font_tile.render(tag, True, accent)
        self.screen.blit(lbl, (cx - lbl.get_width() // 2, rect.bottom - 16))

    def draw_dashed_hline(self, x1, x2, y, color, dash=5, gap=4):
        x = x1
        while x < x2:
            pygame.draw.line(self.screen, color, (x, y), (min(x + dash, x2), y), 1)
            x += dash + gap

    def draw_scope_frame(self, rect, title, readout="", readout_color=TEXT_MUTED):
        pygame.draw.rect(self.screen, (11, 15, 23), rect, border_radius=6)
        pygame.draw.rect(self.screen, (38, 50, 68), rect, 1, border_radius=6)
        # Subtle oscilloscope graticule
        plot_top = rect.y + 18
        plot_bot = rect.bottom - 6
        for frac in (0.25, 0.5, 0.75):
            gy = int(plot_top + (plot_bot - plot_top) * frac)
            pygame.draw.line(self.screen, (20, 28, 40), (rect.x + 6, gy), (rect.right - 6, gy), 1)
        for frac in (0.25, 0.5, 0.75):
            gx = int(rect.x + 6 + (rect.w - 12) * frac)
            pygame.draw.line(self.screen, (20, 28, 40), (gx, plot_top), (gx, plot_bot), 1)
        self.screen.blit(self.font_xs.render(title, True, TEXT_MUTED), (rect.x + 7, rect.y + 4))
        if readout:
            r_surf = self.font_xs.render(readout, True, readout_color)
            self.screen.blit(r_surf, (rect.right - r_surf.get_width() - 7, rect.y + 4))

    def draw_telemetry_deck(self, b_y, s_x, lvl, mx, my):
        pygame.draw.rect(self.screen, PANEL_BG, (0, b_y, s_x, BOTTOM_H))
        pygame.draw.line(self.screen, GRID_LINE, (0, b_y), (s_x, b_y), 2)

        # Header Row: Title, clickable level pills, picosecond clock, status badge
        self.screen.blit(self.font_lg.render(lvl["title"], True, ACCENT_CYAN), (16, b_y + 10))

        for i in range(len(LEVELS)):
            pill = self.level_pill_rect(i, b_y)
            is_cur = (i == self.level_idx)
            is_done = (i in self.completed_levels)
            is_hov = pill.collidepoint(mx, my)
            bg = (36, 56, 78) if is_cur else ((30, 44, 60) if is_hov else (18, 24, 34))
            bd = ACCENT_CYAN if is_cur else (ACCENT_GREEN if is_done else GRID_LINE)
            pygame.draw.rect(self.screen, bg, pill, border_radius=4)
            pygame.draw.rect(self.screen, bd, pill, 1, border_radius=4)
            txt_col = TEXT_WHITE if is_cur else (ACCENT_GREEN if is_done else TEXT_MUTED)
            num = self.font_xs.render(str(i + 1), True, txt_col)
            self.screen.blit(num, (pill.centerx - num.get_width() // 2, pill.centery - num.get_height() // 2))

        clock_txt = self.font_mono.render(f"T+{self.tick * 5:04d}ps", True, TEXT_MUTED)
        self.screen.blit(clock_txt, (s_x - 92 - clock_txt.get_width(), b_y + 15))

        state_colors = {
            "READY": (TEXT_MUTED, (22, 28, 40)),
            "ACTIVE": (ACCENT_AMBER, (46, 36, 18)),
            "LOCKED": (ACCENT_GREEN, (18, 46, 36)),
            "FAULT": (ACCENT_RED, (52, 20, 28)),
        }
        st_fg, st_bg = state_colors.get(self.sim_state, (TEXT_MUTED, (22, 28, 40)))
        badge = pygame.Rect(s_x - 84, b_y + 10, 70, 22)
        pygame.draw.rect(self.screen, st_bg, badge, border_radius=4)
        pygame.draw.rect(self.screen, st_fg, badge, 1, border_radius=4)
        st_lbl = self.font_xs.render(self.sim_state, True, st_fg)
        self.screen.blit(st_lbl, (badge.centerx - st_lbl.get_width() // 2, badge.centery - st_lbl.get_height() // 2))

        # Status log strip (safely clipped within panel width)
        log_rect = pygame.Rect(14, b_y + 40, s_x - 28, 28)
        pygame.draw.rect(self.screen, (16, 22, 32), log_rect, border_radius=5)
        pygame.draw.rect(self.screen, (32, 42, 58), log_rect, 1, border_radius=5)
        msg_surf = self.font_sm.render(self.status_msg, True, self.status_color)
        max_msg_w = log_rect.w - 16
        if msg_surf.get_width() > max_msg_w:
            trimmed = self.status_msg
            while len(trimmed) > 3 and self.font_sm.size(trimmed + "...")[0] > max_msg_w:
                trimmed = trimmed[:-1]
            msg_surf = self.font_sm.render(trimmed + "...", True, self.status_color)
        self.screen.blit(msg_surf, (log_rect.x + 8, log_rect.centery - msg_surf.get_height() // 2))

        # --- Pane 1: Fidelity Oscilloscope Trace ---
        p1 = pygame.Rect(14, b_y + 76, 250, 102)
        min_fid = 90 if lvl["mode"] == "QPU" else 60
        if self.photons:
            fid_str = " ".join(f"{p.pid}:{p.fidelity}%" for p in self.photons[-2:])
            fid_col = ACCENT_GREEN if any(p.fidelity >= min_fid for p in self.photons) else ACCENT_RED
        else:
            fid_str, fid_col = "STANDBY", TEXT_MUTED
        self.draw_scope_frame(p1, "FIDELITY TRACE", fid_str, fid_col)

        plot_x1, plot_x2 = p1.x + 8, p1.right - 8
        plot_y1, plot_y2 = p1.y + 24, p1.bottom - 8
        plot_h = plot_y2 - plot_y1
        thresh_y = int(plot_y2 - (min_fid / 100.0) * plot_h)
        self.draw_dashed_hline(plot_x1, plot_x2, thresh_y, (180, 70, 90))
        self.screen.blit(self.font_xs.render(f"{min_fid}%", True, (180, 80, 100)), (plot_x1 + 2, max(plot_y1 - 2, thresh_y - 13)))

        for idx, p in enumerate(self.photons):
            hist = p.fid_history
            if len(hist) >= 2:
                max_t = max(40, self.tick, hist[-1][0])
                pts = []
                for t_val, fid_val, wl in hist:
                    px = int(plot_x1 + (t_val / max_t) * (plot_x2 - plot_x1))
                    py = int(plot_y2 - (fid_val / 100.0) * plot_h)
                    pts.append((px, py, wl))
                for i in range(1, len(pts)):
                    wl = pts[i][2]
                    if wl == 637:
                        col = ACCENT_ORANGE
                    elif wl == 780:
                        col = ACCENT_RED
                    elif p.pid == "P":
                        col = ACCENT_GREEN
                    else:
                        col = ACCENT_CYAN if idx == 0 else ACCENT_PURPLE
                    pygame.draw.line(self.screen, col, pts[i - 1][:2], pts[i][:2], 2)
                last_wl = pts[-1][2]
                last_col = ACCENT_ORANGE if last_wl == 637 else (ACCENT_RED if last_wl == 780 else (ACCENT_GREEN if p.pid == "P" else (ACCENT_CYAN if idx == 0 else ACCENT_PURPLE)))
                pygame.draw.circle(self.screen, last_col, pts[-1][:2], 3)

        # --- Pane 2: Raman Noise Floor + LED Meter ---
        p2 = pygame.Rect(274, b_y + 76, 244, 102)
        max_noise = max((p.noise for p in self.photons), default=0)
        if self.photons:
            n_str = " ".join(f"{p.pid}:{p.noise}" for p in self.photons)
            n_col = ACCENT_GREEN if max_noise <= 10 else ACCENT_AMBER
        else:
            n_str, n_col = "CLEAN", TEXT_MUTED
        self.draw_scope_frame(p2, "RAMAN NOISE", n_str, n_col)

        nx1, nx2 = p2.x + 8, p2.right - 34
        ny1, ny2 = p2.y + 24, p2.bottom - 8
        nh = ny2 - ny1
        nthresh_y = int(ny2 - (10.0 / 50.0) * nh)
        self.draw_dashed_hline(nx1, nx2, nthresh_y, ACCENT_AMBER)
        self.screen.blit(self.font_xs.render("MAX 10", True, (185, 145, 50)), (nx1 + 2, nthresh_y - 13))

        for idx, p in enumerate(self.photons):
            nhist = p.noise_history
            if len(nhist) >= 2:
                max_t = max(40, self.tick, nhist[-1][0])
                pts = []
                for t_val, n_val in nhist:
                    px = int(nx1 + (t_val / max_t) * (nx2 - nx1))
                    py = int(ny2 - (min(50, n_val) / 50.0) * nh)
                    pts.append((px, py))
                col = ACCENT_AMBER if p.noise > 10 else (ACCENT_CYAN if idx == 0 else ACCENT_PURPLE)
                pygame.draw.lines(self.screen, col, False, pts, 2)
                pygame.draw.circle(self.screen, col, pts[-1], 3)

        # Vertical 8-segment LED noise meter on the right of Pane 2
        led_x = p2.right - 22
        segs = 8
        seg_h = max(3, (nh - (segs - 1) * 2) // segs)
        lit_segs = int(round((min(50, max_noise) / 50.0) * segs)) if self.photons else 0
        for s in range(segs):
            sy = ny2 - (s + 1) * seg_h - s * 2
            if s < lit_segs:
                col = ACCENT_GREEN if s < 2 else (ACCENT_AMBER if s < 5 else ACCENT_RED)
            else:
                col = (24, 32, 44)
            pygame.draw.rect(self.screen, col, (led_x, sy, 14, seg_h), border_radius=2)

        # --- Pane 3: Detector Lock (SNSPD / QPU) or HOM Coincidence Dip (BSM) ---
        p3 = pygame.Rect(528, b_y + 76, 258, 102)
        if lvl["mode"] == "BSM":
            if self.pol_mismatch:
                hom_str, hom_col = "POL MISMATCH", ACCENT_RED
            elif self.hom_signed_ps is not None:
                sign = "+" if self.hom_signed_ps > 0 else ""
                hom_str = f"dt:{sign}{self.hom_signed_ps}ps"
                hom_col = ACCENT_GREEN if self.hom_delta_ps <= 15 else ACCENT_RED
            elif self.photons:
                hom_str, hom_col = "SYNCING...", ACCENT_AMBER
            else:
                hom_str, hom_col = "WINDOW <=15ps", TEXT_MUTED
            self.draw_scope_frame(p3, "HOM INTERFERENCE", hom_str, hom_col)

            hx1, hx2 = p3.x + 10, p3.right - 10
            hy1, hy2 = p3.y + 24, p3.bottom - 18
            hh = hy2 - hy1
            hmid = (hx1 + hx2) // 2

            # Shaded +/-15ps HOM lock zone
            win_half = int((15.0 / 100.0) * ((hx2 - hx1) / 2))
            zone = pygame.Surface((win_half * 2, hh), pygame.SRCALPHA)
            zone.fill((*ACCENT_GREEN, 38))
            self.screen.blit(zone, (hmid - win_half, hy1))

            # Theoretical HOM dip curve: 1 - exp(-(dt / 22)^2)
            dip_pts = []
            steps = 40
            for i in range(steps + 1):
                frac = i / steps
                dt = -100.0 + 200.0 * frac
                val = 1.0 - math.exp(-((dt / 22.0) ** 2))
                px = int(hx1 + frac * (hx2 - hx1))
                py = int(hy2 - val * hh)
                dip_pts.append((px, py))
            pygame.draw.lines(self.screen, (70, 120, 160), False, dip_pts, 2)

            # Measured arrival delta marker
            if self.hom_signed_ps is not None:
                clamped = max(-100, min(100, self.hom_signed_ps))
                mx_pos = int(hmid + (clamped / 100.0) * ((hx2 - hx1) / 2))
                val = 1.0 - math.exp(-((clamped / 22.0) ** 2))
                my_pos = int(hy2 - val * hh)
                mcol = ACCENT_GREEN if (self.hom_delta_ps <= 15 and not self.pol_mismatch) else ACCENT_RED
                pygame.draw.line(self.screen, mcol, (mx_pos, hy1), (mx_pos, hy2), 2)
                pygame.draw.circle(self.screen, mcol, (mx_pos, my_pos), 4)

            self.screen.blit(self.font_xs.render("-100ps", True, TEXT_MUTED), (hx1, p3.bottom - 15))
            z_lbl = self.font_xs.render("0", True, ACCENT_GREEN)
            self.screen.blit(z_lbl, (hmid - z_lbl.get_width() // 2, p3.bottom - 15))
            r_lbl = self.font_xs.render("+100ps", True, TEXT_MUTED)
            self.screen.blit(r_lbl, (hx2 - r_lbl.get_width(), p3.bottom - 15))
        else:
            p0 = self.photons[-1] if self.photons else None
            is_qpu = (lvl["mode"] == "QPU")
            min_f = 90 if is_qpu else 60
            wl_str = f"{p0.wavelength}nm" if p0 else ("QPU >=90%" if is_qpu else "SNSPD 1.5K")
            wl_col = (ACCENT_CYAN if p0.wavelength == 1550 else (ACCENT_ORANGE if p0.wavelength == 637 else ACCENT_RED)) if p0 else TEXT_MUTED
            self.draw_scope_frame(p3, "SPECTRUM / LOCK", wl_str, wl_col)

            sx1, sx2 = p3.x + 10, p3.right - 10
            sy1, sy2 = p3.y + 24, p3.bottom - 28
            sh = sy2 - sy1

            # Optical spectrum analyzer peak (637nm/780nm left, 1550nm right)
            spec_pts = []
            if not p0 or p0.wavelength == 780:
                center_frac, peak_col = 0.25, ACCENT_RED
            elif p0.wavelength == 637:
                center_frac, peak_col = 0.16, ACCENT_ORANGE
            else:
                center_frac, peak_col = 0.75, ACCENT_CYAN
            amp = 0.92 if p0 else 0.20
            for i in range(41):
                f = i / 40.0
                y_val = amp * math.exp(-(((f - center_frac) / 0.065) ** 2))
                if p0 and p0.noise > 0:
                    y_val = min(1.0, y_val + (p0.noise / 85.0) * math.exp(-(((f - 0.56) / 0.14) ** 2)))
                spec_pts.append((int(sx1 + f * (sx2 - sx1)), int(sy2 - y_val * sh)))
            pygame.draw.lines(self.screen, peak_col, False, spec_pts, 2)

            # 3 Detector lock LED pills
            checks = [
                ("1550nm", bool(p0 and p0.wavelength == 1550)),
                ("NOISE<=10", bool(p0 and p0.noise <= 10)),
                (f"FID>={min_f}%", bool(p0 and p0.fidelity >= min_f)),
            ]
            cx_pill = p3.x + 8
            pw = 78
            for label, ok in checks:
                pr = pygame.Rect(cx_pill, p3.bottom - 22, pw, 16)
                if not p0:
                    bg, fg = (20, 26, 36), TEXT_MUTED
                elif ok:
                    bg, fg = (18, 48, 38), ACCENT_GREEN
                else:
                    bg, fg = (52, 22, 28), ACCENT_RED
                pygame.draw.rect(self.screen, bg, pr, border_radius=3)
                pygame.draw.rect(self.screen, fg, pr, 1, border_radius=3)
                t_s = self.font_xs.render(label, True, fg)
                self.screen.blit(t_s, (pr.centerx - t_s.get_width() // 2, pr.centery - t_s.get_height() // 2))
                cx_pill += pw + 4

    def draw_target(self, gx, gy, mode):
        rect = pygame.Rect(gx * TILE_SIZE + 6, gy * TILE_SIZE + 6, TILE_SIZE - 12, TILE_SIZE - 12)
        cx, cy = rect.centerx, rect.centery
        border = ACCENT_GREEN if self.level_complete else (ACCENT_PURPLE if mode == "QPU" else ACCENT_CYAN)
        pygame.draw.rect(self.screen, (24, 24, 48) if mode == "QPU" else (16, 38, 48), rect, border_radius=8)
        pygame.draw.rect(self.screen, border, rect, 2, border_radius=8)

        if mode == "BSM":
            # 50:50 Beam Splitter Prism Cube
            cube = pygame.Rect(cx - 16, cy - 22, 32, 32)
            pygame.draw.rect(self.screen, (35, 85, 115), cube)
            pygame.draw.rect(self.screen, ACCENT_CYAN, cube, 2)
            pygame.draw.line(self.screen, (200, 245, 255), cube.bottomleft, cube.topright, 2)
            lbl = self.font_tile.render("BSM 50:50", True, ACCENT_AMBER)
            self.screen.blit(lbl, (cx - lbl.get_width() // 2, rect.bottom - 16))
        elif mode == "QPU":
            # Fault-Tolerant Quantum Processor Chip (>=90% fidelity)
            chip = pygame.Rect(cx - 14, cy - 20, 28, 28)
            pygame.draw.rect(self.screen, (48, 36, 86), chip, border_radius=4)
            pygame.draw.rect(self.screen, border, chip, 2, border_radius=4)
            for dx in (-6, 6):
                for dy in (-6, 6):
                    pygame.draw.circle(self.screen, (220, 200, 255), (chip.centerx + dx, chip.centery + dy), 3)
            lbl = self.font_tile.render("QPU >=90%", True, ACCENT_PURPLE)
            self.screen.blit(lbl, (cx - lbl.get_width() // 2, rect.bottom - 16))
        else:
            # Superconducting Nanowire Meander (SNSPD)
            pts = [
                (cx - 16, cy - 18), (cx + 16, cy - 18),
                (cx + 16, cy - 11), (cx - 16, cy - 11),
                (cx - 16, cy - 4),  (cx + 16, cy - 4),
                (cx + 16, cy + 3),  (cx - 16, cy + 3),
            ]
            pygame.draw.lines(self.screen, border, False, pts, 2)
            lbl = self.font_tile.render("SNSPD 1.5K", True, ACCENT_CYAN)
            self.screen.blit(lbl, (cx - lbl.get_width() // 2, rect.bottom - 16))

    def draw(self):
        self.screen.fill(BG_DARK)
        lvl = LEVELS[self.level_idx]

        # 1. Optical Breadboard Grid
        for r in range(GRID_ROWS):
            for c in range(GRID_COLS):
                x, y = c * TILE_SIZE, r * TILE_SIZE
                pygame.draw.rect(self.screen, GRID_BG, (x, y, TILE_SIZE, TILE_SIZE))
                pygame.draw.rect(self.screen, GRID_LINE, (x, y, TILE_SIZE, TILE_SIZE), 1)
                # Machined tapped mounting holes
                for hx in (x + 20, x + 60):
                    for hy in (y + 20, y + 60):
                        pygame.draw.circle(self.screen, BREADBOARD_HOLE, (hx, hy), 2)

        # 2. Fixed & Placed Components
        for (gx, gy), comp in self.grid.items():
            self.draw_component(self.screen, gx, gy, comp)

        # 3. Hover Ghost Preview + Precision Reticle
        mx, my = pygame.mouse.get_pos()
        if self.overlay is None and mx < GRID_COLS * TILE_SIZE and my < GRID_ROWS * TILE_SIZE:
            hgx, hgy = mx // TILE_SIZE, my // TILE_SIZE
            emitter_coords = {(unpack_emitter(e)[0], unpack_emitter(e)[1]) for e in lvl["emitters"]}
            if (hgx, hgy) not in self.grid and (hgx, hgy) not in emitter_coords and (hgx, hgy) != lvl["target"]:
                if self.selected and self.inventory.get(self.selected, 0) > 0:
                    self.draw_component(self.screen, hgx, hgy, self.selected, alpha=110)
                    tx, ty = hgx * TILE_SIZE, hgy * TILE_SIZE
                    for cx0, cy0, dx, dy in [
                        (tx + 3, ty + 3, 10, 10),
                        (tx + TILE_SIZE - 4, ty + 3, -10, 10),
                        (tx + 3, ty + TILE_SIZE - 4, 10, -10),
                        (tx + TILE_SIZE - 4, ty + TILE_SIZE - 4, -10, -10),
                    ]:
                        pygame.draw.line(self.screen, ACCENT_CYAN, (cx0, cy0), (cx0 + dx, cy0), 1)
                        pygame.draw.line(self.screen, ACCENT_CYAN, (cx0, cy0), (cx0, cy0 + dy), 1)
                    coord = self.font_xs.render(f"{hgx},{hgy}", True, (90, 130, 160))
                    self.screen.blit(coord, (tx + 6, ty + 5))

        # 4. Emitters & Target
        for i, e in enumerate(lvl["emitters"]):
            ex, ey, edir, wl = unpack_emitter(e)
            self.draw_emitter(ex, ey, chr(65 + i), edir, wl)
        self.draw_target(*lvl["target"], lvl["mode"])

        # 5. Photons & Laser Trails
        for p in self.photons:
            if len(p.trail) > 1:
                for i in range(1, len(p.trail)):
                    x1, y1, wl = p.trail[i - 1]
                    x2, y2, _ = p.trail[i]
                    col = ACCENT_ORANGE if wl == 637 else (ACCENT_RED if wl == 780 else (ACCENT_GREEN if p.pid == "P" else ACCENT_CYAN))
                    width = max(1, int(5 * (i / len(p.trail))))
                    pygame.draw.line(self.screen, col, (x1, y1), (x2, y2), width)

            if p.alive:
                if p.wavelength == 637:
                    glow_key, pcol = "orange", ACCENT_ORANGE
                elif p.wavelength == 780:
                    glow_key, pcol = "red", ACCENT_RED
                elif p.pid == "P":
                    glow_key, pcol = "green", ACCENT_GREEN
                else:
                    glow_key, pcol = "cyan", ACCENT_CYAN
                self.screen.blit(self.glows[glow_key], (p.px - 26, p.py - 26))

                if p.noise > 10:
                    # Jittering Raman noise halo
                    for _ in range(3):
                        jx = p.px + random.randint(-12, 12)
                        jy = p.py + random.randint(-12, 12)
                        pygame.draw.circle(self.screen, ACCENT_AMBER, (int(jx), int(jy)), 2)

                if p.pol == "x":
                    # Diagonal polarization indicator (x-basis)
                    pygame.draw.line(self.screen, ACCENT_PURPLE, (p.px - 9, p.py - 9), (p.px + 9, p.py + 9), 2)
                    pygame.draw.line(self.screen, ACCENT_PURPLE, (p.px - 9, p.py + 9), (p.px + 9, p.py - 9), 2)

                pygame.draw.circle(self.screen, pcol, (int(p.px), int(p.py)), 7)
                pygame.draw.circle(self.screen, TEXT_WHITE, (int(p.px), int(p.py)), 3)

        # 6. Particles & Shockwaves
        for sw in self.shockwaves:
            sw.draw(self.screen)
        for pt in self.particles:
            pt.draw(self.screen)

        # 7. Sidebar
        s_x = GRID_COLS * TILE_SIZE
        pygame.draw.rect(self.screen, SIDEBAR_BG, (s_x, 0, SIDEBAR_W, SCREEN_H))
        pygame.draw.line(self.screen, GRID_LINE, (s_x, 0), (s_x, SCREEN_H), 2)
        self.screen.blit(self.font_lg.render("COMPONENTS", True, TEXT_WHITE), (s_x + 16, 16))
        self.screen.blit(self.font_sm.render("L-Click: Place  |  R-Click: Remove", True, TEXT_MUTED), (s_x + 16, 44))

        # ? INFO Briefing Pill + SFX Mute Toggle Pill
        info_rect = pygame.Rect(s_x + SIDEBAR_W - 154, 14, 62, 26)
        i_hov = (self.overlay is None) and info_rect.collidepoint(mx, my)
        pygame.draw.rect(self.screen, (34, 52, 74) if i_hov else (22, 34, 50), info_rect, border_radius=5)
        pygame.draw.rect(self.screen, ACCENT_CYAN, info_rect, 1, border_radius=5)
        i_txt = self.font_xs.render("? INFO", True, ACCENT_CYAN)
        self.screen.blit(i_txt, (info_rect.centerx - i_txt.get_width() // 2, info_rect.centery - i_txt.get_height() // 2))

        mute_rect = pygame.Rect(s_x + SIDEBAR_W - 86, 14, 70, 26)
        sfx_on = self.sfx.enabled and not self.sfx.muted
        m_hov = (self.overlay is None) and mute_rect.collidepoint(mx, my)
        m_bg = (24, 46, 56) if sfx_on else (36, 26, 32)
        if m_hov:
            m_bg = tuple(min(255, c + 12) for c in m_bg)
        m_bd = ACCENT_CYAN if sfx_on else TEXT_MUTED
        pygame.draw.rect(self.screen, m_bg, mute_rect, border_radius=5)
        pygame.draw.rect(self.screen, m_bd, mute_rect, 1, border_radius=5)
        m_txt = self.font_xs.render("SFX: ON" if sfx_on else "SFX: OFF", True, ACCENT_CYAN if sfx_on else TEXT_MUTED)
        self.screen.blit(m_txt, (mute_rect.centerx - m_txt.get_width() // 2, mute_rect.centery - m_txt.get_height() // 2))

        y = 70
        for key in self.active_components():
            info = COMPONENTS[key]
            count = self.inventory.get(key, 0)
            rect = pygame.Rect(s_x + 16, y, SIDEBAR_W - 32, 54)
            help_rect = pygame.Rect(rect.right - 28, rect.y + 6, 22, 22)
            is_sel = (self.selected == key and count > 0)
            h_hov = (self.overlay is None) and help_rect.collidepoint(mx, my)
            is_hov = (self.overlay is None) and rect.collidepoint(mx, my) and not h_hov and count > 0
            bg_col = (36, 48, 68) if is_sel else ((31, 41, 58) if is_hov else PANEL_BG)
            pygame.draw.rect(self.screen, bg_col, rect, border_radius=6)
            border_col = info["color"] if is_sel else (GRID_LINE if count > 0 else (30, 36, 46))
            pygame.draw.rect(self.screen, border_col, rect, 2 if is_sel else 1, border_radius=6)

            title_col = info["color"] if count > 0 else TEXT_MUTED
            self.screen.blit(self.font_md.render(f"{info['name']} (x{count})", True, title_col), (rect.x + 12, rect.y + 8))
            self.screen.blit(self.font_sm.render(info["desc"], True, TEXT_MUTED), (rect.x + 12, rect.y + 30))

            # '?' component explanation badge
            pygame.draw.rect(self.screen, (42, 62, 86) if h_hov else (20, 28, 42), help_rect, border_radius=11)
            pygame.draw.rect(self.screen, ACCENT_CYAN if h_hov else GRID_LINE, help_rect, 1, border_radius=11)
            q_surf = self.font_xs.render("?", True, TEXT_WHITE if h_hov else ACCENT_CYAN)
            self.screen.blit(q_surf, (help_rect.centerx - q_surf.get_width() // 2, help_rect.centery - q_surf.get_height() // 2))
            y += 60

        # Version & Self-Update Status Pill
        up_snap = self.updater.snapshot()
        up_state = up_snap["state"]
        update_rect = pygame.Rect(s_x + 16, SCREEN_H - 224, SIDEBAR_W - 32, 26)
        u_hov = (self.overlay is None) and update_rect.collidepoint(mx, my)
        if up_state == "AVAILABLE":
            ver = (up_snap["release_info"] or {}).get("version", "")
            u_bg = (22, 62, 48) if u_hov else (16, 48, 38)
            u_bd = ACCENT_GREEN
            u_fg = ACCENT_GREEN
            u_label = f"UPDATE AVAILABLE: v{ver}  •  CLICK"
        elif up_state in ("CHECKING", "DOWNLOADING", "VERIFYING", "INSTALLING"):
            u_bg = (48, 38, 18)
            u_bd = ACCENT_AMBER
            u_fg = ACCENT_AMBER
            u_label = "CHECKING UPDATES..." if up_state == "CHECKING" else "UPDATING APP..."
        else:
            u_bg = (30, 42, 58) if u_hov else (20, 27, 39)
            u_bd = ACCENT_CYAN if u_hov else GRID_LINE
            u_fg = TEXT_WHITE if u_hov else TEXT_MUTED
            u_label = f"v{APP_VERSION}  •  CHECK FOR UPDATES"
        pygame.draw.rect(self.screen, u_bg, update_rect, border_radius=5)
        pygame.draw.rect(self.screen, u_bd, update_rect, 1, border_radius=5)
        u_surf = self.font_xs.render(u_label, True, u_fg)
        self.screen.blit(u_surf, (update_rect.centerx - u_surf.get_width() // 2, update_rect.centery - u_surf.get_height() // 2))

        # Compact keybind legend (centered inside sidebar)
        hint = self.font_xs.render("SPACE Fire  •  R Reset  •  H Info  •  U Update", True, TEXT_MUTED)
        self.screen.blit(hint, (s_x + (SIDEBAR_W - hint.get_width()) // 2, SCREEN_H - 190))

        # Action Buttons (all text centered within button rects)
        fire_rect = pygame.Rect(s_x + 16, SCREEN_H - 166, SIDEBAR_W - 32, 42)
        f_col = (255, 84, 112) if (self.overlay is None and fire_rect.collidepoint(mx, my)) else ACCENT_RED
        pygame.draw.rect(self.screen, f_col, fire_rect, border_radius=6)
        f_txt = self.font_md.render("FIRE LASER [SPACE]", True, TEXT_WHITE)
        self.screen.blit(f_txt, (fire_rect.centerx - f_txt.get_width() // 2, fire_rect.centery - f_txt.get_height() // 2))

        reset_rect = pygame.Rect(s_x + 16, SCREEN_H - 116, SIDEBAR_W - 32, 38)
        r_bg = (34, 44, 62) if (self.overlay is None and reset_rect.collidepoint(mx, my)) else PANEL_BG
        pygame.draw.rect(self.screen, r_bg, reset_rect, border_radius=6)
        pygame.draw.rect(self.screen, GRID_LINE, reset_rect, 1, border_radius=6)
        r_txt = self.font_md.render("RESET LEVEL [R]", True, TEXT_WHITE)
        self.screen.blit(r_txt, (reset_rect.centerx - r_txt.get_width() // 2, reset_rect.centery - r_txt.get_height() // 2))

        can_prev = self.level_idx > 0
        prev_rect = pygame.Rect(s_x + 16, SCREEN_H - 68, 96, 44)
        p_hov = (self.overlay is None) and can_prev and prev_rect.collidepoint(mx, my)
        p_bg = (38, 52, 74) if p_hov else (PANEL_BG if can_prev else (24, 30, 42))
        pygame.draw.rect(self.screen, p_bg, prev_rect, border_radius=6)
        pygame.draw.rect(self.screen, ACCENT_CYAN if p_hov else (GRID_LINE if can_prev else (34, 40, 52)), prev_rect, 1, border_radius=6)
        p_surf = self.font_md.render("<- PREV", True, TEXT_WHITE if can_prev else TEXT_MUTED)
        self.screen.blit(p_surf, (prev_rect.centerx - p_surf.get_width() // 2, prev_rect.centery - p_surf.get_height() // 2))

        next_rect = pygame.Rect(s_x + 120, SCREEN_H - 68, SIDEBAR_W - 136, 44)
        n_col = ((74, 228, 172) if (self.overlay is None and next_rect.collidepoint(mx, my)) else ACCENT_GREEN) if self.level_complete else (30, 38, 50)
        pygame.draw.rect(self.screen, n_col, next_rect, border_radius=6)
        next_txt = "NEXT LEVEL ->" if self.level_complete else "SOLVE TO ADVANCE"
        n_surf = self.font_md.render(next_txt, True, BG_DARK if self.level_complete else TEXT_MUTED)
        self.screen.blit(n_surf, (next_rect.centerx - n_surf.get_width() // 2, next_rect.centery - n_surf.get_height() // 2))

        # 8. Bottom Diagnostic & Oscilloscope Panel
        b_y = GRID_ROWS * TILE_SIZE
        self.draw_telemetry_deck(b_y, s_x, lvl, mx, my)

        # 9. Modal Overlay (Level Briefing or Component Reference)
        if self.overlay is not None:
            self.draw_overlay(mx, my)

        pygame.display.flip()

    def wrap_text(self, font, text, max_w):
        words = text.split(" ")
        lines = []
        cur = ""
        for w in words:
            test = f"{cur} {w}".strip()
            if font.size(test)[0] <= max_w or not cur:
                cur = test
            else:
                lines.append(cur)
                cur = w
        if cur:
            lines.append(cur)
        return lines

    def draw_overlay(self, mx, my):
        dim = pygame.Surface((SCREEN_W, SCREEN_H), pygame.SRCALPHA)
        dim.fill((6, 10, 16, 195))
        self.screen.blit(dim, (0, 0))

        kind, payload = self.overlay
        if kind == "LEVEL":
            lvl = LEVELS[payload]
            new_hw = lvl.get("new_hardware", [])
            card_w = 680
            brief_lines = self.wrap_text(self.font_sm, lvl["briefing"], card_w - 56)

            hw_blocks = []
            for hk in new_hw:
                doc = HARDWARE_DOCS[hk]
                rule_lines = self.wrap_text(self.font_sm, f"{doc['rule']} {doc['physics']}", card_w - 86)
                hw_blocks.append((hk, doc, rule_lines[:2]))

            body_h = 84 + len(brief_lines) * 20
            if hw_blocks:
                body_h += 32 + sum(26 + len(rl) * 18 + 10 for _, _, rl in hw_blocks)
            card_h = min(SCREEN_H - 48, body_h + 74)
            card = pygame.Rect((SCREEN_W - card_w) // 2, (SCREEN_H - card_h) // 2, card_w, card_h)

            pygame.draw.rect(self.screen, (18, 25, 38), card, border_radius=12)
            pygame.draw.rect(self.screen, ACCENT_CYAN, card, 2, border_radius=12)

            # Header badge + Title
            tag = self.font_xs.render(f"EXPERIMENT BRIEFING  •  LEVEL {payload + 1} OF {len(LEVELS)}", True, ACCENT_CYAN)
            self.screen.blit(tag, (card.x + 28, card.y + 20))
            title = self.font_lg.render(lvl["title"], True, TEXT_WHITE)
            self.screen.blit(title, (card.x + 28, card.y + 40))
            pygame.draw.line(self.screen, GRID_LINE, (card.x + 28, card.y + 68), (card.right - 28, card.y + 68), 1)

            y = card.y + 80
            for line in brief_lines:
                self.screen.blit(self.font_sm.render(line, True, TEXT_WHITE), (card.x + 28, y))
                y += 20

            if hw_blocks:
                y += 12
                sec = self.font_xs.render("NEW HARDWARE INTRODUCED IN THIS LEVEL", True, ACCENT_AMBER)
                self.screen.blit(sec, (card.x + 28, y))
                y += 20
                for _, doc, rlines in hw_blocks:
                    bh = 24 + len(rlines) * 18 + 8
                    box = pygame.Rect(card.x + 28, y, card_w - 56, bh)
                    pygame.draw.rect(self.screen, (13, 19, 30), box, border_radius=7)
                    pygame.draw.rect(self.screen, doc["color"], (box.x, box.y, 4, box.h), border_radius=2)
                    pygame.draw.rect(self.screen, (38, 50, 68), box, 1, border_radius=7)
                    self.screen.blit(self.font_md.render(doc["title"], True, doc["color"]), (box.x + 14, box.y + 6))
                    ly = box.y + 26
                    for rl in rlines:
                        self.screen.blit(self.font_sm.render(rl, True, TEXT_MUTED), (box.x + 14, ly))
                        ly += 18
                    y += bh + 8

            btn = pygame.Rect(card.centerx - 135, card.bottom - 54, 270, 38)
            b_col = (84, 206, 255) if btn.collidepoint(mx, my) else ACCENT_CYAN
            pygame.draw.rect(self.screen, b_col, btn, border_radius=7)
            b_txt = self.font_md.render("BEGIN EXPERIMENT  [ENTER]", True, BG_DARK)
            self.screen.blit(b_txt, (btn.centerx - b_txt.get_width() // 2, btn.centery - b_txt.get_height() // 2))

        elif kind == "COMP":
            doc = HARDWARE_DOCS[payload]
            card_w = 560
            phys_lines = self.wrap_text(self.font_sm, doc["physics"], card_w - 56)
            rule_lines = self.wrap_text(self.font_sm, doc["rule"], card_w - 56)
            tip_lines = self.wrap_text(self.font_sm, doc["tip"], card_w - 56)
            card_h = 180 + (len(phys_lines) + len(rule_lines) + len(tip_lines)) * 19 + 82
            card = pygame.Rect((SCREEN_W - card_w) // 2, (SCREEN_H - card_h) // 2, card_w, card_h)

            pygame.draw.rect(self.screen, (18, 25, 38), card, border_radius=12)
            pygame.draw.rect(self.screen, doc["color"], card, 2, border_radius=12)

            # Live procedural component icon preview
            preview = pygame.Surface((TILE_SIZE, TILE_SIZE), pygame.SRCALPHA)
            self.draw_component(preview, 0, 0, payload)
            self.screen.blit(preview, (card.x + 22, card.y + 18))

            self.screen.blit(self.font_xs.render("HARDWARE SPECIFICATION", True, doc["color"]), (card.x + 114, card.y + 28))
            t_lines = self.wrap_text(self.font_lg, doc["title"], card_w - 136)
            ty = card.y + 48
            for tl in t_lines:
                self.screen.blit(self.font_lg.render(tl, True, TEXT_WHITE), (card.x + 114, ty))
                ty += 22
            pygame.draw.line(self.screen, GRID_LINE, (card.x + 28, card.y + 108), (card.right - 28, card.y + 108), 1)

            y = card.y + 120
            for label, col, lines in [
                ("QUANTUM PHYSICS", ACCENT_CYAN, phys_lines),
                ("BENCH MECHANIC", ACCENT_AMBER, rule_lines),
                ("LAB TIP", ACCENT_GREEN, tip_lines),
            ]:
                self.screen.blit(self.font_xs.render(label, True, col), (card.x + 28, y))
                y += 18
                for line in lines:
                    self.screen.blit(self.font_sm.render(line, True, TEXT_WHITE if label != "LAB TIP" else TEXT_MUTED), (card.x + 28, y))
                    y += 19
                y += 12

            btn = pygame.Rect(card.centerx - 105, card.bottom - 52, 210, 36)
            b_col = (50, 68, 94) if btn.collidepoint(mx, my) else PANEL_BG
            pygame.draw.rect(self.screen, b_col, btn, border_radius=7)
            pygame.draw.rect(self.screen, doc["color"], btn, 1, border_radius=7)
            b_txt = self.font_md.render("CLOSE  [CLICK / ESC]", True, TEXT_WHITE)
            self.screen.blit(b_txt, (btn.centerx - b_txt.get_width() // 2, btn.centery - b_txt.get_height() // 2))

        elif kind == "UPDATE":
            snap = self.updater.snapshot()
            state = snap["state"]
            info = snap["release_info"] or {}
            can_install = snap["can_self_install"] and bool(info.get("zip_url"))
            card_w = 580

            notes_raw = info.get("notes") or ""
            notes_lines = []
            for para in notes_raw.splitlines():
                clean_p = para.strip().lstrip("#*-• ").strip()
                if clean_p:
                    notes_lines.extend(self.wrap_text(self.font_sm, clean_p, card_w - 64))
                if len(notes_lines) >= 8:
                    break
            notes_lines = notes_lines[:8]

            card_h = 260 + (len(notes_lines) * 19 + 26 if state == "AVAILABLE" and notes_lines else 0)
            card = pygame.Rect((SCREEN_W - card_w) // 2, (SCREEN_H - card_h) // 2, card_w, card_h)
            self.update_btn_rects = {"card": card}

            accent = ACCENT_GREEN if state == "AVAILABLE" else (ACCENT_RED if state == "ERROR" else ACCENT_CYAN)
            pygame.draw.rect(self.screen, (18, 25, 38), card, border_radius=12)
            pygame.draw.rect(self.screen, accent, card, 2, border_radius=12)

            tag = self.font_xs.render(f"SOFTWARE UPDATE  •  INSTALLED v{APP_VERSION}", True, accent)
            self.screen.blit(tag, (card.x + 28, card.y + 20))

            if state == "AVAILABLE":
                headline = f"1550nm v{info.get('version', '')} is Ready to Install"
            elif state == "CHECKING":
                headline = "Checking GitHub Releases..."
            elif state == "UP_TO_DATE":
                headline = f"1550nm v{APP_VERSION} is Up to Date"
            elif state == "DOWNLOADING":
                headline = f"Downloading 1550nm v{info.get('version', '')}..."
            elif state == "VERIFYING":
                headline = "Verifying Signature & Checksum..."
            elif state == "INSTALLING":
                headline = "Installing Update & Relaunching..."
            elif state == "ERROR":
                headline = "Update Check / Install Notice"
            else:
                headline = "Software Updates"

            self.screen.blit(self.font_lg.render(headline, True, TEXT_WHITE), (card.x + 28, card.y + 42))
            pygame.draw.line(self.screen, GRID_LINE, (card.x + 28, card.y + 72), (card.right - 28, card.y + 72), 1)

            y = card.y + 86
            status_lines = self.wrap_text(self.font_sm, snap["status_text"] or "Querying GitHub Releases...", card_w - 56)
            for sl in status_lines[:3]:
                st_col = ACCENT_RED if state == "ERROR" else (ACCENT_GREEN if state in ("AVAILABLE", "UP_TO_DATE") else TEXT_WHITE)
                self.screen.blit(self.font_sm.render(sl, True, st_col), (card.x + 28, y))
                y += 19

            if state == "AVAILABLE" and notes_lines:
                y += 8
                self.screen.blit(self.font_xs.render("RELEASE NOTES", True, ACCENT_AMBER), (card.x + 28, y))
                y += 18
                box_h = len(notes_lines) * 19 + 14
                nbox = pygame.Rect(card.x + 28, y, card_w - 56, box_h)
                pygame.draw.rect(self.screen, (13, 19, 30), nbox, border_radius=7)
                pygame.draw.rect(self.screen, (38, 50, 68), nbox, 1, border_radius=7)
                ny = nbox.y + 7
                for nl in notes_lines:
                    self.screen.blit(self.font_sm.render(nl, True, TEXT_MUTED), (nbox.x + 12, ny))
                    ny += 19
                y = nbox.bottom + 10

            if state in ("DOWNLOADING", "VERIFYING", "INSTALLING"):
                bar_rect = pygame.Rect(card.x + 28, card.bottom - 98, card_w - 56, 16)
                pygame.draw.rect(self.screen, (13, 19, 30), bar_rect, border_radius=8)
                fill_w = max(8, int(bar_rect.w * snap["progress"]))
                pygame.draw.rect(self.screen, ACCENT_CYAN if state == "DOWNLOADING" else ACCENT_GREEN, (bar_rect.x, bar_rect.y, fill_w, bar_rect.h), border_radius=8)
                pygame.draw.rect(self.screen, (52, 70, 96), bar_rect, 1, border_radius=8)

            # Action buttons row
            btn_y = card.bottom - 56
            if state == "AVAILABLE":
                if can_install:
                    b_inst = pygame.Rect(card.x + 28, btn_y, 210, 38)
                    b_page = pygame.Rect(card.x + 248, btn_y, 174, 38)
                    b_later = pygame.Rect(card.right - 126, btn_y, 98, 38)
                    self.update_btn_rects["install"] = b_inst
                    self.update_btn_rects["open_page"] = b_page
                    self.update_btn_rects["close"] = b_later
                    pygame.draw.rect(self.screen, (74, 228, 172) if b_inst.collidepoint(mx, my) else ACCENT_GREEN, b_inst, border_radius=7)
                    t1 = self.font_md.render("INSTALL & RELAUNCH", True, BG_DARK)
                    self.screen.blit(t1, (b_inst.centerx - t1.get_width() // 2, b_inst.centery - t1.get_height() // 2))
                else:
                    b_page = pygame.Rect(card.x + 28, btn_y, 250, 38)
                    b_later = pygame.Rect(card.right - 140, btn_y, 112, 38)
                    self.update_btn_rects["open_page"] = b_page
                    self.update_btn_rects["close"] = b_later
                pygame.draw.rect(self.screen, (46, 62, 86) if b_page.collidepoint(mx, my) else PANEL_BG, b_page, border_radius=7)
                pygame.draw.rect(self.screen, ACCENT_CYAN, b_page, 1, border_radius=7)
                t2 = self.font_md.render("RELEASE PAGE", True, TEXT_WHITE)
                self.screen.blit(t2, (b_page.centerx - t2.get_width() // 2, b_page.centery - t2.get_height() // 2))
                pygame.draw.rect(self.screen, (46, 56, 72) if b_later.collidepoint(mx, my) else PANEL_BG, b_later, border_radius=7)
                pygame.draw.rect(self.screen, GRID_LINE, b_later, 1, border_radius=7)
                t3 = self.font_md.render("LATER", True, TEXT_MUTED)
                self.screen.blit(t3, (b_later.centerx - t3.get_width() // 2, b_later.centery - t3.get_height() // 2))
            elif state == "DOWNLOADING":
                b_cancel = pygame.Rect(card.centerx - 90, btn_y, 180, 38)
                self.update_btn_rects["cancel"] = b_cancel
                pygame.draw.rect(self.screen, (64, 30, 38) if b_cancel.collidepoint(mx, my) else (48, 22, 28), b_cancel, border_radius=7)
                pygame.draw.rect(self.screen, ACCENT_RED, b_cancel, 1, border_radius=7)
                tc = self.font_md.render("CANCEL DOWNLOAD", True, TEXT_WHITE)
                self.screen.blit(tc, (b_cancel.centerx - tc.get_width() // 2, b_cancel.centery - tc.get_height() // 2))
            elif state in ("VERIFYING", "INSTALLING"):
                wait_lbl = self.font_sm.render("Please wait — do not close the app while installing...", True, TEXT_MUTED)
                self.screen.blit(wait_lbl, (card.centerx - wait_lbl.get_width() // 2, btn_y + 10))
            else:
                b_chk = pygame.Rect(card.x + 28, btn_y, 175, 38)
                b_page = pygame.Rect(card.x + 213, btn_y, 195, 38)
                b_cls = pygame.Rect(card.right - 134, btn_y, 106, 38)
                self.update_btn_rects["recheck"] = b_chk
                self.update_btn_rects["open_page"] = b_page
                self.update_btn_rects["close"] = b_cls
                pygame.draw.rect(self.screen, (42, 58, 82) if b_chk.collidepoint(mx, my) else PANEL_BG, b_chk, border_radius=7)
                pygame.draw.rect(self.screen, ACCENT_CYAN, b_chk, 1, border_radius=7)
                t1 = self.font_md.render("CHECK AGAIN", True, TEXT_WHITE)
                self.screen.blit(t1, (b_chk.centerx - t1.get_width() // 2, b_chk.centery - t1.get_height() // 2))
                pygame.draw.rect(self.screen, (42, 58, 82) if b_page.collidepoint(mx, my) else PANEL_BG, b_page, border_radius=7)
                pygame.draw.rect(self.screen, GRID_LINE, b_page, 1, border_radius=7)
                t2 = self.font_md.render("DOWNLOAD PAGE", True, TEXT_WHITE)
                self.screen.blit(t2, (b_page.centerx - t2.get_width() // 2, b_page.centery - t2.get_height() // 2))
                pygame.draw.rect(self.screen, (46, 56, 72) if b_cls.collidepoint(mx, my) else PANEL_BG, b_cls, border_radius=7)
                pygame.draw.rect(self.screen, GRID_LINE, b_cls, 1, border_radius=7)
                t3 = self.font_md.render("CLOSE", True, TEXT_MUTED)
                self.screen.blit(t3, (b_cls.centerx - t3.get_width() // 2, b_cls.centery - t3.get_height() // 2))

    def run(self):
        while True:
            self.frame += 1
            for event in pygame.event.get():
                if event.type == pygame.QUIT:
                    pygame.quit()
                    sys.exit()
                elif event.type == pygame.MOUSEBUTTONDOWN:
                    self.handle_click(event.pos, event.button)
                elif event.type == pygame.KEYDOWN:
                    if self.overlay is not None:
                        if self.overlay[0] == "UPDATE":
                            st = self.updater.snapshot()["state"]
                            if event.key == pygame.K_ESCAPE and st not in ("DOWNLOADING", "VERIFYING", "INSTALLING"):
                                self.overlay = None
                                self.sfx.play("select")
                            continue
                        if event.key in (pygame.K_ESCAPE, pygame.K_RETURN, pygame.K_SPACE, pygame.K_h):
                            self.overlay = None
                            self.sfx.play("select")
                        continue
                    if event.key == pygame.K_SPACE:
                        self.fire_photons()
                    elif event.key == pygame.K_r:
                        self.sfx.play("select")
                        self.load_level(self.level_idx, show_briefing=False)
                    elif event.key == pygame.K_h:
                        self.overlay = ("LEVEL", self.level_idx)
                        self.sfx.play("select")
                    elif event.key == pygame.K_u:
                        self.sfx.play("select")
                        self.updater.mark_seen()
                        if self.updater.snapshot()["state"] in ("IDLE", "UP_TO_DATE", "ERROR"):
                            self.updater.check_for_updates(silent=False)
                        self.overlay = ("UPDATE", None)
                    elif event.key == pygame.K_m:
                        self.sfx.toggle_mute()
                        if not self.sfx.muted:
                            self.sfx.play("select")
                    elif pygame.K_1 <= event.key <= pygame.K_9:
                        idx = event.key - pygame.K_1
                        if idx < len(LEVELS):
                            self.sfx.play("select")
                            self.load_level(idx)
                    elif event.key == pygame.K_0 and len(LEVELS) >= 10:
                        self.sfx.play("select")
                        self.load_level(9)
                    elif event.key == pygame.K_MINUS and len(LEVELS) >= 11:
                        self.sfx.play("select")
                        self.load_level(10)
                    elif event.key in (pygame.K_LEFTBRACKET, pygame.K_LEFT):
                        self.sfx.play("select")
                        self.load_level(self.level_idx - 1)
                    elif event.key in (pygame.K_RIGHTBRACKET, pygame.K_RIGHT):
                        self.sfx.play("select")
                        self.load_level(self.level_idx + 1)

            if self.sim_active:
                self.tick += 1
                lvl = LEVELS[self.level_idx]
                for p in list(self.photons):
                    if p.alive:
                        msg = p.update(
                            self.grid,
                            lvl["target"],
                            self.tick,
                            self.spawn_fx,
                            self.sfx.play,
                            lvl["mode"],
                        )
                        if msg:
                            self.status_msg = msg
                            self.status_color = ACCENT_RED if msg.startswith("FAILURE") else ACCENT_AMBER
                self.process_distillation()
                self.evaluate_simulation()

            for pt in self.particles:
                pt.update()
            self.particles = [pt for pt in self.particles if pt.life > 0]

            for sw in self.shockwaves:
                sw.update()
            self.shockwaves = [sw for sw in self.shockwaves if sw.life > 0]

            up_snap = self.updater.snapshot()
            if up_snap["ready_to_quit"]:
                pygame.quit()
                sys.exit(0)
            if up_snap["has_unseen_update"] and self.overlay is None and not self.sim_active:
                self.updater.mark_seen()
                self.overlay = ("UPDATE", None)

            self.draw()
            self.clock.tick(FPS)


if __name__ == "__main__":
    Game().run()
