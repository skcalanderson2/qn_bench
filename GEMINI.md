# QN Bench — Quantum Networking Optical Bench

## Project Overview
**QN Bench** is an educational 2D optical-bench puzzle game built in **Rust + `eframe`/`egui`** (targeting **macOS Universal**, **Windows x64**, and **WebAssembly**). The goal is to teach players both the conceptual physics and the physical hardware stack of quantum networking—specifically how single photons are generated, frequency-converted, filtered, buffered, routed, entangled, and used for BB84/E91 QKD + AES-256 encryption.

* **Entry Point:** `src/main.rs` (modular crate under `src/`, original Python `v1.1.0` preserved on branch `python-v1`)
* **Key Crates:** `eframe`, `egui`, `aes`, `ctr`, `sha2`, `serde`, `rodio` (native), `web-sys` (WASM)
* **Asset Policy:** 100% procedural graphics & audio (no external image or audio assets committed). All optical components, breadboard holes, neon radial glows, particle bursts, shockwaves, icons (`gen_icons`), and 17 lab SFX waveforms are generated procedurally.

---

## Core Physics & Hardware Rules
Preserve these physical constraints when adding levels or components:

1. **Quantum Emitters (`ION 780nm` / `NV 637nm`):** Fire a single visible photon (`780nm` Trapped-Ion or `637nm` Diamond NV) at `100%` fidelity, `0` Raman noise, and `+` polarization.
2. **Telecom Fibers (`FIBER`, `LIT_FIBER`, `PMD_FIBER`):**
   * Absorb/destroy non-`1550nm` photons immediately.
   * Transmit `1550nm` (C-band) photons, degrading fidelity by **`-10%` per tile**.
   * `LIT_FIBER` (classical coexistence) also injects **`+15` Raman noise per tile**.
   * `PMD_FIBER` (birefringent) also rotates polarization **`+` ↔ `x`**.
3. **PPLN Quantum Frequency Converter (`QFC`):**
   * Shifts `780nm` -> `1550nm` (`+40` Raman noise) or `637nm` -> `1550nm` (`+70` Raman noise) via Difference Frequency Generation.
4. **Volume Bragg Grating Filter (`VBG`):**
   * Strips **`-50` Raman noise** (clamped at `0`).
5. **Quantum Memory (`MEMORY`):**
   * Models a Diamond NV center / Trapped-Ion buffer.
   * Restores photon fidelity back to **`100%`**.
6. **Optical Delay Line (`DELAY`):**
   * Coiled fiber delay that pauses a photon for **20 frames (`+100ps`)**, equal to exactly 1 tile of travel (`TILE_SIZE = 80`, `speed = 4.0`).
7. **Half-Wave Plate (`HWP`):**
   * Rotates photon polarization **`+` ↔ `x`** without loss to compensate for PMD.
8. **Adaptive Optics (`AO`) & Atmosphere (`ATMOS`):**
   * `ATMOS` free-space turbulence degrades fidelity by **`-25%` per tile** unless wavefront-locked by `AO` (`-5%` per tile).
9. **Entanglement Distillation Node (`DISTILL`):**
   * Consumes two valid `1550nm` photons arriving within `<= 3 frames` (`15ps`) and emits a single purified `1550nm` photon at **`95%` fidelity**.
10. **Mirrors (`MIRROR_SLASH` `/` and `MIRROR_BACK` `\`):**
    * Reflect photons 90° without loss.
11. **Target Detectors:**
    * **SNSPD (1.5K):** Requires `1550nm`, `noise <= 10`, and `fidelity >= 60%`.
    * **BSM (50:50 Beam Splitter):** Requires two valid `1550nm` photons to arrive within **`<= 3 frames` (`15ps`)** with matching polarization (`pol A == pol B`) to trigger Hong-Ou-Mandel (HOM) interference.
    * **QPU (`>=90%`):** Fault-tolerant quantum processor target requiring `1550nm`, `noise <= 10`, and **`fidelity >= 90%`**.

---

## Current Implementation Status
* **Levels Implemented (Hotkeys `1`–`9`, `0`, `-`, `[` / `]`, or clickable pills):**
  * **Level 1 (The Wavelength Barrier):** Introduces `QFC` and `VBG` before `FIBER` and `SNSPD`.
  * **Level 2 (Cryostat Obstacles):** Introduces `WALL` shielding and 45° mirrors.
  * **Level 3 (Long-Haul Attenuation):** 5 fiber tiles (`-50%` total loss), requiring a mid-link `MEMORY` node.
  * **Level 4 (Entanglement Swapping / HOM):** Dual emitters (`ION A` & `ION B`) on asymmetric paths, requiring `DELAY` to equalize arrival times at the `BSM 50:50`.
  * **Level 5 (The Repeater Node):** Asymmetric long-haul BSM combining a 5-tile fiber arm (requiring mid-link `MEMORY`) with a shorter arm (requiring `DELAY` + mirrors) to match arrival timing at `0ps`.
  * **Level 6 (Cross-Axis Multiplexing):** Dual perpendicular emitters (`RIGHT` and `DOWN`) with only 1 `VBG` filter, requiring both beams to cross through the same `VBG` tile before syncing at the `BSM 50:50`.
  * **Level 7 (Classical Coexistence / WDM):** Introduces `LIT_FIBER` (`+15` Raman noise/tile), requiring filtering *after* the lit fiber link.
  * **Level 8 (Polarization Drift / PMD):** Introduces `PMD_FIBER` and `HWP`, requiring polarization alignment alongside `0ps` HOM timing at the `BSM 50:50`.
  * **Level 9 (Entanglement Distillation):** Introduces `DISTILL` (fusing two simultaneous `>=60%` photons into one `95%` photon) to satisfy the strict `QPU >=90%` detector.
  * **Level 10 (Hybrid Quantum Network):** Connects a `780nm` Trapped-Ion emitter and a `637nm` Diamond NV emitter (`+70` QFC noise requiring 2 `VBG`s) into `1550nm` at the `BSM 50:50`.
  * **Level 11 (Satellite Downlink / ATMOS):** Introduces turbulent `ATMOS` free-space tiles (`-25%` loss) stabilized by `AO` (`-5%` loss) before coupling into ground-station fiber.
  * **Level 12 (BB84 QKD & AES-256):** Introduces prepare-and-measure BB84 QKD (`QKD TX` → `BOB RX`), requiring `QFC`, `VBG`, and `HWP` (to fix `PMD_FIBER` polarization basis drift) so `QBER <= 11%` and a real FIPS-197 AES-256 key is derived.
  * **Level 13 (Entangled QKD / E91):** Introduces a central `EPR` entangled Bell-pair source distributing pairs simultaneously to `ALICE RX` and `BOB RX`, verifying Bell violation (`CHSH S > 2.0`) and identical AES-256 key derivation on both ends.
  * **Level 14 (Eavesdropper & AES Link):** Introduces an `EVE` intercept-resend fiber tap (`QBER ~25%`, `CHSH S <= 2.0`, key aborted) requiring players to bypass Eve via 45° mirrors to establish a secure AES-256 channel.
  * **Level 15 (Multi-Hop Repeater Chain):** Introduces the heralded `REPEATER` node (Hop-1 BSM checking `dt <= 15ps` and matching polarization `pol A == pol B`), triggering a classical feedforward herald that releases a fresh `100%` fidelity Hop-2 `1550nm` qubit through `LIT_FIBER` into a fault-tolerant `QPU >=90%`.
  * **Level 16 (Satellite E91 QKD):** Grand Finale combining an orbital `EPR` source, dual `ATMOS` + `AO` downlinks, an `EVE` tap bypass through `LIT_FIBER` (`VBG` after lit fiber) and `PMD_FIBER` (`HWP`), and live `CHSH S > 2.0` + FIPS-197 AES-256 encryption.
  * **Interactive Free-Build Sandbox Mode (`S` key / `SANDBOX` pill):** Unlimited optics, channels, rotatable emitters, EPR sources, all 5 detector modes, starter templates, campaign cloning, and portable JSON blueprint sharing.
* **Graphics, Audio & UX:**
  * Machined optical breadboard grid with mounting holes and hover precision reticle (`gx,gy`).
  * Custom schematic renderers for all hardware components, including emitter direction chevrons.
  * Translucent hover ghost preview before placing components.
  * Multi-layered alpha glow caches, tapered laser trails, Raman noise jitter, and particle/shockwave FX.
  * Live oscilloscope telemetry deck in the bottom diagnostic panel: Fidelity trace (with `60%` threshold), Raman noise trace + 8-segment LED meter (with `MAX 10` threshold), SNSPD spectrum analyzer & lock LEDs (`SINGLE` mode), and HOM coincidence dip curve with `±15ps` interference window (`BSM` mode).
  * Procedural lab audio synthesizer ([audio.py](file:///Users/sysala/Development/python/qn_bench/audio.py)) using stdlib PCM synthesis + `pygame.mixer` (zero `numpy`/external asset dependencies, with silent fallback and mute toggle).
  * Controls: `L-Click` place, `R-Click` remove, `SPACE` fire laser, `R` reset level, `M` mute/unmute SFX, `1-9` / `0` / `-` / `[` `]` (or clickable level pills) jump to level.

---

## Roadmap & Next Steps
Potential directions to expand the project:
1. **New Hardware & Levels:**
   * **Level 5 (QKD & Eavesdropping):** Introduce polarization bases (`+` / `x`) and an eavesdropper (`Eve`) spiking QBER.
   * **Level 6 (Multi-Hop Quantum Repeater):** Combine `MEMORY` buffering + `BSM` swapping across two sequential links with classical feedforward heralding.
2. **Code Refactoring:** Split `qn_bench.py` into `config.py`, `levels.py`, `entities.py`, and `renderer.py` as new hardware/levels are added.

---

## Agent Guidelines
* Keep all new mechanics grounded in real quantum networking hardware and physics.
* Maintain clean, readable Python with zero external asset dependencies unless explicitly requested.
* Always verify level solvability mathematically (fiber tile count vs. `-10%` attenuation, and path-length parity for BSM levels) before finalizing level definitions.
