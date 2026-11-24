"""Procedural macOS .icns icon generator for QN Bench.

Generates a 1024x1024 multi-resolution Retina icon using Pygame primitives
(zero external assets) and packs it directly into build/QN-Bench.icns.
"""

import io
import math
import os
import struct
import sys
from pathlib import Path

os.environ.setdefault("SDL_VIDEODRIVER", "dummy")
import pygame


def draw_glow(surf, cx, cy, radius, color, max_alpha=150):
    g = pygame.Surface((radius * 2, radius * 2), pygame.SRCALPHA)
    for r in range(radius, 0, -4):
        a = int(max_alpha * ((1.0 - r / radius) ** 2.2))
        pygame.draw.circle(g, (*color, a), (radius, radius), r)
    surf.blit(g, (cx - radius, cy - radius))


def render_master_icon(size=1024):
    pygame.init()
    surf = pygame.Surface((size, size), pygame.SRCALPHA)

    # macOS-style inset squircle padding (~10% margin)
    pad = int(size * 0.085)
    card = pygame.Rect(pad, pad, size - pad * 2, size - pad * 2)
    radius = int(card.w * 0.225)

    # Clipped card background
    card_surf = pygame.Surface((size, size), pygame.SRCALPHA)
    pygame.draw.rect(card_surf, (13, 19, 30, 255), card, border_radius=radius)

    # Subtle breadboard grid & tapped mounting holes inside card
    grid_step = 76
    for gx in range(card.x + 56, card.right - 40, grid_step):
        pygame.draw.line(card_surf, (24, 34, 50, 140), (gx, card.y + 30), (gx, card.bottom - 30), 2)
    for gy in range(card.y + 56, card.bottom - 40, grid_step):
        pygame.draw.line(card_surf, (24, 34, 50, 140), (card.x + 30, gy), (card.right - 30, gy), 2)
    for gx in range(card.x + 94, card.right - 60, grid_step):
        for gy in range(card.y + 94, card.bottom - 60, grid_step):
            pygame.draw.circle(card_surf, (8, 12, 20, 220), (gx, gy), 5)

    cy = int(size * 0.44)
    cx = size // 2

    # Ambient glows: 780nm red left, QFC purple center, 1550nm cyan right
    draw_glow(card_surf, card.x + 170, cy, 210, (244, 63, 94), 115)
    draw_glow(card_surf, cx, cy, 240, (167, 139, 250), 135)
    draw_glow(card_surf, card.right - 170, cy, 230, (56, 189, 248), 145)

    # Incoming 780nm visible beam (high frequency wave on left)
    red_pts = []
    for x in range(card.x + 52, cx - 72, 4):
        t = (x - card.x) * 0.065
        red_pts.append((x, cy + int(24 * math.sin(t))))
    if len(red_pts) > 1:
        pygame.draw.lines(card_surf, (244, 63, 94), False, red_pts, 14)
        pygame.draw.lines(card_surf, (255, 200, 212), False, red_pts, 5)

    # Outgoing 1550nm C-band telecom beam (longer wavelength on right)
    cyan_pts = []
    for x in range(cx + 72, card.right - 52, 4):
        t = (x - cx) * 0.032
        cyan_pts.append((x, cy + int(30 * math.sin(t))))
    if len(cyan_pts) > 1:
        pygame.draw.lines(card_surf, (56, 189, 248), False, cyan_pts, 16)
        pygame.draw.lines(card_surf, (215, 246, 255), False, cyan_pts, 6)

    # Central PPLN Frequency Converter crystal prism
    crystal = pygame.Rect(cx - 86, cy - 110, 172, 220)
    pygame.draw.rect(card_surf, (38, 28, 72), crystal, border_radius=26)
    for px in range(crystal.x + 26, crystal.right - 16, 24):
        pygame.draw.line(card_surf, (167, 139, 250), (px, crystal.y + 18), (px, crystal.bottom - 18), 6)
    pygame.draw.rect(card_surf, (216, 200, 255), crystal, 6, border_radius=26)

    # Crisp QN Bench badge text at bottom
    font_main = pygame.font.SysFont("arial,dejavusans", 124, bold=True)
    font_sub = pygame.font.SysFont("couriernew,menlo,monospace", 36, bold=True)
    t_main = font_main.render("QN Bench", True, (238, 244, 252))
    t_sub = font_sub.render("1550nm OPTICAL BENCH", True, (56, 189, 248))
    card_surf.blit(t_main, (cx - t_main.get_width() // 2, card.bottom - 240))
    card_surf.blit(t_sub, (cx - t_sub.get_width() // 2, card.bottom - 95))

    # Mask card_surf to rounded rect cleanly
    mask = pygame.Surface((size, size), pygame.SRCALPHA)
    pygame.draw.rect(mask, (255, 255, 255, 255), card, border_radius=radius)
    card_surf.blit(mask, (0, 0), special_flags=pygame.BLEND_RGBA_MIN)
    surf.blit(card_surf, (0, 0))

    # Crisp outer rim highlight
    pygame.draw.rect(surf, (56, 189, 248, 200), card, 8, border_radius=radius)
    pygame.quit()
    return surf


def _surface_to_png_bytes(surf):
    buf = io.BytesIO()
    pygame.image.save(surf, buf, "icon.png")
    return buf.getvalue()


def build_icns(output_path=Path("build/QN-Bench.icns")):
    """Pack multi-resolution Retina PNG chunks directly into a macOS .icns container."""
    output_path = Path(output_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    master = render_master_icon(1024)
    # Standard Apple ICNS PNG-based icon type codes (16px through 1024px Retina)
    icon_types = [
        (b"icp4", 16),
        (b"icp5", 32),
        (b"icp6", 64),
        (b"ic07", 128),
        (b"ic08", 256),
        (b"ic09", 512),
        (b"ic10", 1024),
        (b"ic11", 32),
        (b"ic12", 64),
        (b"ic13", 256),
        (b"ic14", 512),
    ]

    png_cache = {}
    chunks = []
    for ostype, px in icon_types:
        if px not in png_cache:
            scaled = pygame.transform.smoothscale(master, (px, px)) if px != 1024 else master
            png_cache[px] = _surface_to_png_bytes(scaled)
        data = png_cache[px]
        chunks.append(struct.pack(">4sI", ostype, 8 + len(data)) + data)

    body = b"".join(chunks)
    header = struct.pack(">4sI", b"icns", 8 + len(body))
    output_path.write_bytes(header + body)
    print(f"Generated {output_path} ({len(header) + len(body):,} bytes)")


def build_ico(output_path=Path("build/QN-Bench.ico")):
    """Pack multi-resolution PNG frames directly into a Windows .ico container."""
    output_path = Path(output_path)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    master = render_master_icon(1024)
    sizes = [16, 24, 32, 48, 64, 128, 256]
    images = []
    for px in sizes:
        scaled = pygame.transform.smoothscale(master, (px, px))
        images.append((px, _surface_to_png_bytes(scaled)))

    count = len(images)
    header = struct.pack("<HHH", 0, 1, count)
    offset = 6 + 16 * count
    entries = []
    payloads = []
    for px, data in images:
        dim = 0 if px >= 256 else px
        entries.append(struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(data), offset))
        payloads.append(data)
        offset += len(data)

    raw = header + b"".join(entries) + b"".join(payloads)
    output_path.write_bytes(raw)
    print(f"Generated {output_path} ({len(raw):,} bytes)")


if __name__ == "__main__":
    args = set(sys.argv[1:])
    if "--all" in args:
        build_icns()
        build_ico()
    elif "--ico" in args:
        build_ico()
    elif "--icns" in args:
        build_icns()
    elif sys.platform == "win32":
        build_ico()
    else:
        build_icns()
