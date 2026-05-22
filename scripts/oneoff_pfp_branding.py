#!/usr/bin/env python3
"""
One-off branding helper for Thasia.

Goals:
- Create a framed (rounded-squircle) app icon from a source image
- Generate all Tauri icons via `pnpm tauri icon`
- Extract a small palette from the image and emit an Anasthasia-compatible flavour CSS
- Generate an AVIF copy for the frontend

This script uses Pillow (PIL). Install system-wide for this one-off:
  python3 -m pip install --user pillow pillow-avif-plugin

(No ImageMagick dependency.)

Usage:
  python3 scripts/oneoff_pfp_branding.py --input /path/to/pfp.png

It will write:
  - app-icon.png
  - src-tauri/icons/* (via tauri icon)
  - src/flavours/auric-noir.css
  - static/brand/pfp.avif

Note:
  If your input image has a solid black background and you want transparency,
  you can optionally add `--make-bg-transparent` (best-effort).
"""

from __future__ import annotations

import argparse
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List, Tuple

from PIL import Image, ImageChops, ImageDraw, ImageOps

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_FLAVOUR_PATH = ROOT / "src" / "flavours" / "auric-noir.css"
DEFAULT_STATIC_AVIF = ROOT / "static" / "brand" / "pfp.avif"
DEFAULT_APP_ICON = ROOT / "app-icon.png"


@dataclass(frozen=True)
class RGB:
    r: int
    g: int
    b: int

    def clamp(self) -> "RGB":
        return RGB(
            max(0, min(255, int(self.r))),
            max(0, min(255, int(self.g))),
            max(0, min(255, int(self.b))),
        )

    def to_hex(self) -> str:
        return f"#{self.r:02x}{self.g:02x}{self.b:02x}"


def run(cmd: List[str]) -> None:
    subprocess.run(cmd, cwd=str(ROOT), check=True)


def srgb_luma(c: RGB) -> float:
    # relative luma in sRGB-ish space
    return 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b


def rgb_to_hsv(c: RGB) -> Tuple[float, float, float]:
    r, g, b = c.r / 255.0, c.g / 255.0, c.b / 255.0
    mx = max(r, g, b)
    mn = min(r, g, b)
    d = mx - mn

    h = 0.0
    if d != 0:
        if mx == r:
            h = ((g - b) / d) % 6
        elif mx == g:
            h = (b - r) / d + 2
        else:
            h = (r - g) / d + 4
        h *= 60

    s = 0.0 if mx == 0 else d / mx
    v = mx
    return h, s, v


def mix(a: RGB, b: RGB, t: float) -> RGB:
    # t=0 -> a, t=1 -> b
    return RGB(
        int(round(a.r + (b.r - a.r) * t)),
        int(round(a.g + (b.g - a.g) * t)),
        int(round(a.b + (b.b - a.b) * t)),
    )


def darken(c: RGB, amount: float) -> RGB:
    return mix(c, RGB(0, 0, 0), amount).clamp()


def lighten(c: RGB, amount: float) -> RGB:
    return mix(c, RGB(255, 255, 255), amount).clamp()


def ensure_parent(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def rgb_tuple(c: RGB) -> Tuple[int, int, int]:
    return (c.r, c.g, c.b)


def diagonal_gradient(
    size: int, start: RGB, end: RGB, angle: float, *, scale: float = 2.0
) -> Image.Image:
    grad_size = int(round(size * scale))
    grad = Image.linear_gradient("L").resize(
        (grad_size, grad_size), resample=Image.BICUBIC
    )
    grad = ImageOps.colorize(grad, rgb_tuple(start), rgb_tuple(end)).convert("RGB")
    grad = grad.rotate(angle, resample=Image.BICUBIC, expand=True)
    grad = ImageOps.fit(grad, (size, size), method=Image.BICUBIC, centering=(0.5, 0.5))
    return grad


def squircle_mask(size: int, radius: int) -> Image.Image:
    scale = 4
    mask = Image.new("L", (size * scale, size * scale), 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle(
        [0, 0, size * scale, size * scale], radius=radius * scale, fill=255
    )
    return mask.resize((size, size), resample=Image.LANCZOS)


def extract_palette(input_path: Path, colors: int = 12) -> List[Tuple[int, RGB]]:
    """Returns [(count, RGB)] sorted by count descending."""
    img = Image.open(input_path)
    img = ImageOps.exif_transpose(img).convert("RGBA")

    # Composite on black to avoid transparency skewing palette
    base = Image.new("RGBA", img.size, (0, 0, 0, 255))
    img = Image.alpha_composite(base, img).convert("RGB")

    img = img.resize((256, 256), resample=Image.BOX)
    quant = img.quantize(colors=colors, method=Image.MEDIANCUT, dither=Image.NONE)
    palette = quant.getpalette() or []
    counts = quant.getcolors(256 * 256) or []

    results: List[Tuple[int, RGB]] = []
    for count, idx in counts:
        base_idx = idx * 3
        if base_idx + 2 >= len(palette):
            continue
        r, g, b = palette[base_idx], palette[base_idx + 1], palette[base_idx + 2]
        results.append((count, RGB(r, g, b)))

    results.sort(key=lambda x: x[0], reverse=True)
    return results


def pick_accent(palette: List[Tuple[int, RGB]]) -> RGB:
    # Choose the most "interesting" color: saturated and not too dark.
    best = None
    best_score = -1.0
    for count, c in palette:
        h, s, v = rgb_to_hsv(c)
        l = srgb_luma(c)
        # Skip near-black / near-white
        if l < 35 or l > 235:
            continue
        # Skip near-greys
        if s < 0.18:
            continue
        score = (s * 1.3 + v) * (1.0 + min(1.0, count / 10000.0))
        if score > best_score:
            best_score = score
            best = c

    return best if best is not None else RGB(200, 149, 32)  # fallback gold


def pick_bg(palette: List[Tuple[int, RGB]]) -> RGB:
    # Darkest frequent color
    best = None
    best_l = 1e9
    for count, c in palette[:8]:
        l = srgb_luma(c)
        if l < best_l:
            best_l = l
            best = c
    return best if best is not None else RGB(6, 5, 5)


def write_flavour(path: Path, bg_dark: RGB, accent: RGB) -> None:
    """Emit an Anasthasia flavour CSS."""
    # Derive dark tokens
    surface_dark = lighten(bg_dark, 0.08)
    panel_dark = lighten(bg_dark, 0.12)

    text_dark = RGB(250, 247, 240)
    muted_dark = mix(text_dark, panel_dark, 0.35)

    accent_strong_dark = lighten(accent, 0.20)

    # Derive light tokens from accent (warm parchment)
    bg_light = mix(RGB(255, 255, 255), accent, 0.07)
    surface_light = mix(RGB(255, 255, 255), accent, 0.03)
    panel_light = mix(RGB(255, 255, 255), accent, 0.09)

    text_light = darken(mix(accent, RGB(0, 0, 0), 0.75), 0.15)
    muted_light = mix(text_light, panel_light, 0.55)

    accent_strong_light = darken(accent, 0.22)

    css = f"""/* Auric Noir — generated one-off flavour for Thasia (pfp-inspired) */
:root {{
    --color-anasthasia-bg: {bg_light.to_hex()};
    --color-anasthasia-surface: {surface_light.to_hex()};
    --color-anasthasia-panel: {panel_light.to_hex()};
    --color-anasthasia-border: rgba(20, 16, 12, 0.12);
    --color-anasthasia-text: {text_light.to_hex()};
    --color-anasthasia-muted: {muted_light.to_hex()};
    --color-anasthasia-accent: {accent.to_hex()};
    --color-anasthasia-accent-strong: {accent_strong_light.to_hex()};

    --background-image-accent-gradient: linear-gradient(
        135deg,
        {lighten(accent, 0.35).to_hex()} 0%,
        {accent.to_hex()} 45%,
        {accent_strong_light.to_hex()} 100%
    );

    --radius-sm: 0.375rem;
    --radius-md: 0.5rem;
    --radius-lg: 0.75rem;
    --radius-xl: 1rem;
    --radius-2xl: 1.25rem;
}}

:root.dark {{
    --color-anasthasia-bg: {bg_dark.to_hex()};
    --color-anasthasia-surface: {surface_dark.to_hex()};
    --color-anasthasia-panel: {panel_dark.to_hex()};
    --color-anasthasia-border: rgba(255, 255, 255, 0.08);
    --color-anasthasia-text: {text_dark.to_hex()};
    --color-anasthasia-muted: {muted_dark.to_hex()};
    --color-anasthasia-accent: {accent.to_hex()};
    --color-anasthasia-accent-strong: {accent_strong_dark.to_hex()};

    --background-image-accent-gradient: linear-gradient(
        135deg,
        {lighten(accent, 0.30).to_hex()} 0%,
        {accent.to_hex()} 45%,
        {darken(accent, 0.35).to_hex()} 100%
    );
}}
"""

    ensure_parent(path)
    path.write_text(css, encoding="utf-8")


def make_framed_app_icon(
    input_path: Path,
    output_path: Path,
    size: int = 1024,
    *,
    accent: RGB,
    bg_dark: RGB,
) -> None:
    """Create a framed icon (rounded squircle approximation) using Pillow.

    No border (macOS 26 style). The squircle fill uses a palette-based gradient.
    """
    ensure_parent(output_path)

    # Tunables
    radius = int(round(size * 0.26))
    inset = int(round(size * 0.04))
    offset_y = int(round(size * 0.04))

    base = darken(bg_dark, 0.08)
    mid = mix(bg_dark, accent, 0.24)
    glow = lighten(accent, 0.32)

    base_img = Image.new("RGB", (size, size), rgb_tuple(base))
    grad = diagonal_gradient(size, base, mid, angle=45, scale=2.0)
    bg = ImageChops.overlay(base_img, grad)

    glow_grad = diagonal_gradient(size, RGB(0, 0, 0), glow, angle=-45, scale=2.0)
    glow_grad = Image.blend(Image.new("RGB", (size, size), (0, 0, 0)), glow_grad, 0.35)
    bg = ImageChops.screen(bg, glow_grad)
    bg = bg.convert("RGBA")

    # Portrait preparation
    portrait = Image.open(input_path)
    portrait = ImageOps.exif_transpose(portrait).convert("RGBA")

    target = size - inset * 2
    portrait = ImageOps.contain(portrait, (target, target), method=Image.LANCZOS)
    portrait_canvas = Image.new("RGBA", (target, target), (0, 0, 0, 0))
    px = (target - portrait.width) // 2
    py = (target - portrait.height) // 2
    portrait_canvas.paste(portrait, (px, py), portrait)

    # Outline silhouette behind the portrait
    outline_scale = 1.2
    outline = portrait_canvas.resize(
        (
            int(round(portrait_canvas.width * outline_scale)),
            int(round(portrait_canvas.height * outline_scale)),
        ),
        resample=Image.LANCZOS,
    )
    outline_alpha = outline.split()[-1]
    outline_img = Image.new("RGBA", outline.size, (0, 0, 0, 255))
    outline_img.putalpha(outline_alpha)

    center_x = inset + target // 2
    center_y = inset + offset_y + target // 2
    outline_dest = (
        int(round(center_x - outline.width / 2)),
        int(round(center_y - outline.height / 2)),
    )
    bg.paste(outline_img, outline_dest, outline_img)

    dest = (inset, inset + offset_y)
    bg.paste(portrait_canvas, dest, portrait_canvas)

    mask = squircle_mask(size, radius)
    bg.putalpha(mask)
    bg.save(output_path)


def maybe_make_bg_transparent(input_path: Path, output_path: Path) -> None:
    """Best-effort: treat near-black as transparent."""
    ensure_parent(output_path)
    img = Image.open(input_path)
    img = ImageOps.exif_transpose(img).convert("RGBA")

    r, g, b, a = img.split()
    max_rg = ImageChops.lighter(r, g)
    max_rgb = ImageChops.lighter(max_rg, b)

    threshold = 12
    softness = 24

    def alpha_for(v: int) -> int:
        if v <= threshold:
            return 0
        if v >= threshold + softness:
            return 255
        return int((v - threshold) * 255 / softness)

    mask = max_rgb.point(alpha_for)
    new_alpha = ImageChops.multiply(a, mask)
    img.putalpha(new_alpha)
    img.save(output_path)


def make_frontend_avif(input_path: Path, output_path: Path) -> None:
    ensure_parent(output_path)
    img = Image.open(input_path)
    img = ImageOps.exif_transpose(img)
    if img.mode not in ("RGB", "RGBA"):
        img = img.convert("RGBA")
    img = ImageOps.contain(img, (768, 768), method=Image.LANCZOS)
    img.save(output_path, format="AVIF", quality=85)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--input", required=True, help="Path to the source image (png/jpg/webp)"
    )
    ap.add_argument(
        "--make-bg-transparent",
        action="store_true",
        help="Try to remove a black background before composing",
    )
    ap.add_argument(
        "--flavour",
        default=str(DEFAULT_FLAVOUR_PATH),
        help="Output CSS flavour path (default: src/flavours/auric-noir.css)",
    )
    args = ap.parse_args()

    inp = Path(args.input).expanduser().resolve()
    if not inp.exists():
        raise SystemExit(f"Input not found: {inp}")

    working = inp
    if args.make_bg_transparent:
        tmp = ROOT / ".tmp" / "pfp-transparent.png"
        ensure_parent(tmp)
        maybe_make_bg_transparent(inp, tmp)
        working = tmp

    # 1) Palette + flavour
    pal = extract_palette(working, colors=12)
    bg = pick_bg(pal)
    accent = pick_accent(pal)

    bg_dark = darken(bg, 0.10)
    write_flavour(Path(args.flavour), bg_dark=bg_dark, accent=accent)

    print("\nPalette (count → hex):")
    for count, color in pal:
        print(f"- {count:>6} → {color.to_hex()}")

    print("\nPicked colors:")
    print(f"- bg (darkest frequent)        : {bg.to_hex()}")
    print(f"- bg_dark (bg -10%)            : {bg_dark.to_hex()}")
    print(f"- accent (most saturated)      : {accent.to_hex()}")

    base = darken(bg_dark, 0.08)
    mid = mix(bg_dark, accent, 0.24)
    glow = lighten(accent, 0.32)
    print("\nIcon gradient tokens:")
    print(f"- base (bg_dark -8%)           : {base.to_hex()}")
    print(f"- mid (bg_dark + accent 24%)   : {mid.to_hex()}")
    print(f"- glow (accent +32%)           : {glow.to_hex()}")

    # 2) Framed icon base
    make_framed_app_icon(
        working,
        DEFAULT_APP_ICON,
        size=1024,
        accent=accent,
        bg_dark=darken(bg, 0.10),
    )

    # 3) Tauri icon set
    run(["pnpm", "tauri", "icon", str(DEFAULT_APP_ICON), "--output", "src-tauri/icons"])

    # 4) Frontend AVIF copy (served via /brand/pfp.avif)
    make_frontend_avif(working, DEFAULT_STATIC_AVIF)

    print("\nDone.")
    print(f"- Wrote: {DEFAULT_APP_ICON.relative_to(ROOT)}")
    print(f"- Wrote: {Path(args.flavour).resolve().relative_to(ROOT)}")
    print(f"- Wrote: {DEFAULT_STATIC_AVIF.relative_to(ROOT)}")
    print("\nNext steps:")
    print("- Restart dev server if running")


if __name__ == "__main__":
    main()
