# QN Bench — Future Roadmap & Expansion Plan

Current shipped release: **`v2.10.3`** (20-level campaign, Free-Build Sandbox + `#bp=` share links, Live QKD/AES-256 inspector, auto-firing `STEP` Frame Debugger, UI Help Guide `[F1]`, Searchable Terminology Glossary `[T]`, fixed-square dynamic-grid scrollable optical bench, 1–3 Star Efficiency Ratings & Lab Report with Reset Progress, Touch Erase `[X]` & Colorblind Accessibility `[C]`, PolyForm Noncommercial 1.0.0 / CC BY-NC-SA 4.0 licensing, and all 20/20 In-App YouTube Lab Video Guides).

---

## Completed Follow-Up
- [x] **All 20/20 YouTube Lab Videos Uploaded & Live** (`docs/videos.json` updated with unique links for Levels `1–20`; both WebAssembly `play/` and installed Desktop apps automatically fetch all 20 guides).
  - Optional future touch: create a YouTube playlist for all 20 guides and populate `"playlist_url"` in `docs/videos.json`.

---

## Next Feature Milestones (Prioritized)

### Milestone A — Classroom Puzzle Creator (`#challenge=...` Shareable Assignments)
* **Goal:** Turn QN Bench into an educator authoring tool where instructors can design custom playable puzzles with fixed optics, walls, restricted component inventories, and custom goal prompts — shared as a single clickable WebAssembly URL (`https://skcalanderson2.github.io/qn_bench/play/#challenge=...`).
* **Key Tasks:**
  1. Extend `SandboxBlueprint` (or add `ChallengeBlueprint` in `src/model.rs`) with `inventory: BTreeMap<Placeable, u32>`, `goal: String`, `par: usize`, and `locked_fixed: bool`.
  2. Add an **"EXPORT AS PLAYABLE CHALLENGE"** tab/button in the Sandbox Share modal (`src/ui_modals.rs`) that lets authors specify component budgets and copies a `#challenge=<base64url>` link.
  3. On load (`src/app.rs` startup + live `hashchange`), launch into a locked Challenge Mode enforcing the custom inventory & par rating, with a verifiable completion summary students can paste into assignments.

### Milestone B — Live Bloch / Poincaré Sphere & Real-World Link-Budget Overlay
* **Goal:** Give students a deeper physical intuition for polarization states and fiber attenuation while using the `+5ps` Frame Debugger.
* **Key Tasks:**
  1. **Poincaré / Bloch Sphere Visualizer:** Compact procedural 2D-projected sphere rendered via `egui::Painter` when inspecting/stepping photons on PMD, QKD, or Teleportation levels, animating state rotations (`|+>` ↔ `|x>`, `|Psi>` projection on `TELE-BSM`, and state collapse at `EVE`).
  2. **Real-World Fiber Distance Translator:** Display equivalent `SMF-28` telecom fiber distance (`0.2 dB/km` at `1550nm` vs `~4 dB/km` at `780nm`) in the telemetry deck / Lab Report so tile counts connect to real-world metropolitan and intercontinental spans.

### Milestone C — Quantum Internet Act III (Levels 21–24)
* **Goal:** Introduce cutting-edge quantum networking hardware and protocols beyond Act II.
* **Planned Mechanics & Levels:**
  1. **Level 21 — Time-Bin Encoding & Franson Interferometer:** Encode qubits in early/late time-bins to traverse heavy PMD links immune to polarization drift.
  2. **Level 22 — Optical Quantum Switch (`ROUTER`):** Dynamically route single photons across multi-destination branches.
  3. **Level 23 — 3-Node GHZ Entanglement Distribution:** Synchronize **3 photons** simultaneously across a star topology (`dt <= 15ps` across all three arms).
  4. **Level 24 — Decoy-State BB84 vs. Photon-Number Splitting (PNS):** Introduce weak coherent pulses where Eve exploits multi-photon pulses unless decoy intensities are enabled.
* **Companion Videos:** Add Levels 21–24 scripts to `scripts/generate_lab_videos.py`, generate 1080p YouTube guides, and link them in `docs/videos.json`.

### Milestone D — Completion Certificate & Website Video Curriculum Showcase
* **Goal:** Reward campaign completion and showcase the full 20-lab experience on the public download site.
* **Key Tasks:**
  1. **Quantum Network Engineer Certificate:** Unlock a downloadable procedural SVG/HTML certificate inside the **Quantum Lab Report** modal (`src/ui_modals.rs`) once all 20 levels are completed (displaying custom name, total stars `/60`, completion date, and verification hash).
  2. **Website Showcase (`docs/index.html`):** Add a browsable **20-Lab Video Curriculum** gallery linked directly to the YouTube playlist/videos, and refresh screenshots in `docs/images/` to showcase Level 20 (Quantum Teleportation), Quantum Lab Report, A11Y Colorblind Mode, and the embedded YouTube Video Guide overlay.

### Milestone E — Bench Ergonomics & Lab Grant Budget Mode
* **Goal:** Quality-of-life upgrades for power builders and competitive classroom play.
* **Key Tasks:**
  1. **Click-and-Drag Paint / Erase** in Sandbox Mode for rapidly laying down fiber spans and cryostat walls.
  2. **Optional Lab Grant Budget Challenge:** Realistic hardware pricing per optic (e.g., `Mirror` \$500, `VBG` \$4k, `PPLN QFC` \$35k, `Q-Memory` \$80k) displayed as an optional efficiency metric alongside 1–3 star par ratings.
