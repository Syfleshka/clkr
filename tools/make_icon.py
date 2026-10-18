#!/usr/bin/env python3
"""Render the app icon into assets/: clkr.ico (Windows), icon.png and icon-64.rgba (window icon).

Requires: pip install pillow
"""
import pathlib

from PIL import Image, ImageDraw, ImageFont

root = pathlib.Path(__file__).resolve().parent.parent
assets = root / "assets"
S = 1024  # supersampled canvas

A = (124, 108, 255)
B = (64, 140, 255)

# Diagonal gradient.
grad = Image.new("RGBA", (S, S))
px = grad.load()
for y in range(S):
    for x in range(S):
        t = (x + y) / (2 * (S - 1))
        px[x, y] = tuple(round(a + (b - a) * t) for a, b in zip(A, B)) + (255,)

mask = Image.new("L", (S, S), 0)
pad = int(S * 0.04)
ImageDraw.Draw(mask).rounded_rectangle((pad, pad, S - pad, S - pad), radius=int(S * 0.23), fill=255)
icon = Image.new("RGBA", (S, S), (0, 0, 0, 0))
icon.paste(grad, (0, 0), mask)

# Soft top highlight.
hl = Image.new("RGBA", (S, S), (0, 0, 0, 0))
ImageDraw.Draw(hl).ellipse((-S * 0.3, -S * 0.75, S * 1.3, S * 0.45), fill=(255, 255, 255, 34))
icon = Image.alpha_composite(icon, Image.composite(hl, Image.new("RGBA", (S, S)), mask))

# Cursor-click glyph from the bundled Phosphor subset.
font = ImageFont.truetype(str(assets / "phosphor-subset.ttf"), int(S * 0.66))
glyph = ""
d = ImageDraw.Draw(icon)
bbox = d.textbbox((0, 0), glyph, font=font)
w, h = bbox[2] - bbox[0], bbox[3] - bbox[1]
x = (S - w) / 2 - bbox[0] + S * 0.01
y = (S - h) / 2 - bbox[1] + S * 0.01
shadow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
ImageDraw.Draw(shadow).text((x, y + S * 0.015), glyph, font=font, fill=(20, 10, 80, 90))
icon = Image.alpha_composite(icon, shadow)
ImageDraw.Draw(icon).text((x, y), glyph, font=font, fill=(255, 255, 255, 255))

icon.resize((256, 256), Image.LANCZOS).save(assets / "icon.png")
icon.resize((256, 256), Image.LANCZOS).save(
    assets / "clkr.ico", sizes=[(16, 16), (20, 20), (24, 24), (32, 32), (40, 40), (48, 48), (64, 64), (128, 128), (256, 256)]
)
(assets / "icon-64.rgba").write_bytes(icon.resize((64, 64), Image.LANCZOS).tobytes())
print("icon written")
