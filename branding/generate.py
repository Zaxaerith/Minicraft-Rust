"""Generate Minicraft Rust bitmap branding from the bundled original title.

Requires Pillow only for regeneration: python branding/generate.py
"""

from pathlib import Path

from PIL import Image


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "assets/assets/textures/gui/title.png"
TITLE = ROOT / "assets/assets/textures/gui/title_rust.png"
ICON = ROOT / "assets/branding/minicraft-rust.ico"
ICON_PREVIEW = ROOT / "assets/branding/minicraft-rust.png"

YELLOW = (229, 229, 137, 255)
GREEN = (83, 129, 83, 255)
DARK = (23, 46, 23, 255)

# Two-pixel strokes and the right-leaning silhouette follow the original
# MINICRAFT bitmap; the latter is copied pixel-for-pixel, not re-typeset.
LETTER_MASKS = {
    "R": ("YYYYYYY..", "YY....YY.", "YY....YY.", "YYYYYYY..", "YYYYY....", "YY.YY....", "YY..YY...", "YY...YY..", "YY....YY."),
    "U": ("YY.....YY", "YY.....YY", "YY.....YY", "YY.....YY", "YY.....YY", "YY.....YY", "YY.....YY", ".YYYYYYY.", "..YYYYY.."),
    "S": (".YYYYYYYY", "YY.......", "YY.......", "YYYYYYY..", "..YYYYYY.", ".......YY", ".......YY", "YYYYYYYY.", ".YYYYYY.."),
    "T": ("YYYYYYYYY", "...YYY...", "...YYY...", "...YYY...", "...YYY...", "...YYY...", "...YYY...", "...YYY...", "...YYY..."),
}


def logo_letter(canvas: Image.Image, char: str, left: int, top: int) -> None:
    """Three-colour bevel with the original title's five-pixel italic lean."""
    rows = LETTER_MASKS[char]
    core = set()
    for y in range(11):
        row = rows[y * len(rows) // 11]
        slant = (10 - y) // 2
        for x, pixel in enumerate(row):
            if pixel == "Y":
                core.add((left + x + slant, top + y))

    for x, y in core:
        for dx, dy in ((1, 2), (2, 1), (2, 2)):
            canvas.putpixel((x + dx, y + dy), DARK)
    for x, y in core:
        for dy in (-1, 0, 1):
            for dx in (-1, 0, 1):
                canvas.putpixel((x + dx, y + dy), GREEN)
    for x, y in core:
        canvas.putpixel((x, y), YELLOW)


def original_m(source: Image.Image) -> Image.Image:
    """Rowwise cut: a rectangular crop includes the neighbouring italic I."""
    right_edges = (4, 22, 22, 23, 23, 22, 21, 20, 20, 19, 19, 18, 18, 18, 17, 17)
    letter = Image.new("RGBA", (19, 16))
    for y, right in enumerate(right_edges):
        for x in range(4, right):
            letter.putpixel((x - 4, y), source.getpixel((x, y)))
    return letter


def small_r(canvas: Image.Image, left: int, top: int) -> None:
    """A separate five-pixel R; much smaller than the 19-pixel M."""
    rows = ("YYYY.", "Y...Y", "YYYY.", "Y.Y..", "Y..Y.")
    core = {(left + x, top + y) for y, row in enumerate(rows) for x, px in enumerate(row) if px == "Y"}
    for x, y in core:
        canvas.putpixel((x + 1, y + 1), DARK)
    for x, y in core:
        for dx, dy in ((-1, 0), (0, -1), (1, 0), (0, 1)):
            canvas.putpixel((x + dx, y + dy), GREEN)
    for x, y in core:
        canvas.putpixel((x, y), YELLOW)


def main() -> None:
    source = Image.open(SOURCE).convert("RGBA")
    title = Image.new("RGBA", (160, 16))
    title.alpha_composite(source.crop((0, 0, 102, 16)))
    for index, char in enumerate("RUST"):
        logo_letter(title, char, 105 + 13 * index, 2)
    title.save(TITLE)

    icon = Image.new("RGBA", (32, 32))
    icon.alpha_composite(original_m(source), (2, 13))
    # Superscript R touches the upper-right arm of the M, not a detached mark.
    small_r(icon, 19, 9)
    ICON.parent.mkdir(parents=True, exist_ok=True)
    icon.save(ICON_PREVIEW)
    sizes = (16, 32, 48, 64, 128, 256)
    frames = {size: icon.resize((size, size), Image.Resampling.NEAREST) for size in sizes}
    frames[256].save(
        ICON,
        format="ICO",
        sizes=[(size, size) for size in sizes],
        append_images=[frames[size] for size in sizes if size != 256],
    )


if __name__ == "__main__":
    main()
