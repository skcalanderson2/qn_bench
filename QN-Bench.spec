# -*- mode: python ; coding: utf-8 -*-
"""PyInstaller spec for building the standalone macOS QN Bench.app bundle."""

import sys
sys.path.insert(0, SPECPATH)

from PyInstaller.utils.hooks import collect_data_files
from version import APP_NAME, APP_VERSION, BUNDLE_ID

certifi_datas = collect_data_files("certifi")

a = Analysis(
    ["qn_bench.py"],
    pathex=["."],
    binaries=[],
    datas=certifi_datas,
    hiddenimports=["certifi"],
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=[
        "tkinter",
        "_tkinter",
        "unittest",
        "test",
        "pydoc",
        "doctest",
        "numpy",
        "scipy",
        "matplotlib",
    ],
    noarchive=False,
    optimize=0,
)

pyz = PYZ(a.pure)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name=APP_NAME,
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=False,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.datas,
    strip=False,
    upx=False,
    upx_exclude=[],
    name=APP_NAME,
)

app = BUNDLE(
    coll,
    name=f"{APP_NAME}.app",
    icon="build/QN-Bench.icns",
    bundle_identifier=BUNDLE_ID,
    version=APP_VERSION,
    info_plist={
        "CFBundleName": APP_NAME,
        "CFBundleDisplayName": APP_NAME,
        "CFBundleShortVersionString": APP_VERSION,
        "CFBundleVersion": APP_VERSION,
        "CFBundlePackageType": "APPL",
        "NSHighResolutionCapable": True,
        "LSMinimumSystemVersion": "12.0",
        "LSApplicationCategoryType": "public.app-category.educational-games",
        "NSHumanReadableCopyright": "Copyright © Al Anderson",
    },
)
