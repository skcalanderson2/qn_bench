//! Cross-platform native self-updater (macOS .app + Windows x64) using GitHub Releases.

#![cfg(not(target_arch = "wasm32"))]

use crate::qkd::hex_upper;
use crate::version::*;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Background periodic update check interval (30 minutes — well within GitHub API limits).
pub const PERIODIC_CHECK_INTERVAL: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    Verifying,
    Installing,
    Error,
}

impl UpdateState {
    pub fn is_busy(self) -> bool {
        matches!(
            self,
            UpdateState::Checking
                | UpdateState::Downloading
                | UpdateState::Verifying
                | UpdateState::Installing
        )
    }

    /// The sidebar update button stays hidden unless a newer release is available
    /// or an update download/install is currently in progress.
    pub fn should_show_button(self) -> bool {
        matches!(
            self,
            UpdateState::Available
                | UpdateState::Downloading
                | UpdateState::Verifying
                | UpdateState::Installing
        )
    }
}

#[derive(Debug, Clone)]
pub struct ReleaseInfo {
    pub version: String,
    pub notes: String,
    pub html_url: String,
    pub zip_url: Option<String>,
    pub zip_name: Option<String>,
    pub zip_size: u64,
    pub checksums_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateSnapshot {
    pub state: UpdateState,
    pub status_text: String,
    pub progress: f32,
    pub release_info: Option<ReleaseInfo>,
    pub ready_to_quit: bool,
    pub can_self_install: bool,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: Option<String>,
    body: Option<String>,
    html_url: Option<String>,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

pub fn parse_version(tag: &str) -> (u32, u32, u32) {
    let clean = tag.trim().trim_start_matches(['v', 'V']);
    let mut parts = [0u32; 3];
    for (i, seg) in clean.split('.').take(3).enumerate() {
        let digits: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
        parts[i] = digits.parse().unwrap_or(0);
    }
    (parts[0], parts[1], parts[2])
}

pub fn is_newer_version(latest: &str, current: &str) -> bool {
    parse_version(latest) > parse_version(current)
}

pub fn get_install_target() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
    #[cfg(target_os = "macos")]
    {
        for ancestor in exe.ancestors() {
            if ancestor.extension().and_then(|e| e.to_str()) == Some("app") {
                return Some(ancestor.to_path_buf());
            }
        }
        None
    }
    #[cfg(target_os = "windows")]
    {
        let parent = exe.parent()?.to_path_buf();
        let s = parent.to_string_lossy().to_lowercase();
        // Avoid self-overwriting inside a cargo target/ build folder
        if s.contains("\\target\\debug") || s.contains("\\target\\release") {
            return None;
        }
        Some(parent)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

struct Inner {
    state: UpdateState,
    status_text: String,
    progress: f32,
    release_info: Option<ReleaseInfo>,
    ready_to_quit: bool,
    install_target: Option<PathBuf>,
    last_check: Option<Instant>,
}

pub struct UpdateManager {
    inner: Arc<Mutex<Inner>>,
    cancel: Arc<AtomicBool>,
}

impl UpdateManager {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                state: UpdateState::Idle,
                status_text: String::new(),
                progress: 0.0,
                release_info: None,
                ready_to_quit: false,
                install_target: get_install_target(),
                last_check: None,
            })),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Trigger a silent background check if enough time (`PERIODIC_CHECK_INTERVAL`) has
    /// elapsed since the previous check and no update is already pending or in progress.
    pub fn maybe_periodic_check(&self) {
        let should_check = {
            let g = self.inner.lock().unwrap();
            if g.state.is_busy() || g.state == UpdateState::Available {
                false
            } else {
                match g.last_check {
                    None => true,
                    Some(t) => t.elapsed() >= PERIODIC_CHECK_INTERVAL,
                }
            }
        };
        if should_check {
            self.check_for_updates(true);
        }
    }

    pub fn snapshot(&self) -> UpdateSnapshot {
        let g = self.inner.lock().unwrap();
        UpdateSnapshot {
            state: g.state,
            status_text: g.status_text.clone(),
            progress: g.progress,
            release_info: g.release_info.clone(),
            ready_to_quit: g.ready_to_quit,
            can_self_install: g.install_target.is_some(),
        }
    }

    pub fn cancel_download(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn open_release_page(&self) {
        let url = self
            .inner
            .lock()
            .unwrap()
            .release_info
            .as_ref()
            .map(|r| r.html_url.clone())
            .unwrap_or_else(|| RELEASES_PAGE_URL.to_string());
        let _ = webbrowser::open(&url);
    }

    pub fn check_for_updates(&self, silent: bool) {
        {
            let mut g = self.inner.lock().unwrap();
            if g.state.is_busy() {
                return;
            }
            g.last_check = Some(Instant::now());
            g.state = UpdateState::Checking;
            g.status_text = "Checking GitHub Releases...".to_string();
            g.progress = 0.0;
        }
        let inner = Arc::clone(&self.inner);
        std::thread::spawn(move || {
            let agent = ureq::AgentBuilder::new()
                .timeout(Duration::from_secs(12))
                .user_agent(&format!("{APP_SLUG}/{APP_VERSION}"))
                .build();
            let resp = agent
                .get(RELEASES_API_URL)
                .set("Accept", "application/vnd.github+json")
                .call();
            let rel: GhRelease = match resp {
                Ok(r) => match r.into_json() {
                    Ok(v) => v,
                    Err(_) => {
                        let mut g = inner.lock().unwrap();
                        g.state = if silent { UpdateState::Idle } else { UpdateState::Error };
                        g.status_text = if silent { String::new() } else { "Failed to parse GitHub release metadata.".into() };
                        return;
                    }
                },
                Err(ureq::Error::Status(code, _)) => {
                    let mut g = inner.lock().unwrap();
                    g.state = if silent { UpdateState::Idle } else { UpdateState::Error };
                    g.status_text = if silent {
                        String::new()
                    } else if code == 404 {
                        "No published releases found on GitHub yet.".into()
                    } else {
                        format!("GitHub API error (HTTP {code}).")
                    };
                    return;
                }
                Err(_) => {
                    let mut g = inner.lock().unwrap();
                    g.state = if silent { UpdateState::Idle } else { UpdateState::Error };
                    g.status_text = if silent {
                        String::new()
                    } else {
                        "Unable to reach GitHub. Check your internet connection.".into()
                    };
                    return;
                }
            };

            let tag = rel.tag_name.unwrap_or_default();
            let latest_ver = tag.trim_start_matches(['v', 'V']).to_string();
            let plat_kw = if cfg!(target_os = "windows") { "windows" } else { "macos" };
            let zip_asset = rel
                .assets
                .iter()
                .find(|a| a.name == PLATFORM_ZIP_NAME)
                .or_else(|| {
                    rel.assets
                        .iter()
                        .find(|a| a.name.ends_with(".zip") && a.name.to_lowercase().contains(plat_kw))
                });
            let checksum_asset = rel
                .assets
                .iter()
                .find(|a| a.name == CHECKSUMS_FILE_NAME)
                .or_else(|| rel.assets.iter().find(|a| a.name == "SHA256SUMS.txt"));

            let info = ReleaseInfo {
                version: latest_ver.clone(),
                notes: rel
                    .body
                    .unwrap_or_else(|| "Bug fixes and improvements.".into())
                    .trim()
                    .to_string(),
                html_url: rel.html_url.unwrap_or_else(|| RELEASES_PAGE_URL.to_string()),
                zip_url: zip_asset.map(|a| a.browser_download_url.clone()),
                zip_name: zip_asset.map(|a| a.name.clone()),
                zip_size: zip_asset.map(|a| a.size).unwrap_or(0),
                checksums_url: checksum_asset.map(|a| a.browser_download_url.clone()),
            };

            let mut g = inner.lock().unwrap();
            g.release_info = Some(info);
            if !latest_ver.is_empty() && is_newer_version(&latest_ver, APP_VERSION) {
                g.state = UpdateState::Available;
                g.status_text = format!("Version {latest_ver} is available (installed: {APP_VERSION}).");
            } else {
                g.state = if silent { UpdateState::Idle } else { UpdateState::UpToDate };
                g.status_text = format!("You are up to date (v{APP_VERSION}).");
            }
        });
    }

    pub fn start_install(&self) {
        let (info, target) = {
            let mut g = self.inner.lock().unwrap();
            if g.state != UpdateState::Available {
                return;
            }
            let Some(info) = g.release_info.clone() else { return };
            let Some(target) = g.install_target.clone() else {
                g.state = UpdateState::Error;
                g.status_text = "Self-update only runs in the bundled app. Open Release Page instead.".into();
                return;
            };
            if info.zip_url.is_none() {
                g.state = UpdateState::Error;
                g.status_text = "Release has no update archive for this OS yet. Open Release Page instead.".into();
                return;
            }
            self.cancel.store(false, Ordering::SeqCst);
            g.state = UpdateState::Downloading;
            g.progress = 0.0;
            g.status_text = "Downloading update...".into();
            (info, target)
        };

        let inner = Arc::clone(&self.inner);
        let cancel = Arc::clone(&self.cancel);
        std::thread::spawn(move || {
            if let Err(err) = run_install_worker(&inner, &cancel, &info, &target) {
                let mut g = inner.lock().unwrap();
                g.state = UpdateState::Error;
                g.status_text = err;
            }
        });
    }
}

fn run_install_worker(
    inner: &Arc<Mutex<Inner>>,
    cancel: &Arc<AtomicBool>,
    info: &ReleaseInfo,
    target: &Path,
) -> Result<(), String> {
    let tmp_dir = tempfile::Builder::new()
        .prefix("qn-bench-update-")
        .tempdir()
        .map_err(|e| format!("Failed to create temp directory: {e}"))?;
    let tmp_path = tmp_dir.path().to_path_buf();
    let zip_name = info
        .zip_name
        .clone()
        .unwrap_or_else(|| PLATFORM_ZIP_NAME.to_string());
    let zip_path = tmp_path.join(&zip_name);
    let zip_url = info.zip_url.as_ref().ok_or("Missing download URL")?;

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(90))
        .user_agent(&format!("{APP_SLUG}/{APP_VERSION}"))
        .build();
    let resp = agent
        .get(zip_url)
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;
    let total = if info.zip_size > 0 {
        info.zip_size
    } else {
        resp.header("Content-Length")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0)
    };

    let mut reader = resp.into_reader();
    let mut out_file = File::create(&zip_path).map_err(|e| format!("Write error: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut downloaded = 0u64;

    loop {
        if cancel.load(Ordering::SeqCst) {
            let mut g = inner.lock().unwrap();
            g.state = UpdateState::Available;
            g.progress = 0.0;
            g.status_text = "Update download cancelled.".into();
            return Ok(());
        }
        let n = reader.read(&mut buf).map_err(|e| format!("Stream read error: {e}"))?;
        if n == 0 {
            break;
        }
        out_file.write_all(&buf[..n]).map_err(|e| format!("Write error: {e}"))?;
        hasher.update(&buf[..n]);
        downloaded += n as u64;
        if total > 0 {
            let pct = (downloaded as f32 / total as f32).min(1.0);
            let mb_d = downloaded as f64 / (1024.0 * 1024.0);
            let mb_t = total as f64 / (1024.0 * 1024.0);
            let mut g = inner.lock().unwrap();
            g.progress = pct;
            g.status_text = format!("Downloading... {mb_d:.1} / {mb_t:.1} MB ({}%)", (pct * 100.0) as u32);
        }
    }
    drop(out_file);

    let actual_sha = hex_upper(&hasher.finalize()).to_lowercase();

    {
        let mut g = inner.lock().unwrap();
        g.state = UpdateState::Verifying;
        g.progress = 1.0;
        g.status_text = if cfg!(target_os = "macos") {
            "Verifying SHA-256 checksum and Apple Developer signature...".into()
        } else {
            "Verifying SHA-256 checksum...".into()
        };
    }

    if let Some(sums_url) = &info.checksums_url {
        let sums_text = agent
            .get(sums_url)
            .call()
            .map_err(|e| format!("Failed to download checksums: {e}"))?
            .into_string()
            .map_err(|e| format!("Checksum read error: {e}"))?;
        let mut expected = None;
        for line in sums_text.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let fname = parts[parts.len() - 1].trim_start_matches('*');
                if Path::new(fname).file_name().and_then(|s| s.to_str()) == Some(zip_name.as_str()) {
                    expected = Some(parts[0].to_lowercase());
                    break;
                }
            }
        }
        if let Some(exp) = expected {
            if exp != actual_sha {
                return Err("Checksum mismatch! Update aborted for security.".into());
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        install_macos(inner, &tmp_path, &zip_path, target)?;
        let _ = tmp_dir.keep(); // Detached swap script cleans up tmp_path
        inner.lock().unwrap().ready_to_quit = true;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    {
        install_windows(inner, &tmp_path, &zip_path, target)?;
        let _ = tmp_dir.keep(); // Detached batch script cleans up tmp_path
        inner.lock().unwrap().ready_to_quit = true;
        Ok(())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (tmp_path, zip_path, target);
        Err("Automatic install is supported on macOS and Windows.".into())
    }
}

#[cfg(target_os = "macos")]
fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(target_os = "macos")]
fn install_macos(
    inner: &Arc<Mutex<Inner>>,
    tmp_path: &Path,
    zip_path: &Path,
    target_app: &Path,
) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::process::CommandExt;

    let extract_dir = tmp_path.join("extracted");
    fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;
    let status = Command::new("/usr/bin/ditto")
        .args(["-x", "-k"])
        .arg(zip_path)
        .arg(&extract_dir)
        .status()
        .map_err(|e| format!("Failed to invoke ditto: {e}"))?;
    if !status.success() {
        return Err("Failed to extract downloaded update archive.".into());
    }

    let new_app = fs::read_dir(&extract_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|x| x.path()))
        .find(|p| p.extension().and_then(|e| e.to_str()) == Some("app"))
        .ok_or("Downloaded archive did not contain a .app bundle.")?;

    let verify = Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict"])
        .arg(&new_app)
        .status()
        .map_err(|e| format!("codesign error: {e}"))?;
    if !verify.success() {
        return Err("Signature verification failed! Update aborted.".into());
    }

    let sig_out = Command::new("/usr/bin/codesign")
        .args(["-dvv"])
        .arg(&new_app)
        .output()
        .map_err(|e| format!("codesign inspect error: {e}"))?;
    let stderr = String::from_utf8_lossy(&sig_out.stderr);
    if !stderr.contains(&format!("TeamIdentifier={TEAM_ID}")) {
        return Err("Developer Team ID mismatch! Update aborted.".into());
    }

    let _ = Command::new("/usr/bin/xattr")
        .args(["-dr", "com.apple.quarantine"])
        .arg(&new_app)
        .status();

    {
        let mut g = inner.lock().unwrap();
        g.state = UpdateState::Installing;
        g.status_text = "Installing update and relaunching...".into();
    }

    let target_dir = target_app.parent().ok_or("Invalid app path")?;
    let pid = std::process::id();
    let app_name = target_app.file_name().and_then(|s| s.to_str()).unwrap_or("QN Bench.app");
    let backup_app = target_dir.join(format!(".{app_name}.backup.{pid}"));
    let swap_script = tmp_path.join("swap_and_relaunch.sh");

    let qt = sh_quote(&target_app.to_string_lossy());
    let qn = sh_quote(&new_app.to_string_lossy());
    let qb = sh_quote(&backup_app.to_string_lossy());
    let qd = sh_quote(&target_dir.to_string_lossy());
    let qtmp = sh_quote(&tmp_path.to_string_lossy());

    let body = format!(
        r#"#!/bin/sh
set -u
OLD_PID={pid}
TARGET_APP={qt}
NEW_APP={qn}
BACKUP_APP={qb}
TARGET_DIR={qd}
TMP_DIR={qtmp}

i=0
while kill -0 "$OLD_PID" 2>/dev/null && [ "$i" -lt 75 ]; do
    sleep 0.2
    i=$((i + 1))
done

if [ -w "$TARGET_DIR" ]; then
    rm -rf "$BACKUP_APP"
    mv "$TARGET_APP" "$BACKUP_APP" && mv "$NEW_APP" "$TARGET_APP"
    if [ $? -eq 0 ]; then
        rm -rf "$BACKUP_APP"
    else
        [ -d "$BACKUP_APP" ] && mv "$BACKUP_APP" "$TARGET_APP"
        rm -rf "$TMP_DIR"
        exit 1
    fi
else
    /usr/bin/osascript -e "do shell script \"rm -rf '$BACKUP_APP' && mv '$TARGET_APP' '$BACKUP_APP' && mv '$NEW_APP' '$TARGET_APP' && rm -rf '$BACKUP_APP'\" with administrator privileges" || {{
        rm -rf "$TMP_DIR"
        exit 1
    }}
fi

/usr/bin/open -n "$TARGET_APP"
rm -rf "$TMP_DIR"
"#
    );
    fs::write(&swap_script, body).map_err(|e| e.to_string())?;
    fs::set_permissions(&swap_script, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;

    let mut cmd = Command::new("/bin/sh");
    cmd.arg(&swap_script)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    unsafe {
        cmd.pre_exec(|| {
            libc_setsid();
            Ok(())
        });
    }
    cmd.spawn().map_err(|e| format!("Failed to spawn updater helper: {e}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn libc_setsid() {
    extern "C" {
        fn setsid() -> i32;
    }
    unsafe {
        setsid();
    }
}

#[cfg(target_os = "windows")]
fn install_windows(
    inner: &Arc<Mutex<Inner>>,
    tmp_path: &Path,
    zip_path: &Path,
    target_dir: &Path,
) -> Result<(), String> {
    use std::os::windows::process::CommandExt;

    let extract_dir = tmp_path.join("extracted");
    fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;
    let file = File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Invalid ZIP archive: {e}"))?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(enclosed) = entry.enclosed_name() else {
            return Err("Unsafe path entry in update ZIP.".into());
        };
        let out_path = extract_dir.join(enclosed);
        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = out_path.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut out = File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
        }
    }

    let new_exe = find_exe(&extract_dir).ok_or("Downloaded archive did not contain QN Bench.exe.")?;
    let new_dir = new_exe.parent().ok_or("Invalid extracted directory")?;
    let exe_name = new_exe.file_name().and_then(|s| s.to_str()).unwrap_or("QN Bench.exe");

    {
        let mut g = inner.lock().unwrap();
        g.state = UpdateState::Installing;
        g.status_text = "Installing update and relaunching...".into();
    }

    let swap_bat = tmp_path.join("swap_and_relaunch.bat");
    let pid = std::process::id();
    let bat = format!(
        "@echo off\r\nsetlocal\r\nset OLD_PID={pid}\r\nset \"NEW_DIR={}\"\r\nset \"TARGET_DIR={}\"\r\nset \"TMP_DIR={}\"\r\nset \"EXE_NAME={exe_name}\"\r\nset /a TRIES=0\r\n:waitloop\r\ntasklist /FI \"PID eq %OLD_PID%\" 2>NUL | find /I \"%OLD_PID%\" >NUL\r\nif not errorlevel 1 (\r\n  if %TRIES% GEQ 40 goto do_copy\r\n  set /a TRIES+=1\r\n  ping -n 2 127.0.0.1 >NUL\r\n  goto waitloop\r\n)\r\n:do_copy\r\nrobocopy \"%NEW_DIR%\" \"%TARGET_DIR%\" /MIR /R:5 /W:1 >NUL\r\nstart \"\" \"%TARGET_DIR%\\%EXE_NAME%\"\r\nrmdir /S /Q \"%TMP_DIR%\" 2>NUL\r\n",
        new_dir.display(),
        target_dir.display(),
        tmp_path.display()
    );
    fs::write(&swap_bat, bat).map_err(|e| e.to_string())?;

    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    Command::new("cmd.exe")
        .args(["/c"])
        .arg(&swap_bat)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("Failed to spawn updater script: {e}"))?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn find_exe(dir: &Path) -> Option<PathBuf> {
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        if p.is_file() && p.extension().and_then(|e| e.to_str()).map(|s| s.eq_ignore_ascii_case("exe")).unwrap_or(false) {
            return Some(p);
        }
        if p.is_dir() {
            if let Some(found) = find_exe(&p) {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_button_hidden_until_available_and_version_compare() {
        assert!(!UpdateState::Idle.should_show_button());
        assert!(!UpdateState::Checking.should_show_button());
        assert!(!UpdateState::UpToDate.should_show_button());
        assert!(!UpdateState::Error.should_show_button());

        assert!(UpdateState::Available.should_show_button());
        assert!(UpdateState::Downloading.should_show_button());
        assert!(UpdateState::Verifying.should_show_button());
        assert!(UpdateState::Installing.should_show_button());

        assert!(is_newer_version("v2.11.0", "2.10.0"));
        assert!(!is_newer_version("v2.10.0", "2.10.0"));
        assert!(!is_newer_version("2.9.9", "2.10.0"));
        assert_eq!(PERIODIC_CHECK_INTERVAL, Duration::from_secs(30 * 60));
    }
}
