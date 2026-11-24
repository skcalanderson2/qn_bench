#!/usr/bin/env bash
set -euo pipefail

# One-command macOS build, sign, notarize, staple, package, and GitHub Release script
# Usage:
#   ./release.sh                     # Build & publish current version from version.py
#   ./release.sh --version 1.0.1     # Bump version.py, commit/tag, build & publish
#   ./release.sh --skip-publish      # Build, sign & notarize locally in dist/ only
#   ./release.sh --notes "Highlights"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

SIGN_IDENTITY="${SIGN_IDENTITY:-Developer ID Application: Al Anderson (MR38JF8F3H)}"
NOTARY_PROFILE="${NOTARY_PROFILE:-qn-bench-notary}"
APP_NAME="QN Bench"
APP_SLUG="QN-Bench"
SPEC_FILE="QN-Bench.spec"
VENV_PY="${SCRIPT_DIR}/.venv/bin/python3"
VENV_PYINSTALLER="${SCRIPT_DIR}/.venv/bin/pyinstaller"

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

if [[ ! -x "$VENV_PY" || ! -x "$VENV_PYINSTALLER" ]]; then
    echo "Error: .venv not initialized. Run:" >&2
    echo "  python3 -m venv .venv && .venv/bin/pip install pygame pyinstaller certifi" >&2
    exit 1
fi

if [[ -n "$BUMP_VERSION" ]]; then
    "$VENV_PY" - "$BUMP_VERSION" <<'PY'
import re, sys
from pathlib import Path

new_ver = sys.argv[1].lstrip("vV")
p = Path("version.py")
text = p.read_text(encoding="utf-8")
updated = re.sub(r'APP_VERSION\s*=\s*"[^"]+"', f'APP_VERSION = "{new_ver}"', text, count=1)
p.write_text(updated, encoding="utf-8")
print(f"Updated version.py to {new_ver}")
PY
fi

VERSION="$("$VENV_PY" -c "import version; print(version.APP_VERSION)")"
TAG="v${VERSION}"
APP_PATH="dist/${APP_NAME}.app"
ZIP_PATH="dist/${APP_SLUG}-macos-arm64.zip"
DMG_PATH="dist/${APP_SLUG}-${VERSION}-macos-arm64.dmg"
SUMS_PATH="dist/SHA256SUMS.txt"

echo "==> Building ${APP_NAME} ${TAG} (arm64)..."
rm -rf build dist
"$VENV_PY" build_icon.py
PYINSTALLER_CONFIG_DIR="${SCRIPT_DIR}/build/pyi_cache" "$VENV_PYINSTALLER" --clean --noconfirm "$SPEC_FILE"

echo "==> Signing nested binaries inside ${APP_PATH} (inside-out)..."
sign_item() {
    codesign --force --options runtime --timestamp \
        --entitlements entitlements.plist \
        --sign "$SIGN_IDENTITY" "$1"
}

# Sign all Mach-O libraries and binaries inside Contents/Frameworks and Contents/Resources
while IFS= read -r -d '' f; do
    if file -b "$f" | grep -q "Mach-O"; then
        sign_item "$f"
    fi
done < <(find "${APP_PATH}/Contents" -type f \( -name "*.so" -o -name "*.dylib" -o -perm -111 \) -print0)

# Sign any nested frameworks
while IFS= read -r -d '' fw; do
    sign_item "$fw"
done < <(find "${APP_PATH}/Contents" -type d -name "*.framework" -print0)

# Sign the top-level .app bundle last
sign_item "$APP_PATH"

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
DMG_STAGE="build/dmg_stage"
rm -rf "$DMG_STAGE" "$DMG_PATH"
mkdir -p "$DMG_STAGE"
cp -R "$APP_PATH" "$DMG_STAGE/"
ln -s /Applications "$DMG_STAGE/Applications"

hdiutil create \
    -volname "${APP_NAME} ${VERSION}" \
    -srcfolder "$DMG_STAGE" \
    -ov \
    -format UDZO \
    "$DMG_PATH"

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
    git add version.py
    git commit -m "Release ${TAG}"
    git push origin main
fi

echo "==> Publishing GitHub Release ${TAG}..."
if [[ -z "$RELEASE_NOTES" ]]; then
    RELEASE_NOTES="Desktop release of **QN Bench — Quantum Networking Optical Bench** (${TAG}).

### Downloads
- **macOS (Apple Silicon — Signed & Notarized):** \`QN-Bench-${VERSION}-macos-arm64.dmg\` (drag \`QN Bench.app\` into \`Applications\`)
- **Windows (x64 — Built via GitHub Actions):** \`QN-Bench-${VERSION}-windows-x64-setup.exe\` installer or portable \`QN-Bench-windows-x64.zip\` *(unsigned: click More info → Run anyway if SmartScreen prompts)*
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
