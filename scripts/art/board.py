"""Board objects (256x256, the cell is the central 192px square).

Static objects (plank, keg, note, trigger plate) use a mild 3/4 view with light from
the top-left. Floor-flat things (web, black hole) have no outline.
"""

import math

import numpy as np

from artkit import (
    BLACK,
    OUTLINE,
    OUTLINE_W,
    WHITE,
    Canvas,
    centered_grid,
    curve,
    ellipse,
    hexc,
    line,
    mix,
    over,
    path_of,
    polygon,
    premul,
    rounded_rect,
    smooth_path,
    smoothstep,
)

WOOD = hexc("#a0683a")
WOOD_SHADE = hexc("#7a4a26")
WOOD_LIGHT = hexc("#c98f55")
WOOD_DARK = hexc("#5a3418")
IRON = hexc("#3b3b44")
IRON_LIGHT = hexc("#6a6a78")
KEG_RED = hexc("#c0392b")
KEG_SHADE = hexc("#8a2219")
KEG_LIGHT = hexc("#e8604f")
HAZARD = hexc("#f4d03f")


def rotated_rect(cx, cy, length, width, angle):
    ca, sa = math.cos(angle), math.sin(angle)
    hl, hw = length / 2, width / 2
    return [(cx + ca * x - sa * y, cy + sa * x + ca * y) for x, y in [(-hl, -hw), (hl, -hw), (hl, hw), (-hl, hw)]]


# ---------------------------------------------------------------------------
# Plank barricade
# ---------------------------------------------------------------------------


def board_plank(cv, cx, cy, length, width, angle, thick, rng, nails=True):
    """One thick wooden board in 3/4 view: south-facing side + grained top face."""
    top = rotated_rect(cx, cy, length, width, angle)
    # jag the two ends a little (sawn / broken)
    ca, sa = math.cos(angle), math.sin(angle)
    ends = []
    for (a, b) in [(top[1], top[2]), (top[3], top[0])]:
        mid = ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)
        out = 1 if (a is top[1]) else -1
        j = rng.uniform(-4, 4)
        ends.append((mid[0] + out * ca * j, mid[1] + out * sa * j))
    face = [top[0], top[1], ends[0], top[2], top[3], ends[1]]
    side = [(x, y + thick) for x, y in face]
    L = cv.layer()
    # side = union of the face swept downward
    def prism(c):
        polygon(c, side)
        for i in range(len(face)):
            a, b = face[i], face[(i + 1) % len(face)]
            polygon(c, [a, b, (b[0], b[1] + thick), (a[0], a[1] + thick)])

    L.fill(prism, WOOD_SHADE)
    L.gradient_fill(prism, [(0, WOOD_SHADE), (1, WOOD_DARK)], 0, cy - 20, 0, cy + 60)
    facep = path_of(polygon, face)
    L.fill_shaded(facep, WOOD, WOOD_SHADE, WOOD_LIGHT, rim=7, bevel=0.55, bevel_light=WOOD_LIGHT, hl=(cx - 10, cy - 10), hl_r=(70, 40), hl_amt=0.25)
    with L.clip(facep):
        # grain: long wavy lines along the board
        for k in range(4):
            off = (k - 1.5) * width / 4.5 + rng.uniform(-2, 2)
            pts = []
            for t in np.linspace(-0.5, 0.5, 7):
                wob = math.sin(t * 9 + k * 1.7) * 1.6
                pts.append((cx + ca * t * length - sa * (off + wob), cy + sa * t * length + ca * (off + wob)))
            L.detail(curve(pts), alpha=0.28, width=1.6)
        # a knot
        kt = rng.uniform(-0.25, 0.25)
        kx, ky = cx + ca * kt * length + sa * 4, cy + sa * kt * length - ca * 4
        L.fill(path_of(ellipse, kx, ky, 6, 3.5, angle), WOOD_SHADE)
        L.detail(path_of(ellipse, kx, ky, 6, 3.5, angle), alpha=0.4, width=1.4)
    if nails:
        for t in (-0.4, 0.4):
            nx, ny = cx + ca * t * length, cy + sa * t * length
            nail(L, nx, ny)
    cv.add(L, outline=OUTLINE_W, ao=0.5, ao_sigma=5, ao_offset=(3, 5))
    return L


def nail(L, x, y, r=4.2):
    L.fill(path_of(ellipse, x + 0.8, y + 1.2, r + 1.2), OUTLINE, alpha=0.5)
    L.radial_fill(path_of(ellipse, x, y, r), [(0, hexc("#c8ccd6")), (0.5, IRON_LIGHT), (1, IRON)], x - 1.3, y - 1.3, r * 1.3)
    L.fill(path_of(ellipse, x - 1.2, y - 1.3, 1.2), WHITE, alpha=0.8)


def plank():
    rng = np.random.default_rng(11)
    cv = Canvas(256)
    board_plank(cv, 128, 118, 182, 40, math.radians(37), 13, rng)
    board_plank(cv, 128, 118, 182, 40, math.radians(-37), 13, rng)
    L = cv.layer()
    nail(L, 128, 118, 5.2)
    cv.add(L)
    return cv.image()


# ---------------------------------------------------------------------------
# Spider web
# ---------------------------------------------------------------------------

WEB = hexc("#e8eef5")


def web():
    rng = np.random.default_rng(12)
    cv = Canvas(256)
    cx, cy = 128 + 3, 126
    n = 9
    angles = np.sort((np.arange(n) / n * 2 * math.pi + rng.uniform(-0.15, 0.15, n) + 0.2) % (2 * math.pi))
    anchors = []
    for a in angles:
        # anchor on the cell boundary square (inset a little), pushed toward corners
        d = np.array([math.cos(a), math.sin(a)])
        t = 96 / np.abs(d).max()
        p = np.array([cx, cy]) + d * t * rng.uniform(0.92, 1.02)
        p = np.clip(p, 30, 226)
        anchors.append(p)
    rings = [14, 24, 35, 47, 60, 74, 89]

    def strands(L, col, width, alpha):
        for p in anchors:
            # spokes sag slightly
            mid = (np.array([cx, cy]) + p) / 2 + np.array([0, 2.5])
            L.stroke(curve([(cx, cy), tuple(mid), tuple(p)]), col, width, alpha)
        for r in rings:
            for i in range(n):
                a0, a1 = angles[i], angles[(i + 1) % n] + (2 * math.pi if i == n - 1 else 0)

                def at(a, rr):
                    d = np.array([math.cos(a), math.sin(a)])
                    lim = 96 / np.abs(d).max()
                    return np.array([cx, cy]) + d * min(rr, lim * 0.97)

                p0, p1 = at(a0, r), at(a1, r)
                # sag toward the hub
                m = (p0 + p1) / 2
                hub = np.array([cx, cy])
                sag = (hub - m) / (np.linalg.norm(hub - m) + 1e-6) * (2 + r * 0.06)
                L.stroke(curve([tuple(p0), tuple(m + sag), tuple(p1)]), col, width, alpha)

    U = cv.layer()
    strands(U, OUTLINE, 4.2, 0.3)
    cv.add(U)
    L = cv.layer()
    strands(L, WEB, 2.0, 0.78)
    # dew drops
    for _ in range(7):
        a = rng.uniform(0, 2 * math.pi)
        r = rng.choice(rings)
        x, y = cx + math.cos(a) * r, cy + math.sin(a) * r
        L.fill(path_of(ellipse, x, y, 2.0), WHITE, alpha=0.9)
    L.fill(path_of(ellipse, cx, cy, 3.5), WEB, alpha=0.85)
    cv.add(L)
    return cv.image()


# ---------------------------------------------------------------------------
# Powder keg
# ---------------------------------------------------------------------------


def keg():
    cv = Canvas(256)
    cx = 128
    top_y, bot_y = 92, 206
    rx = 54
    ry = 18
    bulge = 8

    def body(c):
        c.move_to(cx - rx, top_y)
        c.curve_to(cx - rx - bulge, top_y + 40, cx - rx - bulge, bot_y - 40, cx - rx + 2, bot_y)
        c.curve_to(cx - rx + 10, bot_y + ry, cx + rx - 10, bot_y + ry, cx + rx - 2, bot_y)
        c.curve_to(cx + rx + bulge, bot_y - 40, cx + rx + bulge, top_y + 40, cx + rx, top_y)
        c.close_path()

    L = cv.layer()
    # cylinder shading: light from the left, dark right edge
    L.gradient_fill(body, [(0, KEG_SHADE), (0.12, KEG_RED), (0.3, KEG_LIGHT), (0.45, KEG_RED), (0.85, KEG_SHADE), (1, mix(KEG_SHADE, OUTLINE, 0.4))], cx - rx - bulge, 0, cx + rx + bulge, 0)
    with L.clip(body):
        # staves
        for k in range(-3, 4):
            x = cx + k * 16
            L.detail(curve([(x, top_y + 5), (x + k * 1.2, (top_y + bot_y) / 2), (x, bot_y + 14)]), alpha=0.3, width=1.8)
        # hazard band
        band_y0, band_y1 = 132, 160

        def band(c):
            c.rectangle(cx - rx - 20, band_y0, 2 * rx + 40, band_y1 - band_y0)

        L.fill(band, HAZARD)
        with L.clip(band):
            for k in range(-8, 9):
                x = cx + k * 18
                L.fill(path_of(polygon, [(x, band_y0), (x + 9, band_y0), (x + 9 - 14, band_y1), (x - 14, band_y1)]), hexc("#2a2228"))
            L.gradient_fill(band, [(0, (0, 0, 0, 0.35)), (0.3, (1, 1, 1, 0.15)), (0.5, (0, 0, 0, 0.0)), (1, (0, 0, 0, 0.45))], cx - rx - bulge, 0, cx + rx + bulge, 0)
        L.detail(line(cx - rx - 10, band_y0, cx + rx + 10, band_y0), alpha=0.6, width=2.0)
        L.detail(line(cx - rx - 10, band_y1, cx + rx + 10, band_y1), alpha=0.6, width=2.0)
        # iron hoops
        sag = 6
        for y in (top_y + 16, bot_y - 10):

            def hoop(c, y=y):
                c.move_to(cx - rx - bulge, y - 5)
                c.curve_to(cx - 30, y + sag - 5, cx + 30, y + sag - 5, cx + rx + bulge, y - 5)
                c.line_to(cx + rx + bulge, y + 5)
                c.curve_to(cx + 30, y + sag + 5, cx - 30, y + sag + 5, cx - rx - bulge, y + 5)
                c.close_path()

            L.gradient_fill(hoop, [(0, IRON), (0.28, IRON_LIGHT), (0.5, IRON), (1, hexc("#24242c"))], cx - rx, 0, cx + rx, 0)
            for x in (cx - 30, cx + 30):
                L.fill(path_of(ellipse, x, y + sag - 1, 1.8), hexc("#9a9aaa"))
    # lid
    lid = path_of(ellipse, cx, top_y, rx, ry)
    L.fill_shaded(lid, hexc("#b0563f"), hexc("#7a3326"), hexc("#d77a60"), rim=8, bevel=0.6)
    L.detail(path_of(ellipse, cx, top_y, rx - 7, ry - 4), alpha=0.5, width=2.2)
    for k in (-1, 0, 1):
        L.detail(line(cx + k * 15 - 3, top_y - ry + 6, cx + k * 15 + 3, top_y + ry - 6), alpha=0.25, width=1.5)
    # skull emblem on the hazard band
    sx, sy = cx - 4, 146
    L.fill(path_of(ellipse, sx, sy, 15, 15), hexc("#2a2228"))
    L.fill(path_of(ellipse, sx, sy - 2, 9.5, 8.5), hexc("#f6f0e4"))
    L.fill(path_of(rounded_rect, sx - 5.5, sy + 3, 11, 7, 2), hexc("#f6f0e4"))
    L.fill(path_of(ellipse, sx - 3.6, sy - 2, 2.6, 2.8), hexc("#2a2228"))
    L.fill(path_of(ellipse, sx + 3.6, sy - 2, 2.6, 2.8), hexc("#2a2228"))
    L.fill(path_of(polygon, [(sx, sy + 1), (sx - 1.5, sy + 3.8), (sx + 1.5, sy + 3.8)]), hexc("#2a2228"))
    cv.add(L, outline=OUTLINE_W)

    # fuse: rope from the bung, curling up, with a spark at the tip
    L = cv.layer()
    fuse = [(cx + 14, top_y - 2), (cx + 20, top_y - 22), (cx + 34, top_y - 30), (cx + 40, top_y - 46)]
    L.stroke(curve(fuse), hexc("#8a6a44"), 7)
    L.stroke(curve(fuse), hexc("#c9a46e"), 3.2)
    L.fill(path_of(ellipse, cx + 14, top_y - 1, 7, 3.5), hexc("#3a2a24"))
    cv.add(L, outline=4.5)
    tipx, tipy = fuse[-1]
    cv.glow(hexc("#ffcf5a"), tipx, tipy, 18, strength=0.9)
    L = cv.layer()
    star4(L, tipx, tipy, 9, 2.4, hexc("#fff6c8"))
    cv.add(L)
    return cv.image()


def star4(L, x, y, r, w, col, alpha=1.0):
    pts = []
    for k in range(8):
        a = k * math.pi / 4 - math.pi / 2
        rr = r if k % 2 == 0 else w
        pts.append((x + math.cos(a) * rr, y + math.sin(a) * rr))
    L.fill(path_of(smooth_path, pts, tension=0.35), col, alpha)


# ---------------------------------------------------------------------------
# Black hole
# ---------------------------------------------------------------------------

VOID = hexc("#120a1e")
ARM0 = hexc("#5b2a86")
ARM1 = hexc("#c94dff")
RIM = hexc("#ff8ad8")


HOLE_R = 77.0  # ~0.8 cell diameter


def polar(cv):
    """Per-(supersampled)-pixel radius and angle from the canvas centre, in sprite px."""
    xs, ys = centered_grid(cv)
    return np.hypot(xs, ys), np.arctan2(ys, xs)


def hole_base():
    cv = Canvas(256)
    r, _ = polar(cv)
    R = HOLE_R
    # interior falls from violet at the rim into the void
    t = np.clip(r / R, 0, 1)
    inner = np.array(VOID) + (np.array(ARM0) - np.array(VOID)) * (t**3.2)[..., None]
    disc = premul(inner, smoothstep(R + 1.0, R - 1.0, r))
    # bright thin rim just inside the edge, over the disc
    rim = premul(RIM, np.exp(-(((r - (R - 3)) / 3.2) ** 2)) * 0.95)
    # soft magenta halo outside the edge, under the disc
    halo = premul(ARM1, np.exp(-(((r - R) / 11.0) ** 2)) * smoothstep(R + 40, R, r) * (r > R) * 0.55)
    cv.rgba = over(over(rim, disc), halo).astype(np.float32)
    return cv.image()


def hole_swirl():
    cv = Canvas(256)
    r, th = polar(cv)
    R = HOLE_R
    arms = 3
    k = 2.4  # spiral tightness
    phase = th * arms + k * arms * np.log(np.maximum(r, 1) / R) * 1.0
    s = 0.5 + 0.5 * np.cos(phase)
    s = s**3
    env = smoothstep(8, 34, r) * smoothstep(R + 6, R - 10, r)
    a = np.clip(s * env * 0.9, 0, 1)
    col_t = np.clip((r - 10) / (R - 10), 0, 1)[..., None]
    rgb = np.array(ARM0) + (np.array(RIM) - np.array(ARM0)) * col_t
    rgb = rgb + (np.array(ARM1) - rgb) * (np.exp(-((r - R * 0.55) / 18) ** 2))[..., None] * 0.6
    cv.rgba = premul(rgb, a).astype(np.float32)
    return cv.image()


# ---------------------------------------------------------------------------
# Trigger plate (tintable: white / light grey only)
# ---------------------------------------------------------------------------

PALE = (0.93, 0.93, 0.93)
PALE_SHADE = (0.62, 0.62, 0.62)
PALE_DARK = (0.45, 0.45, 0.45)


# Simple angular rune glyphs as polylines in a unit box (x right, y down).
RUNES = [
    [[(0.3, 0), (0.3, 1)], [(0.3, 0.15), (0.8, 0)], [(0.3, 0.45), (0.8, 0.3)]],  # fehu
    [[(0.2, 1), (0.2, 0), (0.8, 0.3), (0.8, 1)]],  # uruz
    [[(0.3, 0), (0.3, 1)], [(0.3, 0.25), (0.75, 0.5), (0.3, 0.75)]],  # thurisaz
    [[(0.3, 0), (0.3, 1)], [(0.3, 0.1), (0.8, 0.35)], [(0.3, 0.4), (0.8, 0.65)]],  # ansuz
    [[(0.25, 1), (0.25, 0), (0.75, 0.25), (0.25, 0.5), (0.75, 1)]],  # raido
    [[(0.75, 0.1), (0.25, 0.5), (0.75, 0.9)]],  # kaunan
    [[(0.15, 0.1), (0.85, 0.9)], [(0.85, 0.1), (0.15, 0.9)]],  # gebo
    [[(0.5, 1), (0.5, 0)], [(0.1, 0.1), (0.5, 0.5), (0.9, 0.1)]],  # algiz
]


def rune(L, idx, cx, cy, size, angle, col, light):
    ca, sa = math.cos(angle), math.sin(angle)

    def tf(p):
        x, y = (p[0] - 0.5) * size * 0.7, (p[1] - 0.5) * size
        return (cx + ca * x - sa * y, cy + sa * x + ca * y)

    for dx, dy, cl, w in ((0.9, 1.1, light, 3.4), (0, 0, col, 3.0)):
        for stroke in RUNES[idx]:
            pts = [tf(p) for p in stroke]
            L.stroke(lambda c, pts=pts, dx=dx, dy=dy: (c.move_to(pts[0][0] + dx, pts[0][1] + dy), [c.line_to(x + dx, y + dy) for x, y in pts[1:]]), cl, w)


def trigger_plate():
    cv = Canvas(256)
    cx, cy, R = 128, 128, 75
    L = cv.layer()
    # plate thickness (3/4 view): darker rim showing below
    L.fill(path_of(ellipse, cx, cy + 5, R, R), PALE_DARK)
    L.fill_shaded(path_of(ellipse, cx, cy, R, R), PALE, PALE_SHADE, WHITE, rim=12, bevel=0.8)
    # recessed inner disc (lit from the top-left, so its upper-left wall is in shadow)
    inner = path_of(ellipse, cx, cy, R - 24, R - 24)
    L.fill_shaded(inner, (0.84, 0.84, 0.84), (0.72, 0.72, 0.72), WHITE, rim=8, bevel=-0.7)
    L.detail(inner, alpha=0.5, width=2.5)
    L.detail(path_of(ellipse, cx, cy, R - 4, R - 4), alpha=0.25, width=1.5)
    for i in range(8):
        a = i / 8 * 2 * math.pi - math.pi / 2
        rx, ry = cx + math.cos(a) * (R - 12.5), cy + math.sin(a) * (R - 12.5)
        rune(L, i, rx, ry, 13, a + math.pi / 2, PALE_DARK, WHITE)
        # dots between runes
        b = a + math.pi / 8
        L.fill(path_of(ellipse, cx + math.cos(b) * (R - 12.5), cy + math.sin(b) * (R - 12.5), 2.0), PALE_DARK)
    cv.add(L, outline=OUTLINE_W * 0.8)
    return cv.image()


# ---------------------------------------------------------------------------
# Note (parchment scroll)
# ---------------------------------------------------------------------------

PARCH = hexc("#f1dfb5")
PARCH_SHADE = hexc("#c9a86b")
PARCH_LIGHT = hexc("#fff4d8")
RIBBON = hexc("#c0392b")


def note():
    cv = Canvas(256)
    cx, cy = 128, 140
    ang = math.radians(-10)
    L = cv.layer()
    c = L.ctx
    c.save()
    c.translate(cx, cy)
    c.rotate(ang)
    w, h = 84, 66
    sheet = lambda c: (c.move_to(-w / 2, -h / 2), c.curve_to(-w / 6, -h / 2 + 5, w / 6, -h / 2 - 5, w / 2, -h / 2), c.line_to(w / 2, h / 2), c.curve_to(w / 6, h / 2 + 5, -w / 6, h / 2 - 5, -w / 2, h / 2), c.close_path())
    L.fill_shaded(sheet, PARCH, PARCH_SHADE, PARCH_LIGHT, rim=10, bevel=0.4)
    for k in range(4):
        y = -h / 2 + 16 + k * 11
        x1 = w / 2 - 14 - (20 if k == 3 else 0)
        L.stroke(curve([(-w / 2 + 12, y), (-w / 2 + 30, y - 1.5), (0, y + 1.0), (x1, y - 0.5)]), hexc("#6b5236"), 2.2, alpha=0.75)
    # rolls at both ends
    for y in (-h / 2, h / 2):
        roll = path_of(rounded_rect, -w / 2 - 6, y - 8, w + 12, 16, 8)
        L.gradient_fill(roll, [(0, PARCH_LIGHT), (0.45, PARCH), (1, PARCH_SHADE)], 0, y - 8, 0, y + 8)
        L.stroke(roll, OUTLINE, 2.0, alpha=0.5)
        L.fill(path_of(ellipse, w / 2 + 6 - 3, y, 3, 6.5), PARCH_SHADE)
        L.detail(path_of(ellipse, w / 2 + 6 - 3, y, 3, 6.5), alpha=0.5, width=1.5)
    # ribbon + wax seal
    L.fill(path_of(polygon, [(10, h / 2 + 4), (18, h / 2 + 26), (12, h / 2 + 22), (6, h / 2 + 28), (4, h / 2 + 4)]), RIBBON)
    L.fill(path_of(polygon, [(-2, h / 2 + 4), (-8, h / 2 + 24), (-12, h / 2 + 19), (-17, h / 2 + 23), (-10, h / 2 + 4)]), mix(RIBBON, BLACK, 0.15))
    L.fill_shaded(path_of(ellipse, 0, h / 2 + 2, 11, 10), RIBBON, mix(RIBBON, BLACK, 0.35), hexc("#ff8a7a"), rim=5, bevel=0.6)
    c.restore()
    cv.add(L, outline=OUTLINE_W * 0.85)
    return cv.image()


SPRITES = {
    "plank": plank,
    "web": web,
    "keg": keg,
    "hole_base": hole_base,
    "hole_swirl": hole_swirl,
    "trigger_plate": trigger_plate,
    "note": note,
}
