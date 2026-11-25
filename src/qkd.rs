//! FIPS-197 AES-256-CTR cryptography and BB84 / E91 (BBM92) QKD protocol engine.

use crate::model::{Pol, TargetMode};
use aes::cipher::{BlockEncrypt, KeyInit, KeyIvInit, StreamCipher};
use aes::Aes256;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

type Aes256Ctr = ctr::Ctr128BE<Aes256>;

#[allow(dead_code)]
pub fn aes256_encrypt_block(key: &[u8; 32], block: &[u8; 16]) -> [u8; 16] {
    let cipher = Aes256::new(key.into());
    let mut out = *block;
    cipher.encrypt_block((&mut out).into());
    out
}

pub fn aes256_ctr_crypt(key: &[u8; 32], nonce: &[u8; 16], data: &[u8]) -> Vec<u8> {
    let mut buf = data.to_vec();
    let mut cipher = Aes256Ctr::new(key.into(), nonce.into());
    cipher.apply_keystream(&mut buf);
    buf
}

pub fn privacy_amplify_aes256(bits: &[u8]) -> [u8; 32] {
    let bit_str: String = bits.iter().map(|&b| if b != 0 { '1' } else { '0' }).collect();
    let payload = format!("QN-BENCH-QKD-v1:{}:{bit_str}", bits.len());
    let digest = Sha256::digest(payload.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(&digest);
    key
}

pub fn hex_upper(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(&mut s, "{b:02X}");
    }
    s
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QkdVerdict {
    Ready,
    Running,
    Secure,
    AbortEve,
    AbortNoKey,
}

#[derive(Debug, Clone, Copy)]
struct PairState {
    bit_plus: u8,
    bit_x: u8,
}

#[derive(Debug, Clone, Copy)]
struct RxRecord {
    basis: Pol,
    bit: u8,
}

#[derive(Debug, Clone)]
pub struct QkdRoundView {
    pub a_basis: Pol,
    pub b_basis: Pol,
    pub b_bit: u8,
    pub matched: bool,
    pub error: bool,
}

pub struct QkdSession {
    pub protocol: TargetMode,
    pub plaintext: String,
    pub nonce: [u8; 16],
    rng: StdRng,
    pub total_emitted: usize,
    pair_states: HashMap<usize, PairState>,
    alice_records: HashMap<usize, RxRecord>,
    bob_records: HashMap<usize, RxRecord>,
    pub rounds: Vec<QkdRoundView>,
    pub sifted_alice: Vec<u8>,
    pub sifted_bob: Vec<u8>,
    pub sifted_errors: usize,
    pub eve_intercepts: usize,
    pub dropped_photons: usize,
    pub qber: f64,
    pub qber_history: Vec<(usize, f64)>,
    pub chsh_s: Option<f64>,
    pub alice_key: Option<[u8; 32]>,
    pub bob_key: Option<[u8; 32]>,
    pub ciphertext_hex: String,
    pub decrypted_text: String,
    pub verdict: QkdVerdict,
}

impl QkdSession {
    pub const QBER_THRESHOLD: f64 = 0.11;
    pub const MIN_SIFTED_BITS: usize = 12;

    pub fn new(protocol: TargetMode, plaintext: &str) -> Self {
        let digest = Sha256::digest(b"qn-bench-ctr-nonce-1550nm");
        let mut nonce = [0u8; 16];
        nonce.copy_from_slice(&digest[..16]);
        let mut s = Self {
            protocol,
            plaintext: plaintext.to_string(),
            nonce,
            rng: StdRng::from_entropy(),
            total_emitted: 0,
            pair_states: HashMap::new(),
            alice_records: HashMap::new(),
            bob_records: HashMap::new(),
            rounds: Vec::new(),
            sifted_alice: Vec::new(),
            sifted_bob: Vec::new(),
            sifted_errors: 0,
            eve_intercepts: 0,
            dropped_photons: 0,
            qber: 0.0,
            qber_history: Vec::new(),
            chsh_s: if protocol == TargetMode::QkdE91 { Some(2.83) } else { None },
            alice_key: None,
            bob_key: None,
            ciphertext_hex: String::new(),
            decrypted_text: String::new(),
            verdict: QkdVerdict::Ready,
        };
        s.reset();
        s
    }

    #[allow(dead_code)]
    pub fn reseed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    pub fn reset(&mut self) {
        self.total_emitted = 0;
        self.pair_states.clear();
        self.alice_records.clear();
        self.bob_records.clear();
        self.rounds.clear();
        self.sifted_alice.clear();
        self.sifted_bob.clear();
        self.sifted_errors = 0;
        self.eve_intercepts = 0;
        self.dropped_photons = 0;
        self.qber = 0.0;
        self.qber_history.clear();
        self.chsh_s = if self.protocol == TargetMode::QkdE91 { Some(2.83) } else { None };
        self.alice_key = None;
        self.bob_key = None;
        self.ciphertext_hex.clear();
        self.decrypted_text.clear();
        self.verdict = QkdVerdict::Ready;
    }

    fn rand_basis(&mut self) -> Pol {
        if self.rng.gen_bool(0.5) { Pol::Plus } else { Pol::Cross }
    }

    fn rand_bit(&mut self) -> u8 {
        if self.rng.gen_bool(0.5) { 1 } else { 0 }
    }

    pub fn prepare_round(&mut self, pair_id: usize) -> (Pol, u8) {
        let basis = self.rand_basis();
        let bit = self.rand_bit();
        self.total_emitted += 1;

        if self.protocol == TargetMode::QkdE91 {
            // Basis-invariant Bell state |Phi+> = (|00> + |11>)/sqrt(2) = (|++> + |-->)/sqrt(2)
            let bit_x = self.rand_bit();
            self.pair_states.insert(pair_id, PairState { bit_plus: bit, bit_x });
        } else {
            self.pair_states.insert(pair_id, PairState { bit_plus: bit, bit_x: bit });
            self.alice_records.insert(
                pair_id,
                RxRecord { basis, bit },
            );
        }
        (basis, bit)
    }

    pub fn eve_intercept(
        &mut self,
        pair_id: usize,
        current_basis: Pol,
        current_bit: u8,
        pmd_flipped: bool,
    ) -> (Pol, u8) {
        let eve_basis = self.rand_basis();
        let st = self.pair_states.get(&pair_id).copied();
        let eve_bit = if self.protocol == TargetMode::QkdE91 {
            let eff = if pmd_flipped { eve_basis.flipped() } else { eve_basis };
            match (st, eff) {
                (Some(s), Pol::Plus) => s.bit_plus,
                (Some(s), Pol::Cross) => s.bit_x,
                _ => current_bit,
            }
        } else if eve_basis == current_basis {
            current_bit
        } else {
            self.rand_bit()
        };
        (eve_basis, eve_bit)
    }

    pub fn measure_photon(
        &mut self,
        role: char,
        pair_id: usize,
        arriving_basis: Pol,
        arriving_bit: u8,
        pmd_flipped: bool,
        eve_tapped: bool,
    ) {
        if eve_tapped {
            self.eve_intercepts += 1;
        }
        let rx_basis = self.rand_basis();
        let st = self.pair_states.get(&pair_id).copied();

        let measured_bit = if self.protocol == TargetMode::QkdE91 && !eve_tapped {
            let eff = if pmd_flipped { rx_basis.flipped() } else { rx_basis };
            match (st, eff) {
                (Some(s), Pol::Plus) => s.bit_plus,
                (Some(s), Pol::Cross) => s.bit_x,
                _ => arriving_bit,
            }
        } else if rx_basis == arriving_basis {
            arriving_bit
        } else {
            self.rand_bit()
        };

        let rec = RxRecord { basis: rx_basis, bit: measured_bit };
        if role == 'A' {
            self.alice_records.insert(pair_id, rec);
        } else {
            self.bob_records.insert(pair_id, rec);
        }

        if self.alice_records.contains_key(&pair_id) && self.bob_records.contains_key(&pair_id) {
            self.reconcile_pair(pair_id);
        }
    }

    pub fn record_drop(&mut self) {
        self.dropped_photons += 1;
    }

    fn reconcile_pair(&mut self, pair_id: usize) {
        let a = self.alice_records[&pair_id];
        let b = self.bob_records[&pair_id];
        let matched = a.basis == b.basis;
        let mut error = false;
        if matched {
            self.sifted_alice.push(a.bit);
            self.sifted_bob.push(b.bit);
            if a.bit != b.bit {
                self.sifted_errors += 1;
                error = true;
            }
            self.qber = self.sifted_errors as f64 / self.sifted_alice.len().max(1) as f64;
            self.qber_history.push((self.sifted_alice.len(), self.qber * 100.0));
        }

        self.rounds.push(QkdRoundView {
            a_basis: a.basis,
            b_basis: b.basis,
            b_bit: b.bit,
            matched,
            error,
        });

        if self.protocol == TargetMode::QkdE91 && !self.sifted_alice.is_empty() {
            let mut vis = (1.0 - 2.0 * self.qber).max(0.0);
            if self.eve_intercepts > 0 {
                vis = vis.min(0.55);
            }
            let s = 2.0 * std::f64::consts::SQRT_2 * vis;
            self.chsh_s = Some((s * 100.0).round() / 100.0);
        }
    }

    pub fn finalize(&mut self) -> (bool, String) {
        let n_sifted = self.sifted_alice.len();
        if n_sifted < Self::MIN_SIFTED_BITS {
            self.verdict = QkdVerdict::AbortNoKey;
            self.alice_key = None;
            self.bob_key = None;
            self.ciphertext_hex.clear();
            self.decrypted_text.clear();
            return (
                false,
                format!(
                    "FAILURE: Only {n_sifted} sifted bits (< {}). Check fiber/QFC/VBG on both arms.",
                    Self::MIN_SIFTED_BITS
                ),
            );
        }

        let bell_ok = match (self.protocol, self.chsh_s) {
            (TargetMode::QkdE91, Some(s)) => s > 2.0,
            (TargetMode::QkdE91, None) => false,
            _ => true,
        };

        if self.qber > Self::QBER_THRESHOLD || !bell_ok {
            self.verdict = QkdVerdict::AbortEve;
            self.alice_key = None;
            self.bob_key = None;
            self.ciphertext_hex.clear();
            self.decrypted_text.clear();
            if self.eve_intercepts > 0 {
                let s_msg = self
                    .chsh_s
                    .map(|s| format!(", CHSH S={s:.2} <= 2.0."))
                    .unwrap_or_else(|| ".".to_string());
                return (
                    false,
                    format!(
                        "SECURITY ABORT! Eve tap detected — QBER {:.1}% > 11%{s_msg} Reroute around EVE!",
                        self.qber * 100.0
                    ),
                );
            }
            return (
                false,
                format!(
                    "SECURITY ABORT! QBER {:.1}% > 11% (polarization basis drift). Add HWP!",
                    self.qber * 100.0
                ),
            );
        }

        self.verdict = QkdVerdict::Secure;
        self.alice_key = Some(privacy_amplify_aes256(&self.sifted_alice));
        self.bob_key = Some(privacy_amplify_aes256(&self.sifted_bob));
        self.re_encrypt();
        let s_tag = self
            .chsh_s
            .map(|s| format!(" | CHSH S={s:.2}"))
            .unwrap_or_default();
        (
            true,
            format!(
                "QKD SECURE! {n_sifted} sifted bits | QBER {:.1}%{s_tag} | AES-256 Key Derived! [K] Inspect",
                self.qber * 100.0
            ),
        )
    }

    pub fn re_encrypt(&mut self) {
        let (Some(ak), Some(bk)) = (self.alice_key, self.bob_key) else {
            self.ciphertext_hex.clear();
            self.decrypted_text.clear();
            return;
        };
        let ct = aes256_ctr_crypt(&ak, &self.nonce, self.plaintext.as_bytes());
        let dec = aes256_ctr_crypt(&bk, &self.nonce, &ct);
        self.ciphertext_hex = hex_upper(&ct);
        self.decrypted_text = String::from_utf8_lossy(&dec).into_owned();
    }
}
