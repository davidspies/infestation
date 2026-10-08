"""Heroes, rats and cyborg rats: top-down, facing north, overhead (rotation-invariant) lighting."""

import math

import numpy as np

from artkit import (
    GOLD,
    GOLD_LIGHT,
    GOLD_SHADE,
    OUTLINE,
    OUTLINE_W,
    STEEL,
    STEEL_DARK,
    STEEL_SHADE,
    WHITE,
    Canvas,
    blur,
    cross_ticks,
    curve,
    ellipse,
    hexc,
    line,
    mirror_x,
    mix,
    path_of,
    polygon,
    rounded_rect,
    sample_spline,
    polyline,
    smooth_path,
    taper_path,
)

CX = 128.0

HERO_COLORS = {
    "hero1": dict(cloak=hexc("#3b6fe0"), shade=hexc("#2a4fa8"), light=hexc("#7fa6ff"), crest=hexc("#4a82ff")),
    "hero2": dict(cloak=hexc("#f08a24"), shade=hexc("#c0601a"), light=hexc("#ffc06a"), crest=hexc("#ff9a30")),
}

LEATHER = hexc("#7a4f36")
LEATHER_SHADE = hexc("#553423")
LEATHER_LIGHT = hexc("#a87552")


# Key hero geometry (256 canvas; the cell is [32, 224]^2).
HELM_Y = 154.0
HELM_R = 26.0
HAND_FRONT = (CX, 101.0)
HAND_BACK = (CX, 113.0)
GUARD_Y = 88.0
SWORD_TIP_Y = 30.0
PAULDRON_DX = 46.0
PAULDRON_Y = 156.0


def sword(L, pal):
    tip = SWORD_TIP_Y
    base = GUARD_Y - 2
    hw = 10.0
    blade_pts = [(CX - hw, base), (CX - hw, tip + 28), (CX - 6.5, tip + 13), (CX, tip), (CX + 6.5, tip + 13), (CX + hw, tip + 28), (CX + hw, base)]
    blade = path_of(polygon, blade_pts)
    # Overhead light: the central ridge catches the light, bevels fall off to the edges.
    L.gradient_fill(blade, [(0, STEEL_SHADE), (0.36, STEEL), (0.5, WHITE), (0.64, STEEL), (1, STEEL_SHADE)], CX - hw, 0, CX + hw, 0)
    L.stroke(line(CX, base - 2, CX, tip + 12), STEEL_SHADE, 1.6, alpha=0.7)
    # grip + pommel
    L.fill_shaded(path_of(rounded_rect, CX - 5, GUARD_Y, 10, 30, 4), LEATHER, LEATHER_SHADE, LEATHER_LIGHT, rim=4)
    L.fill_shaded(path_of(ellipse, CX, GUARD_Y + 33, 7), GOLD, GOLD_SHADE, GOLD_LIGHT, rim=5, hl=(CX, GUARD_Y + 33), hl_r=4, hl_amt=0.8)
    # crossguard with flared ends
    guard_pts = [(CX - 25, GUARD_Y - 4), (CX, GUARD_Y - 6), (CX + 25, GUARD_Y - 4), (CX + 26, GUARD_Y + 5), (CX, GUARD_Y + 7), (CX - 26, GUARD_Y + 5)]
    guard = path_of(smooth_path, guard_pts, tension=0.6)
    L.fill_shaded(guard, GOLD, GOLD_SHADE, GOLD_LIGHT, rim=5, hl=(CX, GUARD_Y), hl_r=(22, 4), hl_amt=0.8)
    L.fill_shaded(path_of(ellipse, CX, GUARD_Y + 0.5, 5.5), pal["crest"], pal["shade"], WHITE, rim=4, hl=(CX - 1, GUARD_Y - 1), hl_r=2.5, hl_amt=1.0)


def hero(name):
    pal = HERO_COLORS[name]
    cv = Canvas(256)

    # --- cloak: narrow under the shoulders, flaring behind (south) ---------
    hem_r = [(186, 196), (178, 210), (160, 205), (145, 214), (CX, 209)]
    right = [(CX, 134), (150, 135), (165, 142), (176, 158), (183, 177)] + hem_r
    cloak = path_of(smooth_path, mirror_x(right, CX))
    L = cv.layer()
    L.fill_shaded(cloak, pal["cloak"], pal["shade"], pal["light"], rim=18, hl=(CX, 172), hl_r=(44, 34), hl_amt=0.35)
    # gold trim along the hem (left side, centre, right side), clipped to the cloak
    hem = [(66, 188)] + [(2 * CX - x, y) for x, y in hem_r[:-1]] + [hem_r[-1]] + hem_r[:-1][::-1] + [(190, 188)]
    with L.clip(cloak):
        L.stroke(curve(hem), GOLD, 9)
        L.stroke(curve([(x, y - 5) for x, y in hem]), GOLD_SHADE, 2, alpha=0.55)
    for pts in ([(152, 176), (158, 190), (162, 202)], [(104, 176), (98, 190), (94, 202)], [(CX, 190), (CX, 205)]):
        L.detail(curve(pts), alpha=0.3)
    cv.add(L, outline=OUTLINE_W)

    # --- arms (sleeves) -------------------------------------
    L = cv.layer()
    arms = [
        [(CX + 42, 150), (CX + 41, 124), (CX + 13, HAND_FRONT[1] + 2)],
        [(CX - 42, 150), (CX - 41, 128), (CX - 13, HAND_BACK[1] + 1)],
    ]
    for pts in arms:
        sp = sample_spline(pts, 30)
        pth = path_of(lambda c, p=sp: taper_path(c, p, lambda u: 19 - 4 * u))
        L.fill_shaded(pth, pal["cloak"], pal["shade"], pal["light"], rim=7, hl=pts[1], hl_r=10, hl_amt=0.45)
    cv.add(L, outline=OUTLINE_W, ao=0.35, ao_sigma=4)

    # --- pauldrons --------------------------------------------------------
    L = cv.layer()
    for sx in (1, -1):
        px = CX + sx * PAULDRON_DX
        pth = path_of(ellipse, px, PAULDRON_Y, 15.5, 18.5)
        L.fill_shaded(pth, GOLD, GOLD_SHADE, GOLD_LIGHT, rim=4)
        pth = path_of(ellipse, px, PAULDRON_Y, 12, 15)
        L.fill_shaded(pth, STEEL, STEEL_SHADE, WHITE, rim=8, hl=(px, PAULDRON_Y - 2), hl_r=(5, 8), hl_amt=0.6)
        # overlapping lames
        for dy in (-5, 5):
            L.detail(curve([(px - 10, PAULDRON_Y + dy + 3), (px, PAULDRON_Y + dy - 2), (px + 10, PAULDRON_Y + dy + 3)]), alpha=0.3, width=2.0)
    cv.add(L, outline=OUTLINE_W, ao=0.45, ao_sigma=5)

    # --- sword ------------------------------------------------------------
    L = cv.layer()
    sword(L, pal)
    cv.add(L, outline=OUTLINE_W, ao=0.4, ao_sigma=4)

    # --- gloved fists on the grip ----------------------------------------
    L = cv.layer()
    for hx, hy in [HAND_BACK, HAND_FRONT]:
        pth = path_of(rounded_rect, hx - 11, hy - 7.5, 22, 15, 7)
        L.fill_shaded(pth, LEATHER, LEATHER_SHADE, LEATHER_LIGHT, rim=5, hl=(hx, hy), hl_r=7, hl_amt=0.6)
        for k in (-4, 0, 4):
            L.detail(line(hx + k, hy - 6, hx + k, hy - 2), alpha=0.35, width=1.8)
    cv.add(L, outline=OUTLINE_W * 0.8, ao=0.4, ao_sigma=3)

    # --- horsehair tassel: springs from the back of the helmet, flows south --
    L = cv.layer()
    y0 = HELM_Y + HELM_R - 12
    length = 44.0

    def tassel_w(u):
        return 10 + 12 * math.sin(min(u / 0.7, 1.0) * math.pi / 2) - 20 * max(0.0, u - 0.7) / 0.3 * 0.95

    sp = sample_spline([(CX, y0), (CX, y0 + length * 0.5), (CX, y0 + length)], 30)
    tassel = path_of(lambda c: taper_path(c, sp, tassel_w))
    plume_col = mix(pal["crest"], pal["light"], 0.75)
    L.fill_shaded(tassel, plume_col, pal["crest"], WHITE, rim=6, hl=(CX, y0 + 14), hl_r=(5, 14), hl_amt=0.45)
    for k in range(-2, 3):
        x = CX + k * 3.5
        L.detail(curve([(x * 0.6 + CX * 0.4, y0 + 6), (x, y0 + length * 0.5), (x + k * 1.6, y0 + length * 0.88)]), alpha=0.25, width=1.5)
    cv.add(L, outline=OUTLINE_W * 0.75, ao=0.4, ao_sigma=4)

    # --- helmet (drawn over the tassel root) --------------------------------
    L = cv.layer()
    helm = path_of(ellipse, CX, HELM_Y, HELM_R, HELM_R)
    nose = path_of(rounded_rect, CX - 5, HELM_Y - HELM_R - 6, 10, 16, 4)
    L.fill_shaded(nose, STEEL, STEEL_SHADE, WHITE, rim=4)
    helm_steel = mix(STEEL, STEEL_SHADE, 0.2)
    L.fill_shaded(helm, helm_steel, STEEL_DARK, WHITE, rim=17, hl=(CX, HELM_Y), hl_r=19, hl_amt=0.55)
    # raised ridge front-to-back
    ridge = path_of(rounded_rect, CX - 4, HELM_Y - HELM_R + 3, 8, 2 * HELM_R - 6, 4)
    L.fill_shaded(ridge, STEEL, STEEL_SHADE, WHITE, rim=3, hl=(CX, HELM_Y), hl_r=(2.5, 16), hl_amt=1.0)
    L.stroke(ridge, OUTLINE, 1.8, alpha=0.5)
    for ang in (205, 240, 300, 335):
        a = math.radians(ang)
        L.fill(path_of(ellipse, CX + math.cos(a) * (HELM_R - 6), HELM_Y + math.sin(a) * (HELM_R - 6), 2.0), STEEL_SHADE)
    cv.add(L, outline=OUTLINE_W, ao=0.5, ao_sigma=6)
    return cv.image()


# ---------------------------------------------------------------------------
# Rats
# ---------------------------------------------------------------------------

FUR = hexc("#8a7364")
FUR_SHADE = hexc("#65524a")
FUR_LIGHT = hexc("#a8927f")
PINK = hexc("#f29bab")
PINK_SHADE = hexc("#d07585")
PINK_LIGHT = hexc("#ffc4cf")
WHISKER = hexc("#f4eee6")
EYE = hexc("#120d18")

# Tail cycle: swing left -> centre -> right -> centre, with a travelling S-wave.
TAIL_SWING = [-1.0, 0.0, 1.0, 0.0]
TAIL_PHASE = [0.0, 0.5 * math.pi, math.pi, 1.5 * math.pi]
BREATH = [1.0, 1.012, 1.024, 1.012]


def rat_body_points(breath=1.0):
    """Right half of the rat silhouette from nose tip to rump (centre line x=CX)."""
    pts = [
        (CX, 46),
        (134, 52),
        (141, 63),
        (148, 76),
        (152, 89),
        (154, 99),
        (160, 111),
        (166, 126),
        (168, 143),
        (165, 161),
        (156, 176),
        (143, 185),
        (CX, 188),
    ]
    return [(CX + (x - CX) * (breath if y > 96 else 1.0), y) for x, y in pts]


def tail_points(frame, length=70.0, base=(CX, 177.0), n=60):
    s = TAIL_SWING[frame]
    ph = TAIL_PHASE[frame]
    ds = length / (n - 1)
    x, y = base
    pts = [(x, y)]
    for i in range(1, n):
        u = i / (n - 1)
        # angle from straight south; positive = toward +x
        th = s * 0.5 * u + 0.5 * math.sin(2 * math.pi * 0.85 * u - ph) * (0.45 + 0.7 * u)
        x += math.sin(th) * ds
        y += math.cos(th) * ds
        pts.append((x, y))
    return np.array(pts)


RAT_EAR = (24.0, 93.0, 12.0)  # dx, y, r
RAT_EYE = (13.5, 70.0)
FRONT_FOOT = (29.0, 106.0)
BACK_FOOT = (35.0, 166.0)


def rat_feet(L, color, shade, light, toes=True):
    for sx in (1, -1):
        for (dx, fy), ang, r in [(FRONT_FOOT, -0.5, (8, 5.5)), (BACK_FOOT, 0.55, (9.5, 6))]:
            fx = CX + sx * dx
            a = sx * ang
            pth = path_of(ellipse, fx + sx * 4, fy, r[0], r[1], a)
            L.fill_shaded(pth, color, shade, light, rim=4)
            if toes:
                for k in (-1, 0, 1):
                    tx = fx + sx * (4 + r[0] * 0.8) * math.cos(a) - k * 3.2 * math.sin(a)
                    ty = fy + sx * (r[0] * 0.8) * math.sin(a) + k * 3.2 * math.cos(a)
                    L.fill(path_of(ellipse, tx, ty, 2.2), mix(color, light, 0.4))


def fur_body(L, breath):
    body = path_of(smooth_path, mirror_x(rat_body_points(breath), CX))
    L.fill_shaded(body, FUR, FUR_SHADE, FUR_LIGHT, rim=22, hl=(CX, 134), hl_r=(20, 42), hl_amt=0.55)
    # lighter muzzle / head dome
    L.fill(path_of(ellipse, CX, 70, 10, 15), FUR_LIGHT, alpha=0.4)
    # fur chevrons along the spine
    for y in (116, 134, 152):
        L.detail(curve([(CX - 9, y + 5), (CX, y), (CX + 9, y + 5)]), alpha=0.22, width=2.2)


def rat_ears(L, col, shade, light, inner, inner_shade, inner_light, which=(1, -1)):
    dx, ey, r = RAT_EAR
    for sx in which:
        ex = CX + sx * dx
        a = sx * 0.5
        ear = path_of(ellipse, ex, ey, r * 1.1, r * 0.9, a)
        L.fill_shaded(ear, col, shade, light, rim=5)
        L.fill_shaded(path_of(ellipse, ex + sx * 2.5, ey - 1, r * 0.66, r * 0.5, a), inner, inner_shade, inner_light, rim=3.5, hl=(ex + sx * 2.5, ey - 1), hl_r=4, hl_amt=0.4)
        # crease where the ear meets the head
        a0 = math.pi * (0.62 if sx > 0 else -0.38)
        L.detail(lambda c, x=ex, a0=a0: (c.new_sub_path(), c.arc(x, ey, r * 0.98, a0, a0 + math.pi * 0.76)), alpha=0.35, width=2.2)


def tail_layer(cv, frame, draw):
    L = cv.layer()
    pts = tail_points(frame)
    draw(L, pts)
    cv.add(L, outline=OUTLINE_W * 0.85)
    return pts


def flesh_tail(L, pts):
    pth = path_of(lambda c: taper_path(c, pts, lambda u: 11 - 8.5 * u**0.9))
    L.fill_shaded(pth, PINK, PINK_SHADE, PINK_LIGHT, rim=4, hl=tuple(pts[8]), hl_r=10, hl_amt=0.4)
    # ring segments across the tail
    for p, nrm, u in cross_ticks(pts, 6):
        w = (11 - 8.5 * u**0.9) / 2 * 0.8
        L.detail(line(*(p + nrm * w), *(p - nrm * w)), alpha=0.22, width=1.4)


def whiskers(L, color=WHISKER, alpha=0.9):
    for sx in (1, -1):
        for (ex, ey), (mx, my) in [((36, 40), (20, 46)), ((40, 54), (22, 53)), ((36, 68), (20, 61))]:
            pts = [(CX + sx * 6, 54), (CX + sx * mx, my), (CX + sx * ex, ey)]
            L.stroke(curve(pts), OUTLINE, 3.2, alpha=0.25)
            L.stroke(curve(pts), color, 1.5, alpha=alpha)


def rat(frame):
    frame = int(frame)
    cv = Canvas(256)
    tail_layer(cv, frame, flesh_tail)

    L = cv.layer()
    rat_feet(L, PINK, PINK_SHADE, PINK_LIGHT)
    cv.add(L, outline=OUTLINE_W * 0.8)

    # body + head + ears share one silhouette outline
    L = cv.layer()
    fur_body(L, BREATH[frame])
    rat_ears(L, FUR, FUR_SHADE, FUR_LIGHT, PINK, PINK_SHADE, PINK_LIGHT)
    cv.add(L, outline=OUTLINE_W, ao=0.4, ao_sigma=5)

    # eyes + nose
    L = cv.layer()
    exo, eyo = RAT_EYE
    for sx in (1, -1):
        ex = CX + sx * exo
        L.fill(path_of(ellipse, ex, eyo, 5.6, 6.8, sx * 0.35), EYE)
        L.fill(path_of(ellipse, ex - sx * 1.2, eyo - 2.2, 1.9), WHITE, alpha=0.95)
        L.fill(path_of(ellipse, ex + sx * 1.6, eyo + 2.4, 1.1), hexc("#ff6a5a"), alpha=0.9)
    L.fill_shaded(path_of(ellipse, CX, 47.5, 6.5, 5.5), PINK, PINK_SHADE, PINK_LIGHT, rim=3, hl=(CX, 52), hl_r=3, hl_amt=0.8)
    cv.add(L, outline=2.0)

    L = cv.layer()
    whiskers(L)
    cv.add(L)
    return cv.image()


# ---------------------------------------------------------------------------
# Cyborg rat
# ---------------------------------------------------------------------------

PLATE = hexc("#6f7f96")
PLATE_SHADE = hexc("#4c5a6e")
PLATE_LIGHT = hexc("#a9b8cc")
CYAN = hexc("#3ef0ff")
RED = hexc("#ff3b3b")
DARK_METAL = hexc("#3a4352")
CYBER_EYE = (-13.5, 70.0)  # the red eye is on the rat's left (-x)


def cable_tail(L, pts):
    w = lambda u: 12 - 7 * u**0.9
    pth = path_of(lambda c: taper_path(c, pts, w))
    L.fill_shaded(pth, DARK_METAL, mix(DARK_METAL, OUTLINE, 0.5), PLATE_LIGHT, rim=4)
    for p, nrm, u in cross_ticks(pts, 7):
        if u > 0.88:
            break
        half = w(u) / 2 + 0.6
        a0, b0 = p + nrm * half, p - nrm * half
        dd = np.array([nrm[1], -nrm[0]])
        seg = path_of(polygon, [a0 - dd * 2.4, a0 + dd * 2.4, b0 + dd * 2.4, b0 - dd * 2.4])
        L.fill_shaded(seg, PLATE_LIGHT, PLATE, hexc("#e2ebf5"), rim=2.5)
        L.stroke(seg, OUTLINE, 1.2, alpha=0.5)


def circuit(L, pts, width=2.0):
    L.stroke(polyline(pts), CYAN, width)
    L.fill(path_of(ellipse, *pts[-1], width * 1.3), CYAN)


def cyborg(frame):
    frame = int(frame)
    cv = Canvas(256)
    tail_pts = tail_layer(cv, frame, cable_tail)
    tip = tail_pts[-1]
    cv.glow(CYAN, tip[0], tip[1], 9, strength=0.9)

    L = cv.layer()
    rat_feet(L, PLATE, PLATE_SHADE, PLATE_LIGHT)
    cv.add(L, outline=OUTLINE_W * 0.8)

    # body: armoured back plates over a furry silhouette
    L = cv.layer()
    body = path_of(smooth_path, mirror_x(rat_body_points(BREATH[frame]), CX))
    L.fill_shaded(body, FUR, FUR_SHADE, FUR_LIGHT, rim=22)
    with L.clip(body):
        back = path_of(ellipse, CX, 140, 37, 52)
        L.fill_shaded(back, PLATE, PLATE_SHADE, PLATE_LIGHT, rim=16, hl=(CX, 136), hl_r=(16, 34), hl_amt=0.6)
    for y0 in (128, 152):  # seams between the three back plates
        L.detail(curve([(CX - 34, y0 + 8), (CX, y0 - 2), (CX + 34, y0 + 8)]), alpha=0.55, width=2.4)
    L.detail(line(CX, 92, CX, 186), alpha=0.35, width=2.0)
    for (x, y) in [(CX - 24, 118), (CX + 24, 118), (CX - 27, 145), (CX + 27, 145), (CX - 22, 170), (CX + 22, 170)]:
        L.fill(path_of(ellipse, x, y, 2.4), PLATE_LIGHT)
        L.fill(path_of(ellipse, x + 0.6, y + 0.6, 1.4), PLATE_SHADE)
    # left half of the head is machine: metal ear + eye plate; right half organic
    rat_ears(L, FUR, FUR_SHADE, FUR_LIGHT, PINK, PINK_SHADE, PINK_LIGHT, which=(1,))
    rat_ears(L, PLATE, PLATE_SHADE, PLATE_LIGHT, DARK_METAL, OUTLINE, PLATE, which=(-1,))
    plate = path_of(smooth_path, [(CX - 3, 58), (CX - 1, 76), (CX - 3, 96), (CX - 14, 100), (CX - 22, 90), (CX - 22, 72), (CX - 15, 60)], tension=0.7)
    L.fill_shaded(plate, PLATE, PLATE_SHADE, PLATE_LIGHT, rim=6, hl=(CX - 11, 78), hl_r=8, hl_amt=0.6)
    L.stroke(plate, OUTLINE, 2.4, alpha=0.75)
    for (x, y) in [(CX - 6, 92), (CX - 17, 93), (CX - 6, 64)]:
        L.fill(path_of(ellipse, x, y, 1.9), PLATE_LIGHT)
        L.fill(path_of(ellipse, x + 0.5, y + 0.5, 1.1), PLATE_SHADE)
    cv.add(L, outline=OUTLINE_W, ao=0.4, ao_sigma=5)

    # circuit traces (glowing)
    G = cv.layer()
    traces = [
        [(CX + 8, 112), (CX + 8, 124), (CX + 20, 132), (CX + 20, 140)],
        [(CX - 8, 150), (CX - 8, 160), (CX - 20, 166)],
        [(CX + 6, 176), (CX + 14, 170), (CX + 24, 170)],
        [(CX - 6, 86), (CX - 6, 96)],
    ]
    for t in traces:
        circuit(G, t)
    halo = blur(G.rgba(), 3.0, cv.ss) * 1.4
    cv.add(halo, mode="add")
    cv.add(G)

    # organic eye, nose, antenna (sprouting from the metal ear)
    L = cv.layer()
    ox, oy = CX - CYBER_EYE[0], CYBER_EYE[1]
    L.fill(path_of(ellipse, ox, oy, 5.6, 6.8, 0.35), EYE)
    L.fill(path_of(ellipse, ox - 1.2, oy - 2.2, 1.9), WHITE, alpha=0.95)
    L.fill_shaded(path_of(ellipse, CX, 47.5, 6.5, 5.5), PINK, PINK_SHADE, PINK_LIGHT, rim=3, hl=(CX, 47), hl_r=3, hl_amt=0.8)
    dx, ey, _ = RAT_EAR
    ant = [(CX - dx - 3, ey - 2), (CX - dx - 12, ey - 18), (CX - dx - 14, ey - 36)]
    L.stroke(curve(ant), OUTLINE, 5.0)
    L.stroke(curve(ant), PLATE_LIGHT, 2.2)
    L.fill(path_of(ellipse, *ant[0], 4.0), DARK_METAL)
    cv.add(L, outline=2.0)

    # red cyber-eye with glow + antenna LED
    rx, ry = CX + CYBER_EYE[0], CYBER_EYE[1]
    cv.glow(RED, rx, ry, 24, strength=0.8, mode="add")
    L = cv.layer()
    L.fill(path_of(ellipse, rx, ry, 8.5), DARK_METAL)
    L.radial_fill(path_of(ellipse, rx, ry, 6.4), [(0, hexc("#fff2f0")), (0.3, hexc("#ff8a7a")), (0.72, RED), (1, hexc("#b01818"))], rx, ry, 6.4)
    L.fill(path_of(ellipse, rx - 1.8, ry - 2.0, 1.6), WHITE)
    tipx, tipy = ant[-1]
    L.radial_fill(path_of(ellipse, tipx, tipy, 4.2), [(0, WHITE), (0.5, CYAN), (1, hexc("#0aa8c0"))], tipx, tipy, 4.2)
    cv.add(L, outline=2.5)
    cv.glow(CYAN, tipx, tipy, 10, strength=0.6, mode="add")

    L = cv.layer()
    whiskers(L, color=hexc("#cfe6ef"), alpha=0.85)
    cv.add(L)
    return cv.image()


SPRITES = {
    "hero1": lambda: hero("hero1"),
    "hero2": lambda: hero("hero2"),
    **{f"rat_{i}": (lambda i=i: rat(i)) for i in range(4)},
    **{f"cyborg_{i}": (lambda i=i: cyborg(i)) for i in range(4)},
}
