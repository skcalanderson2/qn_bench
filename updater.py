"""In-app self-updater for QN Bench — Quantum Networking Optical Bench.

Checks GitHub Releases in a background thread, prompts the user for explicit
permission before downloading, verifies SHA-256 (plus Apple Developer ID
signature on macOS), and atomically swaps & relaunches the bundled app on
macOS (.app) or Windows (x64).
"""

import hashlib
import json
import os
import re
import shlex
import shutil
import ssl
import subprocess
import sys
import tempfile
import threading
import urllib.error
import urllib.request
import webbrowser
import zipfile
from pathlib import Path

from version import (
    APP_NAME,
    APP_SLUG,
    APP_VERSION,
    ASSET_ZIP_NAME,
    CHECKSUMS_FILE_NAME,
    RELEASES_API_URL,
    RELEASES_PAGE_URL,
    TEAM_ID,
)

try:
    import certifi
except ImportError:
    certifi = None


def _ssl_context():
    if certifi is not None:
        return ssl.create_default_context(cafile=certifi.where())
    return ssl.create_default_context()


def parse_version(tag):
    """Parse a semantic version string like 'v1.2.3' into a comparable int tuple."""
    clean = (tag or "").strip().lstrip("vV")
    parts = []
    for seg in clean.split("."):
        m = re.match(r"(\d+)", seg)
        parts.append(int(m.group(1)) if m else 0)
    while len(parts) < 3:
        parts.append(0)
    return tuple(parts[:3])


def is_newer_version(latest, current=APP_VERSION):
    return parse_version(latest) > parse_version(current)


def get_install_target():
    """Return the installed bundle path (.app on macOS, app dir on Windows) or None in dev mode."""
    if not getattr(sys, "frozen", False):
        return None
    exe = Path(sys.executable).resolve()
    if sys.platform == "darwin":
        for parent in exe.parents:
            if parent.suffix == ".app":
                return parent
        return None
    if sys.platform == "win32":
        return exe.parent
    return None


def _safe_extract_zip(zip_path, dest_dir):
    """Extract zip safely, blocking path-traversal entries (Zip Slip)."""
    dest_resolved = Path(dest_dir).resolve()
    with zipfile.ZipFile(zip_path, "r") as zf:
        for member in zf.infolist():
            target = (dest_resolved / member.filename).resolve()
            if dest_resolved != target and dest_resolved not in target.parents:
                raise ValueError(f"Unsafe archive entry: {member.filename}")
        zf.extractall(dest_resolved)


def _http_request(url, timeout=15):
    req = urllib.request.Request(
        url,
        headers={
            "User-Agent": f"{APP_SLUG}/{APP_VERSION}",
            "Accept": "application/vnd.github+json",
        },
    )
    return urllib.request.urlopen(req, timeout=timeout, context=_ssl_context())


class UpdateManager:
    """Thread-safe GitHub Releases updater polled by the Pygame UI loop."""

    def __init__(self):
        self._lock = threading.Lock()
        self.state = "IDLE"  # IDLE, CHECKING, UP_TO_DATE, AVAILABLE, DOWNLOADING, VERIFYING, INSTALLING, ERROR
        self.status_text = ""
        self.progress = 0.0
        self.release_info = None
        self.has_unseen_update = False
        self.ready_to_quit = False
        self._cancel = threading.Event()
        self.install_target = get_install_target()

    def snapshot(self):
        with self._lock:
            return {
                "state": self.state,
                "status_text": self.status_text,
                "progress": self.progress,
                "release_info": dict(self.release_info) if self.release_info else None,
                "has_unseen_update": self.has_unseen_update,
                "ready_to_quit": self.ready_to_quit,
                "can_self_install": self.install_target is not None,
            }

    def mark_seen(self):
        with self._lock:
            self.has_unseen_update = False

    def cancel_download(self):
        self._cancel.set()

    def open_release_page(self):
        with self._lock:
            url = (self.release_info or {}).get("html_url") or RELEASES_PAGE_URL
        webbrowser.open(url)

    def check_for_updates(self, silent=True):
        with self._lock:
            if self.state in ("CHECKING", "DOWNLOADING", "VERIFYING", "INSTALLING"):
                return
            self.state = "CHECKING"
            self.status_text = "Checking GitHub Releases..."
            self.progress = 0.0

        t = threading.Thread(target=self._check_worker, args=(silent,), daemon=True)
        t.start()

    def _check_worker(self, silent):
        try:
            with _http_request(RELEASES_API_URL, timeout=10) as resp:
                data = json.loads(resp.read().decode("utf-8"))
        except urllib.error.HTTPError as exc:
            with self._lock:
                if silent:
                    self.state = "IDLE"
                    self.status_text = ""
                else:
                    self.state = "ERROR"
                    if exc.code == 404:
                        self.status_text = "No published releases found on GitHub yet."
                    else:
                        self.status_text = f"GitHub API error (HTTP {exc.code})."
            return
        except Exception:
            with self._lock:
                if silent:
                    self.state = "IDLE"
                    self.status_text = ""
                else:
                    self.state = "ERROR"
                    self.status_text = "Unable to reach GitHub. Check your internet connection."
            return

        tag = data.get("tag_name", "")
        latest_ver = tag.lstrip("vV")
        assets = data.get("assets", [])

        plat_keyword = "windows" if sys.platform == "win32" else "macos"
        zip_asset = next((a for a in assets if a.get("name") == ASSET_ZIP_NAME), None)
        if zip_asset is None:
            zip_asset = next(
                (a for a in assets if a.get("name", "").endswith(".zip") and plat_keyword in a.get("name", "").lower()),
                None,
            )
        checksum_asset = next((a for a in assets if a.get("name") == CHECKSUMS_FILE_NAME), None)
        if checksum_asset is None:
            checksum_asset = next((a for a in assets if a.get("name") == "SHA256SUMS.txt"), None)

        info = {
            "tag": tag,
            "version": latest_ver,
            "title": data.get("name") or f"{APP_NAME} {tag}",
            "notes": (data.get("body") or "Bug fixes and improvements.").strip(),
            "html_url": data.get("html_url") or RELEASES_PAGE_URL,
            "zip_url": zip_asset.get("browser_download_url") if zip_asset else None,
            "zip_name": zip_asset.get("name") if zip_asset else None,
            "zip_size": zip_asset.get("size", 0) if zip_asset else 0,
            "checksums_url": checksum_asset.get("browser_download_url") if checksum_asset else None,
        }

        with self._lock:
            self.release_info = info
            if latest_ver and is_newer_version(latest_ver, APP_VERSION):
                self.state = "AVAILABLE"
                self.status_text = f"Version {latest_ver} is available (installed: {APP_VERSION})."
                self.has_unseen_update = True
            else:
                self.state = "IDLE" if silent else "UP_TO_DATE"
                self.status_text = f"You are up to date (v{APP_VERSION})."

    def start_install(self):
        with self._lock:
            if self.state != "AVAILABLE" or not self.release_info:
                return
            if self.install_target is None:
                self.state = "ERROR"
                self.status_text = "Self-update only runs in the bundled app. Open Release Page instead."
                return
            if not self.release_info.get("zip_url"):
                self.state = "ERROR"
                self.status_text = "Release has no update archive for this OS yet. Open Release Page instead."
                return
            self._cancel.clear()
            self.state = "DOWNLOADING"
            self.progress = 0.0
            self.status_text = "Downloading update..."
            info = dict(self.release_info)

        t = threading.Thread(target=self._install_worker, args=(info,), daemon=True)
        t.start()

    def _set_error(self, msg):
        with self._lock:
            self.state = "ERROR"
            self.status_text = msg

    def _verify_checksum(self, info, zip_name, actual_sha):
        if not info.get("checksums_url"):
            return True
        with _http_request(info["checksums_url"], timeout=15) as resp:
            sums_text = resp.read().decode("utf-8", errors="replace")
        expected_sha = None
        for line in sums_text.splitlines():
            parts = line.strip().split()
            if len(parts) >= 2 and Path(parts[-1].lstrip("*")).name == zip_name:
                expected_sha = parts[0].lower()
                break
        if expected_sha and expected_sha != actual_sha:
            self._set_error("Checksum mismatch! Update aborted for security.")
            return False
        return True

    def _install_macos(self, tmp_dir, zip_path):
        extract_dir = tmp_dir / "extracted"
        extract_dir.mkdir(parents=True, exist_ok=True)
        res = subprocess.run(
            ["/usr/bin/ditto", "-x", "-k", str(zip_path), str(extract_dir)],
            capture_output=True,
            text=True,
            check=False,
        )
        if res.returncode != 0:
            self._set_error("Failed to extract downloaded update archive.")
            return False

        apps = list(extract_dir.glob("*.app"))
        if not apps:
            self._set_error("Downloaded archive did not contain a .app bundle.")
            return False
        new_app = apps[0]

        verify = subprocess.run(
            ["/usr/bin/codesign", "--verify", "--deep", "--strict", str(new_app)],
            capture_output=True,
            text=True,
            check=False,
        )
        if verify.returncode != 0:
            self._set_error("Signature verification failed! Update aborted.")
            return False

        sig_info = subprocess.run(
            ["/usr/bin/codesign", "-dvv", str(new_app)],
            capture_output=True,
            text=True,
            check=False,
        )
        if f"TeamIdentifier={TEAM_ID}" not in sig_info.stderr:
            self._set_error("Developer Team ID mismatch! Update aborted.")
            return False

        subprocess.run(
            ["/usr/bin/xattr", "-dr", "com.apple.quarantine", str(new_app)],
            capture_output=True,
            check=False,
        )

        with self._lock:
            self.state = "INSTALLING"
            self.status_text = "Installing update and relaunching..."

        target_app = self.install_target
        target_dir = target_app.parent
        backup_app = target_dir / f".{target_app.name}.backup.{os.getpid()}"
        swap_script = tmp_dir / "swap_and_relaunch.sh"

        q_target = shlex.quote(str(target_app))
        q_new = shlex.quote(str(new_app))
        q_backup = shlex.quote(str(backup_app))
        q_target_dir = shlex.quote(str(target_dir))
        q_tmp = shlex.quote(str(tmp_dir))

        script_body = f"""#!/bin/sh
set -u
OLD_PID={os.getpid()}
TARGET_APP={q_target}
NEW_APP={q_new}
BACKUP_APP={q_backup}
TARGET_DIR={q_target_dir}
TMP_DIR={q_tmp}

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
    /usr/bin/osascript -e "do shell script \\"rm -rf '$BACKUP_APP' && mv '$TARGET_APP' '$BACKUP_APP' && mv '$NEW_APP' '$TARGET_APP' && rm -rf '$BACKUP_APP'\\" with administrator privileges" || {{
        rm -rf "$TMP_DIR"
        exit 1
    }}
fi

/usr/bin/open -n "$TARGET_APP"
rm -rf "$TMP_DIR"
"""
        swap_script.write_text(script_body, encoding="utf-8")
        swap_script.chmod(0o700)

        subprocess.Popen(
            ["/bin/sh", str(swap_script)],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
            close_fds=True,
        )
        return True

    def _install_windows(self, tmp_dir, zip_path):
        extract_dir = tmp_dir / "extracted"
        extract_dir.mkdir(parents=True, exist_ok=True)
        _safe_extract_zip(zip_path, extract_dir)

        exe_name = Path(sys.executable).name
        matches = list(extract_dir.rglob(exe_name)) or list(extract_dir.rglob("*.exe"))
        if not matches:
            self._set_error("Downloaded archive did not contain the Windows executable.")
            return False
        new_dir = matches[0].parent
        new_exe_name = matches[0].name

        target_dir = self.install_target
        if not os.access(target_dir, os.W_OK):
            self._set_error("Install folder is read-only. Download the installer from Release Page.")
            return False

        with self._lock:
            self.state = "INSTALLING"
            self.status_text = "Installing update and relaunching..."

        swap_bat = tmp_dir / "swap_and_relaunch.bat"
        bat_body = (
            "@echo off\r\n"
            "setlocal\r\n"
            f"set OLD_PID={os.getpid()}\r\n"
            f'set "NEW_DIR={new_dir}"\r\n'
            f'set "TARGET_DIR={target_dir}"\r\n'
            f'set "TMP_DIR={tmp_dir}"\r\n'
            f'set "EXE_NAME={new_exe_name}"\r\n'
            "set /a TRIES=0\r\n"
            ":waitloop\r\n"
            'tasklist /FI "PID eq %OLD_PID%" 2>NUL | find /I "%OLD_PID%" >NUL\r\n'
            "if not errorlevel 1 (\r\n"
            "  if %TRIES% GEQ 40 goto do_copy\r\n"
            "  set /a TRIES+=1\r\n"
            "  ping -n 2 127.0.0.1 >NUL\r\n"
            "  goto waitloop\r\n"
            ")\r\n"
            ":do_copy\r\n"
            'robocopy "%NEW_DIR%" "%TARGET_DIR%" /MIR /R:5 /W:1 >NUL\r\n'
            'start "" "%TARGET_DIR%\\%EXE_NAME%"\r\n'
            'rmdir /S /Q "%TMP_DIR%" 2>NUL\r\n'
        )
        swap_bat.write_text(bat_body, encoding="utf-8")

        flags = 0
        for attr in ("CREATE_NEW_PROCESS_GROUP", "DETACHED_PROCESS", "CREATE_NO_WINDOW"):
            flags |= getattr(subprocess, attr, 0)

        subprocess.Popen(
            ["cmd.exe", "/c", str(swap_bat)],
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            creationflags=flags,
            close_fds=True,
        )
        return True

    def _install_worker(self, info):
        tmp_dir = Path(tempfile.mkdtemp(prefix="qn-bench-update-"))
        cleanup_tmp = True
        try:
            zip_name = info["zip_name"] or ASSET_ZIP_NAME
            zip_path = tmp_dir / zip_name

            # 1. Stream download & compute SHA-256
            sha = hashlib.sha256()
            total = info.get("zip_size") or 0
            downloaded = 0
            with _http_request(info["zip_url"], timeout=60) as resp, open(zip_path, "wb") as f:
                if not total:
                    cl = resp.headers.get("Content-Length")
                    total = int(cl) if cl and cl.isdigit() else 0
                while True:
                    if self._cancel.is_set():
                        with self._lock:
                            self.state = "AVAILABLE"
                            self.progress = 0.0
                            self.status_text = "Update download cancelled."
                        return
                    chunk = resp.read(65536)
                    if not chunk:
                        break
                    f.write(chunk)
                    sha.update(chunk)
                    downloaded += len(chunk)
                    if total > 0:
                        pct = min(1.0, downloaded / total)
                        mb_done = downloaded / (1024 * 1024)
                        mb_tot = total / (1024 * 1024)
                        with self._lock:
                            self.progress = pct
                            self.status_text = f"Downloading... {mb_done:.1f} / {mb_tot:.1f} MB ({int(pct * 100)}%)"

            actual_sha = sha.hexdigest().lower()

            # 2. Verify SHA-256
            with self._lock:
                self.state = "VERIFYING"
                self.progress = 1.0
                self.status_text = (
                    "Verifying checksum and Apple Developer signature..."
                    if sys.platform == "darwin"
                    else "Verifying SHA-256 checksum..."
                )

            if not self._verify_checksum(info, zip_name, actual_sha):
                return

            # 3. Platform-specific extraction, verification & swap-and-relaunch
            if sys.platform == "darwin":
                ok = self._install_macos(tmp_dir, zip_path)
            elif sys.platform == "win32":
                ok = self._install_windows(tmp_dir, zip_path)
            else:
                self._set_error("Automatic install is supported on macOS and Windows.")
                return

            if ok:
                cleanup_tmp = False
                with self._lock:
                    self.ready_to_quit = True
        except Exception as exc:
            self._set_error(f"Update failed: {exc}")
        finally:
            if cleanup_tmp:
                shutil.rmtree(tmp_dir, ignore_errors=True)
