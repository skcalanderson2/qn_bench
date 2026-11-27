//! Per-level YouTube video guide registry and cross-platform player.
//!
//! Reads baked-in defaults from `docs/videos.json` and asynchronously refreshes
//! from the live GitHub Pages manifest so newly uploaded YouTube videos appear
//! immediately in WebAssembly and installed Desktop apps without rebuilding.

use crate::version::VIDEOS_MANIFEST_URL;
use serde::Deserialize;
use std::collections::BTreeMap;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::{Arc, Mutex};

const EMBEDDED_VIDEOS_JSON: &str = include_str!("../docs/videos.json");

#[derive(Debug, Clone, Deserialize, Default)]
struct RawVideoManifest {
    #[serde(default)]
    #[allow(dead_code)]
    playlist_url: String,
    #[serde(default)]
    levels: BTreeMap<String, String>,
}

/// Extracts an 11-character YouTube video ID from a bare ID or any standard YouTube URL.
pub fn extract_youtube_id(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.is_empty() {
        return None;
    }
    let candidate = if let Some((_, rhs)) = s.split_once("v=") {
        rhs.split(&['&', '#', '?', '/'][..]).next().unwrap_or("")
    } else if let Some((_, rhs)) = s.split_once("youtu.be/") {
        rhs.split(&['&', '#', '?', '/'][..]).next().unwrap_or("")
    } else if let Some((_, rhs)) = s.split_once("/embed/") {
        rhs.split(&['&', '#', '?', '/'][..]).next().unwrap_or("")
    } else if let Some((_, rhs)) = s.split_once("/shorts/") {
        rhs.split(&['&', '#', '?', '/'][..]).next().unwrap_or("")
    } else {
        s
    };
    if candidate.len() == 11
        && candidate
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn parse_manifest_levels(json: &str) -> BTreeMap<usize, String> {
    let mut out = BTreeMap::new();
    let Ok(raw) = serde_json::from_str::<RawVideoManifest>(json) else {
        return out;
    };
    for (k, v) in raw.levels {
        if let (Ok(num), Some(yt_id)) = (k.trim().parse::<usize>(), extract_youtube_id(&v)) {
            if num >= 1 {
                out.insert(num - 1, yt_id);
            }
        }
    }
    out
}

pub struct VideoCatalog {
    levels: BTreeMap<usize, String>,
    #[cfg(not(target_arch = "wasm32"))]
    fetched: Arc<Mutex<Option<BTreeMap<usize, String>>>>,
    #[cfg(target_arch = "wasm32")]
    web_synced: bool,
}

impl VideoCatalog {
    pub fn new() -> Self {
        let levels = parse_manifest_levels(EMBEDDED_VIDEOS_JSON);

        #[cfg(not(target_arch = "wasm32"))]
        let fetched = {
            let slot: Arc<Mutex<Option<BTreeMap<usize, String>>>> = Arc::new(Mutex::new(None));
            let slot_bg = Arc::clone(&slot);
            std::thread::spawn(move || {
                let resp = ureq::get(VIDEOS_MANIFEST_URL)
                    .timeout(std::time::Duration::from_secs(6))
                    .call();
                if let Ok(r) = resp {
                    if let Ok(body) = r.into_string() {
                        let parsed = parse_manifest_levels(&body);
                        if !parsed.is_empty() {
                            if let Ok(mut guard) = slot_bg.lock() {
                                *guard = Some(parsed);
                            }
                        }
                    }
                }
            });
            slot
        };

        #[cfg(target_arch = "wasm32")]
        {
            let _ = VIDEOS_MANIFEST_URL;
            web_bridge::fetch_videos_manifest("../videos.json");
        }

        Self {
            levels,
            #[cfg(not(target_arch = "wasm32"))]
            fetched,
            #[cfg(target_arch = "wasm32")]
            web_synced: false,
        }
    }

    pub fn poll_updates(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Ok(mut guard) = self.fetched.try_lock() {
            if let Some(updated) = guard.take() {
                for (k, v) in updated {
                    self.levels.insert(k, v);
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        if !self.web_synced {
            if let Some(json) = web_bridge::take_videos_manifest() {
                self.web_synced = true;
                for (k, v) in parse_manifest_levels(&json) {
                    self.levels.insert(k, v);
                }
            }
        }
    }

    /// Returns the YouTube video ID for a 0-indexed campaign level, if configured.
    pub fn id_for_level(&self, level_idx: usize) -> Option<&str> {
        self.levels.get(&level_idx).map(|s| s.as_str())
    }
}

/// Opens the YouTube video guide: embedded modal iframe on WebAssembly, default browser on Desktop.
pub fn open_lab_video(title: &str, youtube_id: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        web_bridge::open_youtube_modal(title, youtube_id);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = title;
        let url = format!("https://www.youtube.com/watch?v={youtube_id}");
        let _ = webbrowser::open(&url);
    }
}

/// Returns true while the embedded HTML YouTube modal is open on WebAssembly.
pub fn is_web_video_open() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_bridge::is_youtube_modal_open()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

#[cfg(target_arch = "wasm32")]
mod web_bridge {
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen(inline_js = r#"
let qnVideosJson = null;
let qnVideoModalEl = null;

export function fetch_videos_manifest(url) {
    if (typeof fetch === 'undefined') return;
    fetch(url, { cache: 'no-cache' })
        .then((r) => (r.ok ? r.text() : null))
        .then((txt) => {
            if (txt) qnVideosJson = txt;
        })
        .catch(() => {});
}

export function take_videos_manifest() {
    if (qnVideosJson === null) return undefined;
    const out = qnVideosJson;
    qnVideosJson = null;
    return out;
}

export function is_youtube_modal_open() {
    return qnVideoModalEl !== null;
}

function closeModal() {
    if (qnVideoModalEl) {
        qnVideoModalEl.remove();
        qnVideoModalEl = null;
    }
}

export function open_youtube_modal(title, videoId) {
    closeModal();
    const overlay = document.createElement('div');
    overlay.id = 'qn_youtube_overlay';
    overlay.style.cssText = [
        'position:fixed', 'inset:0', 'z-index:9999',
        'background:rgba(6,10,16,0.84)', 'backdrop-filter:blur(4px)',
        'display:flex', 'align-items:center', 'justify-content:center',
        'padding:20px', 'box-sizing:border-box',
        'font-family:-apple-system,BlinkMacSystemFont,"SF Pro Text","Segoe UI",Roboto,sans-serif'
    ].join(';');

    const card = document.createElement('div');
    card.style.cssText = [
        'width:min(940px,96vw)', 'background:#101622',
        'border:2px solid #38bdf8', 'border-radius:12px',
        'box-shadow:0 24px 64px rgba(0,0,0,0.65)', 'overflow:hidden'
    ].join(';');

    const header = document.createElement('div');
    header.style.cssText = [
        'display:flex', 'align-items:center', 'justify-content:space-between',
        'gap:12px', 'padding:12px 16px', 'border-bottom:1px solid #222c3c',
        'background:#141b27'
    ].join(';');

    const titleWrap = document.createElement('div');
    const kicker = document.createElement('div');
    kicker.textContent = 'LAB VIDEO GUIDE  •  YOUTUBE';
    kicker.style.cssText = 'font-size:11px;font-weight:700;color:#38bdf8;letter-spacing:0.05em;';
    const hTitle = document.createElement('div');
    hTitle.textContent = title;
    hTitle.style.cssText = 'font-size:15px;font-weight:700;color:#eef4fc;margin-top:2px;';
    titleWrap.appendChild(kicker);
    titleWrap.appendChild(hTitle);

    const actions = document.createElement('div');
    actions.style.cssText = 'display:flex;align-items:center;gap:8px;flex-shrink:0;';

    const ytLink = document.createElement('a');
    ytLink.href = 'https://www.youtube.com/watch?v=' + encodeURIComponent(videoId);
    ytLink.target = '_blank';
    ytLink.rel = 'noopener noreferrer';
    ytLink.textContent = 'Open on YouTube';
    ytLink.style.cssText = [
        'padding:6px 12px', 'border-radius:6px', 'border:1px solid #33445e',
        'background:#1b2435', 'color:#eef4fc', 'text-decoration:none',
        'font-size:12px', 'font-weight:600'
    ].join(';');

    const closeBtn = document.createElement('button');
    closeBtn.type = 'button';
    closeBtn.textContent = 'CLOSE  [ESC]';
    closeBtn.style.cssText = [
        'padding:6px 12px', 'border-radius:6px', 'border:1px solid #38bdf8',
        'background:#182b3a', 'color:#eef4fc', 'cursor:pointer',
        'font-size:12px', 'font-weight:700'
    ].join(';');
    closeBtn.addEventListener('click', closeModal);

    actions.appendChild(ytLink);
    actions.appendChild(closeBtn);
    header.appendChild(titleWrap);
    header.appendChild(actions);

    const frameWrap = document.createElement('div');
    frameWrap.style.cssText = 'position:relative;width:100%;aspect-ratio:16/9;background:#000;';

    const iframe = document.createElement('iframe');
    iframe.src = 'https://www.youtube-nocookie.com/embed/' + encodeURIComponent(videoId) + '?rel=0&modestbranding=1&autoplay=1';
    iframe.title = title;
    iframe.style.cssText = 'position:absolute;inset:0;width:100%;height:100%;border:0;';
    iframe.setAttribute('allow', 'accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share');
    iframe.setAttribute('allowfullscreen', 'true');

    frameWrap.appendChild(iframe);
    card.appendChild(header);
    card.appendChild(frameWrap);
    overlay.appendChild(card);

    overlay.addEventListener('click', (e) => {
        if (e.target === overlay) closeModal();
    });
    const onKey = (e) => {
        if (e.key === 'Escape') {
            closeModal();
            window.removeEventListener('keydown', onKey, true);
        }
    };
    window.addEventListener('keydown', onKey, true);

    document.body.appendChild(overlay);
    qnVideoModalEl = overlay;
}
"#)]
    extern "C" {
        pub fn fetch_videos_manifest(url: &str);
        pub fn take_videos_manifest() -> Option<String>;
        pub fn is_youtube_modal_open() -> bool;
        pub fn open_youtube_modal(title: &str, video_id: &str);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_id_extraction_and_manifest_parsing() {
        assert_eq!(extract_youtube_id("dQw4w9WgXcQ").as_deref(), Some("dQw4w9WgXcQ"));
        assert_eq!(
            extract_youtube_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=12s").as_deref(),
            Some("dQw4w9WgXcQ")
        );
        assert_eq!(
            extract_youtube_id("https://youtu.be/AbCdEf123_-?si=xyz").as_deref(),
            Some("AbCdEf123_-")
        );
        assert_eq!(
            extract_youtube_id("https://www.youtube.com/embed/1234567890a").as_deref(),
            Some("1234567890a")
        );
        assert_eq!(extract_youtube_id(""), None);
        assert_eq!(extract_youtube_id("not-a-valid-id"), None);

        let parsed = parse_manifest_levels(
            r#"{"levels": {"1": "https://youtu.be/dQw4w9WgXcQ", "20": "AbCdEf123_-", "2": ""}}"#,
        );
        assert_eq!(parsed.get(&0).map(String::as_str), Some("dQw4w9WgXcQ"));
        assert_eq!(parsed.get(&19).map(String::as_str), Some("AbCdEf123_-"));
        assert!(!parsed.contains_key(&1));

        // Verify the real embedded docs/videos.json parses cleanly and all 20 levels have unique valid IDs
        let raw: RawVideoManifest =
            serde_json::from_str(EMBEDDED_VIDEOS_JSON).expect("valid docs/videos.json");
        let live = parse_manifest_levels(EMBEDDED_VIDEOS_JSON);
        assert_eq!(live.len(), 20, "All 20 campaign levels should have YouTube video guides");
        let mut seen_ids = std::collections::HashSet::new();
        for (k, v) in &raw.levels {
            let id = extract_youtube_id(v)
                .unwrap_or_else(|| panic!("Invalid YouTube link/ID for level {k}: {v}"));
            assert!(
                seen_ids.insert(id.clone()),
                "Duplicate YouTube video ID {id} found at level {k}"
            );
        }
        for idx in 0..20 {
            assert!(live.contains_key(&idx), "Missing video guide for Level {}", idx + 1);
        }
    }
}
