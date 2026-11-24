# 1550nm — Quantum Networking Optical Bench

An educational 2D optical-bench puzzle game built in **Python + Pygame** that teaches the real physics and hardware stack of quantum networking — single-photon transduction (`780nm`/`637nm` → `1550nm`), Raman noise filtering, quantum memories, Hong-Ou-Mandel (HOM) interference, polarization mode dispersion, entanglement distillation, and adaptive optics satellite downlinks.

## Download for macOS (Signed & Notarized)

* **Download Page:** https://skcalanderson2.github.io/qn_bench/
* **GitHub Releases:** https://github.com/skcalanderson2/qn_bench/releases/latest

The macOS `.dmg` installer contains a standalone `1550nm.app` signed with an Apple Developer ID certificate and notarized by Apple.

### In-App Self-Updating
`1550nm.app` automatically checks GitHub Releases in the background on startup (or manually via the sidebar version pill / `U` key). When a newer release is available, it displays the release notes and asks for your permission before downloading, verifying the SHA-256 digest and Apple Developer signature (`TeamIdentifier`), swapping the app bundle, and relaunching.

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

## Building & Publishing a Signed macOS Release

Prerequisites (one-time on your Mac):
1. Valid `Developer ID Application` certificate installed in Keychain.
2. `notarytool` Keychain profile stored (`qn-bench-notary`).
3. GitHub CLI (`gh`) authenticated.
4. Build virtualenv initialized:
   ```bash
   python3 -m venv .venv
   .venv/bin/pip install pygame pyinstaller certifi
   ```

To build, sign, notarize, staple, package `.dmg` + `.zip` + `SHA256SUMS.txt`, and publish a new GitHub Release (which triggers the in-app updater for existing users):

```bash
./release.sh --version 1.0.1 --notes "Describe what changed in this release."
```

To test a local signed & notarized build in `dist/` without publishing to GitHub:

```bash
./release.sh --skip-publish
```
