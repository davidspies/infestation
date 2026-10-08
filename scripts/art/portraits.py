"""Dialogue portraits (front-facing busts) and the big title rat."""

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
    cross_ticks,
    curve,
    ellipse,
    erase,
    hexc,
    line,
    mix,
    path_of,
    polygon,
    rounded_rect,
    sample_spline,
    smooth_path,
    taper_path,
)
from creatures import HERO_COLORS

SKIN = hexc("#f2c19b")
SKIN_SHADE = hexc("#d9926e")
SKIN_LIGHT = hexc("#ffe0c6")
BLUSH = hexc("#f08f86")

HAIR = {
    "hero1": (hexc("#6b4428"), hexc("#4a2c18"), hexc("#946240")),
    "hero2": (hexc("#c0582a"), hexc("#8a3a1a"), hexc("#e8834a")),
}
IRIS = {"hero1": hexc("#3e6fd8"), "hero2": hexc("#5a8a3a")}


def portrait(name):
    pal = HERO_COLORS[name]
    hair, hair_shade, hair_light = HAIR[name]
    cv = Canvas(256)
    CX = 128.0

    # --- plume (behind the helmet), sweeping back to the right ------------
    L = cv.layer()
    plume = path_of(
        smooth_path,
        [(118, 52), (134, 28), (164, 18), (196, 26), (214, 50), (204, 46), (186, 40), (162, 44), (142, 58)],
        tension=0.9,
    )
    L.fill_shaded(plume, mix(pal["crest"], pal["light"], 0.3), pal["shade"], WHITE, rim=8, bevel=0.5, hl=(160, 30), hl_r=(26, 10), hl_amt=0.4)
    for k in range(4):
        L.detail(curve([(134 + k * 2, 44 - k * 3), (160 + k * 4, 30 - k * 1.5), (190 + k * 4, 34 + k * 3)]), alpha=0.25, width=1.6)
    cv.add(L, outline=OUTLINE_W)

    # --- shoulders: cloak + breastplate + pauldrons ------------------------
    L = cv.layer()
    cloak = path_of(smooth_path, [(CX, 176), (178, 182), (214, 198), (232, 222), (236, 241), (20, 241), (24, 222), (42, 198), (78, 182)], tension=0.7)
    L.fill_shaded(cloak, pal["cloak"], pal["shade"], pal["light"], rim=16, bevel=0.5, hl=(110, 205), hl_r=(70, 30), hl_amt=0.25)
    plate = path_of(smooth_path, [(CX, 196), (156, 200), (162, 222), (160, 241), (96, 241), (94, 222), (100, 200)], tension=0.8)
    L.fill_shaded(plate, STEEL, STEEL_SHADE, WHITE, rim=12, bevel=0.7, hl=(118, 218), hl_r=(14, 18), hl_amt=0.5)
    L.detail(curve([(CX, 204), (CX, 239)]), alpha=0.3, width=2.2)
    # gold collar trim + clasp
    L.stroke(curve([(82, 190), (104, 198), (CX, 200), (152, 198), (174, 190)]), GOLD, 7)
    L.stroke(curve([(82, 190), (104, 198), (CX, 200), (152, 198), (174, 190)]), GOLD_SHADE, 2, alpha=0.5)
    for sx in (-1, 1):
        px = CX + sx * 82
        dome = path_of(lambda c, px=px: (c.move_to(px - 32, 232), c.curve_to(px - 34, 196, px + 34, 196, px + 32, 232), c.close_path()))
        L.fill_shaded(dome, STEEL, STEEL_DARK, WHITE, rim=14, bevel=0.9, hl=(px - 8, 210), hl_r=(10, 7), hl_amt=0.8)
        L.stroke(curve([(px - 31, 231), (px, 236), (px + 31, 231)]), GOLD, 6)
        L.detail(curve([(px - 26, 222), (px, 214), (px + 26, 222)]), alpha=0.3, width=2)
    cv.add(L, outline=OUTLINE_W)

    # --- neck + head --------------------------------------------------------
    L = cv.layer()
    L.fill_shaded(path_of(rounded_rect, CX - 20, 150, 40, 48, 12), SKIN_SHADE, mix(SKIN_SHADE, OUTLINE, 0.3), SKIN, rim=8)
    face = path_of(smooth_path, [(CX, 66), (172, 82), (182, 122), (174, 160), (150, 184), (CX, 191), (106, 184), (82, 160), (74, 122), (84, 82)], tension=0.9)
    L.fill_shaded(face, SKIN, SKIN_SHADE, SKIN_LIGHT, rim=18, bevel=0.45, bevel_light=SKIN_LIGHT, hl=(116, 120), hl_r=(34, 40), hl_amt=0.35)
    # ears
    for sx in (-1, 1):
        L.fill_shaded(path_of(ellipse, CX + sx * 54, 130, 9, 14), SKIN, SKIN_SHADE, SKIN_LIGHT, rim=5)
    cv.add(L, outline=OUTLINE_W)

    L = cv.layer()
    # fringe of hair under the brim: a row of pointed locks
    tips = [(84, 112), (96, 122), (110, 116), (124, 126), (138, 117), (152, 124), (166, 114), (176, 108)]
    fringe = [(80, 100), (176, 100)] + [(x, y) for x, y in reversed(tips)]
    L.fill_shaded(path_of(polygon, fringe), hair, hair_shade, hair_light, rim=6, bevel=0.5)
    for (x, y) in tips[1:-1]:
        L.detail(curve([(x - 2, 104), (x, y - 4)]), alpha=0.3, width=1.6)
    # cheeks
    for sx in (-1, 1):
        cv_x = CX + sx * 32
        L.radial_fill(path_of(ellipse, cv_x, 156, 14, 9), [(0, BLUSH + (0.55,)), (1, BLUSH + (0.0,))], cv_x, 156, 14)
    if name == "hero2":
        rng = np.random.default_rng(7)
        for sx in (-1, 1):
            for _ in range(5):
                fx_, fy_ = CX + sx * rng.uniform(22, 40), rng.uniform(148, 160)
                L.fill(path_of(ellipse, fx_, fy_, 1.6), hexc("#b5643c"), alpha=0.8)
    # eyes
    for sx in (-1, 1):
        ex, ey = CX + sx * 22, 139
        L.fill(path_of(ellipse, ex, ey, 11.5, 13.5), WHITE)
        L.fill(path_of(ellipse, ex + sx * 0.5, ey + 1.5, 8, 10), IRIS[name])
        L.fill(path_of(ellipse, ex + sx * 0.5, ey + 2, 4.6, 5.8), OUTLINE)
        L.fill(path_of(ellipse, ex - 3, ey - 3, 2.8), WHITE)
        L.fill(path_of(ellipse, ex + 3, ey + 5, 1.3), WHITE, alpha=0.8)
        L.stroke(path_of(ellipse, ex, ey, 11.5, 13.5), OUTLINE, 3.0)
        # determined brows (angled down toward the nose)
        L.stroke(curve([(CX + sx * 36, 117), (CX + sx * 24, 116), (CX + sx * 10, 122)]), mix(hair_shade, OUTLINE, 0.4), 7)
    # nose + mouth
    L.stroke(curve([(CX - 1, 148), (CX - 4, 160), (CX + 3, 163)]), SKIN_SHADE, 3.5)
    mouth = path_of(lambda c: (c.move_to(CX - 16, 171), c.curve_to(CX - 8, 182, CX + 10, 182, CX + 17, 169), c.curve_to(CX + 6, 175, CX - 8, 175, CX - 16, 171), c.close_path()))
    L.fill(mouth, hexc("#7a2f2a"))
    L.stroke(mouth, OUTLINE, 2.6)
    cv.add(L)

    # --- helmet: dome with brim band, cheek guards, ridge --------------------
    L = cv.layer()
    for sx in (-1, 1):
        guard = path_of(smooth_path, [(CX + sx * 50, 96), (CX + sx * 62, 104), (CX + sx * 60, 146), (CX + sx * 48, 156), (CX + sx * 44, 120)], tension=0.8)
        L.fill_shaded(guard, STEEL, STEEL_SHADE, WHITE, rim=8, bevel=0.8)
        L.fill(path_of(ellipse, CX + sx * 53, 140, 2.4), STEEL_DARK)
    dome = path_of(lambda c: (c.move_to(CX - 62, 100), c.curve_to(CX - 64, 38, CX + 64, 38, CX + 62, 100), c.close_path()))
    L.fill_shaded(dome, mix(STEEL, STEEL_SHADE, 0.15), STEEL_DARK, WHITE, rim=20, bevel=0.9, hl=(104, 62), hl_r=(16, 10), hl_amt=0.8)
    brim = path_of(rounded_rect, CX - 66, 90, 132, 16, 8)
    L.fill_shaded(brim, GOLD, GOLD_SHADE, GOLD_LIGHT, rim=5, bevel=0.8)
    for x in range(-48, 49, 24):
        L.fill(path_of(ellipse, CX + x, 98, 2.4), GOLD_SHADE)
    ridge = path_of(lambda c: (c.move_to(CX - 5, 92), c.curve_to(CX - 6, 60, CX - 5, 52, CX, 46), c.curve_to(CX + 5, 52, CX + 6, 60, CX + 5, 92), c.close_path()))
    L.fill_shaded(ridge, STEEL, STEEL_SHADE, WHITE, rim=3, bevel=0.8)
    L.stroke(ridge, OUTLINE, 1.8, alpha=0.5)
    cv.add(L, outline=OUTLINE_W, ao=0.35, ao_sigma=5, ao_offset=(2, 4))
    return cv.image()


# ---------------------------------------------------------------------------
# Title rat (512x512)
# ---------------------------------------------------------------------------

FUR = hexc("#8a7364")
FUR_SHADE = hexc("#5c4a42")
FUR_LIGHT = hexc("#b39c88")
BELLY = hexc("#cdb9a2")
PINK = hexc("#f29bab")
PINK_SHADE = hexc("#c96f80")
PINK_LIGHT = hexc("#ffc8d2")
RED = hexc("#ff3b3b")
TOOTH = hexc("#fff3d6")


def spiky(pts, spikes, rng, amp=14.0):
    """Insert fur spikes along an open polyline (list of (x, y)) on its left side."""
    out = []
    pts = np.asarray(pts, float)
    for i in range(len(pts) - 1):
        a, b = pts[i], pts[i + 1]
        out.append(tuple(a))
        if i in spikes:
            d = b - a
            n = np.array([d[1], -d[0]]) / (np.linalg.norm(d) + 1e-9)
            m = a + d * 0.45
            out.append(tuple(m + n * amp * rng.uniform(0.8, 1.2) + d * 0.25))
            out.append(tuple(a + d * 0.6))
    out.append(tuple(pts[-1]))
    return out


def title_rat():
    rng = np.random.default_rng(21)
    cv = Canvas(512)
    ow = OUTLINE_W * 1.6

    # --- tail: from the rump, curling up the right side --------------------
    L = cv.layer()
    tail = sample_spline([(400, 452), (456, 446), (486, 392), (474, 318), (436, 286), (410, 300), (414, 330)], 40)
    tw = lambda u: 30 - 24 * u**0.7
    L.fill_shaded(path_of(lambda c: taper_path(c, tail, tw)), PINK, PINK_SHADE, PINK_LIGHT, rim=10, bevel=0.6)
    for p, nrm, u in cross_ticks(tail, 10, n=140):
        w = tw(u) / 2 * 0.85
        L.detail(line(*(p + nrm * w), *(p - nrm * w)), alpha=0.25, width=2.4)
    cv.add(L, outline=ow)

    # --- body: hunched back rising behind the head to the right -------------
    L = cv.layer()
    back = [(196, 210), (250, 176), (318, 170), (380, 196), (426, 250), (448, 320), (444, 400), (420, 456)]
    back = spiky(back, {1, 2, 3, 4, 5}, rng, amp=16)
    body_pts = back + [(380, 486), (250, 490), (150, 482), (110, 430), (120, 330), (150, 260)]
    body = path_of(smooth_path, body_pts, tension=0.55)
    L.fill_shaded(body, FUR, FUR_SHADE, FUR_LIGHT, rim=46, bevel=0.55, bevel_light=FUR_LIGHT, hl=(330, 230), hl_r=(70, 40), hl_amt=0.3)
    # hind haunch
    L.fill_shaded(path_of(ellipse, 378, 400, 64, 70, -0.25), FUR, FUR_SHADE, FUR_LIGHT, rim=34, bevel=0.6, bevel_light=FUR_LIGHT)
    L.detail(path_of(lambda c: (c.new_sub_path(), c.arc(378, 400, 64, math.radians(140), math.radians(290)))), alpha=0.35, width=3)
    # chest / belly
    belly = path_of(smooth_path, [(228, 330), (282, 350), (300, 420), (280, 476), (176, 476), (158, 420), (176, 350)], tension=0.9)
    L.fill_shaded(belly, BELLY, mix(BELLY, FUR_SHADE, 0.5), WHITE, rim=24, bevel=0.45)
    for y in (380, 410, 440):
        L.detail(curve([(206, y), (228, y + 8), (250, y)]), alpha=0.2, width=2.2)
    cv.add(L, outline=ow)

    # --- hind foot ---------------------------------------------------------
    L = cv.layer()
    foot = path_of(smooth_path, [(350, 470), (392, 454), (440, 462), (452, 480), (408, 490), (356, 488)], tension=0.8)
    L.fill_shaded(foot, PINK, PINK_SHADE, PINK_LIGHT, rim=8, bevel=0.6)
    for k in range(3):
        cx_ = 446 - k * 2
        cy_ = 466 + k * 8
        claw = [(cx_, cy_ - 3), (cx_, cy_ + 3), (cx_ + 10, cy_ + 1)]
        L.fill(path_of(polygon, claw), TOOTH)
        L.stroke(path_of(polygon, claw), OUTLINE, 2)
    cv.add(L, outline=ow * 0.85, ao=0.35, ao_sigma=6, ao_offset=(3, 6))

    # --- forelegs, bent, claws digging in ------------------------------------
    L = cv.layer()
    for (sx, sy), (ex, ey), (px, py) in [((176, 330), (150, 400), (146, 462)), ((282, 344), (292, 408), (276, 466))]:
        leg = sample_spline([(sx, sy), (ex, ey), (px, py - 10)], 20)
        L.fill_shaded(path_of(lambda c, l=leg: taper_path(c, l, lambda u: 50 - 14 * u)), FUR, FUR_SHADE, FUR_LIGHT, rim=16, bevel=0.6, bevel_light=FUR_LIGHT)
        paw = path_of(smooth_path, [(px - 34, py + 10), (px - 30, py - 10), (px, py - 18), (px + 30, py - 10), (px + 34, py + 10), (px, py + 16)], tension=0.8)
        L.fill_shaded(paw, PINK, PINK_SHADE, PINK_LIGHT, rim=9, bevel=0.6)
        for k in (-1, 0, 1):
            x = px + k * 13
            L.detail(line(x, py - 12, x, py + 6), alpha=0.4, width=2.4)
        for k in (-1.5, -0.5, 0.5, 1.5):
            cx_ = px + k * 14
            claw = [(cx_ - 4.5, py + 12), (cx_ + 4.5, py + 12), (cx_ + 1, py + 26)]
            L.fill(path_of(polygon, claw), TOOTH)
            L.stroke(path_of(polygon, claw), OUTLINE, 2.0)
    cv.add(L, outline=ow * 0.85, ao=0.45, ao_sigma=8, ao_offset=(3, 6))

    # --- ears (behind the head); the near one is notched -------------------
    L = cv.layer()
    for ex, ey, rx, ry, notch in [(118, 150, 78, 74, True), (318, 104, 58, 60, False)]:
        L.fill_shaded(path_of(ellipse, ex, ey, rx, ry), FUR, FUR_SHADE, FUR_LIGHT, rim=18, bevel=0.6, bevel_light=FUR_LIGHT)
        L.fill_shaded(path_of(ellipse, ex + 6, ey + 6, rx * 0.66, ry * 0.62), PINK, PINK_SHADE, PINK_LIGHT, rim=16, bevel=-0.5)
        if notch:
            erase(L.ctx, path_of(polygon, [(ex - 30, ey - ry - 8), (ex - 16, ey - ry + 26), (ex + 2, ey - ry - 8)]))
    cv.add(L, outline=ow)

    # --- head: turned to our left, snout pointing down-left ----------------
    L = cv.layer()
    crown = spiky([(118, 222), (150, 152), (214, 118), (290, 124), (348, 160)], {1, 2, 3}, rng, amp=14)
    head_pts = crown + [(370, 214), (362, 270), (336, 304), (282, 336), (232, 362), (196, 366), (160, 346), (128, 300)]
    head_pts = spiky(head_pts[:-1], {len(head_pts) - 3}, rng, amp=-14) + [head_pts[-1]]
    head = path_of(smooth_path, head_pts, tension=0.6)
    L.fill_shaded(head, FUR, FUR_SHADE, FUR_LIGHT, rim=46, bevel=0.6, bevel_light=FUR_LIGHT, hl=(220, 190), hl_r=(70, 50), hl_amt=0.35)
    # cheek tufts on the far side
    L.fill(path_of(polygon, [(352, 236), (390, 246), (360, 262), (384, 280), (344, 284)]), FUR)
    # muzzle
    muzzle = path_of(smooth_path, [(214, 270), (262, 284), (282, 320), (252, 352), (204, 366), (168, 350), (162, 312), (180, 282)], tension=0.9)
    L.fill_shaded(muzzle, BELLY, mix(BELLY, FUR_SHADE, 0.45), WHITE, rim=22, bevel=0.5)
    cv.add(L, outline=ow, ao=0.4, ao_sigma=10, ao_offset=(4, 8))

    # --- face ---------------------------------------------------------------
    L = cv.layer()
    eyes = [(176, 226, 31, 24, 1.0), (276, 214, 26, 21, 0.88)]  # the far eye is foreshortened
    for ex, ey, rx, ry, s in eyes:
        eye = path_of(smooth_path, [(ex - rx, ey + 4), (ex - rx * 0.3, ey - ry), (ex + rx, ey - ry * 0.4), (ex + rx * 0.5, ey + ry * 0.7), (ex - rx * 0.4, ey + ry * 0.75)], tension=0.8)
        L.fill(eye, hexc("#2a1416"))
        L.radial_fill(eye, [(0, hexc("#fff4ec")), (0.22, hexc("#ffb0a0")), (0.55, RED), (1, hexc("#8a0c0c"))], ex + 2, ey + 2, rx)
        L.fill(path_of(ellipse, ex + 3, ey + 1, 4.5 * s, 14 * s), hexc("#3a0606"))  # slit pupil
        L.fill(path_of(ellipse, ex - 9 * s, ey - 7 * s, 5 * s), WHITE, alpha=0.95)
        L.stroke(eye, OUTLINE, 4.0)
    # heavy V brows: inner ends low (toward the snout), outer ends high
    for pts in ([(128, 170), (196, 186), (226, 208), (212, 214), (186, 198), (130, 186)], [(318, 164), (268, 180), (244, 202), (258, 206), (276, 192), (322, 178)]):
        brow = path_of(polygon, pts)
        L.fill(brow, FUR_SHADE)
        L.stroke(brow, OUTLINE, 3.5)
    # nose (at the tip of the snout)
    nose_pts = [(176, 300), (204, 304), (204, 322), (184, 330), (164, 324), (160, 308)]
    L.fill_shaded(path_of(smooth_path, nose_pts, tension=0.8), PINK, PINK_SHADE, PINK_LIGHT, rim=8, bevel=0.7)
    L.stroke(path_of(smooth_path, nose_pts, tension=0.8), OUTLINE, 3.0)
    # sneer: mouth line rising to the right, with buck teeth and a fang
    L.stroke(line(186, 330, 192, 342), OUTLINE, 4)
    L.stroke(curve([(160, 346), (192, 342), (236, 334), (272, 306)]), OUTLINE, 5)
    for tx, ty, w, h in [(186, 344, 17, 26), (204, 341, 17, 25)]:
        tooth = path_of(rounded_rect, tx - w / 2, ty, w, h, 4)
        L.fill_shaded(tooth, TOOTH, hexc("#d9c49a"), WHITE, rim=5, bevel=0.6)
        L.stroke(tooth, OUTLINE, 3.0)
    fang = [(250, 324), (262, 318), (258, 340)]
    L.fill(path_of(polygon, fang), TOOTH)
    L.stroke(path_of(polygon, fang), OUTLINE, 2.5)
    cv.add(L)

    # --- whiskers -----------------------------------------------------------
    L = cv.layer()
    for k, (dy, ln) in enumerate([(-26, 118), (-4, 132), (18, 114)]):
        x0, y0 = 158, 318 + k * 7
        pts = [(x0, y0), (x0 - ln * 0.5, y0 + dy * 0.6 - 6), (x0 - ln, y0 + dy)]
        L.stroke(curve(pts), OUTLINE, 6, alpha=0.35)
        L.stroke(curve(pts), hexc("#f4eee6"), 2.8, alpha=0.95)
    for k, (dy, ln) in enumerate([(-30, 100), (-10, 112), (10, 98)]):
        x0, y0 = 250, 296 + k * 7
        pts = [(x0, y0), (x0 + ln * 0.5, y0 + dy * 0.6 - 6), (x0 + ln, y0 + dy)]
        L.stroke(curve(pts), OUTLINE, 6, alpha=0.35)
        L.stroke(curve(pts), hexc("#f4eee6"), 2.8, alpha=0.95)
    cv.add(L)

    for ex, ey, *_ in eyes:
        cv.glow(RED, ex, ey, 64, strength=0.55, mode="add")
    return cv.image()


SPRITES = {
    "portrait_hero1": lambda: portrait("hero1"),
    "portrait_hero2": lambda: portrait("hero2"),
    "title_rat": title_rat,
}
