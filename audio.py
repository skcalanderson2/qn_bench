"""Procedural lab audio synthesizer for 1550nm — Quantum Networking Optical Bench.

Generates 100% procedural 16-bit PCM waveforms using only the Python standard
library and plays them through pygame.mixer (with graceful silent fallback if
no audio device is available).
"""

import array
import math
import random
import pygame


def _clamp16(val):
    if val > 32767:
        return 32767
    if val < -32768:
        return -32768
    return int(val)


def _envelope(t, dur, attack=0.006, release=0.04):
    if t < attack and attack > 0:
        return t / attack
    rem = dur - t
    if rem < release and release > 0:
        return max(0.0, rem / release)
    return 1.0


def _build_pcm(duration_s, sample_rate, channels, sample_fn, volume=0.35):
    """Synthesize a 16-bit signed PCM buffer matching the active mixer format."""
    n_samples = max(1, int(duration_s * sample_rate))
    buf = array.array("h")
    scale = 32767.0 * volume
    for i in range(n_samples):
        t = i / sample_rate
        s = _clamp16(sample_fn(t, duration_s) * scale)
        buf.append(s)
        if channels == 2:
            buf.append(s)
    return buf.tobytes()


class SoundEngine:
    """Self-contained procedural SFX engine for optical-bench interactions."""

    def __init__(self):
        self.enabled = False
        self.muted = False
        self.sounds = {}
        self.sample_rate = 44100
        self.channels = 1

        try:
            if not pygame.mixer.get_init():
                pygame.mixer.init(frequency=44100, size=-16, channels=1, buffer=512)
            cfg = pygame.mixer.get_init()
            if cfg:
                self.sample_rate, _, self.channels = cfg
                self.enabled = True
                self._synthesize_all()
        except pygame.error:
            self.enabled = False

    def toggle_mute(self):
        self.muted = not self.muted
        return self.muted

    def play(self, name):
        if not self.enabled or self.muted:
            return
        snd = self.sounds.get(name)
        if snd is not None:
            snd.play()

    def _make(self, duration_s, fn, volume=0.32):
        raw = _build_pcm(duration_s, self.sample_rate, self.channels, fn, volume=volume)
        return pygame.mixer.Sound(buffer=raw)

    def _synthesize_all(self):
        rng = random.Random(1550)

        # 1. Tactile breadboard placement click
        def s_place(t, d):
            env = math.exp(-t * 65.0) * _envelope(t, d, 0.002, 0.015)
            return env * (0.7 * math.sin(math.tau * 760 * t) + 0.3 * math.sin(math.tau * 1520 * t))

        # 2. Component removal unclick
        def s_remove(t, d):
            env = math.exp(-t * 55.0) * _envelope(t, d, 0.002, 0.02)
            freq = 540 - 180 * (t / d)
            return env * math.sin(math.tau * freq * t)

        # 3. Soft UI selection tick
        def s_select(t, d):
            env = math.exp(-t * 90.0) * _envelope(t, d, 0.001, 0.01)
            return env * math.sin(math.tau * 1120 * t)

        # 4. Trapped-ion 780nm laser pulse chirp
        def s_fire(t, d):
            env = math.exp(-t * 14.0) * _envelope(t, d, 0.004, 0.04)
            # Smooth exponential chirp from 1380Hz down to 480Hz
            phase = math.tau * (480 * t + (900 / 18.0) * (1.0 - math.exp(-18.0 * t)))
            return env * (0.8 * math.sin(phase) + 0.2 * math.sin(phase * 2.0))

        # 5. PPLN QFC nonlinear frequency conversion shimmer
        def s_qfc(t, d):
            env = _envelope(t, d, 0.012, 0.07) * math.exp(-t * 6.0)
            p = t / d
            f1 = 330 + 330 * p
            f2 = 495 + 495 * p
            return env * (0.55 * math.sin(math.tau * f1 * t) + 0.35 * math.sin(math.tau * f2 * t))

        # 6. VBG filter noise-stripping sweep
        noise_table = [rng.uniform(-1.0, 1.0) for _ in range(int(0.16 * self.sample_rate) + 4)]

        def s_vbg(t, d):
            env = _envelope(t, d, 0.006, 0.05) * math.exp(-t * 11.0)
            idx = min(len(noise_table) - 1, int(t * self.sample_rate))
            f = 1650 - 900 * (t / d)
            tone = 0.7 * math.sin(math.tau * f * t)
            hiss = 0.25 * noise_table[idx] * (1.0 - t / d)
            return env * (tone + hiss)

        # 7. Quantum Memory crystalline chime (NV spin buffer)
        def s_memory(t, d):
            env = _envelope(t, d, 0.008, 0.12) * math.exp(-t * 7.5)
            return env * (
                0.55 * math.sin(math.tau * 587.33 * t)
                + 0.30 * math.sin(math.tau * 1174.66 * t)
                + 0.15 * math.sin(math.tau * 880.0 * t)
            )

        # 8. Optical Delay Line subtle coil double-pulse
        def s_delay(t, d):
            p1 = math.exp(-((t - 0.025) / 0.012) ** 2)
            p2 = math.exp(-((t - 0.075) / 0.014) ** 2)
            return (p1 + 0.85 * p2) * math.sin(math.tau * 740 * t)

        # 9. Mirror 90-degree reflection ping
        def s_mirror(t, d):
            env = math.exp(-t * 48.0) * _envelope(t, d, 0.001, 0.015)
            return env * (0.75 * math.sin(math.tau * 2180 * t) + 0.25 * math.sin(math.tau * 3270 * t))

        # 10. SNSPD single-photon detection click + confirmation triad
        def s_snspd_ok(t, d):
            click = math.exp(-t * 140.0) * math.sin(math.tau * 2600 * t) * 0.5
            env = _envelope(t, d, 0.01, 0.16) * math.exp(-t * 4.5)
            chord = (
                0.40 * math.sin(math.tau * 523.25 * t)
                + 0.32 * math.sin(math.tau * 659.25 * t)
                + 0.28 * math.sin(math.tau * 783.99 * t)
            )
            return click + env * chord

        # 11. BSM Hong-Ou-Mandel interference harmonic bell chord
        def s_bsm_ok(t, d):
            env = _envelope(t, d, 0.012, 0.20) * math.exp(-t * 3.6)
            shimmer = 1.0 + 0.08 * math.sin(math.tau * 8.0 * t)
            chord = (
                0.34 * math.sin(math.tau * 523.25 * t)
                + 0.26 * math.sin(math.tau * 783.99 * t)
                + 0.24 * math.sin(math.tau * 1046.50 * t)
                + 0.16 * math.sin(math.tau * 1318.51 * t)
            )
            return env * shimmer * chord

        # 12. Lab fault / decoherence warning buzz
        def s_fail(t, d):
            env = _envelope(t, d, 0.006, 0.08) * math.exp(-t * 5.0)
            s1 = math.sin(math.tau * 148 * t) + 0.35 * math.sin(math.tau * 444 * t)
            s2 = math.sin(math.tau * 156 * t) + 0.25 * math.sin(math.tau * 468 * t)
            return env * 0.45 * (s1 + s2)

        # 13. Half-Wave Plate polarization rotation ping
        def s_hwp(t, d):
            env = _envelope(t, d, 0.003, 0.03) * math.exp(-t * 28.0)
            f = 1280 + 480 * (t / d)
            return env * (0.7 * math.sin(math.tau * f * t) + 0.3 * math.sin(math.tau * f * 1.5 * t))

        # 14. Entanglement Distillation purification shimmer
        def s_distill(t, d):
            env = _envelope(t, d, 0.01, 0.14) * math.exp(-t * 4.5)
            p = t / d
            return env * (
                0.45 * math.sin(math.tau * (440 + 440 * p) * t)
                + 0.35 * math.sin(math.tau * (659.25 + 330 * p) * t)
                + 0.20 * math.sin(math.tau * 1318.5 * t)
            )

        # 15. Adaptive Optics wavefront lock chime
        def s_ao(t, d):
            env = _envelope(t, d, 0.005, 0.06) * math.exp(-t * 16.0)
            return env * (0.6 * math.sin(math.tau * 880 * t) + 0.4 * math.sin(math.tau * 1320 * t))

        self.sounds = {
            "place": self._make(0.055, s_place, 0.28),
            "remove": self._make(0.060, s_remove, 0.24),
            "select": self._make(0.035, s_select, 0.16),
            "fire": self._make(0.150, s_fire, 0.26),
            "qfc": self._make(0.180, s_qfc, 0.24),
            "vbg": self._make(0.150, s_vbg, 0.24),
            "memory": self._make(0.280, s_memory, 0.26),
            "delay": self._make(0.115, s_delay, 0.22),
            "mirror": self._make(0.070, s_mirror, 0.22),
            "snspd_ok": self._make(0.380, s_snspd_ok, 0.30),
            "bsm_ok": self._make(0.480, s_bsm_ok, 0.30),
            "fail": self._make(0.240, s_fail, 0.28),
            "hwp": self._make(0.095, s_hwp, 0.22),
            "distill": self._make(0.320, s_distill, 0.28),
            "ao": self._make(0.140, s_ao, 0.22),
        }
