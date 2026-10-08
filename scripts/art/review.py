"""Review images: contact sheet, fake in-game board mockup, and a downscaled atlas fringe check.

These are for judging the art only; nothing here is shipped.
"""

import numpy as np
from PIL import Image, ImageDraw, ImageFont

FONT = "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf"
BG = (52, 47, 60, 255)


def font(size):
    return ImageFont.truetype(FONT, size)


def gpu_resize(img, size):
    """Approximate GPU trilinear minification: box mip chain, then bilinear."""
    w, h = size
    while img.width >= 2 * w and img.height >= 2 * h:
        img = img.reduce(2)
    return img.resize(size, Image.BILINEAR)


def premul_resize(img, size):
    """Resize in premultiplied space (what the engine effectively sees with alpha-bled atlases)."""
    a = np.asarray(img).astype(np.float32) / 255
    pm = np.concatenate([a[..., :3] * a[..., 3:], a[..., 3:]], -1)
    big = Image.fromarray((pm * 255 + 0.5).astype(np.uint8), "RGBA")
    small = np.asarray(gpu_resize(big, size)).astype(np.float32) / 255
    al = small[..., 3:]
    rgb = np.where(al > 0, small[..., :3] / np.maximum(al, 1e-6), 0)
    return Image.fromarray((np.concatenate([np.clip(rgb, 0, 1), al], -1) * 255 + 0.5).astype(np.uint8), "RGBA")


def contact_sheet(images, cols=10, thumb=168):
    names = list(images)
    rows = -(-len(names) // cols)
    cw, ch = thumb + 16, thumb + 34
    sheet = Image.new("RGBA", (cols * cw + 16, rows * ch + 16), BG)
    d = ImageDraw.Draw(sheet)
    f = font(12)
    for i, name in enumerate(names):
        r, c = divmod(i, cols)
        x0, y0 = 16 + c * cw, 16 + r * ch
        img = images[name]
        s = thumb / max(img.size) if img.width > thumb else 1.0
        if img.width <= 32:
            s = 2.0
        t = img.resize((int(img.width * s), int(img.height * s)), Image.LANCZOS)
        d.rectangle([x0 - 1, y0 - 1, x0 + thumb, y0 + thumb], outline=(80, 74, 90, 255))
        sheet.alpha_composite(t, (x0 + (thumb - t.width) // 2, y0 + (thumb - t.height) // 2))
        d.text((x0, y0 + thumb + 4), name, font=f, fill=(225, 220, 235, 255))
    return sheet


# Board layout for the mockup.
#  # wall  . floor  H/G hero1/hero2  r rat  c cyborg  X plank  K keg  O hole
#  W web  T trigger plate  N note
LAYOUT = [
    "##########",
    "#....#...#",
    "#.H..#.r.#",
    "#..X...K.#",
    "#.r.##O.c#",
    "#.W.T.NG.#",
    "##########",
]
FACING = {(2, 2): 90, (5, 7): -45, (2, 7): 180, (4, 2): -135, (4, 8): 45}  # degrees clockwise from north
FRAMES = {(2, 7): 1, (4, 2): 2, (4, 8): 0}
TINT = (1.0, 0.96, 0.9)
# Suggested runtime drop shadow: the `shadow` sprite drawn ~0.95 cell wide, nudged south.
SHADOW_SIZE = 0.95  # cells (sprite canvas edge length)
SHADOW_DY = 0.07  # cells


def board(images, tex, cell, seed=5):
    rng = np.random.default_rng(seed)
    rows, cols = len(LAYOUT), len(LAYOUT[0])
    W, H = cols * cell, rows * cell
    out = Image.new("RGBA", (W, H), (0, 0, 0, 255))
    floor = tex["floor"]
    wall_top = tex["wall_top"].convert("RGBA")
    wall_front = tex["wall_front"].convert("RGBA")
    is_wall = lambda r, c: not (0 <= r < rows and 0 <= c < cols) or LAYOUT[r][c] == "#"

    # floor tiles
    for r in range(rows):
        for c in range(cols):
            if is_wall(r, c):
                continue
            v = rng.integers(16)
            tile = floor.crop(((v % 4) * 256, (v // 4) * 256, (v % 4 + 1) * 256, (v // 4 + 1) * 256))
            if rng.integers(2):
                tile = tile.transpose(Image.FLIP_LEFT_RIGHT)
            if rng.integers(2):
                tile = tile.transpose(Image.FLIP_TOP_BOTTOM)
            tile = gpu_resize(tile.convert("RGBA"), (cell, cell))
            out.alpha_composite(tile, (c * cell, r * cell))
    # tint
    arr = np.asarray(out).astype(np.float32)
    arr[..., :3] *= np.array(TINT)
    out = Image.fromarray(arr.clip(0, 255).astype(np.uint8), "RGBA")

    # walls: world-aligned top texture (256px per cell), front strip where south is floor
    wt = gpu_resize(wall_top, (cell * 4, cell * 4))
    for r in range(rows):
        for c in range(cols):
            if not is_wall(r, c):
                continue
            sx, sy = (c % 4) * cell, (r % 4) * cell
            out.alpha_composite(wt.crop((sx, sy, sx + cell, sy + cell)), (c * cell, r * cell))
    wf = gpu_resize(wall_front, (cell * 4, cell // 2))
    shadow_layer = Image.new("RGBA", (W, H), (0, 0, 0, 0))
    sd = ImageDraw.Draw(shadow_layer)
    for r in range(rows):
        for c in range(cols):
            if is_wall(r, c) and not is_wall(r + 1, c):
                sx = (c % 4) * cell
                out.alpha_composite(wf.crop((sx, 0, sx + cell, cell // 2)), (c * cell, r * cell + cell - cell // 2))
                for k in range(cell // 5):
                    a = int(90 * (1 - k / (cell // 5)) ** 2)
                    sd.line([(c * cell, (r + 1) * cell + k), ((c + 1) * cell - 1, (r + 1) * cell + k)], fill=(0, 0, 0, a))
    out.alpha_composite(shadow_layer)
    # dark outline around wall regions
    d = ImageDraw.Draw(out)
    lw = max(2, cell // 24)
    for r in range(rows):
        for c in range(cols):
            if not is_wall(r, c):
                continue
            x0, y0, x1, y1 = c * cell, r * cell, (c + 1) * cell, (r + 1) * cell
            if not is_wall(r - 1, c):
                d.line([(x0, y0), (x1, y0)], fill=(29, 23, 38, 255), width=lw)
            if not is_wall(r + 1, c):
                d.line([(x0, y1 - lw // 2), (x1, y1 - lw // 2)], fill=(29, 23, 38, 255), width=lw)
            if not is_wall(r, c - 1):
                d.line([(x0, y0), (x0, y1)], fill=(29, 23, 38, 255), width=lw)
            if not is_wall(r, c + 1):
                d.line([(x1 - lw // 2, y0), (x1 - lw // 2, y1)], fill=(29, 23, 38, 255), width=lw)

    def put(img, r, c, angle=0, scale=4 / 3, tint=None, dy=0.0):
        if tint is not None:
            a = np.asarray(img).astype(np.float32)
            a[..., :3] *= np.array(tint)
            img = Image.fromarray(a.clip(0, 255).astype(np.uint8), "RGBA")
        if angle:
            img = img.rotate(-angle, resample=Image.BICUBIC)
        size = int(round(cell * scale * img.width / 256))
        sm = premul_resize(img, (size, size))
        x = int(c * cell + cell / 2 - size / 2)
        y = int(r * cell + cell / 2 - size / 2 + dy * cell)
        out.alpha_composite(sm, (x, y))

    shadow = images["shadow"]
    for r in range(rows):
        for c in range(cols):
            ch = LAYOUT[r][c]
            ang = FACING.get((r, c), 0)
            if ch == "W":
                put(images["web"], r, c)
            elif ch == "T":
                put(images["trigger_plate"], r, c, tint=(0.35, 0.92, 1.0))
                f = font(int(cell * 0.42))
                dd = ImageDraw.Draw(out)
                dd.text((c * cell + cell / 2, r * cell + cell / 2), "3", font=f, fill=(255, 255, 255, 255), anchor="mm", stroke_width=max(1, cell // 32), stroke_fill=(29, 23, 38, 255))
            elif ch == "O":
                put(images["hole_base"], r, c)
                put(images["hole_swirl"], r, c, angle=37)
            elif ch == "N":
                put(images["note"], r, c)
    for r in range(rows):
        for c in range(cols):
            ch = LAYOUT[r][c]
            ang = FACING.get((r, c), 0)
            if ch in "HGrc":
                put(shadow, r, c, scale=SHADOW_SIZE * 2, dy=SHADOW_DY)
            if ch == "H":
                put(images["hero1"], r, c, ang)
            elif ch == "G":
                put(images["hero2"], r, c, ang)
            elif ch == "r":
                put(images[f"rat_{FRAMES.get((r, c), 0)}"], r, c, ang)
            elif ch == "c":
                put(images[f"cyborg_{FRAMES.get((r, c), 0)}"], r, c, ang)
            elif ch == "X":
                put(images["plank"], r, c)
            elif ch == "K":
                put(images["keg"], r, c)
    return out


def board_mockup(images, tex):
    big = board(images, tex, 96)
    small = board(images, tex, 40)
    W = big.width + 40
    H = big.height + small.height + 60
    out = Image.new("RGBA", (W, H), BG)
    out.alpha_composite(big, (20, 20))
    out.alpha_composite(small, (20, big.height + 40))
    # 2x nearest-neighbour zoom of part of the small board (to inspect pixels)
    crop = small.crop((0, 0, small.width // 2, small.height))
    z = crop.resize((crop.width * 2, crop.height * 2), Image.NEAREST)
    if 40 + small.width + z.width <= W:
        out.alpha_composite(z, (40 + small.width, big.height + 40))
    return out


def downscale_check(sheet):
    """Atlas reduced 4x and 8x (mip-style) over light and dark backgrounds: look for fringes."""
    tiles = []
    for f in (4, 8):
        sm = sheet
        for _ in range({4: 2, 8: 3}[f]):
            sm = sm.reduce(2)
        for bg in ((205, 200, 192, 255), (40, 36, 48, 255)):
            b = Image.new("RGBA", sm.size, bg)
            # straight-alpha 'over' like the engine
            b.alpha_composite(sm)
            tiles.append(b)
    W = tiles[0].width * 2 + 30
    H = tiles[0].height + tiles[2].height + 30
    out = Image.new("RGBA", (W, H), (0, 0, 0, 255))
    out.alpha_composite(tiles[0], (10, 10))
    out.alpha_composite(tiles[1], (20 + tiles[0].width, 10))
    out.alpha_composite(tiles[2], (10, 20 + tiles[0].height))
    out.alpha_composite(tiles[3], (20 + tiles[2].width, 20 + tiles[0].height))
    return out
