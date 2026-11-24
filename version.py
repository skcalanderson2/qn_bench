"""Version and release metadata for QN Bench — Quantum Networking Optical Bench."""

import sys

APP_NAME = "QN Bench"
APP_SLUG = "QN-Bench"
APP_VERSION = "1.1.0"
BUNDLE_ID = "com.alanderson.qn-bench"
GITHUB_REPO = "skcalanderson2/qn_bench"
TEAM_ID = "MR38JF8F3H"
MACOS_ZIP_NAME = f"{APP_SLUG}-macos-arm64.zip"
WINDOWS_ZIP_NAME = f"{APP_SLUG}-windows-x64.zip"
ASSET_ZIP_NAME = WINDOWS_ZIP_NAME if sys.platform == "win32" else MACOS_ZIP_NAME
CHECKSUMS_FILE_NAME = "SHA256SUMS-windows.txt" if sys.platform == "win32" else "SHA256SUMS.txt"
RELEASES_API_URL = f"https://api.github.com/repos/{GITHUB_REPO}/releases/latest"
RELEASES_PAGE_URL = f"https://github.com/{GITHUB_REPO}/releases/latest"
DOWNLOAD_PAGE_URL = "https://skcalanderson2.github.io/qn_bench/"
