"""Regenerates the PortWatch icon set (dev-only helper, requires Pillow).

    python tools/generate_icons.py

Writes PNGs and a multi-size .ico into src-tauri/icons and public/.
"""
import math
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
SIZE = 1024
BG = (16, 19, 24, 255)
EDGE = (42, 49, 59, 255)
DIM = (44, 53, 65, 255)
ACCENT = (91, 214, 196, 255)


def draw_logo() -> Image.Image:
    img = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    d.rounded_rectangle((24, 24, SIZE - 24, SIZE - 24), radius=230, fill=EDGE)
    d.rounded_rectangle((40, 40, SIZE - 40, SIZE - 40), radius=214, fill=BG)
    cx = cy = SIZE // 2
    w = 52
    for r in (170, 320):
        d.ellipse((cx - r, cy - r, cx + r, cy + r), outline=DIM, width=w)

    def arc(r, a0, a1):
        d.arc((cx - r, cy - r, cx + r, cy + r), a0, a1, fill=ACCENT, width=w)
        for a in (a0, a1):
            x = cx + (r - w / 2) * math.cos(math.radians(a))
            y = cy + (r - w / 2) * math.sin(math.radians(a))
            d.ellipse((x - w / 2, y - w / 2, x + w / 2, y + w / 2), fill=ACCENT)

    arc(320, -70, 25)
    arc(170, 110, 200)
    d.ellipse((cx - 58, cy - 58, cx + 58, cy + 58), fill=ACCENT)
    return img


def main() -> None:
    logo = draw_logo()
    out = ROOT / "src-tauri" / "icons"
    out.mkdir(parents=True, exist_ok=True)
    for name, px in (("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256), ("icon.png", 512)):
        logo.resize((px, px), Image.LANCZOS).save(out / name)
    sizes = [(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)]
    logo.resize((256, 256), Image.LANCZOS).save(out / "icon.ico", sizes=sizes)
    pub = ROOT / "public"
    pub.mkdir(exist_ok=True)
    logo.resize((64, 64), Image.LANCZOS).save(pub / "favicon.png")
    print("icons written to", out)


if __name__ == "__main__":
    main()
