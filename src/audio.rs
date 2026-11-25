//! Procedural 44.1 kHz PCM audio synthesizer for QN Bench.
//! Zero external audio assets: synthesizes 17 waveforms in pure Rust and plays
//! them via rodio (native desktop) or Web Audio API (WebAssembly).

use crate::sim::SfxEvent;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::collections::HashMap;
use std::f32::consts::TAU;

const SAMPLE_RATE: u32 = 44_100;

fn envelope(t: f32, dur: f32, attack: f32, release: f32) -> f32 {
    if t < attack && attack > 0.0 {
        return t / attack;
    }
    let rem = dur - t;
    if rem < release && release > 0.0 {
        return (rem / release).max(0.0);
    }
    1.0
}

fn synth(duration_s: f32, volume: f32, f: impl Fn(f32, f32) -> f32) -> Vec<f32> {
    let n = ((duration_s * SAMPLE_RATE as f32) as usize).max(1);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let t = i as f32 / SAMPLE_RATE as f32;
        out.push((f(t, duration_s) * volume).clamp(-1.0, 1.0));
    }
    out
}

fn build_all_waveforms() -> HashMap<SfxEvent, Vec<f32>> {
    let mut rng = StdRng::seed_from_u64(1550);
    let noise_len = (0.16 * SAMPLE_RATE as f32) as usize + 4;
    let noise_table: Vec<f32> = (0..noise_len).map(|_| rng.gen_range(-1.0..1.0)).collect();

    let mut m = HashMap::new();
    m.insert(SfxEvent::Place, synth(0.055, 0.28, |t, d| {
        let env = (-t * 65.0).exp() * envelope(t, d, 0.002, 0.015);
        env * (0.7 * (TAU * 760.0 * t).sin() + 0.3 * (TAU * 1520.0 * t).sin())
    }));
    m.insert(SfxEvent::Remove, synth(0.060, 0.24, |t, d| {
        let env = (-t * 55.0).exp() * envelope(t, d, 0.002, 0.02);
        let freq = 540.0 - 180.0 * (t / d);
        env * (TAU * freq * t).sin()
    }));
    m.insert(SfxEvent::Select, synth(0.035, 0.16, |t, d| {
        let env = (-t * 90.0).exp() * envelope(t, d, 0.001, 0.01);
        env * (TAU * 1120.0 * t).sin()
    }));
    m.insert(SfxEvent::Fire, synth(0.150, 0.26, |t, d| {
        let env = (-t * 14.0).exp() * envelope(t, d, 0.004, 0.04);
        let phase = TAU * (480.0 * t + (900.0 / 18.0) * (1.0 - (-18.0 * t).exp()));
        env * (0.8 * phase.sin() + 0.2 * (phase * 2.0).sin())
    }));
    m.insert(SfxEvent::Qfc, synth(0.180, 0.24, |t, d| {
        let env = envelope(t, d, 0.012, 0.07) * (-t * 6.0).exp();
        let p = t / d;
        let f1 = 330.0 + 330.0 * p;
        let f2 = 495.0 + 495.0 * p;
        env * (0.55 * (TAU * f1 * t).sin() + 0.35 * (TAU * f2 * t).sin())
    }));
    m.insert(SfxEvent::Vbg, synth(0.150, 0.24, |t, d| {
        let env = envelope(t, d, 0.006, 0.05) * (-t * 11.0).exp();
        let idx = ((t * SAMPLE_RATE as f32) as usize).min(noise_table.len() - 1);
        let f = 1650.0 - 900.0 * (t / d);
        let tone = 0.7 * (TAU * f * t).sin();
        let hiss = 0.25 * noise_table[idx] * (1.0 - t / d);
        env * (tone + hiss)
    }));
    m.insert(SfxEvent::Memory, synth(0.280, 0.26, |t, d| {
        let env = envelope(t, d, 0.008, 0.12) * (-t * 7.5).exp();
        env * (0.55 * (TAU * 587.33 * t).sin()
            + 0.30 * (TAU * 1174.66 * t).sin()
            + 0.15 * (TAU * 880.0 * t).sin())
    }));
    m.insert(SfxEvent::Delay, synth(0.115, 0.22, |t, _d| {
        let p1 = (-((t - 0.025) / 0.012).powi(2)).exp();
        let p2 = (-((t - 0.075) / 0.014).powi(2)).exp();
        (p1 + 0.85 * p2) * (TAU * 740.0 * t).sin()
    }));
    m.insert(SfxEvent::Mirror, synth(0.070, 0.22, |t, d| {
        let env = (-t * 48.0).exp() * envelope(t, d, 0.001, 0.015);
        env * (0.75 * (TAU * 2180.0 * t).sin() + 0.25 * (TAU * 3270.0 * t).sin())
    }));
    m.insert(SfxEvent::SnspdOk, synth(0.380, 0.30, |t, d| {
        let click = (-t * 140.0).exp() * (TAU * 2600.0 * t).sin() * 0.5;
        let env = envelope(t, d, 0.01, 0.16) * (-t * 4.5).exp();
        let chord = 0.40 * (TAU * 523.25 * t).sin()
            + 0.32 * (TAU * 659.25 * t).sin()
            + 0.28 * (TAU * 783.99 * t).sin();
        click + env * chord
    }));
    m.insert(SfxEvent::BsmOk, synth(0.480, 0.30, |t, d| {
        let env = envelope(t, d, 0.012, 0.20) * (-t * 3.6).exp();
        let shimmer = 1.0 + 0.08 * (TAU * 8.0 * t).sin();
        let chord = 0.34 * (TAU * 523.25 * t).sin()
            + 0.26 * (TAU * 783.99 * t).sin()
            + 0.24 * (TAU * 1046.50 * t).sin()
            + 0.16 * (TAU * 1318.51 * t).sin();
        env * shimmer * chord
    }));
    m.insert(SfxEvent::Fail, synth(0.240, 0.28, |t, d| {
        let env = envelope(t, d, 0.006, 0.08) * (-t * 5.0).exp();
        let s1 = (TAU * 148.0 * t).sin() + 0.35 * (TAU * 444.0 * t).sin();
        let s2 = (TAU * 156.0 * t).sin() + 0.25 * (TAU * 468.0 * t).sin();
        env * 0.45 * (s1 + s2)
    }));
    m.insert(SfxEvent::Hwp, synth(0.095, 0.22, |t, d| {
        let env = envelope(t, d, 0.003, 0.03) * (-t * 28.0).exp();
        let f = 1280.0 + 480.0 * (t / d);
        env * (0.7 * (TAU * f * t).sin() + 0.3 * (TAU * f * 1.5 * t).sin())
    }));
    m.insert(SfxEvent::Distill, synth(0.320, 0.28, |t, d| {
        let env = envelope(t, d, 0.01, 0.14) * (-t * 4.5).exp();
        let p = t / d;
        env * (0.45 * (TAU * (440.0 + 440.0 * p) * t).sin()
            + 0.35 * (TAU * (659.25 + 330.0 * p) * t).sin()
            + 0.20 * (TAU * 1318.5 * t).sin())
    }));
    m.insert(SfxEvent::Ao, synth(0.140, 0.22, |t, d| {
        let env = envelope(t, d, 0.005, 0.06) * (-t * 16.0).exp();
        env * (0.6 * (TAU * 880.0 * t).sin() + 0.4 * (TAU * 1320.0 * t).sin())
    }));
    m.insert(SfxEvent::Eve, synth(0.110, 0.24, |t, d| {
        let env = envelope(t, d, 0.004, 0.04) * (-t * 18.0).exp();
        let f = 220.0 + 95.0 * (TAU * 45.0 * t).sin();
        env * (0.65 * (TAU * f * t).sin() + 0.35 * (TAU * f * 2.5 * t).sin())
    }));
    m.insert(SfxEvent::AesLock, synth(0.520, 0.30, |t, d| {
        let env = envelope(t, d, 0.01, 0.22) * (-t * 3.2).exp();
        let p = (t / 0.12).min(1.0);
        let chord = 0.32 * (TAU * 440.0 * t).sin()
            + 0.26 * (TAU * 554.37 * t).sin()
            + 0.24 * (TAU * 659.25 * t).sin()
            + 0.18 * (TAU * 880.0 * t).sin() * p;
        env * chord
    }));
    m
}

#[cfg(not(target_arch = "wasm32"))]
pub struct SoundEngine {
    _stream: Option<rodio::OutputStream>,
    handle: Option<rodio::OutputStreamHandle>,
    sounds: HashMap<SfxEvent, Vec<f32>>,
    pub muted: bool,
}

#[cfg(not(target_arch = "wasm32"))]
impl SoundEngine {
    pub fn new(muted: bool) -> Self {
        let (stream, handle) = match rodio::OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(_) => (None, None),
        };
        Self {
            _stream: stream,
            handle,
            sounds: build_all_waveforms(),
            muted,
        }
    }

    pub fn is_active(&self) -> bool {
        self.handle.is_some() && !self.muted
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if !self.muted {
            self.play(SfxEvent::Select);
        }
    }

    pub fn play(&mut self, ev: SfxEvent) {
        if self.muted {
            return;
        }
        if let (Some(h), Some(samples)) = (&self.handle, self.sounds.get(&ev)) {
            let src = rodio::buffer::SamplesBuffer::new(1, SAMPLE_RATE, samples.clone());
            let _ = h.play_raw(src);
        }
    }
}

#[cfg(target_arch = "wasm32")]
struct WasmAudioState {
    ctx: Option<web_sys::AudioContext>,
    buffers: HashMap<SfxEvent, web_sys::AudioBuffer>,
}

#[cfg(target_arch = "wasm32")]
impl WasmAudioState {
    fn unlock(&mut self) {
        if self.ctx.is_none() {
            self.ctx = web_sys::AudioContext::new().ok();
        }
        if let Some(ctx) = &self.ctx {
            if ctx.state() != web_sys::AudioContextState::Running {
                let _ = ctx.resume();
                // Silent 1-sample buffer unlock for Safari / WebKit autoplay policy
                if let Ok(buf) = ctx.create_buffer(1, 1, 22_050.0) {
                    if let Ok(src) = ctx.create_buffer_source() {
                        src.set_buffer(Some(&buf));
                        let _ = src.connect_with_audio_node(&ctx.destination());
                        let _ = src.start();
                    }
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub struct SoundEngine {
    state: std::rc::Rc<std::cell::RefCell<WasmAudioState>>,
    sounds: HashMap<SfxEvent, Vec<f32>>,
    pub muted: bool,
}

#[cfg(target_arch = "wasm32")]
impl SoundEngine {
    pub fn new(muted: bool) -> Self {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;

        let state = std::rc::Rc::new(std::cell::RefCell::new(WasmAudioState {
            ctx: None,
            buffers: HashMap::new(),
        }));

        // Browser autoplay policies (Chrome, Safari, Firefox) require AudioContext
        // creation and resume() to run synchronously inside a trusted DOM user gesture,
        // NOT inside eframe's deferred requestAnimationFrame update() loop.
        if let Some(window) = web_sys::window() {
            let st_clone = std::rc::Rc::clone(&state);
            let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |_ev: web_sys::Event| {
                if let Ok(mut st) = st_clone.try_borrow_mut() {
                    st.unlock();
                }
            });
            let js_fn = cb.as_ref().unchecked_ref();
            // Capture phase (true) ensures this runs before eframe's canvas stops propagation
            for event_name in [
                "pointerdown",
                "pointerup",
                "mousedown",
                "click",
                "touchstart",
                "touchend",
                "keydown",
            ] {
                let _ = window.add_event_listener_with_callback_and_bool(event_name, js_fn, true);
            }
            cb.forget();
        }

        Self {
            state,
            sounds: build_all_waveforms(),
            muted,
        }
    }

    pub fn is_active(&self) -> bool {
        !self.muted
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        if !self.muted {
            self.play(SfxEvent::Select);
        }
    }

    pub fn play(&mut self, ev: SfxEvent) {
        if self.muted {
            return;
        }
        let Ok(mut st) = self.state.try_borrow_mut() else {
            return;
        };
        if st.ctx.is_none() {
            st.ctx = web_sys::AudioContext::new().ok();
        }
        let Some(ctx) = st.ctx.clone() else { return };
        if ctx.state() != web_sys::AudioContextState::Running {
            let _ = ctx.resume();
        }

        let buf = if let Some(existing) = st.buffers.get(&ev) {
            existing.clone()
        } else {
            let Some(samples) = self.sounds.get(&ev) else { return };
            let Ok(b) = ctx.create_buffer(1, samples.len() as u32, SAMPLE_RATE as f32) else {
                return;
            };
            if b.copy_to_channel(samples, 0).is_err() {
                return;
            }
            st.buffers.insert(ev, b.clone());
            b
        };

        let Ok(src) = ctx.create_buffer_source() else { return };
        src.set_buffer(Some(&buf));
        let _ = src.connect_with_audio_node(&ctx.destination());
        let _ = src.start();
    }
}
