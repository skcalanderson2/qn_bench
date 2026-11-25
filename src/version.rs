#![allow(dead_code)]
//! Version and release metadata for QN Bench.

pub const APP_NAME: &str = "QN Bench";
pub const APP_SLUG: &str = "QN-Bench";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUNDLE_ID: &str = "com.alanderson.qn-bench";
pub const GITHUB_REPO: &str = "skcalanderson2/qn_bench";
pub const TEAM_ID: &str = "MR38JF8F3H";

pub const MACOS_ZIP_NAME: &str = "QN-Bench-macos-arm64.zip";
pub const WINDOWS_ZIP_NAME: &str = "QN-Bench-windows-x64.zip";

pub const RELEASES_API_URL: &str =
    "https://api.github.com/repos/skcalanderson2/qn_bench/releases/latest";
pub const RELEASES_PAGE_URL: &str =
    "https://github.com/skcalanderson2/qn_bench/releases/latest";
pub const DOWNLOAD_PAGE_URL: &str = "https://skcalanderson2.github.io/qn_bench/";
pub const PLAY_PAGE_URL: &str = "https://skcalanderson2.github.io/qn_bench/play/";
pub const VIDEOS_MANIFEST_URL: &str = "https://skcalanderson2.github.io/qn_bench/videos.json";

#[cfg(target_os = "windows")]
pub const PLATFORM_ZIP_NAME: &str = WINDOWS_ZIP_NAME;
#[cfg(not(target_os = "windows"))]
pub const PLATFORM_ZIP_NAME: &str = MACOS_ZIP_NAME;

#[cfg(target_os = "windows")]
pub const CHECKSUMS_FILE_NAME: &str = "SHA256SUMS-windows.txt";
#[cfg(not(target_os = "windows"))]
pub const CHECKSUMS_FILE_NAME: &str = "SHA256SUMS.txt";
