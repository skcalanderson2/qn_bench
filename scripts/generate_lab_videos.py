#!/usr/bin/env python3
"""
QN Bench — Automated 1080p YouTube Lab Guide Video & Neural Voiceover Generator.

Generates Conceptual Lab Guide videos (1920x1080, 30fps H.264 + AAC) with
studio-quality Microsoft Edge Neural TTS narration (`edge-tts`) and synced
YouTube `.srt` closed captions for QN Bench Campaign Levels 1–20.

Usage:
    # Generate Level 1 sample video for review:
    python3 scripts/generate_lab_videos.py --levels 1

    # Generate all 20 lab videos:
    python3 scripts/generate_lab_videos.py --levels 1-20

    # Try a different neural voice (e.g. en-US-AvaNeural, en-US-BrianNeural, en-GB-RyanNeural):
    python3 scripts/generate_lab_videos.py --levels 1 --voice en-US-AvaNeural
"""

from __future__ import annotations

import argparse
import asyncio
import json
import math
import os
import re
import subprocess
import sys
import tempfile
import textwrap
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Optional, Tuple

import edge_tts
from PIL import Image, ImageDraw, ImageFont

# --- Video & Optical Bench Palette (matches src/model.rs) ---
W, H = 1920, 1080
FPS = 30
SCENE_GAP_SEC = 0.45

BG_DARK = (12, 16, 23)
GRID_BG = (20, 26, 36)
GRID_LINE = (34, 44, 60)
HOLE_COL = (14, 18, 26)
PANEL_BG = (22, 30, 44)
CARD_BG = (16, 22, 34)
TEXT_WHITE = (238, 244, 252)
TEXT_MUTED = (148, 166, 192)

CYAN = (56, 189, 248)     # 1550nm Telecom C-Band
RED = (244, 63, 94)       # 780nm Visible Ion / Alert
ORANGE = (251, 146, 60)   # 637nm Diamond NV
AMBER = (251, 191, 36)    # Raman Noise / Delay / Warning
GREEN = (52, 211, 153)    # Success / Memory / Distill
PURPLE = (167, 139, 250)  # QFC / EPR / Teleport
TEAL = (45, 212, 191)     # AO / Atmos / QKD RX
ICE = (125, 211, 252)     # CD Fiber / DCF
PINK = (244, 114, 182)    # WDM Dichroic
SILVER = (200, 218, 240)  # Mirrors


@dataclass
class HardwareCard:
    tag: str
    title: str
    accent: Tuple[int, int, int]
    spec: str
    detail: str


@dataclass
class PipelineStage:
    label: str
    sub: str
    color: Tuple[int, int, int]
    wl_after: int = 1550
    noisy_after: bool = False


@dataclass
class LabScript:
    num: int
    slug: str
    title: str
    subtitle: str
    concept_tag: str
    # Bench layout preview (10x6 grid) for Scene 2 failure animation
    emitters: List[Tuple[int, int, str, int]]  # (gx, gy, dir, wl)
    target: Tuple[int, int, str]               # (gx, gy, label)
    extra_target: Optional[Tuple[int, int, str]] = None
    epr_pos: Optional[Tuple[int, int]] = None
    fixed: List[Tuple[int, int, str]] = field(default_factory=list)
    walls: List[Tuple[int, int]] = field(default_factory=list)
    fail_stop: Tuple[int, int] = (4, 2)
    fail_banner: str = "ABSORBED BY FIBER"
    fail_color: Tuple[int, int, int] = RED
    # Scene 3 hardware cards
    cards: List[HardwareCard] = field(default_factory=list)
    # Scene 4 conceptual pipeline (no exact tile coordinates!)
    pipeline: List[PipelineStage] = field(default_factory=list)
    strategy_bullets: List[str] = field(default_factory=list)
    # Narration for each of the 5 scenes
    narration: List[str] = field(default_factory=list)


def build_lab_scripts() -> List[LabScript]:
    """Conceptual Lab Guide definitions for all 20 QN Bench campaign levels."""
    return [
        LabScript(
            num=1,
            slug="the_wavelength_barrier",
            title="The Wavelength Barrier",
            subtitle="Single-Photon Frequency Conversion & Raman Filtering",
            concept_tag="QUANTUM TRANSDUCTION • 780nm -> 1550nm",
            emitters=[(0, 2, "R", 780)],
            target=(9, 2, "SNSPD 1.5K"),
            fixed=[(4, 2, "FIBER"), (5, 2, "FIBER"), (6, 2, "FIBER")],
            fail_stop=(4, 2),
            fail_banner="780nm PHOTON ABSORBED BY SILICA FIBER!",
            fail_color=RED,
            cards=[
                HardwareCard("PPLN QFC", "Frequency Converter", PURPLE,
                             "780nm -> 1550nm  (+40 Raman Noise)",
                             "Uses Difference Frequency Generation in a nonlinear crystal to shift visible ion photons into the telecom C-band."),
                HardwareCard("VBG FILTER", "Volume Bragg Grating", CYAN,
                             "Strips -50 Raman Noise (clamped at 0)",
                             "Ultra-narrow holographic optical filter that transmits 1550nm quantum light while rejecting broadband pump noise."),
                HardwareCard("SNSPD 1.5K", "Superconducting Detector", GREEN,
                             "Requires: 1550nm  |  Noise <= 10  |  Fid >= 60%",
                             "Cryogenic nanowire detector with >90% efficiency — latches immediately if flooded by unfiltered Raman noise."),
            ],
            pipeline=[
                PipelineStage("ION 780nm", "Visible Qubit", RED, wl_after=780),
                PipelineStage("PPLN QFC", "Shift to 1550nm", PURPLE, wl_after=1550, noisy_after=True),
                PipelineStage("VBG FILTER", "Strip Pump Noise", CYAN, wl_after=1550, noisy_after=False),
                PipelineStage("1550 FIBER", "-10% Fid / Tile", CYAN, wl_after=1550, noisy_after=False),
                PipelineStage("SNSPD 1.5K", "LOCKED >=60%", GREEN, wl_after=1550, noisy_after=False),
            ],
            strategy_bullets=[
                "Convert 780nm to 1550nm BEFORE the photon reaches telecom fiber.",
                "Filter out the QFC's +40 Raman pump noise BEFORE the SNSPD detector.",
                "Use ALIGN [L] to preview the wavelength shift and check for fiber hazards.",
            ],
            narration=[
                "Welcome to QN Bench, Lab 1: The Wavelength Barrier. In this lab, you'll solve the very first hardware bottleneck of every real-world quantum network: getting a trapped-ion qubit safely into standard telecom fiber.",
                "Trapped-ion qubits emit single visible photons at 780 nanometers. That's great for local control, but standard silica telecom fiber has massive optical loss at visible wavelengths. If you fire a 780 nanometer photon straight into the fiber span, it is absorbed immediately before ever reaching the detector.",
                "To cross the wavelength barrier, quantum engineers combine two key optics. First, a PPLN Quantum Frequency Converter shifts 780 nanometers into the low-loss 1550 nanometer telecom C-band — though its bright pump laser injects plus 40 Raman scattering noise. Second, a narrowband Volume Bragg Grating filter strips up to 50 units of Raman noise so our cryogenic SNSPD detector won't latch.",
                "Think carefully about hardware ordering. Your frequency converter must sit upstream of the telecom fiber so the photon enters at 1550 nanometers. And your VBG filter must sit downstream of the converter — anywhere before the SNSPD — so noise drops below 10 while fidelity stays above 60 percent.",
                "Now it's your turn on the optical bench. Toggle the Alignment Laser with L to verify your beam turns cyan before the fiber, press Space to fire, and watch the live oscilloscope telemetry lock on!",
            ],
        ),
        LabScript(
            num=2,
            slug="cryostat_obstacles",
            title="Cryostat Obstacles",
            subtitle="Free-Space Optical Routing with 45° Dielectric Mirrors",
            concept_tag="FREE-SPACE BEAM STEERING • MIRRORS",
            emitters=[(0, 1, "R", 780)],
            target=(9, 4, "SNSPD 1.5K"),
            fixed=[(4, 4, "FIBER"), (5, 4, "FIBER"), (6, 4, "FIBER")],
            walls=[(4, 0), (4, 1), (4, 2), (4, 3), (7, 2), (7, 3)],
            fail_stop=(4, 1),
            fail_banner="PHOTON BLOCKED BY CRYOSTAT RADIATION SHIELD!",
            fail_color=RED,
            cards=[
                HardwareCard("MIRROR (\\)", "Back-Diagonal Mirror", SILVER,
                             "Right -> Down  |  Down -> Right (Zero Loss)",
                             "High-reflectivity (>99.99%) multi-layer dielectric coating steers single photons 90 degrees without decoherence."),
                HardwareCard("MIRROR (/)", "Slash-Diagonal Mirror", SILVER,
                             "Right -> Up  |  Down -> Left (Zero Loss)",
                             "Combine slash and back-diagonal mirrors to build periscopes around vacuum chambers and cryostat walls."),
                HardwareCard("PPLN + VBG", "Transduction Stack", PURPLE,
                             "1550nm + Noise <= 10 before SNSPD",
                             "Mirrors reflect both visible and telecom wavelengths cleanly, so you can convert before or between turns."),
            ],
            pipeline=[
                PipelineStage("ION 780nm", "Upper Row", RED, wl_after=780),
                PipelineStage("QFC + VBG", "1550nm Clean", PURPLE, wl_after=1550),
                PipelineStage("MIRRORS", "Step Around Wall", SILVER, wl_after=1550),
                PipelineStage("1550 FIBER", "Lower Corridor", CYAN, wl_after=1550),
                PipelineStage("SNSPD 1.5K", "LOCKED", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Compare (\\) vs (/) orientations: a (\\) mirror turns a rightward beam downward.",
                "A second (\\) mirror turns a downward beam back to the right into the fiber.",
                "Keep total components at or below par (4) with zero hints for a 3-star rating!",
            ],
            narration=[
                "Welcome to Lab 2: Cryostat Obstacles. Real quantum optical tables are packed with vacuum chambers, radiation shields, and cryogenic baffles that block straight-line paths.",
                "Here, a cryostat shielding wall blocks the emitter's direct line of sight, while the telecom fiber link sits three rows lower. Firing straight ahead simply absorbs your photon against the opaque radiation shield.",
                "To route around obstacles without losing quantum fidelity, we use 45-degree high-reflectivity dielectric mirrors. Pay close attention to orientation: a back-slash mirror reflects a rightward beam downward, and a downward beam back to the right. A forward-slash mirror does the opposite.",
                "Build a clean two-mirror periscope to step your beam down into the lower corridor before the wall, while ensuring your PPLN converter and VBG filter prepare a clean 1550 nanometer photon before the fiber and detector.",
                "Try using the Alignment Laser with L to preview every 90-degree reflection in real time before pressing Space to fire!",
            ],
        ),
        LabScript(
            num=3,
            slug="long_haul_attenuation",
            title="Long-Haul Attenuation",
            subtitle="Overcoming Fiber Loss with a Quantum Memory Buffer",
            concept_tag="QUANTUM REPEATER BUFFER • Q-MEMORY",
            emitters=[(0, 2, "R", 780)],
            target=(9, 2, "SNSPD 1.5K"),
            fixed=[(2, 2, "FIBER"), (3, 2, "FIBER"), (4, 2, "FIBER"), (6, 2, "FIBER"), (7, 2, "FIBER")],
            walls=[(5, 0), (5, 1), (5, 3), (5, 4), (5, 5)],
            fail_stop=(9, 2),
            fail_banner="FIDELITY DROPPED TO 50% (< 60% THRESHOLD)!",
            fail_color=AMBER,
            cards=[
                HardwareCard("1550 FIBER", "Exponential Attenuation", CYAN,
                             "-10% Fidelity per Fiber Tile",
                             "Even in the 1550nm low-loss window, five fiber tiles drop fidelity to 50% — below the 60% threshold."),
                HardwareCard("Q-MEMORY", "Atomic / NV Spin Buffer", GREEN,
                             "Restores Photon Fidelity -> 100%",
                             "Stores the flying photonic qubit in a long-lived matter spin state, acting as a repeater refresh node."),
                HardwareCard("NO-CLONING", "Why Not an Amplifier?", RED,
                             "Classical EDFA Amplifiers Fail",
                             "Because unknown quantum states cannot be cloned, quantum networks use heralding and quantum memories instead."),
            ],
            pipeline=[
                PipelineStage("QFC", "780 -> 1550nm", PURPLE, wl_after=1550, noisy_after=True),
                PipelineStage("3x FIBER", "100% -> 70%", CYAN, wl_after=1550, noisy_after=True),
                PipelineStage("Q-MEMORY", "Refresh -> 100%", GREEN, wl_after=1550, noisy_after=True),
                PipelineStage("2x FIBER", "100% -> 80%", CYAN, wl_after=1550, noisy_after=True),
                PipelineStage("VBG -> RX", "80% LOCKED!", GREEN, wl_after=1550, noisy_after=False),
            ],
            strategy_bullets=[
                "Count fiber tiles: 5 tiles x -10% = 50% fidelity, which fails the 60% minimum.",
                "Place Q-Memory mid-link between the two fiber spans to reset fidelity to 100%.",
                "Don't forget VBG filtering before the SNSPD detector!",
            ],
            narration=[
                "Welcome to Lab 3: Long-Haul Attenuation. Even at 1550 nanometers, optical fiber still attenuates single photons — and unlike classical internet signals, the No-Cloning Theorem forbids simple optical amplifiers.",
                "Across five telecom fiber tiles at minus 10 percent fidelity each, a photon starts at 100 percent and arrives at just 50 percent — falling short of the SNSPD detector's 60 percent fidelity threshold.",
                "Quantum networks overcome distance using Quantum Memories — solid-state diamond NV centers or trapped ions that absorb, buffer, and re-emit a refreshed 100 percent fidelity qubit mid-link.",
                "Plan your link budget: convert to 1550 nanometers before the first fiber span, place the Quantum Memory in the central gap so fidelity resets to 100 percent before the second span, and strip Raman noise with a VBG filter before detection.",
                "Watch the Fidelity Trace oscilloscope in the bottom-left panel as you fire — you'll see the trace climb straight back to 100 percent at the memory node!",
            ],
        ),
        LabScript(
            num=4,
            slug="entanglement_swapping_hom",
            title="Entanglement Swapping (HOM)",
            subtitle="Hong-Ou-Mandel Interference & Picosecond Synchronization",
            concept_tag="BELL-STATE MEASUREMENT • HOM <= 15ps",
            emitters=[(0, 1, "R", 780), (0, 4, "R", 780)],
            target=(8, 2, "BSM 50:50"),
            fixed=[(4, 1, "FIBER"), (5, 1, "FIBER"), (4, 4, "FIBER"), (5, 4, "FIBER")],
            walls=[(3, 2), (4, 2), (5, 2), (6, 2)],
            fail_stop=(8, 2),
            fail_banner="HOM TIMING MISMATCH: 100ps EARLY (> 15ps)!",
            fail_color=AMBER,
            cards=[
                HardwareCard("BSM 50:50", "Bell-State Measurement", AMBER,
                             "Requires 2 Photons within <= 15ps",
                             "Interferes two indistinguishable 1550nm photons on a 50:50 beam splitter to swap entanglement between remote nodes."),
                HardwareCard("DELAY LINE", "Coiled Fiber Spool", AMBER,
                             "Adds +100ps (= Exactly 1 Tile)",
                             "Calibrated fiber loop holds a photon for 20 frames so asymmetric paths arrive simultaneously at 0ps."),
                HardwareCard("HOM DIP", "Coincidence Telemetry", CYAN,
                             "Watch the Bottom-Right Scope",
                             "Shows signed arrival offset in picoseconds and highlights the ±15ps quantum interference window."),
            ],
            pipeline=[
                PipelineStage("ARM A (SHORT)", "QFC + VBG + DELAY", AMBER, wl_after=1550),
                PipelineStage("ARM B (LONG)", "QFC + VBG", CYAN, wl_after=1550),
                PipelineStage("SYNC 0ps", "Matched Travel Time", GREEN, wl_after=1550),
                PipelineStage("BSM 50:50", "HOM INTERFERENCE!", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Turn on ALIGN [L] to read the exact picosecond travel time of Arm A and Arm B.",
                "Place the +100ps Delay Line on the SHORTER arm to equalize arrival time to 0ps.",
                "Use mirrors to converge both arms cleanly onto the 50:50 BSM beam splitter.",
            ],
            narration=[
                "Welcome to Lab 4: Entanglement Swapping via Hong-Ou-Mandel Interference. To entangle two distant quantum nodes that never interacted directly, we interfere their photons on a central 50-50 beam splitter.",
                "For Hong-Ou-Mandel interference to succeed, both 1550 nanometer photons must arrive completely indistinguishable — within 15 picoseconds of each other. On this asymmetric bench, the upper path is one tile shorter, so Photon A arrives 100 picoseconds too early and entanglement swapping fails.",
                "The Optical Delay Line solves path-length asymmetry: a calibrated coiled fiber spool that holds a photon for exactly 100 picoseconds — identical to one tile of travel. Meanwhile, the bottom-right HOM Interference plot shows your exact timing offset.",
                "Route both arms through QFC and VBG optics toward the BSM, compare their picosecond lengths using the Alignment Laser HUD, and insert the Delay Line on the shorter arm so both photons hit the beam splitter at delta-t equals zero.",
                "Fire the dual emitters with Space and watch the marker drop right into the green center of the Hong-Ou-Mandel coincidence dip!",
            ],
        ),
        LabScript(
            num=5,
            slug="the_repeater_node",
            title="The Repeater Node",
            subtitle="Combining Attenuation Recovery with Picosecond Sync",
            concept_tag="QUANTUM REPEATER • MEMORY + DELAY",
            emitters=[(0, 1, "R", 780), (0, 4, "R", 780)],
            target=(9, 3, "BSM 50:50"),
            fixed=[(2, 1, "FIBER"), (3, 1, "FIBER"), (4, 1, "FIBER"), (6, 1, "FIBER"), (7, 1, "FIBER"), (4, 4, "FIBER"), (5, 4, "FIBER")],
            walls=[(1, 2), (2, 2), (3, 2), (4, 2), (5, 2), (6, 2), (7, 2)],
            fail_stop=(9, 3),
            fail_banner="UPPER ARM ATTENUATED & LOWER ARM 100ps EARLY!",
            fail_color=AMBER,
            cards=[
                HardwareCard("UPPER ARM", "5-Tile Long-Haul Fiber", GREEN,
                             "Needs Mid-Link Q-Memory",
                             "Without Q-Memory, five fiber tiles drop the upper photon to 50% fidelity before reaching the BSM."),
                HardwareCard("LOWER ARM", "Shorter Optical Path", AMBER,
                             "Needs +100ps Delay Line",
                             "Check the Alignment Laser: the lower arm is 1 tile shorter than the upper arm."),
                HardwareCard("BSM 50:50", "Joint Constraints", CYAN,
                             "Both >=60% Fid, Noise<=10, dt<=15ps",
                             "Real quantum repeaters must satisfy fidelity, noise, and timing budgets simultaneously."),
            ],
            pipeline=[
                PipelineStage("ARM A", "QFC -> Fiber -> MEM -> VBG", GREEN, wl_after=1550),
                PipelineStage("ARM B", "QFC -> VBG -> DELAY", AMBER, wl_after=1550),
                PipelineStage("MIRRORS", "Converge to Col 9", SILVER, wl_after=1550),
                PipelineStage("BSM 50:50", "0ps SWAPPED!", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Solve each arm's physics independently: Memory on the 5-fiber arm, Delay on the shorter arm.",
                "Remember: Q-Memory does not add a 100ps delay — only the Delay Line adds +100ps.",
                "Confirm SYNC: 0ps OK in the Alignment Laser HUD before firing!",
            ],
            narration=[
                "Welcome to Lab 5: The Repeater Node. Real metropolitan quantum repeaters rarely face just one impairment at a time — they must solve long-haul attenuation and picosecond timing together.",
                "Look at the two arms feeding this BSM. The upper link crosses five telecom fiber tiles, which will drop fidelity below 60 percent, while the lower link is one full tile shorter, creating a 100 picosecond timing mismatch.",
                "Remember the exact role of each component: Quantum Memory refreshes fidelity back to 100 percent without adding delay, while the Optical Delay Line adds 100 picoseconds to synchronize arrival times.",
                "Treat the bench as two clean engineering budgets: buffer the long upper fiber span mid-link with Q-Memory, add the Delay Line to the shorter lower path, and verify both paths match in picoseconds on the Alignment Laser HUD.",
                "Step onto the bench, align both arms, and press Space to lock your first full repeater node!",
            ],
        ),
        LabScript(
            num=6,
            slug="cross_axis_multiplexing",
            title="Cross-Axis Multiplexing",
            subtitle="Sharing a Single Optical Filter Across Perpendicular Beams",
            concept_tag="RESOURCE MULTIPLEXING • SHARED VBG",
            emitters=[(0, 2, "R", 780), (3, 0, "D", 780)],
            target=(8, 5, "BSM 50:50"),
            fixed=[(2, 2, "FIBER"), (4, 2, "FIBER"), (5, 2, "FIBER"), (4, 5, "FIBER"), (5, 5, "FIBER"), (6, 5, "FIBER")],
            walls=[(5, 3), (6, 3), (5, 4), (6, 4)],
            fail_stop=(8, 5),
            fail_banner="ONLY 1 VBG AVAILABLE — UNFILTERED ARM LATCHES BSM!",
            fail_color=AMBER,
            cards=[
                HardwareCard("SHARED VBG", "Scarce Filter Budget (x1)", CYAN,
                             "Filters Both Crossing Beams!",
                             "Perpendicular free-space optical beams can cross through the same VBG tile without interfering."),
                HardwareCard("TIMING DELTA", "Asymmetric Turn Lengths", AMBER,
                             "Compare Arm A vs Arm B (ps)",
                             "Turning around the central baffle makes one arm 1 tile shorter — compensate with Delay."),
                HardwareCard("QFC FIRST", "Convert Before Crossing", PURPLE,
                             "Both Arms Must Be 1550nm",
                             "Each arm must pass through its own QFC before reaching the shared VBG and downstream fibers."),
            ],
            pipeline=[
                PipelineStage("ION A (RIGHT)", "QFC -> Shared VBG", RED, wl_after=1550, noisy_after=True),
                PipelineStage("ION B (DOWN)", "QFC -> Shared VBG", RED, wl_after=1550, noisy_after=True),
                PipelineStage("CROSSING", "Single VBG Strips Both!", CYAN, wl_after=1550),
                PipelineStage("BSM 50:50", "Delay Sync -> LOCKED", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Look for the natural geometric intersection where the horizontal and vertical beams cross.",
                "Ensure BOTH photons are already converted to 1550nm (+40 noise) before crossing the VBG.",
                "Check the Alignment HUD to see which arm needs the +100ps Delay Line.",
            ],
            narration=[
                "Welcome to Lab 6: Cross-Axis Multiplexing. Cryogenic optical hardware is expensive and tightly packed, so quantum benches often share scarce components across multiple channels.",
                "In this lab, you have two perpendicular emitters — one firing right and one firing down — but only a single VBG filter in your inventory. If you filter only one arm, the second arm arrives with 40 Raman noise and latches the BSM.",
                "Here is the key physical insight: perpendicular free-space beams can cross through the exact same Volume Bragg Grating tile without colliding, allowing one filter to clean both photons — provided both have already passed through a QFC.",
                "Spot the tile where your horizontal and vertical beams naturally intersect, place your lone VBG right at that crossing, then route both clean beams around the central shielding and balance the shorter arm with the Delay Line.",
                "Toggle ALIGN with L to verify the crossing and timing delta, then fire to lock the BSM!",
            ],
        ),
        LabScript(
            num=7,
            slug="classical_coexistence_wdm",
            title="Classical Coexistence (WDM)",
            subtitle="Spontaneous Raman Scattering in Lit Telecom Fibers",
            concept_tag="COEXISTENCE NOISE • LIT FIBER",
            emitters=[(0, 2, "R", 780)],
            target=(9, 2, "SNSPD 1.5K"),
            fixed=[(3, 2, "LIT_FIBER"), (4, 2, "LIT_FIBER"), (5, 2, "LIT_FIBER")],
            fail_stop=(9, 2),
            fail_banner="LIT FIBER INJECTED +45 RAMAN NOISE AFTER FILTER!",
            fail_color=AMBER,
            cards=[
                HardwareCard("LIT FIBER", "Classical Data Coexistence", AMBER,
                             "-10% Fid  AND  +15 Raman Noise / Tile",
                             "Bright classical internet traffic sharing the same silica fiber scatters broadband Raman photons into the quantum band."),
                HardwareCard("NOISE BUDGET", "QFC (+40) + 3x Lit (+45)", RED,
                             "85 Total Noise -> Needs 2x VBG",
                             "A single VBG removes -50 noise (leaving 35 > 10). Two VBGs downstream strip up to -100."),
                HardwareCard("FILTER ORDER", "Post-Fiber Placement", CYAN,
                             "Place VBGs AFTER Lit Fiber!",
                             "Filtering before the lit fiber cannot remove Raman noise generated inside the fiber span."),
            ],
            pipeline=[
                PipelineStage("QFC", "+40 Noise", PURPLE, wl_after=1550, noisy_after=True),
                PipelineStage("3x LIT FIBER", "+45 More -> 85 Noise!", AMBER, wl_after=1550, noisy_after=True),
                PipelineStage("2x VBG AFTER", "-100 -> 0 Noise", CYAN, wl_after=1550, noisy_after=False),
                PipelineStage("SNSPD 1.5K", "CLEAN LOCK!", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Add up the total Raman noise: QFC (+40) plus 3 Lit Fiber tiles (+45) = 85 noise.",
                "Placing a VBG before the Lit Fiber leaves +45 noise uncleaned at the detector.",
                "Place your VBG filters AFTER the entire Lit Fiber span!",
            ],
            narration=[
                "Welcome to Lab 7: Classical Coexistence. Deploying dedicated dark fiber for quantum links is costly, so metro quantum networks often share lit fibers carrying live classical internet traffic.",
                "When bright classical lasers travel through silica fiber, spontaneous Raman scattering injects plus 15 noise on every single lit fiber tile. If you filter only right after the QFC, the lit fiber floods your photon with fresh noise downstream and latches the SNSPD.",
                "Do the noise math carefully: QFC adds 40 noise, and three lit fiber tiles add 45 more, totaling 85. Each VBG filter removes 50 units of noise, so you need two VBG filters — and their position matters.",
                "Convert to 1550 nanometers before the fiber, let the photon traverse all three lit fiber tiles, and then place your VBG filters afterward to strip both converter and coexistence noise cleanly down to zero.",
                "Watch the Raman Noise oscilloscope and 8-segment LED meter drop from red to green when you fire!",
            ],
        ),
        LabScript(
            num=8,
            slug="polarization_drift_pmd",
            title="Polarization Drift (PMD)",
            subtitle="Compensating Birefringent Fiber with a Half-Wave Plate",
            concept_tag="BIREFRINGENCE • PMD + HWP",
            emitters=[(0, 1, "R", 780), (0, 4, "R", 780)],
            target=(8, 2, "BSM 50:50"),
            fixed=[(3, 1, "PMD_FIBER"), (4, 1, "FIBER"), (5, 1, "FIBER"), (4, 4, "FIBER"), (5, 4, "FIBER")],
            walls=[(3, 2), (4, 2), (5, 2), (6, 2)],
            fail_stop=(8, 2),
            fail_banner="POLARIZATION MISMATCH ('x' vs '+') BREAKS HOM!",
            fail_color=PURPLE,
            cards=[
                HardwareCard("PMD FIBER", "Birefringent Telecom Fiber", PURPLE,
                             "-10% Fid  AND  Flips Pol '+' <-> 'x'",
                             "Thermal and mechanical stress in buried fiber rotates photon polarization, making two photons distinguishable."),
                HardwareCard("HWP (+/x)", "Half-Wave Plate", AMBER,
                             "Rotates '+' <-> 'x' (Zero Loss)",
                             "Quartz crystal retarder introduces a 180-degree phase shift to restore matching polarization frames."),
                HardwareCard("HOM RULES", "Time AND Polarization", CYAN,
                             "dt <= 15ps  AND  Pol A == Pol B",
                             "Even at 0ps arrival, orthogonal polarizations cannot undergo Hong-Ou-Mandel interference."),
            ],
            pipeline=[
                PipelineStage("ARM A", "QFC -> PMD ('x') -> HWP ('+')", PURPLE, wl_after=1550),
                PipelineStage("DELAY", "Match Arm B Timing", AMBER, wl_after=1550),
                PipelineStage("ARM B", "QFC -> VBG -> Fiber ('+')", CYAN, wl_after=1550),
                PipelineStage("BSM 50:50", "0ps + Pol Match!", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Look at the Alignment HUD: it shows each beam's final polarization ('+' or 'x') and flags POL! mismatches.",
                "Insert a Half-Wave Plate (HWP) on the drifted arm to rotate 'x' back to '+'.",
                "Don't forget to also balance arrival times with the Delay Line!",
            ],
            narration=[
                "Welcome to Lab 8: Polarization Drift, also known as Polarization Mode Dispersion or PMD. Buried telecom fibers experience temperature swings and mechanical stress that make the glass slightly birefringent.",
                "As a photon passes through PMD fiber, its polarization rotates from rectilinear plus to diagonal x. Even if two photons arrive at a 50-50 beam splitter at the exact same picosecond, mismatched polarizations make them distinguishable — completely destroying Hong-Ou-Mandel interference.",
                "To compensate polarization drift without optical loss, labs use a Half-Wave Plate — a precision quartz crystal that rotates polarization between plus and x. You can check your polarization parity live in the Alignment Laser HUD.",
                "Insert the Half-Wave Plate on the drifted arm to realign both polarizations, keep your timing synchronized with the Delay Line on the shorter arm, and converge both beams cleanly into the BSM.",
                "Press Space and watch both the timing delta and polarization lock cleanly into entanglement swapping!",
            ],
        ),
        LabScript(
            num=9,
            slug="entanglement_distillation",
            title="Entanglement Distillation",
            subtitle="Purifying Noisy Pairs to Meet Fault-Tolerant QPU Thresholds",
            concept_tag="PURIFICATION • DISTILL -> QPU >=90%",
            emitters=[(0, 1, "R", 780), (0, 4, "R", 780)],
            target=(9, 2, "QPU >=90%"),
            fixed=[(4, 1, "FIBER"), (5, 1, "FIBER"), (4, 4, "FIBER"), (5, 4, "FIBER"), (6, 2, "DISTILL")],
            walls=[(7, 0), (7, 1), (7, 3), (7, 4), (7, 5)],
            fail_stop=(9, 2),
            fail_banner="RAW FIBER FIDELITY 80% < 90% QPU THRESHOLD!",
            fail_color=PURPLE,
            cards=[
                HardwareCard("QPU >=90%", "Fault-Tolerant Processor", PURPLE,
                             "Requires 1550nm, Noise<=10, Fid>=90%",
                             "Error-corrected quantum processors demand ultra-high link fidelity that raw telecom fibers cannot reach alone."),
                HardwareCard("DISTILL", "Entanglement Purifier", GREEN,
                             "2x Photons (<=15ps) -> 1x 95% Photon",
                             "Consumes two noisy >=60% photons arriving together and emits one purified 95% photon rightward."),
                HardwareCard("TIMING SYNC", "Same <=15ps Coincidence", AMBER,
                             "Sync Both Arms into DISTILL",
                             "Just like a BSM, distillation requires both incoming arms to arrive within 15 picoseconds."),
            ],
            pipeline=[
                PipelineStage("ARM A (80%)", "QFC + VBG + Delay", CYAN, wl_after=1550),
                PipelineStage("ARM B (80%)", "QFC + VBG", CYAN, wl_after=1550),
                PipelineStage("DISTILL", "Purify -> 95% Qubit P", GREEN, wl_after=1550),
                PipelineStage("QPU >=90%", "FAULT-TOLERANT LOCK!", PURPLE, wl_after=1550),
            ],
            strategy_bullets=[
                "Notice the walls force you to route into the DISTILL node rather than directly to the QPU.",
                "Align both 80% arms to arrive at DISTILL within <=15ps using the Delay Line.",
                "DISTILL automatically emits the purified 95% green photon straight into the QPU!",
            ],
            narration=[
                "Welcome to Lab 9: Entanglement Distillation. Connecting distributed Quantum Processing Units requires fault-tolerant fidelities of 90 percent or higher — far stricter than a standard detector.",
                "Even crossing just two telecom fiber tiles drops each photon to 80 percent fidelity, below the QPU's 90 percent threshold. You simply cannot satisfy a fault-tolerant QPU with a single raw fiber link.",
                "Entanglement Distillation solves this by sacrificing quantity for quality: the DISTILL node consumes two lower-fidelity photons arriving within 15 picoseconds and outputs a single purified 95 percent fidelity photon straight toward the QPU.",
                "Convert, filter, and route both arms into the distillation node with equalized picosecond travel times using your Delay Line. Once both photons fuse, watch the green purified photon lock the QPU.",
                "Toggle ALIGN with L to check the Hop-2 preview beam from DISTILL, then fire with Space!",
            ],
        ),
        LabScript(
            num=10,
            slug="hybrid_quantum_network",
            title="Hybrid Quantum Network",
            subtitle="Linking a 780nm Trapped Ion with a 637nm Diamond NV Center",
            concept_tag="HETEROGENEOUS HARDWARE • 780nm + 637nm",
            emitters=[(0, 1, "R", 780), (0, 4, "R", 637)],
            target=(9, 2, "BSM 50:50"),
            fixed=[(4, 1, "FIBER"), (5, 1, "FIBER"), (6, 1, "FIBER"), (5, 4, "FIBER"), (6, 4, "FIBER")],
            walls=[(3, 2), (4, 2), (5, 2), (6, 2), (7, 2)],
            fail_stop=(9, 2),
            fail_banner="637nm NV QFC ADDS +70 NOISE — 1 VBG LEAVES 20!",
            fail_color=ORANGE,
            cards=[
                HardwareCard("NV 637nm", "Diamond NV Emitter", ORANGE,
                             "Visible Red-Orange 637nm Photon",
                             "Solid-state diamond defect qubits emit at 637nm — a much larger frequency gap to 1550nm than 780nm."),
                HardwareCard("QFC NOISE", "Wavelength-Dependent Pump", PURPLE,
                             "780->1550 (+40)  |  637->1550 (+70)",
                             "Because 637nm transduction adds +70 Raman noise, a single VBG (-50) leaves 20 noise (> 10 limit)!"),
                HardwareCard("FILTER SPLIT", "Allocate 3x VBG Wisely", CYAN,
                             "1x VBG on Ion  •  2x VBG on NV",
                             "Once both are shifted to clean 1550nm, the BSM cannot tell which hardware platform created them!"),
            ],
            pipeline=[
                PipelineStage("ION 780nm", "QFC (+40) -> 1x VBG -> Delay", RED, wl_after=1550),
                PipelineStage("NV 637nm", "QFC (+70) -> 2x VBG", ORANGE, wl_after=1550),
                PipelineStage("1550 C-BAND", "Indistinguishable!", CYAN, wl_after=1550),
                PipelineStage("BSM 50:50", "HYBRID ENTANGLEMENT!", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Give the 780nm Ion arm 1 VBG filter (+40 - 50 = 0) and the 637nm NV arm 2 VBG filters (+70 - 100 = 0).",
                "Check the Alignment HUD for path length: balance the shorter arm with the Delay Line.",
                "Once both reach 1550nm with noise <= 10 at 0ps, hybrid entanglement succeeds!",
            ],
            narration=[
                "Welcome to Lab 10: Hybrid Quantum Network. The future quantum internet won't run on one hardware type — it will connect trapped ions, neutral atoms, superconducting circuits, and diamond NV centers together.",
                "Here, a 780 nanometer Trapped Ion meets a 637 nanometer Diamond NV center. Shifting 637 nanometers all the way to 1550 requires a stronger frequency conversion step that injects plus 70 Raman noise. A single VBG filter only removes 50 — leaving 20 noise, which still latches the BSM.",
                "That's why your inventory gives you three VBG filters: allocate one to the 780 nanometer Ion arm and two in series to the 637 nanometer NV arm. Once both photons are transduced and cleaned, they become completely indistinguishable in the 1550 nanometer C-band.",
                "Check your path lengths with the Alignment Laser, add the Delay Line to the shorter arm, and converge both beams into the 50-50 beam splitter.",
                "Fire with Space to erase the hardware difference and entangle an ion with a diamond!",
            ],
        ),
        LabScript(
            num=11,
            slug="satellite_downlink_atmos",
            title="Satellite Downlink (ATMOS)",
            subtitle="Stabilizing Atmospheric Turbulence with Adaptive Optics",
            concept_tag="FREE-SPACE QUANTUM LINK • ATMOS + AO",
            emitters=[(0, 1, "R", 780)],
            target=(9, 4, "SNSPD 1.5K"),
            fixed=[(4, 1, "ATMOS"), (5, 1, "ATMOS"), (6, 1, "ATMOS"), (8, 4, "FIBER")],
            walls=[(3, 2), (4, 2), (5, 2), (6, 2), (3, 3), (3, 4), (3, 5)],
            fail_stop=(6, 1),
            fail_banner="TURBULENCE LOST -75% FIDELITY (-25%/TILE)!",
            fail_color=TEAL,
            cards=[
                HardwareCard("ATMOS", "Atmospheric Turbulence", TEAL,
                             "-25% Fidelity / Tile (Unlocked)",
                             "Air-density eddies distort optical wavefronts across satellite downlinks, destroying link fidelity."),
                HardwareCard("ADAPT OPT", "Deformable Mirror (AO)", TEAL,
                             "Cuts ATMOS Loss: -25% -> -5% / Tile",
                             "Closed-loop wavefront sensing pre-compensates atmospheric phase distortion in real time."),
                HardwareCard("UPSTREAM", "Lock Before Turbulence", AMBER,
                             "Place AO BEFORE ATMOS Tiles!",
                             "With AO locked, 3 ATMOS tiles + 1 ground fiber lose only 25% total (arriving at 75% >= 60%)."),
            ],
            pipeline=[
                PipelineStage("QFC + VBG", "1550nm Clean", PURPLE, wl_after=1550),
                PipelineStage("ADAPT OPT", "Wavefront Locked", TEAL, wl_after=1550),
                PipelineStage("3x ATMOS", "Only -15% Total!", TEAL, wl_after=1550),
                PipelineStage("GROUND FIBER", "75% -> SNSPD LOCK!", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Place Adaptive Optics (AO) upstream of the first ATMOS tile so the wavefront is locked.",
                "Convert to 1550nm before coupling into the ground station fiber at row 4.",
                "Use two (\\) mirrors to steer down from the atmospheric corridor into the ground link.",
            ],
            narration=[
                "Welcome to Lab 11: Satellite Downlink. Beyond a few hundred kilometers, fiber loss becomes insurmountable, so intercontinental quantum links beam photons from satellites through free space.",
                "The challenge is Earth's turbulent atmosphere. Each uncompensated ATMOS tile strips 25 percent fidelity — across three tiles plus a ground fiber, your photon drops to just 15 percent and fails completely.",
                "Ground telescopes overcome turbulence with Adaptive Optics: a real-time wavefront sensor and deformable mirror that locks the optical wavefront, slashing atmospheric loss from 25 percent down to just 5 percent per tile.",
                "Place Adaptive Optics before the photon enters the ATMOS corridor, steer the stabilized beam down into the ground station's 1550 nanometer fiber with mirrors, and make sure QFC and VBG prepare a clean telecom photon.",
                "Fire with Space and watch the teal Adaptive Optics lock guide your satellite photon safely to the ground detector!",
            ],
        ),
        LabScript(
            num=12,
            slug="bb84_qkd_aes256",
            title="BB84 QKD & AES-256",
            subtitle="Prepare-and-Measure Quantum Key Distribution & Live Encryption",
            concept_tag="QUANTUM CRYPTOGRAPHY • BB84 + FIPS-197 AES-256",
            emitters=[(0, 2, "R", 780)],
            target=(9, 2, "BOB RX"),
            fixed=[(3, 2, "PMD_FIBER"), (4, 2, "FIBER"), (5, 2, "FIBER")],
            fail_stop=(9, 2),
            fail_banner="PMD ROTATED BASES -> QBER ~50% -> AES KEY ABORTED!",
            fail_color=RED,
            cards=[
                HardwareCard("QKD TX / RX", "BB84 Polarized Burst", TEAL,
                             "36 Qubits in Random '+' / 'x' Bases",
                             "Alice sends random bits in random bases; Bob measures randomly and sifts matching rounds."),
                HardwareCard("QBER <= 11%", "Security Threshold", RED,
                             "PMD Causes ~50% Error Rate!",
                             "If birefringent PMD rotates Alice's bases, Bob gets garbage on matching rounds and aborts the key."),
                HardwareCard("AES-256", "SHA-256 -> Live Cipher", GREEN,
                             "Press [K] to Inspect & Encrypt!",
                             "When QBER <= 11%, both sides hash sifted bits into a real FIPS-197 AES-256-CTR encryption key."),
            ],
            pipeline=[
                PipelineStage("QKD TX", "36 Polarized Qubits", RED, wl_after=780),
                PipelineStage("QFC + VBG", "Clean 1550nm Stream", PURPLE, wl_after=1550),
                PipelineStage("PMD + HWP", "Align Basis Frame!", AMBER, wl_after=1550),
                PipelineStage("BOB RX", "QBER <=11% -> AES-256!", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Prepare clean 1550nm photons before the fiber span with QFC and VBG.",
                "Insert a Half-Wave Plate (HWP) to undo the PMD fiber's polarization basis rotation.",
                "After locking, press [K] to inspect the real 256-bit key and encrypt custom messages!",
            ],
            narration=[
                "Welcome to Lab 12: BB84 Quantum Key Distribution and AES-256. Now we turn quantum physics into real cryptography.",
                "Alice fires a burst of 36 single photons encoded in random plus and x polarization bases. Bob measures in random bases and keeps only matching rounds. Without correcting the PMD fiber's polarization drift, Bob's basis frame is rotated — spiking the Quantum Bit Error Rate near 50 percent and forcing the protocol to abort key generation.",
                "Adding a Half-Wave Plate aligns Bob's polarization reference frame with Alice's, dropping QBER well below the 11 percent security threshold. Once sifted, both ends run SHA-256 privacy amplification to derive an identical 256-bit FIPS-197 AES key.",
                "Convert and filter the stream before the fiber, add your HWP to compensate PMD, and watch the live Basis Sifting stream and QBER chart lock in a real 256-bit key.",
                "Once locked, click Inspect QKD and AES-256 — or press K — to type your own secret message and watch Bob decrypt it!",
            ],
        ),
        LabScript(
            num=13,
            slug="entangled_qkd_e91",
            title="Entangled QKD (E91)",
            subtitle="Bell-Pair Distribution & CHSH Inequality Verification",
            concept_tag="BELL VIOLATION • E91 CHSH S > 2.0",
            emitters=[(4, 2, "L", 780), (4, 2, "R", 780)],
            epr_pos=(4, 2),
            target=(9, 2, "BOB RX"),
            extra_target=(0, 2, "ALICE RX"),
            fixed=[(1, 2, "FIBER"), (7, 2, "PMD_FIBER")],
            fail_stop=(7, 2),
            fail_banner="BOTH EPR ARMS NEED 1550nm + BOB NEEDS HWP ALIGNMENT!",
            fail_color=PURPLE,
            cards=[
                HardwareCard("EPR SOURCE", "Entangled Bell Pairs", PURPLE,
                             "Emits Left -> Alice & Right -> Bob",
                             "Generates correlated |Phi+> photon pairs simultaneously — neither party needs to trust the source!"),
                HardwareCard("CHSH S > 2", "Bell's Inequality Test", GREEN,
                             "Classical <= 2.0  |  Quantum ~2.83",
                             "Violating Bell's CHSH bound proves genuine quantum entanglement and guarantees no hidden eavesdropper."),
                HardwareCard("DUAL ARMS", "Convert, Filter & Align", CYAN,
                             "Alice: QFC+VBG  •  Bob: QFC+VBG+HWP",
                             "Both arms start at 780nm; Bob's arm also crosses birefringent PMD fiber."),
            ],
            pipeline=[
                PipelineStage("ALICE ARM", "QFC + VBG -> Fiber -> ALICE", CYAN, wl_after=1550),
                PipelineStage("EPR SOURCE", "Entangled |Phi+> Pairs", PURPLE, wl_after=780),
                PipelineStage("BOB ARM", "QFC + VBG -> PMD -> HWP -> BOB", TEAL, wl_after=1550),
                PipelineStage("E91 VERIFY", "S > 2.0 -> AES-256!", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Remember both Left and Right EPR photons start at 780nm — convert & filter BOTH arms.",
                "Place an HWP on Bob's arm to compensate the PMD fiber before BOB RX.",
                "Watch the QBER / Bell CHSH readout verify S > 2.00 and lock identical AES-256 keys!",
            ],
            narration=[
                "Welcome to Lab 13: Entangled QKD, based on Artur Ekert's 1991 protocol. Instead of Alice preparing qubits, a central EPR source fires correlated Bell-state photon pairs simultaneously to both Alice and Bob.",
                "Both outgoing photons begin at 780 nanometers, so each arm must be converted to 1550 nanometers and filtered before entering fiber. In addition, Bob's arm crosses a birefringent PMD fiber that twists his polarization frame.",
                "What makes E91 extraordinary is Bell's inequality: Alice and Bob compute the CHSH correlation score S. Any classical or tampered channel stays at or below 2.0, while genuine quantum entanglement reaches roughly 2.83.",
                "Build clean transduction and filtering on both left and right arms, add a Half-Wave Plate to align Bob's PMD drift, and verify that both receivers derive the exact same 256-bit AES key.",
                "Press Space to stream 36 entangled Bell pairs and inspect the live CHSH score!",
            ],
        ),
        LabScript(
            num=14,
            slug="eavesdropper_aes_link",
            title="Eavesdropper & AES Link",
            subtitle="Catching an Intercept-Resend Tap & Rerouting Securely",
            concept_tag="NO-CLONING SECURITY • EVE TAP BYPASS",
            emitters=[(4, 2, "L", 780), (4, 2, "R", 780)],
            epr_pos=(4, 2),
            target=(9, 2, "BOB RX"),
            extra_target=(0, 2, "ALICE RX"),
            fixed=[(1, 2, "FIBER"), (7, 2, "EVE"), (8, 2, "FIBER"), (7, 4, "FIBER"), (8, 4, "FIBER")],
            walls=[(7, 1), (8, 1), (7, 3), (8, 3)],
            fail_stop=(7, 2),
            fail_banner="EVE INTERCEPTED PHOTONS -> QBER ~25% -> KEY ABORTED!",
            fail_color=RED,
            cards=[
                HardwareCard("EVE TAP", "Intercept-Resend Attack", RED,
                             "Spikes QBER ~25%  &  CHSH S <= 2.0",
                             "Because Eve must guess '+' or 'x' when measuring, she irreversibly collapses the quantum state."),
                HardwareCard("NO FILTERING", "Cannot Clean Eve Out", AMBER,
                             "Optical Filters Won't Stop Eve",
                             "Once Eve measures a qubit, entanglement is broken — the only fix is physically bypassing the tap."),
                HardwareCard("DARK BYPASS", "Lower Metro Corridor", SILVER,
                             "Route Around Eve via 45° Mirrors",
                             "Drop Bob's beam down before Eve's splice, cross the clean lower fiber, and turn up to BOB RX."),
            ],
            pipeline=[
                PipelineStage("ALICE ARM", "QFC + VBG -> ALICE RX", CYAN, wl_after=1550),
                PipelineStage("BOB QFC", "Convert Before Turn", PURPLE, wl_after=1550, noisy_after=True),
                PipelineStage("MIRROR BYPASS", "Drop Below EVE Tap!", SILVER, wl_after=1550),
                PipelineStage("BOB RX", "CHSH S>2.0 -> AES-256!", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Try firing straight through Eve once to watch QBER spike to ~25% and abort the key!",
                "You cannot filter out an eavesdropper — steer Bob's beam downward BEFORE the Eve tile.",
                "Cross the clean lower fiber corridor and reflect back up into BOB RX.",
            ],
            narration=[
                "Welcome to Lab 14: Eavesdropper and AES Link. Here you'll see why quantum cryptography catches wiretappers using the laws of physics.",
                "An adversary named Eve has spliced an intercept-resend tap onto Bob's direct fiber. To steal bits, Eve must measure each passing photon in a guessed basis — irreversibly collapsing entanglement, spiking QBER to roughly 25 percent, and crashing Bell CHSH below 2.0. The AES engine detects her instantly and discards the key.",
                "No optical filter can undo an eavesdropper's measurement. Instead, you must physically route Bob's entangled beam around the compromised span using 45-degree mirrors and the clean dark fiber corridor below.",
                "Convert and filter Alice's arm on the left, then convert Bob's arm, drop it down before Eve's tap, strip its Raman noise, cross the untampered lower fiber, and reflect up into Bob's receiver.",
                "Try firing through Eve first to see the quantum alarm trip — then route around her to lock a secure AES-256 channel!",
            ],
        ),
        LabScript(
            num=15,
            slug="multi_hop_repeater_chain",
            title="Multi-Hop Repeater Chain",
            subtitle="Heralded Quantum Repeater Feedforward & Dual-Hop Architecture",
            concept_tag="MULTI-HOP NETWORK • REPEATER -> QPU",
            emitters=[(0, 0, "R", 780), (0, 5, "R", 637)],
            target=(9, 0, "QPU >=90%"),
            fixed=[(3, 0, "FIBER"), (4, 0, "PMD_FIBER"), (5, 2, "FIBER"), (3, 5, "FIBER"), (5, 3, "REPEATER"), (6, 3, "LIT_FIBER"), (7, 3, "LIT_FIBER")],
            walls=[(9, 3), (9, 2), (7, 1)],
            fail_stop=(5, 3),
            fail_banner="HOP-1 NEEDS 0ps + POL MATCH; HOP-2 NEEDS VBG + MEMORY!",
            fail_color=CYAN,
            cards=[
                HardwareCard("REPEATER", "Heralded Hop-1 BSM", CYAN,
                             "dt<=15ps & Pol Match -> Emits 100% H",
                             "Performs a Bell swap on Hop-1 photons and uses classical feedforward to launch a fresh 1550nm qubit onto Hop-2."),
                HardwareCard("HOP-1 ARMS", "Hybrid Ion + NV + PMD", ORANGE,
                             "780nm (+HWP)  •  637nm (2x VBG + Delay)",
                             "Both Hop-1 arms must reach the Repeater clean, time-synced, and with matching polarization."),
                HardwareCard("HOP-2 LINK", "Lit Fiber -> QPU >=90%", GREEN,
                             "Strip +30 Noise & Refresh to 100%",
                             "Two Lit Fibers drop Hop-2 to 80% fidelity with +30 noise — add VBG and Q-Memory before QPU!"),
            ],
            pipeline=[
                PipelineStage("HOP-1 HYBRID", "Ion (HWP) + NV (2xVBG, Delay)", ORANGE, wl_after=1550),
                PipelineStage("REPEATER", "Heralded 100% Qubit H", CYAN, wl_after=1550),
                PipelineStage("2x LIT FIBER", "80% Fid, +30 Noise", AMBER, wl_after=1550, noisy_after=True),
                PipelineStage("VBG + MEMORY", "100% -> QPU LOCK!", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Break the puzzle into two independent hops: Hop-1 into REPEATER, then Hop-2 into QPU.",
                "On Hop-1: fix PMD with HWP on the Ion arm, give the 637nm NV arm 2 VBGs, and sync timing with Delay.",
                "On Hop-2: steer around the walls, filter the Lit-Fiber noise with VBG, and restore 100% fidelity with Q-Memory!",
            ],
            narration=[
                "Welcome to Lab 15: Multi-Hop Repeater Chain. Scaling quantum networks across continents means chaining multiple heralded repeater hops end to end.",
                "In Hop 1, a hybrid 780 nanometer Ion arm and a 637 nanometer Diamond NV arm converge on a central Repeater node. If either arm has a timing mismatch, polarization mismatch from PMD, or leftover Raman noise, the Hop-1 Bell swap fails and Hop 2 never fires.",
                "When Hop 1 succeeds, the Repeater sends a classical feedforward herald and releases a fresh 100 percent fidelity 1550 nanometer qubit rightward onto Hop 2. That second hop crosses two lit fibers, adding 30 noise and dropping fidelity to 80 percent — too low for the 90 percent QPU.",
                "Solve it one hop at a time: clean, polar-align, and time-sync both hybrid arms into the Repeater, then steer the heralded Hop-2 qubit through a VBG filter and Quantum Memory to hit the QPU at 100 percent fidelity.",
                "Toggle ALIGN with L to preview both Hop-1 timing and the teal Hop-2 beam, then press Space!",
            ],
        ),
        LabScript(
            num=16,
            slug="satellite_e91_qkd",
            title="Satellite E91 QKD",
            subtitle="Act I Grand Finale: Orbital EPR, Adaptive Optics & Eve Bypass",
            concept_tag="ACT I FINALE • ORBITAL E91 AES-256",
            emitters=[(4, 1, "L", 780), (4, 1, "R", 780)],
            epr_pos=(4, 1),
            target=(9, 5, "BOB RX"),
            extra_target=(1, 5, "ALICE RX"),
            fixed=[(1, 2, "ATMOS"), (1, 3, "ATMOS"), (6, 1, "ATMOS"), (7, 1, "ATMOS"), (9, 3, "EVE"), (8, 2, "LIT_FIBER"), (6, 3, "PMD_FIBER"), (7, 5, "FIBER")],
            walls=[(3, 2), (3, 3), (3, 4), (3, 5), (7, 3), (8, 3), (7, 4), (8, 4)],
            fail_stop=(9, 3),
            fail_banner="UNLOCKED ATMOS + EVE TAP + METRO PMD & LIT FIBER!",
            fail_color=TEAL,
            cards=[
                HardwareCard("DUAL AO", "Stabilize Both Downlinks", TEAL,
                             "AO Upstream of BOTH ATMOS Spans",
                             "Both Alice's and Bob's orbital paths cross turbulent ATMOS tiles and need Adaptive Optics."),
                HardwareCard("EVE BYPASS", "Turn Left Before Tap", RED,
                             "Avoid EVE via Lit + PMD Metro Link",
                             "Bypassing Eve sends Bob's beam through a Lit Fiber (+15 noise) and PMD Fiber (basis flip)."),
                HardwareCard("POST-CLEAN", "VBG After Lit + HWP After PMD", PURPLE,
                             "Preserve CHSH S > 2.0 -> AES-256",
                             "Filter Bob's Raman noise AFTER the Lit Fiber and restore his basis frame with an HWP."),
            ],
            pipeline=[
                PipelineStage("ORBITAL EPR", "Dual 780nm Bell Pairs", PURPLE, wl_after=780),
                PipelineStage("AO + ATMOS", "Wavefront-Locked Downlinks", TEAL, wl_after=1550),
                PipelineStage("METRO BYPASS", "Dodge Eve -> Lit+VBG -> PMD+HWP", AMBER, wl_after=1550),
                PipelineStage("ALICE & BOB", "CHSH S>2.0 -> AES-256!", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Place AO before ATMOS on BOTH Alice's and Bob's arms, and convert both to 1550nm.",
                "On Bob's ground station, detour left BEFORE Eve through the metro Lit & PMD fibers.",
                "Place Bob's VBG AFTER the Lit Fiber and HWP after the PMD Fiber to lock AES-256!",
            ],
            narration=[
                "Welcome to Lab 16: Satellite E91 QKD — the Act One Grand Finale. Everything you've mastered so far comes together in an orbital intercontinental cryptography link.",
                "An EPR satellite beams entangled photon pairs through turbulent atmosphere to two distant ground stations. On Bob's ground link, an eavesdropper is tapping the direct route, forcing you to detour through a noisy lit and birefringent metro fiber.",
                "Plan both arms carefully: lock Adaptive Optics before the ATMOS tiles on both Alice's and Bob's paths, and convert both to 1550 nanometers. On Bob's bypass, save your VBG filter until after the lit fiber, and use your Half-Wave Plate to cancel PMD basis drift.",
                "When both ground stations receive clean, aligned entangled pairs without touching Eve, Bell CHSH climbs above 2.0 and Alice and Bob lock a live intercontinental AES-256 key.",
                "Take your time, check both beams with the Alignment Laser, and fire to complete Act One!",
            ],
        ),
        LabScript(
            num=17,
            slug="chromatic_dispersion",
            title="Chromatic Dispersion",
            subtitle="Act II: Recompressing Broadened Single-Photon Wavepackets",
            concept_tag="QUANTUM INTERNET ACT II • CD FIBER + DCF",
            emitters=[(0, 2, "R", 780)],
            target=(9, 2, "SNSPD 1.5K"),
            fixed=[(3, 2, "CD_FIBER"), (4, 2, "CD_FIBER"), (5, 2, "CD_FIBER"), (6, 2, "LIT_FIBER")],
            fail_stop=(9, 2),
            fail_banner="PHOTON PULSE BROADENED BY CHROMATIC DISPERSION!",
            fail_color=ICE,
            cards=[
                HardwareCard("CD FIBER", "Dispersive Metro Fiber", ICE,
                             "-10% Fid  AND  Adds Dispersion (CD)",
                             "Different frequency components of a short single-photon pulse travel at different group velocities, stretching the pulse."),
                HardwareCard("DCF COMP", "Dispersion Compensator", ICE,
                             "Recompresses Pulse (CD -> 0)",
                             "High-negative-dispersion fiber spool or chirped Bragg grating undoes accumulated chromatic dispersion."),
                HardwareCard("ORDERING", "Both DCF & VBG Downstream", AMBER,
                             "After CD Fiber AND After Lit Fiber",
                             "Placing DCF before CD Fiber or VBG before Lit Fiber leaves downstream impairments uncorrected!"),
            ],
            pipeline=[
                PipelineStage("QFC", "780 -> 1550nm (+40N)", PURPLE, wl_after=1550, noisy_after=True),
                PipelineStage("3x CD + LIT", "CD=3, Noise=55, Fid=60%", ICE, wl_after=1550, noisy_after=True),
                PipelineStage("VBG + DCF", "Noise=5, CD=0!", CYAN, wl_after=1550),
                PipelineStage("SNSPD 1.5K", "TRANSFORM-LIMITED LOCK!", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Convert to 1550nm before the dispersive fiber span using PPLN QFC.",
                "Notice the single VBG (-50) reduces QFC (+40) + Lit (+15) = 55 down to 5 (<= 10) ONLY if placed AFTER Lit Fiber!",
                "Place DCF AFTER all CD Fiber tiles to recompress the wavepacket before SNSPD.",
            ],
            narration=[
                "Welcome to Quantum Internet Act Two, Lab 17: Chromatic Dispersion. Real single-photon wavepackets have finite spectral bandwidth, and silica glass has a wavelength-dependent refractive index.",
                "As a short 1550 nanometer photon travels through dispersive CD fiber, its slightly bluer and redder spectral components drift apart in time — smearing the wavepacket until detectors and interferometers reject it.",
                "Telecom engineers undo this broadening with Dispersion Compensating Fiber, or DCF — a specialty negative-dispersion spool or chirped grating that recompresses the pulse back to transform-limited width. Watch the icy-blue dispersion halo around the photon when it leaves CD fiber.",
                "Pay close attention to ordering here: convert with QFC before the fiber, then place BOTH your VBG filter and your DCF spool downstream of the CD and Lit fibers so both Raman noise and dispersion are cleared before the SNSPD.",
                "Fire with Space and try the +5ps single-step frame debugger with the period key to watch DCF snap the pulse back together!",
            ],
        ),
        LabScript(
            num=18,
            slug="wdm_wavelength_routing",
            title="WDM Wavelength Routing",
            subtitle="Merging Visible & Telecom Beams with a Dichroic Router",
            concept_tag="WAVELENGTH MULTIPLEXING • DICHROIC WDM (\\)",
            emitters=[(0, 2, "R", 780), (3, 0, "D", 637)],
            target=(9, 2, "BSM 50:50"),
            fixed=[(7, 2, "FIBER"), (8, 2, "FIBER")],
            fail_stop=(3, 2),
            fail_banner="1550nm PASSES STRAIGHT THROUGH WDM — ONLY VISIBLE REFLECTS!",
            fail_color=PINK,
            cards=[
                HardwareCard("WDM (\\)", "Dichroic Beam Combiner", PINK,
                             "1550nm Passes Straight  |  Visible Reflects \\",
                             "Thin-film dielectric coating merges two wavelengths onto one shared optical axis with zero splitting loss."),
                HardwareCard("QFC TRICK", "1550nm Ignores Downstream QFC", PURPLE,
                             "Convert Arm A Before, Arm B After!",
                             "A photon already at 1550nm passes through a second QFC unchanged — letting you convert Arm B on the shared trunk!"),
                HardwareCard("SHARED VBG", "2x VBG Cleans Both Arms", CYAN,
                             "Strips +40 (Ion) & +70 (NV) -> 0",
                             "Once both photons share row 2, two VBG filters clean both arms before the fiber and BSM."),
            ],
            pipeline=[
                PipelineStage("ARM A (1550)", "QFC First -> Passes WDM", CYAN, wl_after=1550),
                PipelineStage("ARM B (637)", "Delay -> Reflects WDM (\\)!", PINK, wl_after=637),
                PipelineStage("SHARED TRUNK", "QFC (B only) -> 2x VBG", PURPLE, wl_after=1550),
                PipelineStage("BSM 50:50", "MULTIPLEXED HOM LOCK!", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "You have NO regular mirrors — the WDM Dichroic at the beam crossing is the only way to turn Arm B!",
                "Keep Arm B at 637nm until AFTER it reflects off the WDM onto row 2; convert Arm A to 1550nm BEFORE the WDM.",
                "Place the second QFC and both VBGs on the shared horizontal trunk, and add Delay on the shorter Arm B.",
            ],
            narration=[
                "Welcome to Lab 18: WDM Wavelength Routing. How do you merge two independent emitters onto a single optical fiber without losing half your photons at a 50-50 beam splitter?",
                "You use a Dichroic WDM Router. Unlike an ordinary mirror, a dichroic coating is wavelength-selective: it transmits 1550 nanometer telecom light straight through, while reflecting visible 637 or 780 nanometer photons 90 degrees like a back-slash mirror.",
                "Notice you have no ordinary mirrors in your inventory — only one WDM router. If you convert Arm B to 1550 nanometers too early, it shoots straight through the WDM and off the bench! Instead, convert Arm A to 1550 before the WDM so it passes straight through, and leave Arm B at 637 nanometers so the WDM reflects it onto row two.",
                "Here's the clever part: a PPLN QFC leaves photons that are already at 1550 nanometers untouched. So placing your second QFC downstream on the shared trunk converts only Arm B — and then two shared VBG filters clean both photons before the BSM!",
                "Balance the shorter vertical arm with the Delay Line, verify 0 picoseconds on the Alignment Laser, and fire!",
            ],
        ),
        LabScript(
            num=19,
            slug="dispersive_metro_link",
            title="Dispersive Metro Link",
            subtitle="Dual-Arm Dispersion Compensation & Entanglement Distillation",
            concept_tag="METRO QPU TRUNK • CD + LIT + DISTILL",
            emitters=[(0, 1, "R", 780), (0, 4, "R", 780)],
            target=(9, 2, "QPU >=90%"),
            fixed=[(3, 1, "CD_FIBER"), (4, 1, "CD_FIBER"), (5, 1, "FIBER"), (3, 4, "CD_FIBER"), (4, 4, "LIT_FIBER"), (5, 4, "FIBER"), (8, 2, "DISTILL")],
            walls=[(9, 0), (9, 1), (9, 3), (9, 4), (9, 5)],
            fail_stop=(8, 2),
            fail_banner="DISTILL REJECTS DISPERSED OR NOISY PHOTONS!",
            fail_color=ICE,
            cards=[
                HardwareCard("DUAL CD", "Both Metro Arms Dispersed", ICE,
                             "Each Arm Needs DCF Downstream",
                             "Entanglement distillation requires transform-limited pulses — any uncompensated dispersion is rejected."),
                HardwareCard("ARM B NOISE", "Lit Fiber Mid-Span (+15)", AMBER,
                             "Filter Arm B AFTER Lit Fiber!",
                             "Save Arm B's VBG filter until after its Lit Fiber tile so total noise drops from 55 to 5 (<= 10)."),
                HardwareCard("DISTILL -> QPU", "Purify Two 70% -> One 95%", GREEN,
                             "Sync at 0ps with Delay Line",
                             "Balance the shorter upper arm with Delay so both 70% photons fuse into a 95% qubit for the QPU."),
            ],
            pipeline=[
                PipelineStage("ARM A", "QFC+VBG -> CD -> DCF + Delay", ICE, wl_after=1550),
                PipelineStage("ARM B", "QFC -> CD+Lit -> VBG + DCF", AMBER, wl_after=1550),
                PipelineStage("DISTILL", "Purify 70%+70% -> 95%!", GREEN, wl_after=1550),
                PipelineStage("QPU >=90%", "METRO QPU LOCKED!", PURPLE, wl_after=1550),
            ],
            strategy_bullets=[
                "Both arms cross 3 fiber tiles (-30% fid -> 70%), which is valid for DISTILL (>=60%) once cleaned.",
                "Place DCF AFTER the CD Fiber tiles on BOTH arms, and place Arm B's VBG AFTER its Lit Fiber.",
                "Equalize arrival timing into DISTILL using the +100ps Delay Line on the shorter upper arm.",
            ],
            narration=[
                "Welcome to Lab 19: Dispersive Metro Link. Before tackling Quantum Teleportation, let's build a realistic fault-tolerant metro QPU trunk combining attenuation, Raman coexistence, and chromatic dispersion.",
                "Both metro arms cross three fiber tiles, dropping fidelity to 70 percent — well below the QPU's 90 percent requirement. Both arms also accumulate chromatic dispersion, and the lower arm crosses a lit fiber. The distillation node will reject any photon that arrives dispersed, noisy, or out of sync.",
                "Audit each arm's downstream cleanup carefully: both arms need a DCF spool after their CD fiber spans, and the lower arm must keep its VBG filter after the lit fiber so noise drops cleanly below 10.",
                "Check the Alignment Laser to see which arm is 100 picoseconds shorter, insert your Delay Line to match arrival times at the distillation purifier, and let the purified 95 percent photon lock straight into the QPU.",
                "Press Space to fire and watch both metro arms purify into a fault-tolerant QPU link!",
            ],
        ),
        LabScript(
            num=20,
            slug="quantum_teleportation",
            title="Quantum Teleportation",
            subtitle="Act II Finale: Bennett et al. (1993) State Transfer via EPR & Herald",
            concept_tag="ACT II FINALE • QUANTUM TELEPORTATION",
            emitters=[(0, 5, "R", 780), (6, 1, "L", 780), (6, 1, "R", 780)],
            epr_pos=(6, 1),
            target=(6, 4, "TELEPORT RX"),
            fixed=[(3, 3, "TELEPORT_BSM"), (9, 2, "CD_FIBER"), (9, 3, "PMD_FIBER")],
            walls=[(5, 2), (5, 3), (5, 4), (5, 5)],
            fail_stop=(5, 5),
            fail_banner="|Psi> CANNOT CROSS THE WALL — TELEPORT IT VIA EPR!",
            fail_color=PURPLE,
            cards=[
                HardwareCard("TELE-BSM", "Bell-State Analyzer", AMBER,
                             "Sync |Psi> ('x') & EPR-Alice (<=15ps)",
                             "Projects the unknown state |Psi> and Alice's EPR photon into a Bell state, broadcasting a classical herald."),
                HardwareCard("EPR PAIR", "Entangled Resource", PURPLE,
                             "Left -> TELE-BSM  |  Right -> Bob",
                             "When TELE-BSM fires, |Psi>'s quantum state ('x') instantly projects onto Bob's distant EPR photon!"),
                HardwareCard("PAULI FIX", "DCF + HWP at Bob", ICE,
                             "Clear CD & Undo PMD Before RX",
                             "Bob's path crosses CD and PMD fiber — recompress with DCF and apply HWP Pauli correction before TELEPORT RX."),
            ],
            pipeline=[
                PipelineStage("|Psi> + EPR-A", "Sync 500ps -> TELE-BSM", AMBER, wl_after=1550),
                PipelineStage("HERALD!", "State 'x' Teleported -> Bob!", PURPLE, wl_after=1550),
                PipelineStage("BOB CHANNEL", "CD + PMD -> DCF + HWP", ICE, wl_after=1550),
                PipelineStage("TELEPORT RX", "|Psi> ('x') RECONSTRUCTED!", GREEN, wl_after=1550),
            ],
            strategy_bullets=[
                "Notice |Psi> (bottom-left) is walled off from Bob — it NEVER physically travels to TELEPORT RX!",
                "Route |Psi> and EPR-Alice symmetrically into TELE-BSM (both are 500ps — no delay needed!).",
                "Route EPR-Bob through the right-side CD and PMD fibers, then apply DCF and HWP before TELEPORT RX!",
            ],
            narration=[
                "Welcome to Lab 20: Quantum Teleportation — the Act Two Finale, based on Bennett and colleagues' landmark 1993 protocol.",
                "At the bottom-left sits an unknown quantum state Psi in diagonal x polarization. A solid cryostat wall completely blocks it from reaching Bob's receiver — Psi can never physically cross to the right side of the bench.",
                "Instead, we teleport its quantum state using entanglement and a classical herald. Alice interferes Psi with her half of an entangled EPR pair at the TELE-BSM analyzer within 15 picoseconds. The moment that Bell measurement locks, Psi's diagonal state is projected onto Bob's distant EPR photon in mid-flight!",
                "Build three clean 1550 nanometer paths: route Psi and EPR-Alice symmetrically into TELE-BSM so they arrive together at 500 picoseconds, and route EPR-Bob around the right-hand dispersive and birefringent fibers — applying DCF recompression and an HWP Pauli correction before TELEPORT RX.",
                "Press Space and watch the purple teleportation shockwave transfer the quantum state straight across the wall to Bob. Congratulations on mastering the Quantum Networking Optical Bench!",
            ],
        ),
    ]


# --- Typography & Drawing Helpers ---

def load_font(size: int, bold: bool = False, mono: bool = False) -> ImageFont.FreeTypeFont:
    candidates = []
    if mono:
        candidates = [
            "/System/Library/Fonts/SFNSMono.ttf",
            "/System/Library/Fonts/Menlo.ttc",
        ]
    elif bold:
        candidates = [
            "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
            "/System/Library/Fonts/HelveticaNeue.ttc",
            "/System/Library/Fonts/SFNS.ttf",
        ]
    else:
        candidates = [
            "/System/Library/Fonts/SFNS.ttf",
            "/System/Library/Fonts/HelveticaNeue.ttc",
            "/System/Library/Fonts/Supplemental/Arial.ttf",
        ]
    for path in candidates:
        if os.path.exists(path):
            try:
                return ImageFont.truetype(path, size=size)
            except OSError:
                continue
    return ImageFont.load_default()


FONT_CACHE: dict[Tuple[int, bool, bool], ImageFont.FreeTypeFont] = {}


def get_font(size: int, bold: bool = False, mono: bool = False) -> ImageFont.FreeTypeFont:
    key = (size, bold, mono)
    if key not in FONT_CACHE:
        FONT_CACHE[key] = load_font(size, bold=bold, mono=mono)
    return FONT_CACHE[key]


def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * max(0.0, min(1.0, t))


def ease_out(t: float) -> float:
    t = max(0.0, min(1.0, t))
    return 1.0 - (1.0 - t) ** 3


def blend(c1: Tuple[int, int, int], c2: Tuple[int, int, int], t: float) -> Tuple[int, int, int]:
    t = max(0.0, min(1.0, t))
    return (
        int(c1[0] + (c2[0] - c1[0]) * t),
        int(c1[1] + (c2[1] - c1[1]) * t),
        int(c1[2] + (c2[2] - c1[2]) * t),
    )


def draw_glow_circle(img: Image.Image, cx: float, cy: float, radius: float, color: Tuple[int, int, int], intensity: float = 0.65):
    overlay = Image.new("RGBA", img.size, (0, 0, 0, 0))
    od = ImageDraw.Draw(overlay)
    for i in range(14, 0, -1):
        frac = i / 14.0
        r = radius * frac
        a = int(255 * intensity * 0.42 * ((1.0 - frac * 0.88) ** 2))
        if a > 0:
            od.ellipse((cx - r, cy - r, cx + r, cy + r), fill=(color[0], color[1], color[2], a))
    img.alpha_composite(overlay)


def draw_breadboard_bg(img: Image.Image, frame_idx: int):
    d = ImageDraw.Draw(img)
    d.rectangle((0, 0, W, H), fill=BG_DARK)
    # Subtle ambient corner glows
    draw_glow_circle(img, 240, 140, 420, PURPLE, 0.16)
    draw_glow_circle(img, W - 260, H - 140, 460, CYAN, 0.15)
    # Faint breadboard grid lines
    d = ImageDraw.Draw(img)
    step = 80
    for x in range(0, W, step):
        d.line((x, 0, x, H), fill=(19, 25, 36), width=1)
    for y in range(0, H, step):
        d.line((0, y, W, y), fill=(19, 25, 36), width=1)
    for x in range(40, W, step):
        for y in range(40, H, step):
            d.ellipse((x - 2, y - 2, x + 2, y + 2), fill=(24, 32, 46))


def draw_top_chrome(d: ImageDraw.ImageDraw, lab: LabScript, scene_label: str, progress: float):
    # Top subtle header bar
    d.rectangle((0, 0, W, 72), fill=(14, 20, 30))
    d.line((0, 72, W, 72), fill=GRID_LINE, width=2)
    d.text((48, 22), f"QN BENCH  •  LAB {lab.num:02d} / 20  •  {lab.title.upper()}",
           font=get_font(22, bold=True, mono=True), fill=CYAN)
    bbox = d.textbbox((0, 0), scene_label, font=get_font(20, bold=True, mono=True))
    sw = bbox[2] - bbox[0]
    d.rounded_rectangle((W - sw - 84, 16, W - 44, 56), radius=8, fill=(22, 34, 52), outline=GRID_LINE, width=2)
    d.text((W - sw - 64, 24), scene_label, font=get_font(20, bold=True, mono=True), fill=AMBER)
    # Bottom progress bar
    d.rectangle((0, H - 6, W, H), fill=(20, 28, 40))
    d.rectangle((0, H - 6, int(W * max(0.0, min(1.0, progress))), H), fill=CYAN)


def draw_wrapped(d: ImageDraw.ImageDraw, x: int, y: int, text: str, font: ImageFont.FreeTypeFont,
                 fill: Tuple[int, int, int], max_chars: int, line_h: int) -> int:
    cur_y = y
    for line in textwrap.wrap(text, width=max_chars):
        d.text((x, cur_y), line, font=font, fill=fill)
        cur_y += line_h
    return cur_y


# --- Scene Renderers ---

def render_scene_1_intro(lab: LabScript, t: float, total_prog: float, frame_idx: int) -> Image.Image:
    img = Image.new("RGBA", (W, H), BG_DARK + (255,))
    draw_breadboard_bg(img, frame_idx)
    d = ImageDraw.Draw(img)

    # Animated glowing 780nm -> 1550nm photon wave graphic across top-center
    wave_y = 260
    pts_left = []
    pts_right = []
    phase = t * 5.0
    for x in range(260, 920, 6):
        y = wave_y + math.sin((x - 260) * 0.045 - phase) * 32
        pts_left.append((x, y))
    for x in range(1000, 1660, 6):
        y = wave_y + math.sin((x - 1000) * 0.024 - phase) * 36
        pts_right.append((x, y))
    if len(pts_left) > 1:
        d.line(pts_left, fill=RED, width=5)
    if len(pts_right) > 1:
        d.line(pts_right, fill=CYAN, width=6)

    # Center nonlinear crystal emblem
    draw_glow_circle(img, W / 2, wave_y, 95, PURPLE, 0.55)
    d = ImageDraw.Draw(img)
    d.rounded_rectangle((W // 2 - 48, wave_y - 58, W // 2 + 48, wave_y + 58),
                        radius=12, fill=(38, 28, 72), outline=PURPLE, width=4)
    for gx in range(-28, 32, 14):
        d.line((W // 2 + gx, wave_y - 44, W // 2 + gx, wave_y + 44), fill=PURPLE, width=3)

    # Kicker pill
    kicker = f"CONCEPTUAL LAB GUIDE  •  LAB {lab.num:02d} OF 20"
    kf = get_font(26, bold=True, mono=True)
    kb = d.textbbox((0, 0), kicker, font=kf)
    kw = kb[2] - kb[0]
    d.rounded_rectangle((W // 2 - kw // 2 - 28, 385, W // 2 + kw // 2 + 28, 437),
                        radius=24, fill=(18, 32, 48), outline=CYAN, width=2)
    d.text((W // 2 - kw // 2, 396), kicker, font=kf, fill=CYAN)

    # Main Title (animated gentle slide-up)
    slide = int((1.0 - ease_out(min(1.0, t / 0.8))) * 24)
    tf = get_font(78, bold=True)
    title_str = f"Level {lab.num}: {lab.title}"
    tb = d.textbbox((0, 0), title_str, font=tf)
    tw = tb[2] - tb[0]
    d.text((W // 2 - tw // 2, 476 + slide), title_str, font=tf, fill=TEXT_WHITE)

    # Subtitle
    sf = get_font(34)
    sb = d.textbbox((0, 0), lab.subtitle, font=sf)
    sw = sb[2] - sb[0]
    d.text((W // 2 - sw // 2, 588 + slide), lab.subtitle, font=sf, fill=TEXT_MUTED)

    # Concept tag badge
    cf = get_font(24, bold=True, mono=True)
    cb = d.textbbox((0, 0), lab.concept_tag, font=cf)
    cw = cb[2] - cb[0]
    d.rounded_rectangle((W // 2 - cw // 2 - 26, 676, W // 2 + cw // 2 + 26, 728),
                        radius=10, fill=(28, 22, 48), outline=PURPLE, width=2)
    d.text((W // 2 - cw // 2, 688), lab.concept_tag, font=cf, fill=PURPLE)

    # Footer brand
    ft = "QN Bench — Quantum Networking Optical Bench  •  No-Spoiler Physics Guide"
    ff = get_font(22)
    fb = d.textbbox((0, 0), ft, font=ff)
    d.text((W // 2 - (fb[2] - fb[0]) // 2, 955), ft, font=ff, fill=(100, 120, 148))

    d.rectangle((0, H - 6, int(W * total_prog), H), fill=CYAN)
    return img.convert("RGB")


def draw_mini_tile(d: ImageDraw.ImageDraw, x0: int, y0: int, ts: int, kind: str, label: str = ""):
    pad = 8
    r = (x0 + pad, y0 + pad, x0 + ts - pad, y0 + ts - pad)
    cx, cy = x0 + ts // 2, y0 + ts // 2
    f_sm = get_font(14, bold=True, mono=True)

    def draw_centered_tag(txt: str, col: Tuple[int, int, int]):
        bb = d.textbbox((0, 0), txt, font=f_sm)
        tw = bb[2] - bb[0]
        d.text((cx - tw // 2, y0 + ts - 30), txt, font=f_sm, fill=col)

    if kind == "WALL":
        d.rounded_rectangle(r, radius=8, fill=(48, 58, 74), outline=(88, 104, 128), width=2)
        for off in range(-20, 40, 16):
            d.line((x0 + pad + off, y0 + pad, x0 + pad + off - 12, y0 + ts - pad), fill=(34, 42, 56), width=3)
    elif kind == "FIBER":
        d.rectangle((x0, cy - 18, x0 + ts, cy + 18), fill=(22, 34, 52))
        d.line((x0, cy - 18, x0 + ts, cy - 18), fill=(48, 92, 130), width=2)
        d.line((x0, cy + 18, x0 + ts, cy + 18), fill=(48, 92, 130), width=2)
        d.line((x0, cy, x0 + ts, cy), fill=CYAN, width=6)
        draw_centered_tag("1550nm", (110, 175, 215))
    elif kind == "LIT_FIBER":
        d.rectangle((x0, cy - 18, x0 + ts, cy + 18), fill=(44, 32, 20))
        d.line((x0, cy - 4, x0 + ts, cy - 4), fill=CYAN, width=4)
        d.line((x0, cy + 6, x0 + ts, cy + 6), fill=AMBER, width=3)
        draw_centered_tag("LIT +15", AMBER)
    elif kind == "PMD_FIBER":
        d.rectangle((x0, cy - 18, x0 + ts, cy + 18), fill=(36, 24, 54))
        pts = [(x0 + dx, cy + int(9 * math.sin(dx * 0.14))) for dx in range(0, ts + 1, 6)]
        d.line(pts, fill=PURPLE, width=4)
        draw_centered_tag("PMD", PURPLE)
    elif kind == "CD_FIBER":
        d.rectangle((x0, cy - 18, x0 + ts, cy + 18), fill=(18, 34, 56))
        d.line((x0, cy - 4, x0 + ts, cy - 14), fill=ICE, width=3)
        d.line((x0, cy + 4, x0 + ts, cy + 14), fill=ICE, width=3)
        d.line((x0, cy, x0 + ts, cy), fill=AMBER, width=2)
        draw_centered_tag("CD FIBER", ICE)
    elif kind == "ATMOS":
        d.rounded_rectangle(r, radius=10, fill=(16, 38, 46), outline=TEAL, width=2)
        for ro in (-18, 0, 18):
            pts = [(x0 + 16 + dx, cy + ro + int(6 * math.sin(dx * 0.12 + ro))) for dx in range(0, ts - 32, 6)]
            d.line(pts, fill=TEAL, width=3)
        draw_centered_tag("ATMOS", TEAL)
    elif kind == "EVE":
        d.rounded_rectangle(r, radius=10, fill=(52, 18, 24), outline=RED, width=3)
        d.ellipse((cx - 18, cy - 22, cx + 18, cy + 14), outline=RED, width=3)
        d.ellipse((cx - 7, cy - 11, cx + 7, cy + 3), fill=AMBER)
        draw_centered_tag("EVE TAP", RED)
    elif kind in ("DISTILL", "REPEATER", "TELEPORT_BSM"):
        col = GREEN if kind == "DISTILL" else (CYAN if kind == "REPEATER" else AMBER)
        d.rounded_rectangle(r, radius=10, fill=(22, 38, 48), outline=col, width=3)
        d.polygon([(cx, y0 + 18), (cx + 20, cy - 8), (cx, cy + 14), (cx - 20, cy - 8)], outline=col, width=3)
        tag = "DISTILL" if kind == "DISTILL" else ("REPEAT" if kind == "REPEATER" else "TELE-BSM")
        draw_centered_tag(tag, col)
    elif kind == "EPR":
        d.rounded_rectangle(r, radius=10, fill=(36, 22, 58), outline=PURPLE, width=3)
        d.polygon([(cx, y0 + 16), (cx + 20, cy - 8), (cx, cy + 14), (cx - 20, cy - 8)], fill=(64, 42, 114), outline=TEXT_WHITE)
        draw_centered_tag("EPR PAIR", PURPLE)
    elif kind == "EMITTER":
        is_nv = "637" in label
        col = ORANGE if is_nv else RED
        bg = (48, 28, 16) if is_nv else (48, 20, 28)
        d.rounded_rectangle(r, radius=10, fill=bg, outline=col, width=3)
        d.ellipse((cx - 12, cy - 20, cx + 12, cy + 4), fill=col)
        draw_centered_tag(f"ION {label}" if not is_nv else f"NV {label}", col)
    elif kind == "TARGET":
        col = PURPLE if ("QPU" in label or "TELE" in label) else (AMBER if "BSM" in label else (TEAL if "RX" in label else CYAN))
        d.rounded_rectangle(r, radius=10, fill=(18, 34, 48), outline=col, width=3)
        d.rectangle((cx - 16, cy - 24, cx + 16, cy + 4), outline=col, width=3)
        draw_centered_tag(label[:10], col)


def render_scene_2_problem(lab: LabScript, t: float, dur: float, total_prog: float, frame_idx: int) -> Image.Image:
    img = Image.new("RGBA", (W, H), BG_DARK + (255,))
    draw_breadboard_bg(img, frame_idx)
    d = ImageDraw.Draw(img)
    draw_top_chrome(d, lab, "01 / THE PHYSICAL BOTTLENECK", total_prog)

    # Bench canvas (10x6 tiles at 128px each = 1280x768)
    ts = 116
    bw, bh = 10 * ts, 6 * ts
    ox = (W - bw) // 2
    oy = 148
    d.rounded_rectangle((ox - 12, oy - 12, ox + bw + 12, oy + bh + 12), radius=14, fill=(14, 19, 28), outline=GRID_LINE, width=3)
    d.rectangle((ox, oy, ox + bw, oy + bh), fill=GRID_BG)
    for r in range(6):
        for c in range(10):
            x0, y0 = ox + c * ts, oy + r * ts
            d.rectangle((x0, y0, x0 + ts, y0 + ts), outline=GRID_LINE, width=1)
            for hx in (26, ts - 26):
                for hy in (26, ts - 26):
                    d.ellipse((x0 + hx - 3, y0 + hy - 3, x0 + hx + 3, y0 + hy + 3), fill=HOLE_COL)

    for (gx, gy) in lab.walls:
        draw_mini_tile(d, ox + gx * ts, oy + gy * ts, ts, "WALL")
    for (gx, gy, tk) in lab.fixed:
        draw_mini_tile(d, ox + gx * ts, oy + gy * ts, ts, tk)
    if lab.epr_pos:
        draw_mini_tile(d, ox + lab.epr_pos[0] * ts, oy + lab.epr_pos[1] * ts, ts, "EPR")
    for (gx, gy, _, wl) in lab.emitters:
        if lab.epr_pos == (gx, gy):
            continue
        draw_mini_tile(d, ox + gx * ts, oy + gy * ts, ts, "EMITTER", f"{wl}nm")
    draw_mini_tile(d, ox + lab.target[0] * ts, oy + lab.target[1] * ts, ts, "TARGET", lab.target[2])
    if lab.extra_target:
        draw_mini_tile(d, ox + lab.extra_target[0] * ts, oy + lab.extra_target[1] * ts, ts, "TARGET", lab.extra_target[2])

    # Animate a looping naive photon beam hitting lab.fail_stop with a red/amber impact shockwave!
    cycle = (t % 3.4) / 3.4
    em0 = lab.emitters[0]
    sx = ox + em0[0] * ts + ts * 0.5
    sy = oy + em0[1] * ts + ts * 0.5
    fx = ox + lab.fail_stop[0] * ts + ts * 0.5
    fy = oy + lab.fail_stop[1] * ts + ts * 0.5
    pcol = ORANGE if em0[3] == 637 else RED

    if cycle < 0.62:
        u = cycle / 0.62
        px = lerp(sx, fx, u)
        py = lerp(sy, fy, u)
        tx = lerp(sx, fx, max(0.0, u - 0.28))
        ty = lerp(sy, fy, max(0.0, u - 0.28))
        d.line((tx, ty, px, py), fill=pcol, width=8)
        draw_glow_circle(img, px, py, 34, pcol, 0.78)
        d = ImageDraw.Draw(img)
        d.ellipse((px - 11, py - 11, px + 11, py + 11), fill=pcol)
        d.ellipse((px - 5, py - 5, px + 5, py + 5), fill=TEXT_WHITE)
    else:
        imp = (cycle - 0.62) / 0.38
        rad = 14 + imp * 85
        d = ImageDraw.Draw(img)
        d.ellipse((fx - rad, fy - rad, fx + rad, fy + rad), outline=lab.fail_color, width=5)
        for k in range(10):
            ang = k * (math.tau / 10.0) + 0.3
            pr = imp * 68
            px, py = fx + math.cos(ang) * pr, fy + math.sin(ang) * pr
            d.ellipse((px - 5, py - 5, px + 5, py + 5), fill=lab.fail_color)

    # Bottom Diagnostic Failure Callout Bar
    bf = get_font(28, bold=True, mono=True)
    bb = d.textbbox((0, 0), lab.fail_banner, font=bf)
    bw_txt = bb[2] - bb[0]
    bar_x1 = W // 2 - bw_txt // 2 - 42
    bar_x2 = W // 2 + bw_txt // 2 + 42
    d.rounded_rectangle((bar_x1, 895, bar_x2, 962), radius=12, fill=(44, 18, 26), outline=lab.fail_color, width=3)
    d.text((W // 2 - bw_txt // 2, 912), lab.fail_banner, font=bf, fill=lab.fail_color)

    d.text((ox, 988), "Why the naive path fails — let's examine the hardware stack needed to fix it.",
           font=get_font(24), fill=TEXT_MUTED)
    return img.convert("RGB")


def render_scene_3_hardware(lab: LabScript, t: float, dur: float, total_prog: float, frame_idx: int) -> Image.Image:
    img = Image.new("RGBA", (W, H), BG_DARK + (255,))
    draw_breadboard_bg(img, frame_idx)
    d = ImageDraw.Draw(img)
    draw_top_chrome(d, lab, "02 / QUANTUM HARDWARE & PHYSICS", total_prog)

    d.text((96, 118), "Key Hardware & Physical Rules for This Lab", font=get_font(44, bold=True), fill=TEXT_WHITE)
    d.text((96, 178), lab.subtitle, font=get_font(26), fill=TEXT_MUTED)

    n = max(1, len(lab.cards))
    active_idx = min(n - 1, int((t / max(0.1, dur)) * n))
    gap = 36
    total_w = W - 192
    card_w = (total_w - gap * (n - 1)) // n
    cy0, cy1 = 250, 880

    for i, card in enumerate(lab.cards):
        cx0 = 96 + i * (card_w + gap)
        cx1 = cx0 + card_w
        is_hi = (i == active_idx)
        bg = (22, 32, 48) if is_hi else CARD_BG
        bd = card.accent if is_hi else GRID_LINE
        bw = 4 if is_hi else 2
        if is_hi:
            draw_glow_circle(img, (cx0 + cx1) / 2, (cy0 + cy1) / 2, 260, card.accent, 0.14)
            d = ImageDraw.Draw(img)
        d.rounded_rectangle((cx0, cy0, cx1, cy1), radius=18, fill=bg, outline=bd, width=bw)

        # Tag badge
        tf = get_font(22, bold=True, mono=True)
        tb = d.textbbox((0, 0), card.tag, font=tf)
        d.rounded_rectangle((cx0 + 30, cy0 + 30, cx0 + 58 + (tb[2] - tb[0]), cy0 + 72),
                            radius=8, fill=(14, 20, 32), outline=card.accent, width=2)
        d.text((cx0 + 44, cy0 + 38), card.tag, font=tf, fill=card.accent)

        # Title
        d.text((cx0 + 30, cy0 + 102), card.title, font=get_font(36, bold=True), fill=TEXT_WHITE)
        d.line((cx0 + 30, cy0 + 158, cx1 - 30, cy0 + 158), fill=GRID_LINE, width=2)

        # Key Rule Highlight Box
        d.rounded_rectangle((cx0 + 28, cy0 + 182, cx1 - 28, cy0 + 286),
                            radius=12, fill=(12, 18, 28), outline=card.accent, width=2)
        d.text((cx0 + 44, cy0 + 196), "BENCH RULE", font=get_font(16, bold=True, mono=True), fill=card.accent)
        draw_wrapped(d, cx0 + 44, cy0 + 224, card.spec, get_font(22, bold=True), TEXT_WHITE, 30, 28)

        # Physics Explanation
        d.text((cx0 + 32, cy0 + 320), "REAL-WORLD PHYSICS", font=get_font(17, bold=True, mono=True), fill=TEXT_MUTED)
        draw_wrapped(d, cx0 + 32, cy0 + 356, card.detail, get_font(25), TEXT_WHITE, 31, 36)

        # Animated bottom accent bar on active card
        if is_hi:
            d.rounded_rectangle((cx0 + 30, cy1 - 24, cx1 - 30, cy1 - 14), radius=5, fill=card.accent)

    d.text((96, 938), "Tip: Shift+Click any hardware tile inside QN Bench to open its full Physics Specification at any time.",
           font=get_font(23), fill=TEXT_MUTED)
    return img.convert("RGB")


def render_scene_4_strategy(lab: LabScript, t: float, dur: float, total_prog: float, frame_idx: int) -> Image.Image:
    img = Image.new("RGBA", (W, H), BG_DARK + (255,))
    draw_breadboard_bg(img, frame_idx)
    d = ImageDraw.Draw(img)
    draw_top_chrome(d, lab, "03 / CONCEPTUAL SIGNAL FLOW (NO SPOILERS)", total_prog)

    d.text((96, 114), "Conceptual Strategy & Signal Flow", font=get_font(44, bold=True), fill=TEXT_WHITE)
    d.text((96, 170), "Understand the ordering principle — you decide the exact tile placements on the bench!",
           font=get_font(25), fill=TEXT_MUTED)

    # Animated pipeline diagram
    stages = lab.pipeline
    n = max(1, len(stages))
    pipe_y = 385
    box_w = min(290, (W - 192 - (n - 1) * 54) // n)
    box_h = 152
    total_pw = n * box_w + (n - 1) * 54
    start_x = (W - total_pw) // 2

    # Continuous traveling photon pulse along the pipeline
    pulse_u = (t % 4.0) / 4.0
    pulse_x = lerp(start_x + box_w * 0.5, start_x + total_pw - box_w * 0.5, pulse_u)

    # Connector lines
    for i in range(n - 1):
        x1 = start_x + i * (box_w + 54) + box_w
        x2 = x1 + 54
        st = stages[i]
        lcol = RED if st.wl_after == 780 else (ORANGE if st.wl_after == 637 else CYAN)
        d.line((x1, pipe_y, x2, pipe_y), fill=lcol, width=6)
        d.polygon([(x2, pipe_y), (x2 - 14, pipe_y - 9), (x2 - 14, pipe_y + 9)], fill=lcol)

    for i, st in enumerate(stages):
        bx0 = start_x + i * (box_w + 54)
        bx1 = bx0 + box_w
        by0 = pipe_y - box_h // 2
        by1 = pipe_y + box_h // 2
        bcx = (bx0 + bx1) / 2
        lit = abs(pulse_x - bcx) < box_w * 0.65
        if lit:
            draw_glow_circle(img, bcx, pipe_y, 110, st.color, 0.28)
            d = ImageDraw.Draw(img)
        d.rounded_rectangle((bx0, by0, bx1, by1), radius=16,
                            fill=(24, 34, 52) if lit else CARD_BG,
                            outline=st.color, width=4 if lit else 2)
        lf = get_font(24, bold=True, mono=True)
        lb = d.textbbox((0, 0), st.label, font=lf)
        d.text((bcx - (lb[2] - lb[0]) / 2, by0 + 34), st.label, font=lf, fill=st.color)
        sf = get_font(20)
        sb = d.textbbox((0, 0), st.sub, font=sf)
        d.text((bcx - (sb[2] - sb[0]) / 2, by0 + 86), st.sub, font=sf, fill=TEXT_WHITE)

    # Glowing traveling photon dot on top of the pipeline connectors
    stage_idx = min(n - 1, int(pulse_u * n))
    cur_st = stages[stage_idx]
    p_col = RED if cur_st.wl_after == 780 else (ORANGE if cur_st.wl_after == 637 else CYAN)
    draw_glow_circle(img, pulse_x, pipe_y - box_h // 2 - 28, 28, p_col, 0.75)
    d = ImageDraw.Draw(img)
    d.ellipse((pulse_x - 10, pipe_y - box_h // 2 - 38, pulse_x + 10, pipe_y - box_h // 2 - 18), fill=p_col)
    d.ellipse((pulse_x - 4, pipe_y - box_h // 2 - 32, pulse_x + 4, pipe_y - box_h // 2 - 24), fill=TEXT_WHITE)
    if cur_st.noisy_after:
        for k in range(3):
            jx = pulse_x + math.sin(t * 22 + k * 2.1) * 16
            jy = pipe_y - box_h // 2 - 28 + math.cos(t * 19 + k * 1.7) * 12
            d.ellipse((jx - 4, jy - 4, jx + 4, jy + 4), fill=AMBER)

    # Strategy checklist panel
    d.rounded_rectangle((96, 535, W - 96, 930), radius=18, fill=CARD_BG, outline=GRID_LINE, width=2)
    d.text((136, 566), "KEY DESIGN CHECKLIST", font=get_font(22, bold=True, mono=True), fill=GREEN)
    cur_y = 620
    for idx, bullet in enumerate(lab.strategy_bullets):
        reveal = ease_out(min(1.0, max(0.0, (t - idx * 1.2) / 0.6)))
        col = blend(GRID_LINE, TEXT_WHITE, reveal)
        bcol = blend(GRID_LINE, GREEN, reveal)
        d.rounded_rectangle((136, cur_y + 4, 174, cur_y + 42), radius=8, fill=(18, 46, 36), outline=bcol, width=2)
        d.text((149, cur_y + 9), str(idx + 1), font=get_font(24, bold=True, mono=True), fill=bcol)
        draw_wrapped(d, 198, cur_y + 6, bullet, get_font(29), col, 72, 38)
        cur_y += 96

    return img.convert("RGB")


def render_scene_5_outro(lab: LabScript, t: float, total_prog: float, frame_idx: int) -> Image.Image:
    img = Image.new("RGBA", (W, H), BG_DARK + (255,))
    draw_breadboard_bg(img, frame_idx)
    d = ImageDraw.Draw(img)
    draw_top_chrome(d, lab, "04 / YOUR TURN ON THE BENCH", total_prog)

    draw_glow_circle(img, W / 2, 420, 280, GREEN, 0.20)
    d = ImageDraw.Draw(img)

    h1 = "Ready to Build Your Optical Link?"
    f1 = get_font(66, bold=True)
    b1 = d.textbbox((0, 0), h1, font=f1)
    d.text((W // 2 - (b1[2] - b1[0]) // 2, 230), h1, font=f1, fill=TEXT_WHITE)

    # 3 quick pro-tip pills
    tips = [
        ("ALIGN [L]", "Preview beam routing & ps timing", CYAN),
        ("FIRE [SPACE]", "Launch photons & check scopes", RED),
        ("STEP [.]", "Inspect +5ps frame by frame", AMBER),
    ]
    tw_box = 460
    start_x = (W - (tw_box * 3 + 56)) // 2
    for i, (k, desc, col) in enumerate(tips):
        bx0 = start_x + i * (tw_box + 28)
        d.rounded_rectangle((bx0, 370, bx0 + tw_box, 530), radius=16, fill=CARD_BG, outline=col, width=3)
        kf = get_font(30, bold=True, mono=True)
        kb = d.textbbox((0, 0), k, font=kf)
        d.text((bx0 + (tw_box - (kb[2] - kb[0])) // 2, 405), k, font=kf, fill=col)
        df = get_font(24)
        db = d.textbbox((0, 0), desc, font=df)
        d.text((bx0 + (tw_box - (db[2] - db[0])) // 2, 462), desc, font=df, fill=TEXT_MUTED)

    # Call-to-action pill
    url_txt = "Play Free in Browser or Desktop:  skcalanderson2.github.io/qn_bench/play/"
    uf = get_font(30, bold=True, mono=True)
    ub = d.textbbox((0, 0), url_txt, font=uf)
    uw = ub[2] - ub[0]
    d.rounded_rectangle((W // 2 - uw // 2 - 40, 650, W // 2 + uw // 2 + 40, 730),
                        radius=14, fill=(18, 46, 38), outline=GREEN, width=3)
    d.text((W // 2 - uw // 2, 672), url_txt, font=uf, fill=GREEN)

    sub = f"QN Bench  •  Lab {lab.num:02d}: {lab.title}  •  Aim for 3/3 Optimal Stars!"
    sf = get_font(26)
    sb = d.textbbox((0, 0), sub, font=sf)
    d.text((W // 2 - (sb[2] - sb[0]) // 2, 810), sub, font=sf, fill=TEXT_MUTED)
    return img.convert("RGB")


# --- Audio Synthesis (edge-tts) & SRT Subtitle Generation ---

def fmt_srt_time(sec: float) -> str:
    ms = int(round(max(0.0, sec) * 1000))
    hh = ms // 3_600_000
    ms %= 3_600_000
    mm = ms // 60_000
    ms %= 60_000
    ss = ms // 1000
    ms %= 1000
    return f"{hh:02d}:{mm:02d}:{ss:02d},{ms:03d}"


def probe_duration(path: Path) -> float:
    out = subprocess.check_output(
        ["ffprobe", "-v", "error", "-show_entries", "format=duration",
         "-of", "default=noprint_wrappers=1:nokey=1", str(path)],
        text=True,
    ).strip()
    return float(out)


async def synth_scene_audio(text: str, out_mp3: Path, voice: str, rate: str) -> List[Tuple[float, float, str]]:
    """Synthesizes one scene via edge-tts and returns sentence (start, end, text) timings."""
    comm = edge_tts.Communicate(text, voice=voice, rate=rate)
    captions: List[Tuple[float, float, str]] = []
    with open(out_mp3, "wb") as fh:
        async for chunk in comm.stream():
            if chunk["type"] == "audio":
                fh.write(chunk["data"])
            elif chunk["type"] in ("SentenceBoundary", "WordBoundary"):
                start = chunk["offset"] / 10_000_000.0
                dur = chunk["duration"] / 10_000_000.0
                captions.append((start, start + dur, chunk["text"]))
    if not captions:
        dur = probe_duration(out_mp3)
        captions.append((0.0, dur, text))
    return captions


def generate_lab_video(lab: LabScript, out_dir: Path, voice: str, rate: str) -> Tuple[Path, Path, float]:
    out_dir.mkdir(parents=True, exist_ok=True)
    stem = f"level_{lab.num:02d}_{lab.slug}"
    out_mp4 = out_dir / f"{stem}.mp4"
    out_srt = out_dir / f"{stem}.srt"

    print(f"\n==> Generating Lab {lab.num:02d}: {lab.title} (voice={voice})...")
    with tempfile.TemporaryDirectory(prefix=f"qn_vid_{lab.num:02d}_") as tmp:
        tmp_path = Path(tmp)
        scene_wavs: List[Path] = []
        scene_durs: List[float] = []
        srt_entries: List[Tuple[float, float, str]] = []
        cursor_sec = 0.0

        for idx, line in enumerate(lab.narration):
            mp3_p = tmp_path / f"scene_{idx}.mp3"
            wav_p = tmp_path / f"scene_{idx}.wav"
            caps = asyncio.run(synth_scene_audio(line, mp3_p, voice=voice, rate=rate))
            # Convert each scene to 48kHz stereo WAV + trailing pause so audio/video stay sample-accurate
            pad_sec = SCENE_GAP_SEC if idx < len(lab.narration) - 1 else 1.2
            subprocess.run(
                ["ffmpeg", "-y", "-v", "error", "-i", str(mp3_p),
                 "-af", f"apad=pad_dur={pad_sec}",
                 "-ar", "48000", "-ac", "2", str(wav_p)],
                check=True,
            )
            dur = probe_duration(wav_p)
            for (cs, ce, ct) in caps:
                srt_entries.append((cursor_sec + cs, cursor_sec + ce, ct))
            scene_wavs.append(wav_p)
            scene_durs.append(dur)
            cursor_sec += dur

        # Concatenate audio scenes & loudness-normalize for YouTube (-16 LUFS)
        concat_list = tmp_path / "concat.txt"
        concat_list.write_text("\n".join(f"file '{p}'" for p in scene_wavs), encoding="utf-8")
        master_wav = tmp_path / "master.wav"
        subprocess.run(
            ["ffmpeg", "-y", "-v", "error", "-f", "concat", "-safe", "0", "-i", str(concat_list),
             "-af", "loudnorm=I=-16:TP=-1.5:LRA=11", "-ar", "48000", "-ac", "2", str(master_wav)],
            check=True,
        )
        total_dur = sum(scene_durs)

        # Write YouTube .srt closed captions
        srt_lines = []
        for idx, (st, en, txt) in enumerate(srt_entries, start=1):
            srt_lines.append(f"{idx}\n{fmt_srt_time(st)} --> {fmt_srt_time(en)}\n{txt}\n")
        out_srt.write_text("\n".join(srt_lines), encoding="utf-8")

        # Stream 1080p raw RGB frames directly into ffmpeg H.264 encoder + master audio
        ffmpeg_cmd = [
            "ffmpeg", "-y", "-v", "error",
            "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{W}x{H}", "-r", str(FPS), "-i", "-",
            "-i", str(master_wav),
            "-c:v", "libx264", "-preset", "medium", "-crf", "18", "-pix_fmt", "yuv420p",
            "-c:a", "aac", "-b:a", "192k", "-movflags", "+faststart",
            "-shortest", str(out_mp4),
        ]
        proc = subprocess.Popen(ffmpeg_cmd, stdin=subprocess.PIPE)
        assert proc.stdin is not None

        frame_idx = 0
        elapsed = 0.0
        for s_idx, s_dur in enumerate(scene_durs):
            n_frames = max(1, int(round(s_dur * FPS)))
            for f in range(n_frames):
                local_t = f / FPS
                prog = (elapsed + local_t) / max(0.1, total_dur)
                if s_idx == 0:
                    frame_img = render_scene_1_intro(lab, local_t, prog, frame_idx)
                elif s_idx == 1:
                    frame_img = render_scene_2_problem(lab, local_t, s_dur, prog, frame_idx)
                elif s_idx == 2:
                    frame_img = render_scene_3_hardware(lab, local_t, s_dur, prog, frame_idx)
                elif s_idx == 3:
                    frame_img = render_scene_4_strategy(lab, local_t, s_dur, prog, frame_idx)
                else:
                    frame_img = render_scene_5_outro(lab, local_t, prog, frame_idx)
                proc.stdin.write(frame_img.tobytes())
                frame_idx += 1
            elapsed += s_dur

        proc.stdin.close()
        ret = proc.wait()
        if ret != 0:
            raise RuntimeError(f"ffmpeg exited with status {ret}")

    size_mb = out_mp4.stat().st_size / (1024 * 1024)
    print(f"    Saved {out_mp4.name} ({total_dur:.1f}s, {size_mb:.2f} MB) + {out_srt.name}")
    return out_mp4, out_srt, total_dur


def write_youtube_metadata(labs: List[LabScript], out_dir: Path):
    md_path = out_dir / "youtube_metadata.md"
    lines = [
        "# QN Bench — YouTube Lab Guide Upload Metadata",
        "",
        "Upload each `.mp4` (and attach its matching `.srt` captions in YouTube Studio), then paste each 11-character YouTube video ID into `docs/videos.json` and push to `main` — both the WebAssembly app and installed macOS/Windows apps will automatically display the **Watch Video Guide** button!",
        "",
    ]
    for lab in labs:
        lines.append(f"## Lab {lab.num:02d}: {lab.title}")
        lines.append(f"- **File:** `level_{lab.num:02d}_{lab.slug}.mp4` + `level_{lab.num:02d}_{lab.slug}.srt`")
        lines.append(f"- **YouTube Title:** `QN Bench Lab {lab.num}: {lab.title} — {lab.subtitle}`")
        lines.append("- **Description:**")
        lines.append("```text")
        lines.append(f"Conceptual Quantum Networking Lab Guide for QN Bench Level {lab.num}: {lab.title} (No Exact Spoilers!).")
        lines.append("")
        lines.append(f"Concept: {lab.concept_tag}")
        for b in lab.strategy_bullets:
            lines.append(f"• {b}")
        lines.append("")
        lines.append("▶ Play QN Bench Free in Browser (WebAssembly): https://skcalanderson2.github.io/qn_bench/play/")
        lines.append("⬇ Download macOS Universal & Windows Desktop Apps: https://skcalanderson2.github.io/qn_bench/")
        lines.append("#QuantumNetworking #QuantumPhysics #QKD #Optics #QNBench")
        lines.append("```")
        lines.append("")
    tmp_md = out_dir / f".youtube_metadata_{os.getpid()}.md.tmp"
    tmp_md.write_text("\n".join(lines), encoding="utf-8")
    os.replace(tmp_md, md_path)


def parse_level_range(spec: str) -> List[int]:
    out: List[int] = []
    for part in spec.split(","):
        part = part.strip()
        if "-" in part:
            a, b = part.split("-", 1)
            out.extend(range(int(a), int(b) + 1))
        elif part:
            out.append(int(part))
    return sorted(set(x for x in out if 1 <= x <= 20))


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate QN Bench 1080p YouTube Lab Guide videos.")
    parser.add_argument("--levels", default="1", help="Levels to generate, e.g. '1', '1,2,3', or '1-20'")
    parser.add_argument("--voice", default="en-US-AndrewNeural", help="Microsoft Edge Neural TTS voice")
    parser.add_argument("--rate", default="+0%", help="Speech rate adjustment, e.g. '-5%' or '+5%'")
    parser.add_argument("--out", default="videos", help="Output directory (gitignored)")
    parser.add_argument("--skip-existing", action="store_true", help="Skip levels whose .mp4 already exists")
    args = parser.parse_args()

    wanted = parse_level_range(args.levels)
    if not wanted:
        print("No valid levels specified (1-20).", file=sys.stderr)
        return 1

    all_labs = build_lab_scripts()
    selected = [lab for lab in all_labs if lab.num in wanted]
    out_dir = Path(args.out)

    for lab in selected:
        out_mp4 = out_dir / f"level_{lab.num:02d}_{lab.slug}.mp4"
        if args.skip_existing and out_mp4.exists() and out_mp4.stat().st_size > 100_000:
            print(f"==> Skipping Lab {lab.num:02d} (already exists: {out_mp4.name})")
            continue
        generate_lab_video(lab, out_dir, voice=args.voice, rate=args.rate)

    write_youtube_metadata(all_labs, out_dir)
    print(f"\nDone! Metadata for all {len(all_labs)} labs written to {out_dir / 'youtube_metadata.md'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
