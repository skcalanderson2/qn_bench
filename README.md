# QN Bench — Quantum Networking Optical Bench

An educational 2D optical-bench puzzle game built in **Python + Pygame** that teaches the real physics and hardware stack of quantum networking — single-photon transduction (`780nm`/`637nm` → `1550nm`), Raman noise filtering, quantum memories, Hong-Ou-Mandel (HOM) interference, polarization mode dispersion, entanglement distillation, and adaptive optics satellite downlinks.

## Download for macOS (Signed & Notarized)

* **Download Page:** https://skcalanderson2.github.io/qn_bench/
* **GitHub Releases:** https://github.com/skcalanderson2/qn_bench/releases/latest

The macOS `.dmg` installer contains a standalone `QN Bench.app` signed with an Apple Developer ID certificate and notarized by Apple.

### In-App Self-Updating
`QN Bench.app` automatically checks GitHub Releases in the background on startup (or manually via the sidebar version pill / `U` key). When a newer release is available, it displays the release notes and asks for your permission before downloading, verifying the SHA-256 digest and Apple Developer signature (`TeamIdentifier`), swapping the app bundle, and relaunching.

---

## Screenshots

| Level 1: Wavelength Transduction & Filtering | Level 4: Entanglement Swapping (HOM) |
| --- | --- |
| ![Level 1: Wavelength Transduction](docs/images/level1-transduction.png) | ![Level 4: Entanglement Swapping](docs/images/level4-hom-swapping.png) |

| Level 9: Entanglement Distillation | Level 11: Satellite Downlink (ATMOS) |
| --- | --- |
| ![Level 9: Entanglement Distillation](docs/images/level9-distillation.png) | ![Level 11: Satellite Downlink](docs/images/level11-satellite-ao.png) |

---

## Running from Source

```bash
python3 -m venv .venv
.venv/bin/pip install pygame certifi
.venv/bin/python3 qn_bench.py
```

### Controls
* **L-Click** — Place selected optical component
* **R-Click** — Remove placed component
* **SPACE** — Fire single photon(s)
* **R** — Reset current level
* **H** — Open Experiment Briefing overlay
* **U** — Check for software updates
* **M** — Mute / unmute procedural lab audio
* **1–9, 0, -, [ / ]** — Switch levels

---

