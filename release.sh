#!/usr/bin/env bash
set -euo pipefail

# One-command Rust macOS Universal (arm64 + x86_64) build, sign, notarize, staple, DMG/ZIP & GitHub Release script
# Usage:
#   ./release.sh                     # Build & publish current version from Cargo.toml
#   ./release.sh --version 2.0.1     # Bump Cargo.toml, build, commit, push & publish
#   ./release.sh --skip-publish      # Build, sign & notarize locally in dist/ only
#   ./release.sh --notes "Highlights"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

SIGN_IDENTITY="${SIGN_IDENTITY:-Developer ID Application: Al Anderson (MR38JF8F3H)}"
NOTARY_PROFILE="${NOTARY_PROFILE:-qn-bench-notary}"
APP_NAME="QN Bench"
APP_SLUG="QN-Bench"
BUNDLE_ID="com.alanderson.qn-bench"

BUMP_VERSION=""
SKIP_PUBLISH=0
SKIP_NOTARIZE=0
RELEASE_NOTES=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --version)
            BUMP_VERSION="$2"
            shift 2
            ;;
        --skip-publish)
            SKIP_PUBLISH=1
            shift
            ;;
        --no-notarize)
            SKIP_NOTARIZE=1
            shift
            ;;
        --notes)
            RELEASE_NOTES="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

if [[ -n "$BUMP_VERSION" ]]; then
    python3 - "$BUMP_VERSION" <<'PY'
import re, sys
from pathlib import Path
new_ver = sys.argv[1].lstrip("vV")
p = Path("Cargo.toml")
txt = p.read_text(encoding="utf-8")
txt = re.sub(r'(?m)^version\s*=\s*"[^"]+"', f'version = "{new_ver}"', txt, count=1)
p.write_text(txt, encoding="utf-8")
print(f"Updated Cargo.toml version to {new_ver}")
PY
fi

VERSION="$(python3 -c 'import re; print(re.search(r"(?m)^version\s*=\s*\"([^\"]+)\"", open("Cargo.toml").read()).group(1))')"
TAG="v${VERSION}"
APP_PATH="dist/${APP_NAME}.app"
# Keep QN-Bench-macos-arm64.zip filename so v1.x macOS self-updaters upgrade seamlessly to Universal v2.x!
ZIP_PATH="dist/${APP_SLUG}-macos-arm64.zip"
DMG_PATH="dist/${APP_SLUG}-${VERSION}-macos-universal.dmg"
SUMS_PATH="dist/SHA256SUMS.txt"

echo "==> Running Rust physics & FIPS-197 AES-256 test suite..."
cargo test --offline

echo "==> Generating procedural Retina macOS icon (.icns)..."
rm -rf build dist
cargo run --release --bin gen_icons -- --icns

echo "==> Compiling Rust macOS Universal Binary (arm64 + x86_64) for ${APP_NAME} ${TAG}..."
cargo build --release --bin qn_bench --target aarch64-apple-darwin
cargo build --release --bin qn_bench --target x86_64-apple-darwin

echo "==> Assembling ${APP_PATH} bundle..."
mkdir -p "${APP_PATH}/Contents/MacOS" "${APP_PATH}/Contents/Resources"
lipo -create \
    "target/aarch64-apple-darwin/release/qn_bench" \
    "target/x86_64-apple-darwin/release/qn_bench" \
    -output "${APP_PATH}/Contents/MacOS/${APP_NAME}"
chmod +x "${APP_PATH}/Contents/MacOS/${APP_NAME}"
cp "build/${APP_SLUG}.icns" "${APP_PATH}/Contents/Resources/${APP_SLUG}.icns"

cat > "${APP_PATH}/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleDisplayName</key>
    <string>${APP_NAME}</string>
    <key>CFBundleExecutable</key>
    <string>${APP_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleIconFile</key>
    <string>${APP_SLUG}</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSMinimumSystemVersion</key>
    <string>12.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.educational-games</string>
    <key>NSHumanReadableCopyright</key>
    <string>Copyright © Al Anderson</string>
</dict>
</plist>
PLIST

echo "==> Signing ${APP_PATH} with Developer ID (Hardened Runtime)..."
codesign --force --options runtime --timestamp \
    --sign "$SIGN_IDENTITY" "${APP_PATH}/Contents/MacOS/${APP_NAME}"
codesign --force --options runtime --timestamp \
    --sign "$SIGN_IDENTITY" "$APP_PATH"

echo "==> Verifying code signature..."
codesign --verify --deep --strict --verbose=2 "$APP_PATH"

echo "==> Creating ZIP archive for notarization & self-updater..."
rm -f "$ZIP_PATH"
ditto -c -k --keepParent "$APP_PATH" "$ZIP_PATH"

if [[ "$SKIP_NOTARIZE" -eq 0 ]]; then
    echo "==> Submitting ${APP_NAME}.app ZIP to Apple Notary Service..."
    xcrun notarytool submit "$ZIP_PATH" \
        --keychain-profile "$NOTARY_PROFILE" \
        --wait

    echo "==> Stapling notarization ticket to ${APP_PATH}..."
    xcrun stapler staple "$APP_PATH"
    xcrun stapler validate "$APP_PATH"

    echo "==> Re-creating self-updater ZIP from stapled ${APP_PATH}..."
    rm -f "$ZIP_PATH"
    ditto -c -k --keepParent "$APP_PATH" "$ZIP_PATH"
fi

echo "==> Creating macOS installer DMG (${DMG_PATH})..."
DMG_STAGE="$(mktemp -d)"
trap 'rm -rf "$DMG_STAGE"' EXIT
cp -R "$APP_PATH" "$DMG_STAGE/"
ln -s /Applications "$DMG_STAGE/Applications"

hdiutil create \
    -volname "${APP_NAME} ${VERSION}" \
    -srcfolder "$DMG_STAGE" \
    -ov \
    -format UDZO \
    "$DMG_PATH"
rm -rf "$DMG_STAGE"

echo "==> Signing DMG..."
codesign --force --timestamp --sign "$SIGN_IDENTITY" "$DMG_PATH"

if [[ "$SKIP_NOTARIZE" -eq 0 ]]; then
    echo "==> Notarizing DMG..."
    xcrun notarytool submit "$DMG_PATH" \
        --keychain-profile "$NOTARY_PROFILE" \
        --wait

    echo "==> Stapling notarization ticket to DMG..."
    xcrun stapler staple "$DMG_PATH"
    xcrun stapler validate "$DMG_PATH"

    echo "==> Verifying Gatekeeper assessment..."
    spctl --assess --type execute --verbose "$APP_PATH"
fi

echo "==> Generating SHA-256 checksums..."
(
    cd dist
    shasum -a 256 "$(basename "$DMG_PATH")" "$(basename "$ZIP_PATH")" > "$(basename "$SUMS_PATH")"
)
cat "$SUMS_PATH"

if [[ "$SKIP_PUBLISH" -eq 1 ]]; then
    echo "==> Build complete (skipped GitHub Release publish)."
    exit 0
fi

if [[ -n "$BUMP_VERSION" ]]; then
    git add Cargo.toml Cargo.lock
    git commit -m "Release ${TAG}"
    git push origin main
fi

echo "==> Publishing GitHub Release ${TAG}..."
if [[ -z "$RELEASE_NOTES" ]]; then
    RELEASE_NOTES="Desktop & WebAssembly release of **QN Bench — Quantum Networking Optical Bench** (${TAG}), powered by Rust + egui.

### Play or Download
- **Play in Browser (WebAssembly):** https://skcalanderson2.github.io/qn_bench/play/
- **macOS Universal (Apple Silicon & Intel — Signed & Notarized):** \`QN-Bench-${VERSION}-macos-universal.dmg\`
- **Windows (x64 — Built via GitHub Actions):** \`QN-Bench-${VERSION}-windows-x64-setup.exe\` or portable \`QN-Bench-windows-x64.zip\` *(unsigned: click More info → Run anyway if SmartScreen prompts)*
- **Auto-Update Archives (\`.zip\`):** Used automatically by the opt-in in-app updater (verified via SHA-256 + Apple Developer ID signature on macOS)."
fi

if gh release view "$TAG" >/dev/null 2>&1; then
    gh release upload "$TAG" "$DMG_PATH" "$ZIP_PATH" "$SUMS_PATH" --clobber
    gh release edit "$TAG" --title "${APP_NAME} ${TAG}" --notes "$RELEASE_NOTES"
else
    gh release create "$TAG" "$DMG_PATH" "$ZIP_PATH" "$SUMS_PATH" \
        --title "${APP_NAME} ${TAG}" \
        --notes "$RELEASE_NOTES"
fi

echo "==> Released ${TAG} successfully!"
