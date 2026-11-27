# QN Bench — Quantum Networking Optical Bench

## Project Overview
**QN Bench** is an educational 2D optical-bench puzzle game built in **Rust + `eframe`/`egui`** (targeting **WebAssembly**, **macOS Universal**, and **Windows x64**). It teaches both the conceptual physics and the physical hardware stack of quantum networking — single-photon transduction, Raman noise filtering, quantum memories, Hong-Ou-Mandel (HOM) interference, polarization mode dispersion, chromatic dispersion, dichroic WDM routing, entanglement distillation, multi-hop repeaters, Bennett et al. (1993) Quantum Teleportation, and end-to-end **BB84 / E91 QKD + FIPS-197 AES-256** cryptography.

* **Current Shipped Version:** `v2.11.0` (legacy Python `v1.1.0` preserved on branch `python-v1`)
* **Repository:** `https://github.com/skcalanderson2/qn_bench`
* **Play in Browser (WASM):** `https://skcalanderson2.github.io/qn_bench/play/`
* **Download Site:** `https://skcalanderson2.github.io/qn_bench/`
* **Key Crates:** `eframe` 0.31, `egui` 0.31, `aes`, `ctr`, `sha2`, `serde`/`serde_json`, `rand`, `png`; native-only: `rodio`, `ureq`, `zip`, `webbrowser`, `tempfile`; WASM-only: `wasm-bindgen`, `wasm-bindgen-futures`, `web-sys`.
* **Asset Policy:** 100% procedural graphics & audio in the app binary (zero external image or audio files committed to `src/`). All optical schematics, breadboard holes, radial glows, particles, shockwaves, painted vector stars, app icons (`src/bin/gen_icons.rs`), and 17 lab SFX PCM waveforms are generated procedurally.

---

## Architecture Map (`src/`)
* `src/main.rs` — Native (`eframe::run_native`) & WebAssembly (`eframe::WebRunner`) entry points.
* `src/model.rs` — Core domain enums (`Dir`, `Pol`, `Placeable`, `Tile`, `TargetMode`, `SimState`, `SandboxTool`, `Overlay`), `LevelDef`, `SandboxBlueprint` + pure-Rust base64url `#bp=` share URL codec, palette constants.
* `src/levels.rs` — All **21 campaign labs (Lab 0 Orientation + Levels 1–20)**, `level_par_components(idx)` optimal optic par table, and the `HARDWARE_DOCS` encyclopedia (29 hardware specs).
* `src/glossary.rs` — Complete Quantum Networking Terminology & Abbreviation Encyclopedia (31 quantum physics & QKD/AES terms + category/search helpers complementing `HARDWARE_DOCS`).
* `src/sim.rs` — Decoupled optical-bench physics engine (`BenchSim`, `Photon`, `Particle`, `Shockwave`), dynamic viewport/edge-expanding grid bounds (`bench_cols` x `bench_rows` up to `64x48`), 64-step Undo/Redo, Live Alignment Laser tracer, auto-firing `+5ps` single-step frame debugger, 1–3 star efficiency ratings, Markdown Lab Report generator, Touch Erase primary-click handler, and 9 comprehensive unit tests covering all 20 levels + NIST FIPS-197 AES-256 vectors.
* `src/qkd.rs` — BB84 & E91/BBM92 QKD sifting, QBER & CHSH `S` computation, Eve intercept-resend simulation, SHA-256 privacy amplification, and real FIPS-197 AES-256-CTR encryption/decryption.
* `src/render_bench.rs` — Procedural `egui::Painter` optical breadboard with fixed `76px` squares (`BENCH_TILE_PX`) inside `egui::ScrollArea::both()` (`ScrollStyle::solid()`) that adds squares on window resize and adds scrollbars on edge placement, pinned `ALIGN` HUD & `GRID WxH` strip, all hardware schematics, photon trails/glows, Colorblind/High-Contrast pattern overlays (`colorblind_mode`), Touch Erase red reticle, and hover tile/photon tooltips (`Shift+Click` to open full spec).
* `src/render_scopes.rs` — Responsive bottom telemetry deck (2-row header + status strip with star badges + 3 oscilloscope panes: Fidelity / Raman Noise / Spectrum or HOM Dip or QKD/AES-256), always-enabled `STEP` button + ASCII `>`/`||` playback controls, `REPORT` button, `VIDEO` pill, and 20 level pills (`1..20` + `SANDBOX`).
* `src/ui_modals.rs` — All 8 modal overlays rendered strictly on `egui::Order::Foreground` above `modal_backdrop` (`egui::Order::Middle`): `Level` Briefing (with 2-tier progressive opt-in hints + `▶ WATCH VIDEO GUIDE [V]`), `Comp` Hardware Spec, `UiHelp` Interface Manual (`[F1]` / `?`), `Glossary` Searchable Terminology & Component Guide (`[T]`), `QkdAes` Live Cryptography Inspector, `SandboxShare` (`#bp=` URL + JSON + Starter Labs), `LabReport` (20-level star table + Copy Markdown Report), and native `Update` modal (with pure-Rust rendered Markdown release notes).
* `src/audio.rs` — 17 procedural PCM lab SFX. Native uses `rodio`; WebAssembly uses a `#[wasm_bindgen(inline_js = ...)]` Web Audio API bridge (capture-phase DOM unlock, `window.AudioContext || window.webkitAudioContext`, `new Float32Array` + `getChannelData(0).set()`, `AudioBuffer` caching, `ctx.resume().then(trigger)` + `src.start(0)`) verified working in both Chrome and Safari.
* `src/video.rs` — Per-level YouTube Lab Video Guide registry (`VideoCatalog`, reading baked-in `docs/videos.json` + live async refresh from GitHub Pages so new videos activate with zero app rebuild) + cross-platform playback (embedded `youtube-nocookie.com` DOM modal overlay on WebAssembly; default browser via `webbrowser` on Desktop).
* `src/updater.rs` — Native desktop startup + 30-minute periodic GitHub Releases self-updater (verifies SHA-256 + Apple `TeamIdentifier` on macOS); sidebar button stays hidden until an update is available, then lights up in bright amber-gold.
* `src/app.rs` — `eframe::App` shell, fixed 60 Hz simulation accumulator, sidebar palette (`? UI HELP [F1]`, `GLOSSARY [T]`, `ERASE TOOL [X]`, `A11Y PATTERNS [C]`, Sandbox tabs, bottom-anchored `allocate_ui_with_layout` playback & navigation rows including `FIRE LASER [SPACE]`, `STEP [.]`, and `PAUSE/RESUME [P]`), keyboard shortcuts, `#bp=` URL hash listener, and `eframe::Storage` persistence (`PersistedState`).
* `scripts/generate_lab_videos.py` — Automated 1080p 30fps YouTube Conceptual Lab Guide generator using Pillow + `ffmpeg` + Microsoft Edge Neural TTS (`edge-tts`, `en-US-AndrewNeural`) + synced `.srt` captions. Outputs to gitignored `videos/`.
* `docs/` — GitHub Pages download site (`docs/index.html`), live video manifest (`docs/videos.json`), and Retina Rust screenshots (`docs/images/`).

---

## Core Physics & Hardware Rules
Preserve these physical constraints when adding levels or components:

1. **Quantum Emitters (`ION 780nm` / `NV 637nm` / `|Psi> 'x'`):** Fire a single visible photon (`780nm` Trapped-Ion or `637nm` Diamond NV) at `100%` fidelity, `0` Raman noise, `0` dispersion, and `+` (or `x` for `|Psi>`) polarization.
2. **Telecom Fibers (`FIBER`, `LIT_FIBER`, `PMD_FIBER`, `CD_FIBER`):**
   * Absorb/destroy non-`1550nm` photons immediately.
   * Transmit `1550nm` (C-band) photons, degrading fidelity by **`-10%` per tile**.
   * `LIT_FIBER` (classical coexistence) also injects **`+15` Raman noise per tile**.
   * `PMD_FIBER` (birefringent) also rotates polarization **`+` ↔ `x`**.
   * `CD_FIBER` (dispersive) also adds **`+1` Chromatic Dispersion** (`dispersion > 0` rejected by detectors, `DISTILL`, `REPEATER`, and `TELE-BSM`).
3. **PPLN Quantum Frequency Converter (`QFC`):** Shifts `780nm -> 1550nm` (`+40` Raman noise) or `637nm -> 1550nm` (`+70` Raman noise). Leaves `1550nm` photons unchanged (`+0` noise).
4. **Volume Bragg Grating Filter (`VBG`):** Strips **`-50` Raman noise** (clamped at `0`). Can filter perpendicular crossing beams.
5. **Quantum Memory (`MEMORY`):** Restores photon fidelity back to **`100%`** with zero delay or noise.
6. **Optical Delay Line (`DELAY`):** Pauses a photon for **20 frames (`+100ps`)**, equal to exactly 1 tile of travel (`TILE_SIZE = 80`, `speed = 4.0`).
7. **Half-Wave Plate (`HWP`):** Rotates photon polarization **`+` ↔ `x`** without loss.
8. **Adaptive Optics (`AO`) & Atmosphere (`ATMOS`):** `ATMOS` degrades fidelity by **`-25%` per tile** unless wavefront-locked upstream by `AO` (`-5%` per tile).
9. **Dispersion Compensating Fiber (`DCF`):** Clears accumulated Chromatic Dispersion (`dispersion -> 0`) without loss; must be placed *after* `CD_FIBER`.
10. **Dichroic WDM Router (`WDM (\)`):** Transmits `1550nm` straight through; reflects visible (`780nm`/`637nm`) photons 90° like a `\` mirror.
11. **Mirrors (`MIRROR_SLASH` `/` and `MIRROR_BACK` `\`):** Reflect all wavelengths 90° without loss.
12. **Active Nodes:**
    * **Entanglement Distillation (`DISTILL`):** Consumes two valid `1550nm` photons (`fid >= 60%`, `noise <= 10`, `CD == 0`, `dt <= 15ps`) and emits a single purified `95%` photon Right (`P`).
    * **Heralded Quantum Repeater (`REPEATER`):** Hop-1 BSM checking `dt <= 15ps` and `pol A == pol B` (`fid >= 60%`, `noise <= 10`, `CD == 0`) -> releases a heralded `100%` Hop-2 `1550nm` qubit Right (`H`).
    * **Teleportation Analyzer (`TELE-BSM`):** Projects `|Psi>` and Alice's EPR photon arriving within `<= 15ps` (`1550nm`, `noise <= 10`, `fid >= 60%`, `CD == 0`) -> broadcasts classical herald projecting `|Psi>`'s polarization onto Bob's distant EPR photon in flight.
    * **Eavesdropper Tap (`EVE`):** Intercept-resend QKD tap spiking `QBER ~25%` and collapsing `CHSH S <= 2.0` (forces QKD abort; must be bypassed physically).
13. **Target Detectors:**
    * **SNSPD (1.5K):** `1550nm`, `noise <= 10`, `CD == 0`, `fidelity >= 60%`.
    * **BSM (50:50):** Two valid `1550nm` photons within `<= 3 frames` (`15ps`) with matching polarization (`pol A == pol B`).
    * **QPU (`>=90%`):** `1550nm`, `noise <= 10`, `CD == 0`, `fidelity >= 90%`.
    * **QKD RX (BB84 / E91):** `>= 12` sifted bits, `QBER <= 11%`, plus `CHSH S > 2.0` for E91 -> derives live FIPS-197 AES-256-CTR key.
    * **TELEPORT RX:** Requires `TELE-BSM` herald PLUS Bob's valid `1550nm` photon arriving with `CD == 0` and intact `|Psi>` polarization (compensated via `HWP` Pauli fix).

---

## Implemented Campaign (Lab 0 + Levels 1–20) & Sandbox
* **Orientation (Lab 0):**
  * Lab 0 Optical Bench Orientation & Terminology (`ION 780nm`, `QFC`, pre-installed `VBG`, `SNSPD`, core quantum terminology primer & Glossary quick-link).
* **Act I (Levels 1–16):**
  * L1 The Wavelength Barrier (`QFC`, `VBG`, `FIBER`, `SNSPD`) • L2 Cryostat Obstacles (`WALL`, Mirrors) • L3 Long-Haul Attenuation (`MEMORY`) • L4 Entanglement Swapping (`DELAY`, `BSM`) • L5 The Repeater Node (`MEMORY` + `DELAY`) • L6 Cross-Axis Multiplexing (shared `VBG`) • L7 Classical Coexistence (`LIT_FIBER`) • L8 Polarization Drift (`PMD_FIBER`, `HWP`) • L9 Entanglement Distillation (`DISTILL`, `QPU >=90%`) • L10 Hybrid Quantum Network (`780nm` Ion + `637nm` NV) • L11 Satellite Downlink (`ATMOS`, `AO`) • L12 BB84 QKD & AES-256 • L13 Entangled QKD (E91, `EPR`, `CHSH S > 2`) • L14 Eavesdropper & AES Link (`EVE` bypass) • L15 Multi-Hop Repeater Chain (`REPEATER`) • L16 Satellite E91 QKD (Act I Grand Finale).
* **Act II (Levels 17–20):**
  * L17 Chromatic Dispersion (`CD_FIBER`, `DCF`) • L18 WDM Wavelength Routing (`WDM (\)`) • L19 Dispersive Metro Link (`CD_FIBER` + `LIT_FIBER` + `DISTILL` -> `QPU`) • L20 Quantum Teleportation (`|Psi> 'x'` + `EPR` + `TELE-BSM` + `DCF` + `HWP` -> `TELEPORT RX`).
* **Free-Build Sandbox (`S`):** Unlimited optics/channels/rotatable emitters/EPR/detectors, starter labs, campaign cloning, JSON import/export, and playable `#bp=<base64url>` share links.
* **YouTube Lab Video Guides:** All **21/21** generated 1080p `.mp4` + `.srt` guides (Lab 0 + Levels 1–20) uploaded to YouTube and active in `docs/videos.json` (verified unique & valid in `cargo test`).

---

## Build, Verification & Release Pipeline
* **Local Verification (Sandboxed):** `cargo test --offline && cargo check --offline --target wasm32-unknown-unknown` (must pass with 0 errors, 0 warnings).
* **Full Multi-Platform Release:** `./release.sh --version X.Y.Z`
  * Compiles macOS Universal (`arm64` + `x86_64`), signs with `Developer ID Application: Al Anderson (MR38JF8F3H)` (Hardened Runtime), notarizes & staples via Keychain profile `qn-bench-notary`, verifies Gatekeeper (`spctl`), bumps `Cargo.toml`, commits/tags/pushes, and publishes the GitHub Release.
  * GitHub Actions automatically runs **Deploy GitHub Pages** (`.github/workflows/deploy-pages.yml` — 2-job `build` + `deploy`, Trunk WASM with `data-wasm-opt="0"` in `web/index.html`) and **Build Windows Release** (`.github/workflows/build-windows.yml` — unsigned x64 `.exe` + `.zip` attached to the release).
  * Follow up with `gh release edit vX.Y.Z --notes "..."` for clean Markdown release notes.

---

## Engineering Gotchas & Agent Guidelines
* **egui Default Font Glyphs:** `Ubuntu-Light` / `Hack` do **not** render `↶`, `↷`, `→`, `←`, `↑`, `↓`, `✓`, or `★` (they render as `□` tofu). Always use ASCII (`UNDO`, `REDO`, `->`, `<-`, `>`, `v`, `<`, `^`, `OK`, `*`) for UI text, or draw shapes procedurally with `egui::Painter` (like `draw_star_trio`).
* **Modal Layering (Safari / Cross-Platform):** Every modal `egui::Window` in `src/ui_modals.rs` MUST set `.order(egui::Order::Foreground)` so `modal_backdrop` (`egui::Order::Middle`) never paints on top of the modal.
* **Sidebar `bottom_up` Layout:** Always wrap fixed-height horizontal rows inside `Layout::bottom_up` with `ui.allocate_ui_with_layout(Vec2::new(w, h), Layout::left_to_right(Align::Center), ...)` to prevent clipping.
* **WASM Packaging:** Keep `data-wasm-opt="0"` in `web/index.html` (Trunk's `wasm-opt` fails on Rust bulk-memory instructions) and preserve the Safari-safe WebAudio `Float32Array` + `getChannelData(0).set()` + `resume().then()` path in `src/audio.rs`.
* **Physics Integrity:** Keep all mechanics grounded in real quantum networking physics and verify both failure modes and winning solutions in `cargo test` before releasing.
* **Future Plan:** See [ROADMAP.md](file:///Users/sysala/SynologyDrive/python/qn_bench/ROADMAP.md) for upcoming milestones.
