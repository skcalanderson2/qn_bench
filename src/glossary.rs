//! Complete Quantum Networking Terminology & Abbreviation Glossary for QN Bench.
//! Complements `crate::levels::HARDWARE_DOCS` so learners can browse and search
//! every hardware component, acronym, physical phenomenon, and QKD/AES protocol.

use crate::model::*;
use egui::Color32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlossaryTab {
    All,
    Optics,
    ChannelsNodes,
    QuantumTerms,
    QkdCrypto,
}

impl GlossaryTab {
    pub const ALL_TABS: [GlossaryTab; 5] = [
        GlossaryTab::All,
        GlossaryTab::Optics,
        GlossaryTab::ChannelsNodes,
        GlossaryTab::QuantumTerms,
        GlossaryTab::QkdCrypto,
    ];

    pub fn label(self) -> &'static str {
        match self {
            GlossaryTab::All => "ALL",
            GlossaryTab::Optics => "OPTICS",
            GlossaryTab::ChannelsNodes => "CHANNELS & NODES",
            GlossaryTab::QuantumTerms => "QUANTUM TERMS",
            GlossaryTab::QkdCrypto => "QKD & CRYPTO",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GlossaryState {
    pub query: String,
    pub tab: GlossaryTab,
}

impl Default for GlossaryState {
    fn default() -> Self {
        Self {
            query: String::new(),
            tab: GlossaryTab::All,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GlossaryTerm {
    pub abbr: &'static str,
    pub full_name: &'static str,
    pub category: GlossaryTab,
    pub color: Color32,
    pub summary: &'static str,
    pub in_bench: &'static str,
}

/// Categorize each `HARDWARE_DOCS` key into either placeable `Optics` or bench `ChannelsNodes`.
pub fn hardware_doc_tab(key: &str) -> GlossaryTab {
    match key {
        "QFC" | "VBG" | "MEMORY" | "DELAY" | "HWP" | "AO" | "DCF" | "WDM" | "MIRROR_SLASH"
        | "MIRROR_BACK" => GlossaryTab::Optics,
        _ => GlossaryTab::ChannelsNodes,
    }
}

/// Short human-readable badge text for a `HARDWARE_DOCS` key.
pub fn hardware_doc_badge(key: &str) -> &str {
    match key {
        "MIRROR_SLASH" => "MIRROR /",
        "MIRROR_BACK" => "MIRROR \\",
        "LIT_FIBER" => "LIT FIBER",
        "PMD_FIBER" => "PMD FIBER",
        "CD_FIBER" => "CD FIBER",
        "TELEPORT_BSM" => "TELE-BSM",
        "TELEPORT_RX" => "TELEPORT RX",
        "QKD_TX" => "QKD TX",
        "QKD_RX" => "QKD RX",
        "NV637" => "NV 637nm",
        "ION" => "ION 780nm",
        other => other,
    }
}

/// Case-insensitive substring filter used by the Terminology Guide search bar.
pub fn matches_search(query: &str, fields: &[&str]) -> bool {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return true;
    }
    fields.iter().any(|f| f.to_ascii_lowercase().contains(&q))
}

pub const GLOSSARY_TERMS: &[GlossaryTerm] = &[
    // --- Quantum Physics & Optical Networking Terms ---
    GlossaryTerm {
        abbr: "QUBIT",
        full_name: "Quantum Bit (Photonic Qubit)",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_CYAN,
        summary: "The fundamental unit of quantum information. Unlike a classical bit (strictly 0 or 1), a qubit can exist in a coherent superposition of states and be entangled with distant qubits. In quantum networks, single photons carry qubits between matter nodes.",
        in_bench: "Every moving glowing pulse on the bench represents a single photonic qubit carrying wavelength, fidelity, polarization, and timing state.",
    },
    GlossaryTerm {
        abbr: "FIDELITY",
        full_name: "Quantum State Fidelity (0% to 100%)",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_GREEN,
        summary: "A measure of how closely a received quantum state matches the ideal intended state (100% = perfect, 0% = completely lost/decohered). Optical loss, fiber attenuation, and atmospheric turbulence degrade fidelity.",
        in_bench: "Tracked on the bottom-left Fidelity Trace scope. SNSPD/BSM/Teleport require >= 60%; Fault-Tolerant QPU targets require >= 90%.",
    },
    GlossaryTerm {
        abbr: "1550nm",
        full_name: "Telecom C-Band Wavelength (1550 nanometers)",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_CYAN,
        summary: "The infrared optical wavelength window (~1530-1565nm) where standard silica optical fiber has its absolute lowest absorption loss (~0.2 dB/km). Essential for metropolitan and long-haul quantum links.",
        in_bench: "Shown in Cyan. All fiber tiles absorb non-1550nm photons immediately, so convert visible qubits with a QFC before fiber.",
    },
    GlossaryTerm {
        abbr: "780nm / 637nm",
        full_name: "Visible Matter-Qubit Emission Wavelengths",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_RED,
        summary: "Natural optical transitions of stationary quantum hardware: ~780nm (Rubidium / Trapped-Ion emitters, Red) and 637nm (Diamond Nitrogen-Vacancy centers, Orange). Great for local traps, but heavily absorbed in telecom fiber.",
        in_bench: "ION (780nm) needs 1 VBG after QFC (+40 noise); NV (637nm) undergoes a larger frequency shift (+70 noise) and needs 2 VBGs.",
    },
    GlossaryTerm {
        abbr: "PPLN / DFG",
        full_name: "Periodically Poled Lithium Niobate & Difference Frequency Generation",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_PURPLE,
        summary: "An engineered nonlinear crystal waveguide that mixes a single visible quantum photon with a bright classical pump laser to shift its wavelength down to 1550nm while preserving its quantum state.",
        in_bench: "This is the real-world physics inside the PPLN QFC component. Its bright pump laser creates Raman noise that must be filtered out.",
    },
    GlossaryTerm {
        abbr: "RAMAN NOISE",
        full_name: "Spontaneous Raman Scattering Noise",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_AMBER,
        summary: "Inelastic scattering of bright classical laser light (from QFC pump lasers or coexisting classical data traffic in Lit Fiber) off silica glass vibrations, generating stray background photons inside the 1550nm quantum band.",
        in_bench: "Shown on the center Raman Noise scope and LED bar. Detectors latch if noise > 10; each VBG Filter strips -50 noise.",
    },
    GlossaryTerm {
        abbr: "POL (+/- x)",
        full_name: "Photon Polarization & Measurement Bases ('+' Rectilinear vs. 'x' Diagonal)",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_HWP,
        summary: "The oscillation orientation of a photon's electric field. Rectilinear ('+') encodes Horizontal/Vertical (0/90 deg); Diagonal ('x') encodes +45/-45 deg. Measuring a '+' photon in the 'x' basis yields a random 50/50 result.",
        in_bench: "Shown as '+' or 'x' in hover readouts and Alignment Laser HUD. PMD Fiber flips '+' <-> 'x'; Half-Wave Plate (HWP) rotates it back.",
    },
    GlossaryTerm {
        abbr: "PMD",
        full_name: "Polarization Mode Dispersion (Birefringent Drift)",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_PURPLE,
        summary: "Mechanical stress and temperature shifts in buried optical fiber make orthogonal polarizations travel at slightly different speeds, rotating the polarization state along the link.",
        in_bench: "PMD Fiber tiles flip photon polarization ('+' <-> 'x'), breaking HOM interference, QKD basis alignment, or Teleportation until corrected with an HWP.",
    },
    GlossaryTerm {
        abbr: "CD",
        full_name: "Chromatic Dispersion (Pulse Broadening)",
        category: GlossaryTab::QuantumTerms,
        color: Color32::from_rgb(125, 211, 252),
        summary: "Because glass has a slightly different refractive index for each tiny frequency slice of a short photon wavepacket, the pulse stretches out in time as it travels through long fiber spans.",
        in_bench: "CD Fiber tiles add dispersion (CD > 0), causing detectors and BSMs to reject the broadened pulse. Place a DCF spool AFTER CD Fiber to recompress it.",
    },
    GlossaryTerm {
        abbr: "HOM",
        full_name: "Hong-Ou-Mandel Two-Photon Quantum Interference",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_AMBER,
        summary: "When two truly indistinguishable photons (same wavelength, same polarization, arriving at the exact same time) meet on a 50:50 beam splitter, destructive quantum interference forces both photons to exit the same port together.",
        in_bench: "Displayed on the HOM Interference scope (coincidence dip at dt = 0ps). Requires matching polarization and arrival within <= 15ps.",
    },
    GlossaryTerm {
        abbr: "ps (PICOSECOND)",
        full_name: "Picosecond (10^-12 seconds) & Optical Path Timing",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_AMBER,
        summary: "One trillionth of a second. Light travels ~0.2 millimeters in optical fiber per picosecond, so matching photon arrival times for quantum interference requires picosecond-scale path-length precision.",
        in_bench: "Each bench frame = 5ps; 1 tile of travel = 100ps (20 frames); 1 Delay Line adds exactly +100ps. Use the STEP button to inspect +5ps at a time.",
    },
    GlossaryTerm {
        abbr: "EPR / SPDC",
        full_name: "Einstein-Podolsky-Rosen Entangled Pair & Spontaneous Parametric Down-Conversion",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_PURPLE,
        summary: "SPDC uses a nonlinear crystal to split one pump photon into a pair of entangled photons (an EPR Bell pair, e.g. |Phi+>). Measuring one photon instantly correlates the outcome of its twin regardless of distance.",
        in_bench: "EPR sources emit twin photons in opposite directions for E91 entangled QKD and Bennett et al. Quantum Teleportation.",
    },
    GlossaryTerm {
        abbr: "BSM",
        full_name: "Bell-State Measurement (Entanglement Swapping)",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_AMBER,
        summary: "A joint quantum measurement on two incoming photons (typically using a 50:50 beam splitter + detectors) that projects them into a maximally entangled Bell state, swapping entanglement between two distant nodes that never directly met.",
        in_bench: "Appears as the BSM target, inside Heralded Repeaters, and as TELE-BSM in Level 20. Always requires 1550nm, clean noise, CD=0, and dt <= 15ps.",
    },
    GlossaryTerm {
        abbr: "HERALD / FEEDFORWARD",
        full_name: "Heralding & Classical Feedforward Signaling",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_CYAN,
        summary: "A classical confirmation signal triggered when a probabilistic quantum event (like a BSM click) succeeds. Feedforward tells downstream hardware which gate or Pauli correction to apply without violating relativity.",
        in_bench: "Used by REPEATER (releases a fresh 100% Hop-2 qubit after Hop-1 BSM locks) and TELE-BSM (broadcasts classical herald to Bob's TELEPORT RX).",
    },
    GlossaryTerm {
        abbr: "DISTILLATION",
        full_name: "Entanglement Purification / Distillation",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_GREEN,
        summary: "A quantum error-suppression protocol that consumes two lower-fidelity entangled pairs and performs local gates + measurement to output a single much higher-fidelity pair suitable for fault-tolerant quantum computing.",
        in_bench: "The DISTILL node takes two synchronized valid 1550nm photons (>= 60% fid) and outputs one purified 95% photon to reach QPU (>= 90%).",
    },
    GlossaryTerm {
        abbr: "REPEATER",
        full_name: "Quantum Repeater Architecture",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_CYAN,
        summary: "Because the No-Cloning Theorem forbids simple optical amplifiers for qubits, long-distance quantum networks divide links into shorter hops joined by quantum memories, entanglement swapping (BSM), and heralding.",
        in_bench: "Featured in Level 5 (Memory + BSM) and Level 15 (full 2-hop Heralded Quantum Repeater chain feeding a QPU).",
    },
    GlossaryTerm {
        abbr: "|Psi> TELEPORT",
        full_name: "Quantum Teleportation (Bennett et al., 1993)",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_PURPLE,
        summary: "Transfers an unknown quantum state |Psi> from Alice to Bob without physically sending the original particle across the gap, using a pre-shared EPR pair, Alice's Bell-State Measurement, and classical feedforward.",
        in_bench: "Level 20 finale: interfere |Psi> ('x') with EPR-Alice at TELE-BSM while routing EPR-Bob through DCF + HWP to TELEPORT RX.",
    },
    GlossaryTerm {
        abbr: "PAULI FIX",
        full_name: "Pauli Frame Correction (X / Z Gate)",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_HWP,
        summary: "The final local single-qubit rotation Bob applies based on Alice's 2-bit classical BSM outcome (and channel drift) to reconstruct the exact original quantum state |Psi>.",
        in_bench: "Represented on the optical bench by placing an HWP to restore Bob's photon to the exact |Psi> ('x') polarization state before TELEPORT RX.",
    },
    GlossaryTerm {
        abbr: "WDM",
        full_name: "Wavelength-Division Multiplexing",
        category: GlossaryTab::QuantumTerms,
        color: Color32::from_rgb(244, 114, 182),
        summary: "Combining multiple different optical wavelengths onto the same optical path or fiber using wavelength-selective dichroic filters, enabling shared infrastructure without 50% beam-splitter loss.",
        in_bench: "WDM Dichroic (\\) passes 1550nm straight through while reflecting visible 780nm/637nm 90 degrees; Lit Fiber models classical-quantum WDM coexistence.",
    },
    GlossaryTerm {
        abbr: "WAVEFRONT / AO",
        full_name: "Atmospheric Wavefront Distortion & Adaptive Optics",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_TEAL,
        summary: "Turbulent pockets of warm and cool air bend free-space laser beams unevenly (making stars twinkle). Adaptive Optics measures the distorted phase front and flexes a deformable mirror hundreds of times per second to cancel it.",
        in_bench: "Place AO upstream of ATMOS tiles to cut atmospheric fidelity loss from -25%/tile down to -5%/tile.",
    },
    GlossaryTerm {
        abbr: "CRYO 1.5K",
        full_name: "Cryogenic Superconducting Operation (1.5 Kelvin)",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_CYAN,
        summary: "Near absolute zero (-271.6 C). Superconducting Nanowire Single-Photon Detectors (SNSPDs) operate at ~1.5K so a single 1550nm photon's tiny energy breaks superconductivity and creates a measurable voltage click.",
        in_bench: "SNSPDs require clean links (noise <= 10); excess Raman photons flood the nanowire and cause detector latching.",
    },
    GlossaryTerm {
        abbr: "NO-CLONING",
        full_name: "Quantum No-Cloning Theorem",
        category: GlossaryTab::QuantumTerms,
        color: ACCENT_RED,
        summary: "A fundamental law of quantum mechanics proving it is impossible to create an identical copy of an arbitrary unknown quantum state. This prevents classical repeaters from amplifying qubits — and guarantees eavesdropper detection in QKD.",
        in_bench: "Explains both why Quantum Memories/Repeaters are needed AND why Eve's fiber tap inevitably leaves a detectable QBER/CHSH signature.",
    },

    // --- QKD & Cryptography Terms ---
    GlossaryTerm {
        abbr: "QKD",
        full_name: "Quantum Key Distribution",
        category: GlossaryTab::QkdCrypto,
        color: ACCENT_TEAL,
        summary: "Uses single photons or entangled pairs to let Alice and Bob establish a shared secret cryptographic key whose security is guaranteed by quantum physics rather than computational hardness assumptions.",
        in_bench: "Levels 12, 13, 14, and 16 run live QKD streams and derive real 256-bit AES keys you can inspect with [K].",
    },
    GlossaryTerm {
        abbr: "BB84",
        full_name: "Bennett & Brassard (1984) Prepare-and-Measure QKD",
        category: GlossaryTab::QkdCrypto,
        color: ACCENT_CYAN,
        summary: "The first QKD protocol. Alice encodes random bits (0/1) in randomly chosen '+' or 'x' polarization bases and sends single photons to Bob, who independently measures in random '+' or 'x' bases.",
        in_bench: "Introduced in Level 12. Matching bases form the sifted key; mismatched bases are discarded.",
    },
    GlossaryTerm {
        abbr: "E91 / BBM92",
        full_name: "Ekert (1991) / Bennett-Brassard-Mermin (1992) Entangled QKD",
        category: GlossaryTab::QkdCrypto,
        color: ACCENT_PURPLE,
        summary: "Instead of Alice preparing photons, an untrusted central EPR source distributes entangled pairs to both Alice and Bob. Even if an adversary controls the source, Bell's CHSH inequality certifies genuine quantum entanglement.",
        in_bench: "Introduced in Level 13 and expanded in Levels 14 & 16. Requires both QBER <= 11% AND Bell CHSH S > 2.0.",
    },
    GlossaryTerm {
        abbr: "SIFTING",
        full_name: "Classical Basis Sifting",
        category: GlossaryTab::QkdCrypto,
        color: ACCENT_GREEN,
        summary: "After quantum transmission, Alice and Bob publicly announce which basis ('+' or 'x') they used for each photon — never the bit value itself. Rounds where their bases matched (~50%) become the raw sifted key.",
        in_bench: "Visualized live in the center Basis Sifting scope on QKD levels (needs >= 12 sifted bits to lock).",
    },
    GlossaryTerm {
        abbr: "QBER",
        full_name: "Quantum Bit Error Rate (11% Security Bound)",
        category: GlossaryTab::QkdCrypto,
        color: ACCENT_RED,
        summary: "The fraction of sifted key bits where Alice and Bob disagree. In BB84/E91 (Shor-Preskill bound), QBER <= 11% allows secure error correction and privacy amplification; higher QBER indicates severe drift or an eavesdropper.",
        in_bench: "Plotted on the bottom-left QKD scope with a red dashed MAX 11% line. Uncompensated PMD spikes QBER ~50%; Eve spikes QBER ~25%.",
    },
    GlossaryTerm {
        abbr: "CHSH S",
        full_name: "Clauser-Horne-Shimony-Holt Bell Inequality Parameter",
        category: GlossaryTab::QkdCrypto,
        color: ACCENT_PURPLE,
        summary: "A correlation score testing local realism. Any classical or eavesdropped hidden-variable channel obeys S <= 2.0, whereas genuine quantum entanglement violates the bound up to the Tsirelson limit S = 2*sqrt(2) ~= 2.83.",
        in_bench: "Monitored on E91 levels (13, 14, 16). Must satisfy S > 2.0; Eve's intercept-resend collapses S <= 2.0 and triggers an automatic abort.",
    },
    GlossaryTerm {
        abbr: "EVE TAP",
        full_name: "Intercept-Resend Eavesdropping Attack",
        category: GlossaryTab::QkdCrypto,
        color: ACCENT_RED,
        summary: "An eavesdropper ('Eve') intercepts photons mid-fiber, guesses a measurement basis ('+' or 'x'), and re-sends a replacement photon. Guessing wrong 50% of the time introduces ~25% errors into Alice & Bob's sifted key.",
        in_bench: "EVE tiles cannot be filtered optically — spot the QBER ~25% / CHSH collapse and physically reroute the quantum link around the tap with mirrors.",
    },
    GlossaryTerm {
        abbr: "SHA-256",
        full_name: "Privacy Amplification via SHA-256 Cryptographic Hash",
        category: GlossaryTab::QkdCrypto,
        color: ACCENT_AMBER,
        summary: "Compresses the reconciled sifted key bits through a cryptographic hash function so any partial information leaked to an adversary becomes negligible, yielding a uniform 256-bit symmetric key.",
        in_bench: "Shown in Step 2 of the QKD & AES-256 Inspector [K], turning Alice and Bob's sifted bits into identical 64-hex-character keys.",
    },
    GlossaryTerm {
        abbr: "AES-256-CTR",
        full_name: "FIPS-197 Advanced Encryption Standard (256-bit Counter Mode)",
        category: GlossaryTab::QkdCrypto,
        color: ACCENT_GREEN,
        summary: "The U.S. NIST standard symmetric block cipher using a 256-bit key in Counter (CTR) stream mode. Real quantum networks use QKD to continuously refresh AES-256 session keys for high-speed classical data encryption.",
        in_bench: "QN Bench runs real NIST FIPS-197 AES-256-CTR encryption live — press [K] on any locked QKD level to edit plaintext and inspect ciphertext.",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels::HARDWARE_DOCS;

    #[test]
    fn glossary_covers_all_hardware_and_uses_safe_glyphs() {
        let forbidden = ['→', '←', '↑', '↓', '✓', '★', '↶', '↷', '⏸'];
        assert_eq!(HARDWARE_DOCS.len(), 29);

        let mut optics_count = 0;
        let mut nodes_count = 0;
        for doc in HARDWARE_DOCS {
            assert!(!doc.key.is_empty());
            assert!(!doc.title.is_empty());
            assert!(!hardware_doc_badge(doc.key).is_empty());
            match hardware_doc_tab(doc.key) {
                GlossaryTab::Optics => optics_count += 1,
                GlossaryTab::ChannelsNodes => nodes_count += 1,
                other => panic!("Unexpected hardware category {other:?} for {}", doc.key),
            }
            for text in [doc.title, doc.physics, doc.rule, doc.tip] {
                for ch in forbidden {
                    assert!(
                        !text.contains(ch),
                        "Forbidden tofu glyph '{ch}' found in hardware doc {}",
                        doc.key
                    );
                }
            }
        }
        assert_eq!(optics_count, 10);
        assert_eq!(nodes_count, 19);

        assert!(GLOSSARY_TERMS.len() >= 25);
        for term in GLOSSARY_TERMS {
            assert!(!term.abbr.is_empty());
            assert!(!term.full_name.is_empty());
            assert!(!term.summary.is_empty());
            assert!(!term.in_bench.is_empty());
            assert!(matches!(
                term.category,
                GlossaryTab::QuantumTerms | GlossaryTab::QkdCrypto
            ));
            for text in [term.abbr, term.full_name, term.summary, term.in_bench] {
                for ch in forbidden {
                    assert!(
                        !text.contains(ch),
                        "Forbidden tofu glyph '{ch}' found in glossary term {}",
                        term.abbr
                    );
                }
            }
        }

        // Case-insensitive search sanity checks
        assert!(matches_search("ppln", &["PPLN / DFG", "Nonlinear crystal"]));
        assert!(matches_search("chsh", &["CHSH S", "Bell inequality"]));
        assert!(!matches_search("teleport", &["BB84", "Prepare-and-Measure"]));
    }
}
