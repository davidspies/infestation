"""Effect particles. Everything is white/grey so the engine can tint it, except
`shadow` (black) and `splinter` (wood colours)."""

import math

import numpy as np

from artkit import (
    WHITE,
    Canvas,
    centered_grid,
    curve,
    ellipse,
    line,
    path_of,
    polygon,
    polyline,
    premul,
    rounded_rect,
    sample_spline,
    smoothstep,
    taper_path,
)
from board import WOOD, WOOD_LIGHT, WOOD_SHADE

GREY = (0.78, 0.78, 0.78)
DARK_GREY = (0.55, 0.55, 0.55)
grid = centered_grid


def set_alpha(cv, rgb, a):
    cv.rgba = premul(rgb, np.clip(a, 0, 1)).astype(np.float32)
    return cv.image()


def glow():
    cv = Canvas(128)
    xs, ys = grid(cv)
    r = np.hypot(xs, ys) / 61.0
    a = np.exp(-(r**2) * 4.0) * smoothstep(1.0, 0.6, r)
    a = (a - a.min()) / a.max()
    return set_alpha(cv, WHITE, a * (r < 1))


def shadow():
    cv = Canvas(128)
    xs, ys = grid(cv)
    r = np.hypot(xs / 60.0, ys / 48.0)
    # soft plateau: dense under the body, feathering smoothly to zero at the rim
    a = 0.55 * smoothstep(1.0, 0.4, r) ** 1.2
    return set_alpha(cv, (0, 0, 0), a)


def puff():
    rng = np.random.default_rng(31)
    cv = Canvas(128)
    xs, ys = grid(cv)
    # a few big soft billows around a core; edge distance via a smooth union
    blobs = [(0.0, 3.0, 30.0)]
    n = 7
    for k in range(n):
        a = k / n * 2 * math.pi + rng.uniform(-0.25, 0.25)
        d = rng.uniform(22, 27)
        blobs.append((math.cos(a) * d, math.sin(a) * d * 0.85 + 3, rng.uniform(15, 21)))
    k_smooth = 6.0
    acc = np.zeros_like(xs)
    for bx, by, br in blobs:
        acc += np.exp(-(np.hypot(xs - bx, ys - by) - br) / k_smooth)
    sdf = -k_smooth * np.log(acc + 1e-12)  # smooth-min of circle SDFs
    a = smoothstep(2.5, -2.5, sdf)
    # internal shading: billows are lighter toward the top-left, darker in the creases
    shade = np.zeros_like(xs)
    for bx, by, br in blobs:
        d = np.hypot(xs - (bx - br * 0.3), ys - (by - br * 0.35)) / br
        shade = np.maximum(shade, smoothstep(1.1, 0.0, d))
    depth = smoothstep(0, 14, -sdf)
    lum = 0.72 + 0.2 * shade + 0.08 * depth
    rgb = np.stack([np.clip(lum, 0, 1)] * 3, -1)
    return set_alpha(cv, rgb, a * 0.96)


def spark():
    cv = Canvas(128)
    xs, ys = grid(cv)
    half_len = 58
    u = np.clip(np.abs(xs) / half_len, 0, 1)
    width = 7.5 * (1 - u**1.6)
    core = smoothstep(width * 0.5 + 0.8, width * 0.5 - 0.8, np.abs(ys)) * (u < 1)
    halo = np.exp(-(ys / (width * 1.3 + 1e-3)) ** 2) * (1 - u) ** 1.5 * 0.6
    a = np.clip(np.maximum(core, halo), 0, 1)
    lum = 0.85 + 0.15 * core
    return set_alpha(cv, np.stack([lum] * 3, -1), a)


def star():
    cv = Canvas(128)
    xs, ys = grid(cv)
    r = np.hypot(xs, ys)
    th = np.arctan2(ys, xs)
    # 4-point star: radius as a function of angle
    k = np.abs(np.cos(2 * th)) ** 6
    rr = 10 + 46 * k
    body = smoothstep(rr + 1.0, rr - 1.0, r)
    halo = np.exp(-((r / 28) ** 2)) * 0.55
    a = np.clip(np.maximum(body, halo) * smoothstep(62, 50, r), 0, 1)
    lum = 0.85 + 0.15 * smoothstep(20, 0, r)
    return set_alpha(cv, np.stack([lum] * 3, -1), a)


def ring():
    cv = Canvas(128)
    xs, ys = grid(cv)
    r = np.hypot(xs, ys)
    R = 0.42 * 128
    a = np.exp(-(((r - R) / 3.2) ** 2))
    a = a * smoothstep(63, 60, r)
    return set_alpha(cv, WHITE, a)


def slash():
    cv = Canvas(256)
    xs, ys = grid(cv)
    r = np.hypot(xs, ys)
    th = np.arctan2(ys, xs)  # -pi/2 is north
    # crescent across the top: angles from 200deg to 340deg (i.e. -160..-20)
    a0, a1 = math.radians(-165), math.radians(-15)
    u = np.clip((th - a0) / (a1 - a0), 0, 1)
    inside_span = (th > a0) & (th < a1)
    R_out = 104.0
    thick = 26 * np.sin(np.pi * u) ** 0.8
    # crisp outer edge, feathered (motion-blurred) inner edge
    outer = smoothstep(R_out + 1.0, R_out - 1.0, r)
    inner = smoothstep(R_out - thick - 14, R_out - thick + 4, r)
    a = outer * inner * inside_span
    a = a * smoothstep(0.0, 0.08, u) * smoothstep(1.0, 0.92, u)
    lum = 0.75 + 0.25 * smoothstep(R_out - thick, R_out - 3, r)
    return set_alpha(cv, np.stack([lum] * 3, -1), np.clip(a, 0, 1))


def shard():
    cv = Canvas(128)
    L = cv.layer()
    pts = [(46, 52), (70, 40), (88, 56), (82, 82), (56, 88), (40, 72)]
    L.fill(path_of(polygon, pts), GREY)
    L.fill(path_of(polygon, [(70, 40), (88, 56), (82, 82), (66, 62)]), DARK_GREY)
    L.fill(path_of(polygon, [(46, 52), (70, 40), (66, 62), (52, 64)]), (0.95, 0.95, 0.95))
    cv.add(L, outline=4.0)
    return cv.image()


def splinter():
    cv = Canvas(128)
    L = cv.layer()
    pts = [(24, 70), (52, 58), (84, 52), (106, 44), (96, 56), (70, 66), (40, 76)]
    L.fill(path_of(polygon, pts), WOOD)
    L.fill(path_of(polygon, [(24, 70), (52, 58), (84, 52), (106, 44), (84, 58), (50, 66)]), WOOD_LIGHT)
    L.stroke(line(40, 70, 90, 54), WOOD_SHADE, 1.5, alpha=0.8)
    cv.add(L, outline=4.0)
    return cv.image()


def strand():
    cv = Canvas(128)
    L = cv.layer()
    pts = [(26, 80), (44, 58), (62, 70), (72, 50), (92, 54), (102, 40)]
    L.stroke(curve(pts), (0.3, 0.3, 0.3), 6, alpha=0.35)
    L.stroke(curve(pts), WHITE, 3.0)
    cv.add(L)
    return cv.image()


def tuft():
    cv = Canvas(128)
    L = cv.layer()
    # a compact lock of fur: three thick locks from a rounded base, curling together
    locks = [((58, 94), (44, 70), (46, 42), 24), ((64, 94), (60, 62), (74, 34), 28), ((70, 94), (82, 72), (98, 58), 20)]
    for a, m, b, w in locks:
        sp = sample_spline([a, m, b], 24)
        L.fill(path_of(lambda c, p=sp, ww=w: taper_path(c, p, lambda u: ww * (1 - u) ** 0.9 + 0.5)), (0.9, 0.9, 0.9))
    L.fill(path_of(ellipse, 66, 92, 16, 9), (0.9, 0.9, 0.9))
    for a, m, b, w in locks:
        sp = sample_spline([(a[0], a[1] - 6), (m[0] + 3, m[1] + 4), (b[0] + 2, b[1] + 12)], 16)
        L.stroke(polyline(sp), (0.62, 0.62, 0.62), 1.8)
    cv.add(L, outline=3.5)
    return cv.image()


def confetti():
    cv = Canvas(32)
    L = cv.layer()
    L.fill(path_of(rounded_rect, 6, 10, 20, 12, 3), WHITE)
    cv.add(L)
    return cv.image()


SPRITES = {
    "glow": glow,
    "shadow": shadow,
    "puff": puff,
    "spark": spark,
    "star": star,
    "ring": ring,
    "slash": slash,
    "shard": shard,
    "splinter": splinter,
    "strand": strand,
    "tuft": tuft,
    "confetti": confetti,
}
