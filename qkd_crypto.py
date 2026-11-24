"""Pure-Python FIPS-197 AES-256-CTR and QKD protocol engine for QN Bench.

Zero external dependencies: implements genuine AES-256 (14 rounds) verified
against NIST FIPS-197 Appendix C.3, SHA-256 privacy amplification, and
BB84 / E91 (BBM92) basis sifting, QBER calculation, and CHSH Bell estimation.
"""

import hashlib
import math
import random

# --- Standard FIPS-197 AES S-Box ---
_SBOX = (
    0x63, 0x7C, 0x77, 0x7B, 0xF2, 0x6B, 0x6F, 0xC5, 0x30, 0x01, 0x67, 0x2B, 0xFE, 0xD7, 0xAB, 0x76,
    0xCA, 0x82, 0xC9, 0x7D, 0xFA, 0x59, 0x47, 0xF0, 0xAD, 0xD4, 0xA2, 0xAF, 0x9C, 0xA4, 0x72, 0xC0,
    0xB7, 0xFD, 0x93, 0x26, 0x36, 0x3F, 0xF7, 0xCC, 0x34, 0xA5, 0xE5, 0xF1, 0x71, 0xD8, 0x31, 0x15,
    0x04, 0xC7, 0x23, 0xC3, 0x18, 0x96, 0x05, 0x9A, 0x07, 0x12, 0x80, 0xE2, 0xEB, 0x27, 0xB2, 0x75,
    0x09, 0x83, 0x2C, 0x1A, 0x1B, 0x6E, 0x5A, 0xA0, 0x52, 0x3B, 0xD6, 0xB3, 0x29, 0xE3, 0x2F, 0x84,
    0x53, 0xD1, 0x00, 0xED, 0x20, 0xFC, 0xB1, 0x5B, 0x6A, 0xCB, 0xBE, 0x39, 0x4A, 0x4C, 0x58, 0xCF,
    0xD0, 0xEF, 0xAA, 0xFB, 0x43, 0x4D, 0x33, 0x85, 0x45, 0xF9, 0x02, 0x7F, 0x50, 0x3C, 0x9F, 0xA8,
    0x51, 0xA3, 0x40, 0x8F, 0x92, 0x9D, 0x38, 0xF5, 0xBC, 0xB6, 0xDA, 0x21, 0x10, 0xFF, 0xF3, 0xD2,
    0xCD, 0x0C, 0x13, 0xEC, 0x5F, 0x97, 0x44, 0x17, 0xC4, 0xA7, 0x7E, 0x3D, 0x64, 0x5D, 0x19, 0x73,
    0x60, 0x81, 0x4F, 0xDC, 0x22, 0x2A, 0x90, 0x88, 0x46, 0xEE, 0xB8, 0x14, 0xDE, 0x5E, 0x0B, 0xDB,
    0xE0, 0x32, 0x3A, 0x0A, 0x49, 0x06, 0x24, 0x5C, 0xC2, 0xD3, 0xAC, 0x62, 0x91, 0x95, 0xE4, 0x79,
    0xE7, 0xC8, 0x37, 0x6D, 0x8D, 0xD5, 0x4E, 0xA9, 0x6C, 0x56, 0xF4, 0xEA, 0x65, 0x7A, 0xAE, 0x08,
    0xBA, 0x78, 0x25, 0x2E, 0x1C, 0xA6, 0xB4, 0xC6, 0xE8, 0xDD, 0x74, 0x1F, 0x4B, 0xBD, 0x8B, 0x8A,
    0x70, 0x3E, 0xB5, 0x66, 0x48, 0x03, 0xF6, 0x0E, 0x61, 0x35, 0x57, 0xB9, 0x86, 0xC1, 0x1D, 0x9E,
    0xE1, 0xF8, 0x98, 0x11, 0x69, 0xD9, 0x8E, 0x94, 0x9B, 0x1E, 0x87, 0xE9, 0xCE, 0x55, 0x28, 0xDF,
    0x8C, 0xA1, 0x89, 0x0D, 0xBF, 0xE6, 0x42, 0x68, 0x41, 0x99, 0x2D, 0x0F, 0xB0, 0x54, 0xBB, 0x16,
)

_RCON = (0x00, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36)


def _xtime(a):
    return (((a << 1) ^ 0x1B) & 0xFF) if (a & 0x80) else (a << 1)


def _expand_key_256(key32):
    """Expand a 32-byte key into 15 round keys (15 x 16 bytes) per FIPS-197."""
    if len(key32) != 32:
        raise ValueError("AES-256 requires a 32-byte key")
    w = [list(key32[i : i + 4]) for i in range(0, 32, 4)]
    for i in range(8, 60):
        temp = list(w[i - 1])
        if i % 8 == 0:
            temp = [_SBOX[temp[1]] ^ _RCON[i // 8], _SBOX[temp[2]], _SBOX[temp[3]], _SBOX[temp[0]]]
        elif i % 8 == 4:
            temp = [_SBOX[b] for b in temp]
        w.append([w[i - 8][j] ^ temp[j] for j in range(4)])
    round_keys = []
    for r in range(15):
        rk = []
        for c in range(4):
            rk.extend(w[r * 4 + c])
        round_keys.append(rk)
    return round_keys


def aes256_encrypt_block(key32, block16):
    """Encrypt a single 16-byte block with AES-256 (FIPS-197)."""
    if len(block16) != 16:
        raise ValueError("AES block must be 16 bytes")
    rks = _expand_key_256(key32)
    s = [block16[i] ^ rks[0][i] for i in range(16)]

    for rnd in range(1, 15):
        # SubBytes
        s = [_SBOX[b] for b in s]
        # ShiftRows (column-major state)
        s = [
            s[0], s[5], s[10], s[15],
            s[4], s[9], s[14], s[3],
            s[8], s[13], s[2], s[7],
            s[12], s[1], s[6], s[11],
        ]
        # MixColumns (all rounds except final round 14)
        if rnd < 14:
            ns = [0] * 16
            for c in range(4):
                i = c * 4
                a0, a1, a2, a3 = s[i], s[i + 1], s[i + 2], s[i + 3]
                t = a0 ^ a1 ^ a2 ^ a3
                ns[i] = a0 ^ t ^ _xtime(a0 ^ a1)
                ns[i + 1] = a1 ^ t ^ _xtime(a1 ^ a2)
                ns[i + 2] = a2 ^ t ^ _xtime(a2 ^ a3)
                ns[i + 3] = a3 ^ t ^ _xtime(a3 ^ a0)
            s = ns
        # AddRoundKey
        rk = rks[rnd]
        s = [s[i] ^ rk[i] for i in range(16)]
    return bytes(s)


def aes256_ctr_crypt(key32, nonce16, data):
    """Encrypt or decrypt arbitrary bytes using AES-256-CTR."""
    if len(nonce16) != 16:
        raise ValueError("CTR nonce must be 16 bytes")
    out = bytearray()
    ctr_int = int.from_bytes(nonce16, "big")
    for off in range(0, len(data), 16):
        ctr_block = ((ctr_int + (off // 16)) & ((1 << 128) - 1)).to_bytes(16, "big")
        keystream = aes256_encrypt_block(key32, ctr_block)
        chunk = data[off : off + 16]
        out.extend(b ^ k for b, k in zip(chunk, keystream))
    return bytes(out)


def privacy_amplify_aes256(bits, salt="QN-BENCH-QKD-v1"):
    """Distill sifted key bits into a 256-bit (32-byte) AES key via SHA-256."""
    bit_str = "".join("1" if b else "0" for b in bits)
    payload = f"{salt}:{len(bits)}:{bit_str}".encode("utf-8")
    return hashlib.sha256(payload).digest()


class QKDSession:
    """Tracks live photon-by-photon BB84 or E91/BBM92 QKD sifting & AES-256 derivation."""

    QBER_THRESHOLD = 0.11  # Shor-Preskill asymptotic security bound (~11%)
    MIN_SIFTED_BITS = 12   # Minimum sifted bits required to lock session

    def __init__(self, protocol="QKD_BB84", plaintext="QUANTUM LINK SECURED BY 1550nm QKD"):
        self.protocol = protocol  # "QKD_BB84" or "QKD_E91"
        self.plaintext = plaintext
        self.nonce = hashlib.sha256(b"qn-bench-ctr-nonce-1550nm").digest()[:16]
        self.rng = random.Random()
        self.reset()

    def reset(self):
        self.total_emitted = 0
        self.pair_states = {}     # pair_id -> quantum state dict (true Bell state for E91 or prepared state for BB84)
        self.alice_records = {}   # pair_id -> {basis, bit, eve_tapped}
        self.bob_records = {}     # pair_id -> {basis, bit, eve_tapped}
        self.rounds = []          # ordered list of reconciled round dicts
        self.sifted_alice = []
        self.sifted_bob = []
        self.sifted_errors = 0
        self.eve_intercepts = 0
        self.dropped_photons = 0
        self.qber = 0.0
        self.qber_history = []
        self.chsh_s = 2.83 if self.protocol == "QKD_E91" else None
        self.alice_key = None
        self.bob_key = None
        self.ciphertext_hex = ""
        self.decrypted_text = ""
        self.verdict = "READY"    # READY, RUNNING, SECURE, ABORT_EVE, ABORT_NO_KEY

    def prepare_round(self, pair_id):
        """Prepare a BB84 polarized qubit or a basis-invariant E91 Bell pair |Phi+>."""
        basis = self.rng.choice(("+", "x"))
        bit = self.rng.choice((0, 1))
        self.total_emitted += 1
        if self.protocol == "QKD_E91":
            # True Bell state |Phi+> = (|00> + |11>)/sqrt(2) = (|++> + |-->)/sqrt(2):
            # Perfectly correlated in BOTH '+' and 'x' bases unless collapsed by Eve or rotated by PMD!
            self.pair_states[pair_id] = {
                "bit_plus": bit,
                "bit_x": self.rng.choice((0, 1)),
            }
        else:
            self.pair_states[pair_id] = {
                "basis": basis,
                "bit": bit,
            }
            self.alice_records[pair_id] = {
                "basis": basis,
                "bit": bit,
                "eve_tapped": False,
            }
        return basis, bit

    def eve_intercept(self, pair_id, current_basis, current_bit, pmd_flipped=False):
        """Simulate an Eve intercept-resend tap: measures in random basis and collapses the qubit."""
        eve_basis = self.rng.choice(("+", "x"))
        st = self.pair_states.get(pair_id, {})
        if self.protocol == "QKD_E91":
            eff_basis = ("x" if eve_basis == "+" else "+") if pmd_flipped else eve_basis
            eve_bit = st.get("bit_plus" if eff_basis == "+" else "bit_x", current_bit)
        else:
            # In BB84, current_basis (Photon.pol) already incorporates any PMD/HWP rotations.
            eve_bit = current_bit if eve_basis == current_basis else self.rng.choice((0, 1))
        return eve_basis, eve_bit

    def measure_photon(self, role, pair_id, arriving_basis, arriving_bit, pmd_flipped=False, eve_tapped=False):
        """Measure an arriving photon at ALICE RX or BOB RX in an independent random basis."""
        if eve_tapped:
            self.eve_intercepts += 1

        rx_basis = self.rng.choice(("+", "x"))
        st = self.pair_states.get(pair_id, {})

        if self.protocol == "QKD_E91" and not eve_tapped:
            # Entangled Bell pair: if PMD rotated this arm's polarization frame, rx_basis maps to the orthogonal basis
            eff_basis = ("x" if rx_basis == "+" else "+") if pmd_flipped else rx_basis
            measured_bit = st.get("bit_plus" if eff_basis == "+" else "bit_x", arriving_bit)
        else:
            # BB84 photon (arriving_basis already reflects PMD/HWP) or E91 photon collapsed by Eve
            if rx_basis == arriving_basis:
                measured_bit = arriving_bit
            else:
                measured_bit = self.rng.choice((0, 1))

        rec = {
            "basis": rx_basis,
            "bit": measured_bit,
            "eve_tapped": eve_tapped,
        }
        if role == "A":
            self.alice_records[pair_id] = rec
        else:
            self.bob_records[pair_id] = rec

        if pair_id in self.alice_records and pair_id in self.bob_records:
            self._reconcile_pair(pair_id)

    def record_drop(self):
        self.dropped_photons += 1

    def _reconcile_pair(self, pair_id):
        a = self.alice_records[pair_id]
        b = self.bob_records[pair_id]
        basis_match = (a["basis"] == b["basis"])
        error = False
        if basis_match:
            self.sifted_alice.append(a["bit"])
            self.sifted_bob.append(b["bit"])
            if a["bit"] != b["bit"]:
                self.sifted_errors += 1
                error = True
            self.qber = self.sifted_errors / max(1, len(self.sifted_alice))
            self.qber_history.append((len(self.sifted_alice), self.qber * 100.0))

        self.rounds.append({
            "id": pair_id,
            "a_basis": a["basis"],
            "b_basis": b["basis"],
            "a_bit": a["bit"],
            "b_bit": b["bit"],
            "match": basis_match,
            "error": error,
            "eve": a["eve_tapped"] or b["eve_tapped"],
        })

        # Update Bell CHSH S-value estimate for E91 entangled sessions:
        # Quantum Tsirelson bound S = 2*sqrt(2) ≈ 2.828; scales down with depolarizing noise/Eve (1 - 2*QBER).
        if self.protocol == "QKD_E91" and self.sifted_alice:
            vis = max(0.0, 1.0 - 2.0 * self.qber)
            # If Eve tapped photons directly, entanglement is broken (classical bound <= 2.0)
            if self.eve_intercepts > 0:
                vis = min(vis, 0.55)
            self.chsh_s = round(2.0 * math.sqrt(2.0) * vis, 2)

    def finalize(self):
        """Run security gate, privacy amplification -> AES-256 key, and encrypt/decrypt demo."""
        n_sifted = len(self.sifted_alice)
        if n_sifted < self.MIN_SIFTED_BITS:
            self.verdict = "ABORT_NO_KEY"
            self.alice_key = None
            self.bob_key = None
            self.ciphertext_hex = ""
            self.decrypted_text = ""
            return False, f"FAILURE: Only {n_sifted} sifted bits (< {self.MIN_SIFTED_BITS}). Check fiber/QFC/VBG on both arms."

        bell_ok = True if self.protocol != "QKD_E91" else (self.chsh_s is not None and self.chsh_s > 2.0)
        if self.qber > self.QBER_THRESHOLD or not bell_ok:
            self.verdict = "ABORT_EVE"
            self.alice_key = None
            self.bob_key = None
            self.ciphertext_hex = ""
            self.decrypted_text = ""
            if self.eve_intercepts > 0:
                return (
                    False,
                    f"SECURITY ABORT! Eve tap detected — QBER {self.qber * 100:.1f}% > 11%"
                    + (f", CHSH S={self.chsh_s:.2f} <= 2.0." if self.chsh_s is not None else ".")
                    + " Reroute around EVE!",
                )
            return (
                False,
                f"SECURITY ABORT! QBER {self.qber * 100:.1f}% > 11% (polarization basis drift). Add HWP!",
            )

        # Secure channel! Run SHA-256 privacy amplification independently on Alice & Bob
        self.verdict = "SECURE"
        self.alice_key = privacy_amplify_aes256(self.sifted_alice)
        self.bob_key = privacy_amplify_aes256(self.sifted_bob)
        self.re_encrypt()
        s_tag = f" | CHSH S={self.chsh_s:.2f}" if self.chsh_s is not None else ""
        return (
            True,
            f"QKD SECURE! {n_sifted} sifted bits | QBER {self.qber * 100:.1f}%{s_tag} | AES-256 Key Derived! [K] Inspect",
        )

    def re_encrypt(self):
        """Re-encrypt current plaintext with Alice's AES-256 key and decrypt with Bob's key."""
        if not self.alice_key or not self.bob_key:
            self.ciphertext_hex = ""
            self.decrypted_text = ""
            return
        pt_bytes = self.plaintext.encode("utf-8")
        ct = aes256_ctr_crypt(self.alice_key, self.nonce, pt_bytes)
        dec = aes256_ctr_crypt(self.bob_key, self.nonce, ct)
        self.ciphertext_hex = ct.hex().upper()
        self.decrypted_text = dec.decode("utf-8", errors="replace")
