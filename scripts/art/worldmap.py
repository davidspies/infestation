"""World-map dressing: level medallions and props (256x256, mild 3/4 view, top-left light)."""

import math

import numpy as np

from artkit import (
    BLACK,
    GOLD,
    GOLD_LIGHT,
    GOLD_SHADE,
    OUTLINE,
    OUTLINE_W,
    STEEL,
    STEEL_SHADE,
    WHITE,
    Canvas,
    curve,
    ellipse,
    erase,
    hexc,
    line,
    mix,
    path_of,
    polygon,
    polyline,
    rounded_rect,
)
from board import IRON, IRON_LIGHT, KEG_LIGHT, KEG_RED, KEG_SHADE, WOOD, WOOD_DARK, WOOD_LIGHT, WOOD_SHADE, nail

CX = 128.0


def cyl_stops(base, shade, light, dark=None):
    dark = mix(shade, OUTLINE, 0.35) if dark is None else dark
    return [(0, shade), (0.16, light), (0.42, base), (0.82, shade), (1, dark)]


def cylinder(L, cx, top, bottom, rx, ry, base, shade, light, lid=None, lid_shade=None, bulge=0.0):
    """Upright cylinder/barrel in 3/4 view: body with an elliptical bottom, plus a lid ellipse."""

    def body(c):
        c.move_to(cx - rx, top)
        c.curve_to(cx - rx - bulge, top + (bottom - top) * 0.33, cx - rx - bulge, top + (bottom - top) * 0.67, cx - rx, bottom)
        c.curve_to(cx - rx, bottom + ry * 1.33, cx + rx, bottom + ry * 1.33, cx + rx, bottom)
        c.curve_to(cx + rx + bulge, top + (bottom - top) * 0.67, cx + rx + bulge, top + (bottom - top) * 0.33, cx + rx, top)
        c.close_path()

    L.gradient_fill(body, cyl_stops(base, shade, light), cx - rx - bulge, 0, cx + rx + bulge, 0)
    if lid is not None:
        L.fill_shaded(path_of(ellipse, cx, top, rx, ry), lid, lid_shade or shade, light, rim=max(4, ry * 0.5), bevel=0.6)
    return body


def band(L, cx, y, rx, ry, h, base, shade, light, bulge=0.0):
    """A hoop/band around a cylinder at height y (follows the front curve)."""

    def p(c):
        c.move_to(cx - rx - bulge, y - h / 2)
        c.curve_to(cx - rx * 0.5, y - h / 2 + ry * 1.3, cx + rx * 0.5, y - h / 2 + ry * 1.3, cx + rx + bulge, y - h / 2)
        c.line_to(cx + rx + bulge, y + h / 2)
        c.curve_to(cx + rx * 0.5, y + h / 2 + ry * 1.3, cx - rx * 0.5, y + h / 2 + ry * 1.3, cx - rx - bulge, y + h / 2)
        c.close_path()

    L.gradient_fill(p, cyl_stops(base, shade, light), cx - rx - bulge, 0, cx + rx + bulge, 0)
    return p


# ---------------------------------------------------------------------------
# Medallions
# ---------------------------------------------------------------------------

NODE_R = 100.0
NODE_Y = 122.0
NODE_THICK = 12.0


def medallion(face, face_shade, face_light, side, rim_detail, center_detail=None):
    cv = Canvas(256)
    L = cv.layer()
    # thickness: the disc's south-facing side
    side_path = path_of(lambda c: (ellipse(c, CX, NODE_Y + NODE_THICK, NODE_R, NODE_R), c.rectangle(CX - NODE_R, NODE_Y, 2 * NODE_R, NODE_THICK)))
    L.gradient_fill(side_path, cyl_stops(side, mix(side, OUTLINE, 0.4), mix(side, WHITE, 0.25)), CX - NODE_R, 0, CX + NODE_R, 0)
    face_path = path_of(ellipse, CX, NODE_Y, NODE_R, NODE_R)
    L.fill_shaded(face_path, face, face_shade, face_light, rim=16, bevel=0.9, bevel_light=face_light, hl=(CX - 30, NODE_Y - 34), hl_r=(46, 36), hl_amt=0.3)
    # inner groove
    L.detail(path_of(ellipse, CX, NODE_Y, NODE_R - 22, NODE_R - 22), alpha=0.45, width=3)
    L.stroke(lambda c: (c.new_sub_path(), c.arc(CX, NODE_Y, NODE_R - 20, math.radians(200), math.radians(290))), face_light, 2, alpha=0.6)
    rim_detail(L)
    if center_detail:
        center_detail(L)
    cv.add(L, outline=OUTLINE_W)
    return cv


def node_locked():
    face, shade, light = hexc("#5f6670"), hexc("#3c4149"), hexc("#8e96a1")

    def rim(L):
        rng = np.random.default_rng(41)
        # chips bitten out of the rim
        for ang, size in [(-40, 15), (160, 12), (232, 10)]:
            a = math.radians(ang)
            x, y = CX + math.cos(a) * NODE_R, NODE_Y + math.sin(a) * NODE_R
            pts = [(x + math.cos(a + k) * size * rng.uniform(0.7, 1.2), y + math.sin(a + k) * size * rng.uniform(0.7, 1.2)) for k in np.linspace(0, 2 * math.pi, 7)[:-1]]
            erase(L.ctx, path_of(polygon, pts))
        # cracks
        L.detail(curve([(CX + 40, NODE_Y - 88), (CX + 30, NODE_Y - 60), (CX + 44, NODE_Y - 40), (CX + 32, NODE_Y - 18)]), alpha=0.55, width=2.4)
        L.detail(curve([(CX - 92, NODE_Y + 30), (CX - 66, NODE_Y + 24), (CX - 54, NODE_Y + 40)]), alpha=0.5, width=2.2)
        # worn rune notches
        for i in range(10):
            a = i / 10 * 2 * math.pi
            r0, r1 = NODE_R - 16, NODE_R - 7
            L.stroke(line(CX + math.cos(a) * r0, NODE_Y + math.sin(a) * r0, CX + math.cos(a) * r1, NODE_Y + math.sin(a) * r1), shade, 3.2, alpha=0.8)

    cv = medallion(face, shade, light, hexc("#454b54"), rim)
    return cv.image()


def node_open():
    face, shade, light = hexc("#d0955a"), hexc("#8a5528"), hexc("#ffe2b0")

    def rim(L):
        for i in range(12):
            a = i / 12 * 2 * math.pi + math.pi / 12
            x, y = CX + math.cos(a) * (NODE_R - 11), NODE_Y + math.sin(a) * (NODE_R - 11)
            L.fill_shaded(path_of(ellipse, x, y, 4.6), hexc("#ffe2a8"), shade, WHITE, rim=3, bevel=0.8)
        inner = path_of(ellipse, CX, NODE_Y, NODE_R - 26, NODE_R - 26)
        L.fill_shaded(inner, mix(face, light, 0.3), face, light, rim=10, bevel=-0.5)
        # warm sheen pooled in the middle: inviting
        L.radial_fill(inner, [(0, light + (0.55,)), (1, light + (0.0,))], CX - 10, NODE_Y - 12, NODE_R - 26)
        L.stroke(lambda c: (c.new_sub_path(), c.arc(CX, NODE_Y, NODE_R - 3, math.radians(195), math.radians(285))), WHITE, 3, alpha=0.55)

    cv = medallion(face, shade, light, hexc("#8a5a30"), rim)
    return cv.image()


LEAF = hexc("#b8892c")


def node_done():
    face, shade, light = GOLD, GOLD_SHADE, GOLD_LIGHT

    def rim(L):
        L.fill_shaded(path_of(ellipse, CX, NODE_Y, NODE_R - 30, NODE_R - 30), mix(GOLD, GOLD_LIGHT, 0.2), GOLD, GOLD_LIGHT, rim=12, bevel=-0.6)
        # laurel: two branches from the bottom curving up each side
        for sx in (-1, 1):
            for i in range(9):
                t = 0.12 + i * 0.095
                a = math.pi / 2 + sx * t * math.pi * 0.95
                r = NODE_R - 15
                x, y = CX + math.cos(a) * r, NODE_Y + math.sin(a) * r
                tang = a + sx * math.pi / 2
                for side in (-1, 1):
                    la = tang + side * 0.55
                    lx, ly = x + math.cos(la) * 7, y + math.sin(la) * 7
                    leaf = path_of(ellipse, lx, ly, 8, 3.8, la)
                    L.fill_shaded(leaf, hexc("#e6b844"), hexc("#a8781e"), GOLD_LIGHT, rim=3, bevel=0.8)
                    L.stroke(leaf, OUTLINE, 1.4, alpha=0.55)
        L.fill_shaded(path_of(ellipse, CX, NODE_Y + NODE_R - 13, 7, 6), hexc("#e6b844"), hexc("#a8781e"), GOLD_LIGHT, rim=3, bevel=0.8)
        # shine streak
        L.stroke(lambda c: (c.new_sub_path(), c.arc(CX, NODE_Y, NODE_R - 40, math.radians(205), math.radians(250))), WHITE, 5, alpha=0.6)

    cv = medallion(face, shade, light, hexc("#b8861e"), rim)
    return cv.image()


# ---------------------------------------------------------------------------
# Props
# ---------------------------------------------------------------------------


def prop_barrel():
    cv = Canvas(256)
    L = cv.layer()
    top, bot, rx, ry = 84, 196, 52, 17
    body = cylinder(L, CX, top, bot, rx, ry, WOOD, WOOD_SHADE, WOOD_LIGHT, bulge=9)
    with L.clip(body):
        for k in range(-3, 4):
            x = CX + k * 15
            L.detail(curve([(x, top + 4), (x + k * 1.5, (top + bot) / 2), (x, bot + 14)]), alpha=0.3, width=1.8)
        for y in (top + 18, (top + bot) / 2 + 4, bot - 10):
            band(L, CX, y, rx, ry, 10, IRON, hexc("#24242c"), IRON_LIGHT, bulge=9)
    L.fill_shaded(path_of(ellipse, CX, top, rx, ry), WOOD_LIGHT, WOOD_SHADE, hexc("#e3b07a"), rim=7, bevel=0.6)
    L.detail(path_of(ellipse, CX, top, rx - 7, ry - 4), alpha=0.5, width=2.0)
    for k in (-1, 0, 1):
        L.detail(line(CX + k * 16 - 3, top - ry + 6, CX + k * 16 + 3, top + ry - 6), alpha=0.25, width=1.4)
    cv.add(L, outline=OUTLINE_W)
    return cv.image()


def box(L, x, y, w, depth, h, top, top_shade, top_light, front, front_shade):
    """Box in 3/4 view: top face (x..x+w, y..y+depth) above a front face of height h."""
    top_path = path_of(rounded_rect, x, y, w, depth, 3)
    front_path = path_of(rounded_rect, x, y + depth, w, h, 3)
    L.gradient_fill(front_path, [(0, front), (1, front_shade)], 0, y + depth, 0, y + depth + h)
    L.fill_shaded(top_path, top, top_shade, top_light, rim=6, bevel=0.7)
    return top_path, front_path


def prop_crate():
    cv = Canvas(256)
    L = cv.layer()
    x, y, w, d, h = 58, 62, 140, 52, 96
    _, front_path = box(L, x, y, w, d, h, WOOD_LIGHT, WOOD, hexc("#e3b07a"), WOOD, WOOD_SHADE)
    # top planks
    for k in (1, 2):
        L.detail(line(x + 4, y + k * d / 3, x + w - 4, y + k * d / 3), alpha=0.4, width=2)
    # front: frame + diagonal brace
    fy = y + d
    with L.clip(front_path):
        for k in (1, 2):
            L.detail(line(x, fy + k * h / 3, x + w, fy + k * h / 3), alpha=0.35, width=2)
    for fr in [(x, fy, 16, h), (x + w - 16, fy, 16, h), (x, fy, w, 14), (x, fy + h - 14, w, 14)]:
        p = path_of(rounded_rect, *fr, 2)
        L.gradient_fill(p, [(0, WOOD_LIGHT), (1, WOOD)], 0, fy, 0, fy + h)
        L.stroke(p, OUTLINE, 2, alpha=0.5)
    bp = path_of(polygon, [(x + 16, fy + h - 14), (x + 16, fy + h - 34), (x + w - 34, fy + 14), (x + w - 16, fy + 14), (x + w - 16, fy + 34), (x + 34, fy + h - 14)])
    L.gradient_fill(bp, [(0, WOOD_LIGHT), (1, WOOD)], 0, fy, 0, fy + h)
    L.stroke(bp, OUTLINE, 2, alpha=0.5)
    for nx, ny in [(x + 8, fy + 7), (x + w - 8, fy + 7), (x + 8, fy + h - 7), (x + w - 8, fy + h - 7)]:
        nail(L, nx, ny, 3.2)
    cv.add(L, outline=OUTLINE_W)
    return cv.image()


def small_keg(L, cx, top, h, rx, ry, fuse=False):
    body = cylinder(L, cx, top, top + h, rx, ry, KEG_RED, KEG_SHADE, KEG_LIGHT, bulge=5)
    with L.clip(body):
        for y in (top + 12, top + h - 8):
            band(L, cx, y, rx, ry, 7, IRON, hexc("#24242c"), IRON_LIGHT, bulge=5)
        yb = top + h * 0.52
        hz = band(L, cx, yb, rx, ry, 10, hexc("#f4d03f"), hexc("#b8941e"), hexc("#fff3a0"), bulge=5)
        with L.clip(hz):
            for k in range(-5, 6):
                x = cx + k * 10
                L.fill(path_of(polygon, [(x, yb - 12), (x + 5, yb - 12), (x - 3, yb + 16), (x - 8, yb + 16)]), hexc("#2a2228"))
    L.fill_shaded(path_of(ellipse, cx, top, rx, ry), hexc("#b0563f"), hexc("#7a3326"), hexc("#d77a60"), rim=5, bevel=0.6)
    L.detail(path_of(ellipse, cx, top, rx - 5, ry - 3), alpha=0.45, width=1.8)
    return body


def prop_kegs():
    cv = Canvas(256)
    L = cv.layer()
    small_keg(L, 92, 130, 62, 34, 11)
    small_keg(L, 164, 130, 62, 34, 11)
    cv.add(L, outline=OUTLINE_W)
    L = cv.layer()
    small_keg(L, 128, 70, 62, 34, 11)
    fuse = [(136, 69), (142, 56), (152, 50), (156, 40)]
    L.stroke(curve(fuse), hexc("#8a6a44"), 6)
    L.stroke(curve(fuse), hexc("#c9a46e"), 2.6)
    cv.add(L, outline=OUTLINE_W, ao=0.45, ao_sigma=5, ao_offset=(2, 5))
    return cv.image()


BOOK_COLORS = [hexc(h) for h in ("#b8433a", "#3b6fe0", "#4f9a4a", "#d9a636", "#7a4fb0", "#2f8a8a", "#c06a2a", "#a03a60")]


def prop_bookshelf():
    rng = np.random.default_rng(51)
    cv = Canvas(256)
    L = cv.layer()
    x, w, top, d, h = 50, 156, 40, 20, 172
    box(L, x, top, w, d, h, WOOD, WOOD_SHADE, WOOD_LIGHT, WOOD_SHADE, WOOD_DARK)
    fy = top + d
    inner = path_of(rounded_rect, x + 10, fy + 8, w - 20, h - 16, 2)
    L.fill(inner, hexc("#3a2414"))
    shelves = 3
    sh = (h - 16) / shelves
    for s in range(shelves):
        y0 = fy + 8 + s * sh
        y1 = y0 + sh - 8
        bx = x + 12
        while bx < x + w - 18:
            bw = rng.uniform(9, 15)
            bh = rng.uniform(0.62, 0.92) * (y1 - y0)
            col = BOOK_COLORS[rng.integers(len(BOOK_COLORS))]
            lean = rng.random() < 0.12
            if bx + bw > x + w - 12:
                break
            p = path_of(rounded_rect, bx, y1 - bh, bw, bh, 1.5)
            L.gradient_fill(p, [(0, mix(col, WHITE, 0.25)), (0.4, col), (1, mix(col, BLACK, 0.3))], bx, 0, bx + bw, 0)
            L.stroke(p, OUTLINE, 1.6, alpha=0.7)
            L.stroke(line(bx + 2, y1 - bh * 0.75, bx + bw - 2, y1 - bh * 0.75), GOLD, 1.6, alpha=0.8)
            bx += bw + (6 if lean else 0.5)
        board_p = path_of(rounded_rect, x + 8, y1, w - 16, 8, 1)
        L.gradient_fill(board_p, [(0, WOOD_LIGHT), (1, WOOD_SHADE)], 0, y1, 0, y1 + 8)
    cv.add(L, outline=OUTLINE_W)
    return cv.image()


def flask(L, x, y, r, neck, col):
    """Round-bottom potion flask; (x, y) = centre of the bulb."""
    L.fill(path_of(rounded_rect, x - neck / 2, y - r - 16, neck, 18, 2), mix(WHITE, col, 0.2), alpha=0.9)
    bulb = path_of(ellipse, x, y, r, r)
    L.fill(bulb, mix(WHITE, col, 0.15), alpha=0.9)
    with L.clip(bulb):
        L.radial_fill(path_of(lambda c: c.rectangle(x - r, y - r * 0.15, 2 * r, 2 * r)), [(0, mix(col, WHITE, 0.4)), (1, col)], x - r * 0.3, y, r * 1.2)
    L.fill(path_of(ellipse, x - r * 0.4, y - r * 0.4, r * 0.22, r * 0.32, -0.6), WHITE, alpha=0.85)
    L.stroke(bulb, OUTLINE, 2.4)
    L.stroke(path_of(rounded_rect, x - neck / 2, y - r - 16, neck, 18, 2), OUTLINE, 2.4)
    L.fill(path_of(rounded_rect, x - neck / 2 - 1, y - r - 22, neck + 2, 8, 2), hexc("#a8784a"))
    L.stroke(path_of(rounded_rect, x - neck / 2 - 1, y - r - 22, neck + 2, 8, 2), OUTLINE, 2.0)


def prop_table():
    cv = Canvas(256)
    L = cv.layer()
    # legs
    for lx in (52, 194):
        p = path_of(rounded_rect, lx, 150, 12, 58, 3)
        L.gradient_fill(p, [(0, WOOD), (1, WOOD_DARK)], lx, 0, lx + 12, 0)
    x, y, w, d, h = 40, 96, 176, 56, 14
    box(L, x, y, w, d, h, WOOD_LIGHT, WOOD, hexc("#e3b07a"), WOOD_SHADE, WOOD_DARK)
    for k in (1, 2, 3):
        L.detail(line(x + k * w / 4, y + 4, x + k * w / 4, y + d - 4), alpha=0.3, width=1.8)
    cv.add(L, outline=OUTLINE_W)
    L = cv.layer()
    flask(L, 82, 108, 15, 8, hexc("#5ad66a"))
    flask(L, 128, 100, 12, 7, hexc("#4aa8ff"))
    # tall bottle
    bp = path_of(lambda c: (rounded_rect(c, 160, 72, 24, 46, 6), rounded_rect(c, 167, 56, 10, 20, 2)))
    L.gradient_fill(bp, [(0, hexc("#6a3a7a")), (0.3, hexc("#b06ac8")), (1, hexc("#4a2458"))], 160, 0, 184, 0)
    L.stroke(bp, OUTLINE, 2.4)
    L.fill(path_of(ellipse, 166, 84, 2.5, 8), WHITE, alpha=0.7)
    L.fill(path_of(rounded_rect, 165, 50, 14, 8, 2), hexc("#a8784a"))
    L.stroke(path_of(rounded_rect, 165, 50, 14, 8, 2), OUTLINE, 2.0)
    # open book
    book = path_of(polygon, [(96, 128), (124, 122), (128, 126), (132, 122), (160, 128), (158, 140), (128, 136), (98, 140)])
    L.fill(book, hexc("#f4ead2"))
    L.stroke(book, OUTLINE, 2.2)
    cv.add(L, outline=3.0, ao=0.4, ao_sigma=3, ao_offset=(2, 3))
    return cv.image()


GREEN = hexc("#5ee05a")
GREEN_DARK = hexc("#2a8a3a")


def prop_cauldron():
    cv = Canvas(256)
    cv.glow(GREEN, CX, 92, 70, strength=0.35)
    L = cv.layer()
    # legs
    for lx in (84, 166):
        L.fill(path_of(polygon, [(lx - 6, 180), (lx + 6, 180), (lx + 2, 214), (lx - 3, 214)]), IRON)
    pot = path_of(lambda c: (c.move_to(56, 112), c.curve_to(48, 210, 208, 210, 200, 112), c.close_path()))
    L.gradient_fill(pot, cyl_stops(hexc("#3a3a46"), hexc("#1e1e26"), hexc("#7a7a8a")), 50, 0, 206, 0)
    # rim + liquid
    L.fill_shaded(path_of(ellipse, CX, 112, 76, 22), hexc("#4a4a58"), hexc("#26262e"), hexc("#8a8a9a"), rim=6, bevel=0.8)
    goo = path_of(ellipse, CX, 113, 64, 15)
    L.radial_fill(goo, [(0, hexc("#c8ff9a")), (0.5, GREEN), (1, GREEN_DARK)], CX - 10, 110, 64, sy=0.3)
    for bx, by, br in [(104, 110, 7), (146, 114, 5), (126, 104, 4), (160, 108, 3.5), (92, 116, 3)]:
        L.fill(path_of(ellipse, bx, by, br, br * 0.8), hexc("#b8ff8a"))
        L.stroke(path_of(ellipse, bx, by, br, br * 0.8), GREEN_DARK, 1.4)
        L.fill(path_of(ellipse, bx - br * 0.3, by - br * 0.3, br * 0.3), WHITE, alpha=0.8)
    cv.add(L, outline=OUTLINE_W)
    # rising bubbles
    L = cv.layer()
    for bx, by, br in [(118, 76, 6), (142, 60, 4.5), (110, 48, 3.5)]:
        L.fill(path_of(ellipse, bx, by, br), hexc("#b8ff8a"), alpha=0.9)
        L.fill(path_of(ellipse, bx - br * 0.3, by - br * 0.3, br * 0.35), WHITE, alpha=0.9)
    cv.add(L, outline=2.5)
    return cv.image()


CYAN = hexc("#3ef0ff")
COPPER = hexc("#c87a3a")


def prop_tesla():
    cv = Canvas(256)
    cv.glow(CYAN, CX, 66, 56, strength=0.45)
    L = cv.layer()
    cylinder(L, CX, 186, 204, 46, 13, hexc("#4c5a6e"), hexc("#2c3442"), hexc("#a9b8cc"), lid=hexc("#6f7f96"), lid_shade=hexc("#4c5a6e"))
    coil = cylinder(L, CX, 86, 186, 22, 7, COPPER, hexc("#7a4418"), hexc("#ffc890"))
    with L.clip(coil):
        for y in np.arange(90, 192, 5.5):
            L.stroke(curve([(CX - 24, y), (CX, y + 6), (CX + 24, y)]), hexc("#5a3010"), 1.6, alpha=0.8)
    # torus on top
    L.fill_shaded(path_of(ellipse, CX, 78, 44, 16), hexc("#d9e2ec"), hexc("#6a788a"), WHITE, rim=8, bevel=0.9)
    L.fill(path_of(ellipse, CX, 76, 18, 6), hexc("#4c5a6e"))
    L.fill_shaded(path_of(ellipse, CX, 58, 16, 16), STEEL, STEEL_SHADE, WHITE, rim=8, bevel=0.9, hl=(CX - 5, 53), hl_r=4, hl_amt=0.9)
    cv.add(L, outline=OUTLINE_W)
    # arcs
    L = cv.layer()
    rng = np.random.default_rng(71)
    for a0 in (-2.6, -0.5, -1.6):
        pts = [(CX + math.cos(a0) * 18, 58 + math.sin(a0) * 18)]
        for k in range(5):
            r = 18 + (k + 1) * 7
            a = a0 + rng.uniform(-0.25, 0.25)
            pts.append((CX + math.cos(a) * r, 58 + math.sin(a) * r * 0.9))
        L.stroke(polyline(pts), CYAN, 5, alpha=0.6)
        L.stroke(polyline(pts), WHITE, 2)
    cv.add(L)
    return cv.image()


BRASS = hexc("#c9a24a")
BRASS_SHADE = hexc("#8a6a24")
BRASS_LIGHT = hexc("#ffe6a0")
TEAL = hexc("#3a9a96")
TEAL_SHADE = hexc("#226a66")
TEAL_LIGHT = hexc("#8ae0d8")


def vpipe(L, x, top, bottom, r, base, shade, light):
    p = path_of(rounded_rect, x - r, top, 2 * r, bottom - top, 2)
    L.gradient_fill(p, cyl_stops(base, shade, light), x - r, 0, x + r, 0)
    L.stroke(p, OUTLINE, 2.4)
    for y in (top + 6, bottom - 6):
        f = path_of(rounded_rect, x - r - 4, y - 4, 2 * r + 8, 8, 3)
        L.gradient_fill(f, cyl_stops(base, shade, light), x - r - 4, 0, x + r + 4, 0)
        L.stroke(f, OUTLINE, 2.0)


def prop_pipes():
    cv = Canvas(256)
    L = cv.layer()
    vpipe(L, 84, 70, 206, 15, BRASS, BRASS_SHADE, BRASS_LIGHT)
    vpipe(L, 128, 100, 210, 18, TEAL, TEAL_SHADE, TEAL_LIGHT)
    vpipe(L, 172, 60, 204, 13, BRASS, BRASS_SHADE, BRASS_LIGHT)
    # elbow connecting the outer pipes over the top
    elbow = path_of(lambda c: (c.new_sub_path(), c.arc(128, 70, 44, math.pi, 2 * math.pi)))
    L.stroke(elbow, OUTLINE, 30)
    L.stroke(elbow, BRASS_SHADE, 24)
    L.stroke(elbow, BRASS, 16)
    L.stroke(lambda c: (c.new_sub_path(), c.arc(128, 70, 48, math.pi * 1.1, math.pi * 1.6)), BRASS_LIGHT, 5)
    # gauge on the teal pipe
    L.fill_shaded(path_of(ellipse, 128, 132, 16, 16), hexc("#f4efe4"), hexc("#b8b0a0"), WHITE, rim=4)
    L.stroke(path_of(ellipse, 128, 132, 16, 16), OUTLINE, 3)
    L.stroke(line(128, 132, 137, 124), hexc("#c0392b"), 2.5)
    # red valve wheel
    vx, vy = 84, 150
    L.stroke(path_of(ellipse, vx, vy, 17, 17), OUTLINE, 10)
    L.stroke(path_of(ellipse, vx, vy, 17, 17), hexc("#d64a3a"), 5)
    for a in (0, math.pi / 2):
        L.stroke(line(vx - 16 * math.cos(a), vy - 16 * math.sin(a), vx + 16 * math.cos(a), vy + 16 * math.sin(a)), hexc("#d64a3a"), 3.5)
    L.fill(path_of(ellipse, vx, vy, 5), hexc("#8a2a20"))
    cv.add(L, outline=OUTLINE_W * 0.8)
    return cv.image()


LEAF_GREEN = hexc("#4f9a3a")
LEAF_SHADE = hexc("#2f6a2a")
LEAF_LIGHT = hexc("#8fd060")


def clumps(L, blobs, base, shade, light):
    for x, y, r in blobs:
        L.fill_shaded(path_of(ellipse, x, y, r, r * 0.92), base, shade, light, rim=r * 0.5, bevel=0.8, bevel_light=light, hl=(x - r * 0.3, y - r * 0.35), hl_r=r * 0.5, hl_amt=0.35)


def prop_tree():
    cv = Canvas(256)
    L = cv.layer()
    L.fill(path_of(polygon, [(118, 150), (138, 150), (142, 206), (114, 206)]), WOOD_SHADE)
    L.gradient_fill(path_of(polygon, [(118, 150), (138, 150), (142, 206), (114, 206)]), [(0, WOOD), (1, WOOD_DARK)], 114, 0, 142, 0)
    cv.add(L, outline=OUTLINE_W)
    L = cv.layer()
    blobs = [(128, 150, 44), (84, 132, 36), (172, 132, 36), (102, 92, 38), (154, 90, 38), (128, 112, 40), (128, 64, 30)]
    clumps(L, blobs, LEAF_GREEN, LEAF_SHADE, LEAF_LIGHT)
    for x, y, r in [(102, 92, 38), (154, 90, 38), (128, 112, 40), (84, 132, 36), (172, 132, 36)]:
        L.detail(lambda c, x=x, y=y, r=r: (c.new_sub_path(), c.arc(x, y, r * 0.7, math.radians(20), math.radians(80))), alpha=0.22, width=2.2)
    cv.add(L, outline=OUTLINE_W, ao=0.4, ao_sigma=6, ao_offset=(3, 6))
    return cv.image()


def prop_bush():
    cv = Canvas(256)
    L = cv.layer()
    blobs = [(98, 150, 30), (158, 150, 30), (128, 158, 32), (112, 122, 30), (146, 120, 30)]
    clumps(L, blobs, hexc("#5aa648"), hexc("#2f6a2a"), hexc("#9ae070"))
    for bx, by in [(104, 126), (150, 140), (126, 152), (164, 118), (92, 146)]:
        L.fill_shaded(path_of(ellipse, bx, by, 5.5), hexc("#e0405a"), hexc("#9a1a32"), hexc("#ff9aa8"), rim=3)
        L.fill(path_of(ellipse, bx - 1.6, by - 1.8, 1.5), WHITE)
    cv.add(L, outline=OUTLINE_W)
    return cv.image()


FIRE = [hexc("#fff4b0"), hexc("#ffd23f"), hexc("#ff8a1e"), hexc("#e0401a")]


def flame(L, x, y, w, h, wob=0.0):
    """Teardrop flame with a hot core; (x, y) is the base centre."""
    def p(c, s=1.0):
        c.move_to(x, y)
        c.curve_to(x - w * s, y - 2, x - w * 0.6 * s, y - h * 0.55 * s, x + wob, y - h * s)
        c.curve_to(x + w * 0.6 * s, y - h * 0.55 * s, x + w * s, y - 2, x, y)
        c.close_path()

    L.fill(p, FIRE[3])
    L.fill(lambda c: p(c, 0.78), FIRE[2])
    L.fill(lambda c: p(c, 0.55), FIRE[1])
    L.fill(lambda c: p(c, 0.32), FIRE[0])
    return p


def prop_brazier():
    cv = Canvas(256)
    cv.glow(hexc("#ff9a3a"), CX, 92, 80, strength=0.5)
    L = cv.layer()
    for (x0, x1) in [(96, 70), (160, 186), (128, 128)]:
        L.stroke(line(x0, 140, x1, 210), OUTLINE, 11)
        L.stroke(line(x0, 140, x1, 210), IRON_LIGHT, 5)
    bowl = path_of(lambda c: (c.move_to(66, 116), c.curve_to(70, 160, 186, 160, 190, 116), c.close_path()))
    L.gradient_fill(bowl, cyl_stops(hexc("#4a4a58"), hexc("#24242c"), hexc("#8a8a9a")), 66, 0, 190, 0)
    L.fill_shaded(path_of(ellipse, CX, 116, 62, 16), hexc("#4a4a58"), hexc("#24242c"), hexc("#8a8a9a"), rim=5, bevel=0.8)
    L.fill(path_of(ellipse, CX, 117, 52, 11), hexc("#ff6a1a"))
    L.radial_fill(path_of(ellipse, CX, 117, 52, 11), [(0, hexc("#ffe08a")), (1, hexc("#c0301a"))], CX, 117, 52, sy=0.25)
    cv.add(L, outline=OUTLINE_W)
    L = cv.layer()
    flame(L, 104, 120, 18, 46, wob=-4)
    flame(L, 152, 120, 17, 44, wob=4)
    flame(L, 128, 122, 24, 74, wob=2)
    cv.add(L, outline=3.0)
    L = cv.layer()
    for ex, ey, er in [(96, 52, 2.5), (160, 40, 2.2), (140, 30, 1.8), (110, 34, 1.6)]:
        L.fill(path_of(ellipse, ex, ey, er), FIRE[1])
    cv.add(L)
    return cv.image()


BANNER_RED = hexc("#c0392b")
BANNER_SHADE = hexc("#8a2219")


def prop_banner():
    cv = Canvas(256)
    L = cv.layer()
    # pole + crossbar
    L.gradient_fill(path_of(rounded_rect, 122, 40, 12, 172, 5), [(0, WOOD_LIGHT), (0.4, WOOD), (1, WOOD_DARK)], 122, 0, 134, 0)
    L.fill_shaded(path_of(ellipse, 128, 38, 9), GOLD, GOLD_SHADE, GOLD_LIGHT, rim=4, bevel=0.8)
    L.fill(path_of(ellipse, 128, 210, 24, 7), WOOD_SHADE)
    cv.add(L, outline=OUTLINE_W)
    L = cv.layer()
    L.gradient_fill(path_of(rounded_rect, 72, 52, 112, 10, 5), [(0, WOOD_LIGHT), (1, WOOD_SHADE)], 0, 52, 0, 62)
    for x in (72, 184):
        L.fill_shaded(path_of(ellipse, x, 57, 7), GOLD, GOLD_SHADE, GOLD_LIGHT, rim=3, bevel=0.8)
    cloth = [(80, 62), (176, 62), (176, 176), (128, 154), (80, 176)]
    cp = path_of(polygon, cloth)
    L.gradient_fill(cp, [(0, BANNER_SHADE), (0.18, BANNER_RED), (0.5, hexc("#e0584a")), (0.8, BANNER_RED), (1, BANNER_SHADE)], 80, 0, 176, 0)
    with L.clip(cp):
        L.stroke(path_of(polygon, cloth), GOLD, 10)
    L.stroke(cp, GOLD_SHADE, 1.5, alpha=0.6)
    # emblem: crossed swords over a shield-ish shape
    shield = path_of(lambda c: (c.move_to(108, 82), c.line_to(148, 82), c.curve_to(148, 112, 140, 124, 128, 132), c.curve_to(116, 124, 108, 112, 108, 82), c.close_path()))
    L.fill_shaded(shield, GOLD, GOLD_SHADE, GOLD_LIGHT, rim=5, bevel=0.8)
    L.stroke(shield, OUTLINE, 2.2, alpha=0.8)
    L.fill(path_of(polygon, [(128, 90), (132, 100), (142, 100), (134, 107), (137, 118), (128, 111), (119, 118), (122, 107), (114, 100), (124, 100)]), BANNER_SHADE)
    for x in (100, 156):
        L.detail(line(x, 70, x, 150), alpha=0.18, width=3)
    cv.add(L, outline=OUTLINE_W)
    return cv.image()


CHEESE = hexc("#f6c84a")
CHEESE_SHADE = hexc("#d29a22")
CHEESE_LIGHT = hexc("#fff0a0")


def prop_cheese():
    cv = Canvas(256)
    L = cv.layer()
    # wedge: top triangle + front face + rind on the right
    top = [(60, 126), (194, 96), (204, 124)]
    front = [(60, 126), (204, 124), (204, 176), (60, 172)]
    L.gradient_fill(path_of(polygon, front), [(0, CHEESE), (1, CHEESE_SHADE)], 0, 126, 0, 176)
    L.fill_shaded(path_of(polygon, top), CHEESE_LIGHT, CHEESE, WHITE, rim=8, bevel=0.6)
    L.fill(path_of(polygon, [(204, 124), (212, 120), (212, 172), (204, 176)]), hexc("#e0a630"))
    L.gradient_fill(path_of(polygon, [(204, 124), (212, 120), (212, 172), (204, 176)]), [(0, hexc("#f0b840")), (1, hexc("#b88218"))], 204, 0, 212, 0)
    L.fill(path_of(polygon, [(194, 96), (204, 124), (212, 120), (202, 94)]), hexc("#f0c050"))
    for hx, hy, r in [(96, 146, 9), (140, 156, 7), (170, 140, 10), (118, 162, 5), (188, 160, 5)]:
        L.fill(path_of(ellipse, hx, hy, r, r * 0.85), CHEESE_SHADE)
        L.fill(path_of(ellipse, hx + 1, hy + 1.5, r * 0.75, r * 0.6), hexc("#b07a14"))
    for hx, hy, r in [(130, 114, 6), (166, 108, 5)]:
        L.fill(path_of(ellipse, hx, hy, r, r * 0.55), CHEESE_SHADE)
    L.detail(line(60, 126, 204, 124), alpha=0.5, width=2.2)
    cv.add(L, outline=OUTLINE_W)
    return cv.image()


BONE = hexc("#efe6d2")
BONE_SHADE = hexc("#bfae8e")


def bone(L, x0, y0, x1, y1, w=10):
    L.stroke(line(x0, y0, x1, y1), BONE_SHADE, w + 3)
    L.stroke(line(x0, y0, x1, y1), BONE, w)
    a = math.atan2(y1 - y0, x1 - x0)
    n = (-math.sin(a), math.cos(a))
    for (x, y) in ((x0, y0), (x1, y1)):
        for s in (-1, 1):
            L.fill_shaded(path_of(ellipse, x + n[0] * s * w * 0.55, y + n[1] * s * w * 0.55, w * 0.62), BONE, BONE_SHADE, WHITE, rim=3, bevel=0.6)


def prop_bones():
    cv = Canvas(256)
    L = cv.layer()
    bone(L, 70, 176, 176, 150)
    bone(L, 90, 140, 190, 186, 9)
    bone(L, 148, 196, 200, 164, 8)
    cv.add(L, outline=OUTLINE_W * 0.8)
    L = cv.layer()
    skull = path_of(lambda c: (ellipse(c, 112, 132, 36, 32), rounded_rect(c, 92, 140, 40, 28, 8)))
    L.fill_shaded(skull, BONE, BONE_SHADE, WHITE, rim=10, bevel=0.7)
    for ex in (98, 124):
        L.fill(path_of(ellipse, ex, 138, 8.5, 9.5), hexc("#3a2a24"))
    L.fill(path_of(polygon, [(111, 148), (106, 158), (116, 158)]), hexc("#3a2a24"))
    for tx in (102, 110, 118):
        L.detail(line(tx, 160, tx, 168), alpha=0.6, width=2)
    cv.add(L, outline=OUTLINE_W * 0.8, ao=0.4, ao_sigma=5, ao_offset=(2, 5))
    return cv.image()


WAX = hexc("#f4ead2")
WAX_SHADE = hexc("#c9b48a")


def candle(L, x, top, h, r):
    body = path_of(rounded_rect, x - r, top, 2 * r, h, 3)
    L.gradient_fill(body, [(0, WAX_SHADE), (0.25, WHITE), (0.5, WAX), (1, WAX_SHADE)], x - r, 0, x + r, 0)
    L.stroke(body, OUTLINE, 2.6)
    L.fill(path_of(ellipse, x, top, r, r * 0.35), WAX)
    L.stroke(path_of(ellipse, x, top, r, r * 0.35), OUTLINE, 2.0, alpha=0.6)
    # drips
    L.fill(path_of(rounded_rect, x - r + 2, top, 5, 14, 2.5), WHITE)
    L.fill(path_of(rounded_rect, x + r - 8, top, 5, 9, 2.5), WHITE)
    L.stroke(line(x, top - 1, x, top - 6), OUTLINE, 2)


def prop_candles():
    cv = Canvas(256)
    spots = [(96, 106, 70, 13), (128, 82, 100, 15), (160, 116, 64, 12), (112, 142, 46, 11), (150, 150, 40, 10)]
    for x, top, h, r in spots:
        cv.glow(hexc("#ffb84a"), x, top - 16, 34, strength=0.35)
    L = cv.layer()
    for x, top, h, r in spots:
        candle(L, x, top, h + (186 - top - h), r)
    cv.add(L, outline=OUTLINE_W * 0.6)
    L = cv.layer()
    for x, top, h, r in spots:
        flame(L, x, top - 6, 7, 22)
    cv.add(L, outline=2.2)
    return cv.image()


RUG = hexc("#a8323a")
RUG_DARK = hexc("#6a1a2a")


def prop_rug():
    cv = Canvas(256)
    L = cv.layer()
    rx, ry = 94, 64
    # fringe
    for k in range(-6, 7):
        for sx in (-1, 1):
            y = CX + k * 8
            L.stroke(line(CX + sx * (rx - 2) * math.sqrt(max(0, 1 - (k * 8 / ry) ** 2)) , y, CX + sx * ((rx - 2) * math.sqrt(max(0, 1 - (k * 8 / ry) ** 2)) + 10), y), hexc("#e8d8b0"), 2.6)
    L.fill(path_of(ellipse, CX, CX, rx, ry), RUG)
    for r_off, col, w in [(8, GOLD, 5), (20, RUG_DARK, 7), (32, GOLD, 3)]:
        L.stroke(path_of(ellipse, CX, CX, rx - r_off, ry - r_off), col, w)
    # central diamond medallion
    L.fill(path_of(polygon, [(CX, CX - 26), (CX + 38, CX), (CX, CX + 26), (CX - 38, CX)]), GOLD)
    L.fill(path_of(polygon, [(CX, CX - 15), (CX + 22, CX), (CX, CX + 15), (CX - 22, CX)]), RUG_DARK)
    L.fill(path_of(ellipse, CX, CX, 5), GOLD)
    for sx in (-1, 1):
        L.fill(path_of(polygon, [(CX + sx * 56, CX - 8), (CX + sx * 64, CX), (CX + sx * 56, CX + 8), (CX + sx * 48, CX)]), GOLD)
    cv.add(L, outline=OUTLINE_W * 0.7)
    return cv.image()


STONE = hexc("#a39d94")
STONE_SHADE = hexc("#6e6962")
STONE_LIGHT = hexc("#d4cec4")


def prop_stairs():
    cv = Canvas(256)
    L = cv.layer()
    x, y, w, h = 52, 44, 152, 168
    frame = path_of(rounded_rect, x, y, w, h, 10)
    L.fill_shaded(frame, STONE, STONE_SHADE, STONE_LIGHT, rim=10, bevel=0.8)
    hole = path_of(rounded_rect, x + 18, y + 18, w - 36, h - 30, 4)
    L.fill(hole, hexc("#120e16"))
    with L.clip(hole):
        steps = 6
        sh = (h - 30) / steps
        for i in range(steps):
            t = i / steps
            col = mix(STONE, hexc("#120e16"), t**0.8)
            sy = y + 18 + i * sh
            inset = i * 3
            L.fill(path_of(rounded_rect, x + 18 + inset, sy, w - 36 - 2 * inset, sh * 0.7, 2), col)
            L.fill(path_of(rounded_rect, x + 18 + inset, sy + sh * 0.7, w - 36 - 2 * inset, sh * 0.3, 1), mix(col, BLACK, 0.45))
        L.gradient_fill(hole, [(0, (0, 0, 0, 0.0)), (1, (0.07, 0.05, 0.09, 0.85))], 0, y + 18, 0, y + h - 12)
    L.stroke(hole, OUTLINE, 3)
    for k in (1, 2):
        L.detail(line(x + 4, y + k * h / 3, x + 16, y + k * h / 3), alpha=0.4, width=2)
        L.detail(line(x + w - 16, y + k * h / 3 + 10, x + w - 4, y + k * h / 3 + 10), alpha=0.4, width=2)
    cv.add(L, outline=OUTLINE_W)
    return cv.image()


def prop_well():
    cv = Canvas(256)
    L = cv.layer()
    # posts behind
    for px in (64, 192):
        L.gradient_fill(path_of(rounded_rect, px - 6, 50, 12, 110, 3), [(0, WOOD_LIGHT), (1, WOOD_DARK)], px - 6, 0, px + 6, 0)
    L.gradient_fill(path_of(rounded_rect, 56, 44, 144, 12, 4), [(0, WOOD_LIGHT), (1, WOOD_SHADE)], 0, 44, 0, 56)
    L.stroke(line(128, 56, 128, 104), hexc("#c9a46e"), 3)
    cv.add(L, outline=OUTLINE_W * 0.8)
    L = cv.layer()
    rx, ry = 76, 26
    top, bot = 120, 186
    wall = cylinder(L, CX, top, bot, rx, ry, STONE, STONE_SHADE, STONE_LIGHT)
    with L.clip(wall):
        for row, y in enumerate((top + 14, top + 34, top + 54)):
            L.detail(curve([(CX - rx, y), (CX, y + ry * 1.2), (CX + rx, y)]), alpha=0.45, width=2.2)
            for k in range(-3, 4):
                x = CX + (k + (0.5 if row % 2 else 0)) * 24
                yy = y + ry * 1.2 * max(0.0, 1 - ((x - CX) / rx) ** 2) ** 0.5
                L.detail(line(x, yy - 1, x, yy + 18), alpha=0.4, width=2)
    L.fill_shaded(path_of(ellipse, CX, top, rx, ry), STONE_LIGHT, STONE, WHITE, rim=8, bevel=0.6)
    water = path_of(ellipse, CX, top + 2, rx - 16, ry - 8)
    L.radial_fill(water, [(0, hexc("#3a6a8a")), (1, hexc("#0e1a2a"))], CX, top + 6, rx - 16, sy=0.35)
    L.stroke(water, OUTLINE, 2.5)
    L.stroke(curve([(CX - 30, top + 2), (CX - 10, top - 2), (CX + 6, top + 1)]), WHITE, 2, alpha=0.6)
    # bucket hanging into the well
    b = path_of(polygon, [(118, 100), (138, 100), (135, 118), (121, 118)])
    L.gradient_fill(b, [(0, WOOD_LIGHT), (1, WOOD_SHADE)], 118, 0, 138, 0)
    L.stroke(b, OUTLINE, 2.4)
    cv.add(L, outline=OUTLINE_W)
    return cv.image()


def prop_flag():
    cv = Canvas(256)
    L = cv.layer()
    L.fill(path_of(ellipse, 112, 206, 30, 9), hexc("#7a6a54"))
    L.gradient_fill(path_of(rounded_rect, 106, 40, 10, 168, 4), [(0, WOOD_LIGHT), (0.4, WOOD), (1, WOOD_DARK)], 106, 0, 116, 0)
    L.fill_shaded(path_of(ellipse, 111, 38, 8), GOLD, GOLD_SHADE, GOLD_LIGHT, rim=4, bevel=0.8)
    cv.add(L, outline=OUTLINE_W)
    L = cv.layer()
    pennant = path_of(lambda c: (c.move_to(116, 48), c.curve_to(150, 40, 170, 62, 204, 70), c.curve_to(170, 80, 150, 98, 116, 100), c.close_path()))
    L.gradient_fill(pennant, [(0, BANNER_SHADE), (0.3, BANNER_RED), (0.6, hexc("#e0584a")), (1, BANNER_RED)], 116, 0, 204, 0)
    L.stroke(curve([(118, 60), (150, 56), (176, 70)]), GOLD, 3, alpha=0.9)
    L.stroke(curve([(118, 88), (150, 86), (176, 76)]), GOLD, 3, alpha=0.9)
    cv.add(L, outline=OUTLINE_W * 0.85)
    return cv.image()


SPRITES = {
    "node_locked": node_locked,
    "node_open": node_open,
    "node_done": node_done,
    "prop_barrel": prop_barrel,
    "prop_crate": prop_crate,
    "prop_kegs": prop_kegs,
    "prop_bookshelf": prop_bookshelf,
    "prop_table": prop_table,
    "prop_cauldron": prop_cauldron,
    "prop_tesla": prop_tesla,
    "prop_pipes": prop_pipes,
    "prop_tree": prop_tree,
    "prop_bush": prop_bush,
    "prop_brazier": prop_brazier,
    "prop_banner": prop_banner,
    "prop_cheese": prop_cheese,
    "prop_bones": prop_bones,
    "prop_candles": prop_candles,
    "prop_rug": prop_rug,
    "prop_stairs": prop_stairs,
    "prop_well": prop_well,
    "prop_flag": prop_flag,
}
