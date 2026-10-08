"""Seamless stone textures (neutral warm greys; the engine multiplies a per-region tint).

- floor.png      1024x1024: 4x4 self-contained 256px flagstone tiles with half-width grout.
- wall_top.png   1024x1024: tileable top surface of thick walls (chunky cut stones).
- wall_front.png 1024x128:  horizontally tileable south face (two courses of blocks).
"""

import math

import numpy as np
from PIL import Image
from scipy import ndimage

WARM = np.array([1.0, 0.972, 0.93])  # warm-grey tint applied to luminance


def to_image(lum, tint=WARM):
    rgb = np.clip(lum[..., None] * tint[None, None, :], 0, 1)
    return Image.fromarray((rgb * 255 + 0.5).astype(np.uint8), "RGB")


def noise(rng, shape, sigma, wrap=True):
    """Band-limited noise, normalised to zero mean / unit std."""
    n = ndimage.gaussian_filter(rng.standard_normal(shape), sigma, mode="wrap" if wrap else "reflect")
    return (n - n.mean()) / (n.std() + 1e-9)


def fbm(rng, shape, sigmas, weights, wrap=True):
    return sum(w * noise(rng, shape, s, wrap) for s, w in zip(sigmas, weights))


def wrap_gradient(a):
    """Central differences on a torus: (d/dy, d/dx)."""
    gy = (np.roll(a, -1, 0) - np.roll(a, 1, 0)) / 2
    gx = (np.roll(a, -1, 1) - np.roll(a, 1, 1)) / 2
    return gy, gx


def smoothstep(e0, e1, x):
    t = np.clip((x - e0) / (e1 - e0), 0, 1)
    return t * t * (3 - 2 * t)


# ---------------------------------------------------------------------------
# Floor
# ---------------------------------------------------------------------------

FLOOR_MEAN = 0.62  # luminance of #9a948c is ~0.585; WARM tint scales channels
GROUT_HALF = 6.0  # px of grout on each tile edge


def rounded_box_sdf(xs, ys, cx, cy, hw, hh, r):
    qx = np.abs(xs - cx) - (hw - r)
    qy = np.abs(ys - cy) - (hh - r)
    outside = np.hypot(np.maximum(qx, 0), np.maximum(qy, 0))
    inside = np.minimum(np.maximum(qx, qy), 0)
    return outside + inside - r


SPLIT_H = (6,)  # variants made of two half slabs (split across y)
SPLIT_V = (12,)
CHIPPED = (3, 9, 14)
CRACKED = (2, 7, 11, 15)
MOSSY = (5, 10, 13)


def floor_tile(rng, variant):
    n = 256
    ys, xs = np.mgrid[0:n, 0:n].astype(np.float64) + 0.5
    wob = noise(rng, (n, n), 9, wrap=False) * 1.3 + noise(rng, (n, n), 3, wrap=False) * 0.35
    r = rng.uniform(8, 16)
    inset = n / 2 - GROUT_HALF - 1.0
    if variant in SPLIT_H:
        a = rounded_box_sdf(xs, ys, n / 2, n / 4, inset, n / 4 - GROUT_HALF - 1.0, r)
        b = rounded_box_sdf(xs, ys, n / 2, 3 * n / 4, inset, n / 4 - GROUT_HALF - 1.0, r)
        sdf = np.minimum(a, b)
        tone_map = np.where(ys < n / 2, rng.uniform(-0.03, 0.03), rng.uniform(-0.03, 0.03))
    elif variant in SPLIT_V:
        a = rounded_box_sdf(xs, ys, n / 4, n / 2, n / 4 - GROUT_HALF - 1.0, inset, r)
        b = rounded_box_sdf(xs, ys, 3 * n / 4, n / 2, n / 4 - GROUT_HALF - 1.0, inset, r)
        sdf = np.minimum(a, b)
        tone_map = np.where(xs < n / 2, rng.uniform(-0.03, 0.03), rng.uniform(-0.03, 0.03))
    else:
        sdf = rounded_box_sdf(xs, ys, n / 2, n / 2, inset, inset, r)
        tone_map = 0.0
    sdf = sdf + wob
    chip_mask = np.zeros((n, n))
    if variant in CHIPPED:
        # angular bite out of one corner: the broken face is rough, darker stone
        corner = [(0, 0), (n, n), (n, 0)][CHIPPED.index(variant)]
        cx, cy = corner
        ux, uy = (1 if cx == 0 else -1), (1 if cy == 0 else -1)
        a1, a2 = rng.uniform(26, 40), rng.uniform(26, 40)
        # half-plane cut through the corner
        cut = ((xs - cx) * ux / a1 + (ys - cy) * uy / a2 - 1) * min(a1, a2) / 1.4
        cut += noise(rng, (n, n), 2, wrap=False) * 1.6
        chip_mask = smoothstep(1.0, -1.0, cut) * smoothstep(1.0, -1.0, sdf)
    inside = smoothstep(0.8, -0.8, sdf)
    depth = np.clip(-sdf, 0, None)

    lum = FLOOR_MEAN + rng.uniform(-0.035, 0.035) + tone_map
    top = lum + fbm(rng, (n, n), [40, 14, 4], [0.018, 0.012, 0.008], wrap=False)
    top += noise(rng, (n, n), 0.7, wrap=False) * 0.012
    # gentle settling gradient across the slab
    ang = rng.uniform(0, 2 * np.pi)
    top += ((xs - n / 2) * np.cos(ang) + (ys - n / 2) * np.sin(ang)) / n * rng.uniform(0.0, 0.035)
    for _ in range(rng.integers(6, 14)):
        px, py = rng.uniform(20, 236, 2)
        rad = rng.uniform(0.8, 2.0)
        top -= 0.05 * np.exp(-((xs - px) ** 2 + (ys - py) ** 2) / (2 * rad**2))
    # bevel: the slab edge rolls off (overhead light, symmetric so tiles can be mirrored)
    bevel = 1 - smoothstep(0, 9, depth)
    top -= bevel**1.6 * 0.07
    top += smoothstep(1.5, 4, depth) * (1 - smoothstep(4, 9, depth)) * 0.012
    if variant in CRACKED:
        top = crack(rng, top, xs, ys)
    # chipped area: lower, rougher, darker
    top = top * (1 - chip_mask) + (FLOOR_MEAN - 0.07 + noise(rng, (n, n), 1.2, wrap=False) * 0.02) * chip_mask
    moss = np.zeros((n, n))
    if variant in MOSSY:
        m = noise(rng, (n, n), 2.5, wrap=False) + noise(rng, (n, n), 12, wrap=False) * 0.8
        rimzone = smoothstep(22, 3, depth) * inside
        moss = smoothstep(1.0, 2.0, m) * rimzone
    grout = FLOOR_MEAN - 0.11 + noise(rng, (n, n), 1.5, wrap=False) * 0.012
    lum_img = grout * (1 - inside) + top * inside
    return lum_img, moss


def crack(rng, top, xs, ys):
    """A jagged, branching hairline crack entering the slab from one edge."""
    n = top.shape[0]
    side = rng.integers(4)
    start = [(rng.uniform(40, 216), 8), (248, rng.uniform(40, 216)), (rng.uniform(40, 216), 248), (8, rng.uniform(40, 216))][side]
    ang = [np.pi / 2, np.pi, -np.pi / 2, 0][side] + rng.uniform(-0.6, 0.6)
    pts = [np.array(start, float)]
    for _ in range(rng.integers(5, 9)):
        ang += rng.uniform(-0.7, 0.7)
        pts.append(pts[-1] + rng.uniform(12, 24) * np.array([np.cos(ang), np.sin(ang)]))
    from PIL import ImageDraw

    im = Image.new("L", (n * 2, n * 2), 0)
    d = ImageDraw.Draw(im)
    for i, (a, b) in enumerate(zip(pts[:-1], pts[1:])):
        w = max(1, int(round(3 - i * 0.3)))
        d.line([tuple(a * 2), tuple(b * 2)], fill=255, width=w)
    # a short branch
    k = len(pts) // 2
    b_ang = ang + rng.choice([-1, 1]) * 0.9
    tip = pts[k] + rng.uniform(14, 22) * np.array([np.cos(b_ang), np.sin(b_ang)])
    d.line([tuple(pts[k] * 2), tuple(tip * 2)], fill=255, width=1)
    mask = np.asarray(im.resize((n, n), Image.LANCZOS), np.float64) / 255
    glint = ndimage.shift(mask, (1.0, 1.0), order=1)
    return top - mask * 0.16 + np.clip(glint - mask, 0, 1) * 0.03


def floor_texture(seed=1):
    rng = np.random.default_rng(seed)
    lum = np.zeros((1024, 1024))
    moss = np.zeros((1024, 1024))
    for v in range(16):
        ty, tx = divmod(v, 4)
        l, m = floor_tile(rng, v)
        lum[ty * 256 : (ty + 1) * 256, tx * 256 : (tx + 1) * 256] = l
        moss[ty * 256 : (ty + 1) * 256, tx * 256 : (tx + 1) * 256] = m
    rgb = np.clip(lum[..., None] * WARM[None, None, :], 0, 1)
    moss_col = np.array([0.50, 0.53, 0.44])
    rgb = rgb * (1 - moss[..., None] * 0.55) + moss_col * moss[..., None] * 0.55
    return Image.fromarray((np.clip(rgb, 0, 1) * 255 + 0.5).astype(np.uint8), "RGB")


# ---------------------------------------------------------------------------
# Wall top: chunky irregular stones (relaxed Voronoi on a torus)
# ---------------------------------------------------------------------------


def torus_voronoi(points, size):
    """Per-pixel nearest site index and distance to the nearest Voronoi edge (wrapping)."""
    ys, xs = np.mgrid[0:size, 0:size].astype(np.float64) + 0.5
    offs = [(dx, dy) for dx in (-size, 0, size) for dy in (-size, 0, size)]
    allp = np.array([(p[0] + dx, p[1] + dy) for dx, dy in offs for p in points])
    ids = np.array([i for _ in offs for i in range(len(points))])
    best = np.full((size, size), np.inf)
    bi = np.zeros((size, size), int)
    bp = np.zeros((size, size, 2))
    for k, p in enumerate(allp):
        d = (xs - p[0]) ** 2 + (ys - p[1]) ** 2
        m = d < best
        bi[m] = ids[k]
        bp[m] = p
        best = np.where(m, d, best)
    # distance to the nearest bisector between the owning site and any other site
    edge = np.full((size, size), np.inf)
    for p in allp:
        lx, ly = p[0] - bp[..., 0], p[1] - bp[..., 1]
        L = np.hypot(lx, ly)
        valid = L > 1e-6
        dist = ((xs - p[0]) ** 2 + (ys - p[1]) ** 2 - best) / (2 * np.where(valid, L, 1))
        edge = np.where(valid, np.minimum(edge, dist), edge)
    return bi, edge


def lloyd(points, size, iters, grid=128):
    """Lloyd relaxation on a torus (evaluated on a coarse grid)."""
    pts = np.array(points, float)
    ys, xs = (np.mgrid[0:grid, 0:grid].astype(np.float64) + 0.5) * size / grid
    for _ in range(iters):
        best = np.full(xs.shape, np.inf)
        idx = np.zeros(xs.shape, int)
        for i, p in enumerate(pts):
            dx = (xs - p[0] + size / 2) % size - size / 2
            dy = (ys - p[1] + size / 2) % size - size / 2
            d = dx * dx + dy * dy
            m = d < best
            idx[m] = i
            best[m] = d[m]
        new = []
        for i, p in enumerate(pts):
            m = idx == i
            dx = ((xs[m] - p[0] + size / 2) % size - size / 2).mean()
            dy = ((ys[m] - p[1] + size / 2) % size - size / 2).mean()
            new.append(((p[0] + dx) % size, (p[1] + dy) % size))
        pts = np.array(new)
    return pts


def wrap_edt(mask, pad=96):
    """Euclidean distance transform on a torus (via wrap padding)."""
    p = np.pad(mask, pad, mode="wrap")
    d = ndimage.distance_transform_edt(p)
    return d[pad:-pad, pad:-pad]


def wall_top_texture(seed=2, g=4, mortar_lum=0.38):
    rng = np.random.default_rng(seed)
    size = 1024
    pts = [((i + 0.5 + rng.uniform(-0.4, 0.4)) * size / g, (j + 0.5 + rng.uniform(-0.4, 0.4)) * size / g) for j in range(g) for i in range(g)]
    pts = lloyd(pts, size, 3)
    sid, edge = torus_voronoi(pts, size)
    wob = noise(rng, (size, size), 10) * 2.5 + noise(rng, (size, size), 3) * 0.8
    gap, rr = 6.0, 16.0
    core = (edge + wob) > gap + rr
    stone_d = wrap_edt(~core) - rr  # signed-ish distance outside the rounded stone
    stone = smoothstep(0.9, -0.9, stone_d)
    depth = wrap_edt(stone_d < 0)
    tone = rng.uniform(-0.05, 0.05, len(pts))
    top = 0.67 + tone[sid]
    top += fbm(rng, (size, size), [50, 16, 5, 1.5, 0.7], [0.028, 0.02, 0.014, 0.010, 0.008])
    pits = smoothstep(2.3, 3.1, noise(rng, (size, size), 1.8))
    top -= pits * 0.07
    # rounded edges: roll-off plus a top-left catch light (static, world-aligned texture)
    R = 24.0
    ds = ndimage.gaussian_filter(np.minimum(depth, R), 2.0, mode="wrap")
    gy, gx = wrap_gradient(ds)
    nrm = np.hypot(gx, gy) + 1e-6
    facing = (gx + gy) / nrm / math.sqrt(2)
    rim = 1 - smoothstep(0, R, depth)
    top -= rim**2.0 * 0.16
    top += np.clip(facing, 0, 1) * rim**1.2 * 0.12
    top -= np.clip(-facing, 0, 1) * rim**1.2 * 0.07
    mortar = mortar_lum + noise(rng, (size, size), 1.5) * 0.025
    mortar -= smoothstep(8, 0, np.clip(stone_d, 0, None)) * 0.05
    lum = mortar * (1 - stone) + top * stone
    return to_image(lum)


# ---------------------------------------------------------------------------
# Wall front: two courses of rectangular blocks
# ---------------------------------------------------------------------------


def wall_front_texture(seed=3):
    rng = np.random.default_rng(seed)
    w, h = 1024, 128
    ys, xs = np.mgrid[0:h, 0:w].astype(np.float64) + 0.5
    lum = np.zeros((h, w))
    mortar_half = 3.5
    stone_mask = np.zeros((h, w))
    depth = np.zeros((h, w))
    shade_id = np.zeros((h, w))
    course_h = h / 2
    for row in range(2):
        y0 = row * course_h
        # random widths summing to w (so it tiles horizontally)
        # random block widths summing exactly to w, none narrower than 96px
        while True:
            widths = []
            while sum(widths) < w - 176:
                widths.append(int(rng.choice([112, 128, 144, 160, 176])))
            rest = w - sum(widths)
            if 96 <= rest <= 192:
                widths.append(rest)
                break
        offset = rng.uniform(0, w)
        x = offset
        for bw in widths:
            cx = (x + bw / 2) % w
            dx = (xs - cx + w / 2) % w - w / 2
            hw, hh = bw / 2, course_h / 2
            dy = ys - (y0 + hh)
            qx, qy = np.abs(dx) - (hw - 6), np.abs(dy) - (hh - 6)
            sdf = np.hypot(np.maximum(qx, 0), np.maximum(qy, 0)) + np.minimum(np.maximum(qx, qy), 0) - 6
            d = -sdf
            m = d > depth
            depth = np.where(m, d, depth)
            shade_id = np.where(m, rng.uniform(-0.05, 0.05), shade_id)
            x += bw
    wob = noise(rng, (h, w), 4) * 1.0
    e = depth + wob
    stone_mask = smoothstep(mortar_half - 0.8, mortar_half + 0.8, e)
    d = np.clip(e - mortar_half, 0, None)
    base = 0.47 + shade_id
    tex = fbm(rng, (h, w), [16, 6, 2, 0.7], [0.02, 0.018, 0.012, 0.01])
    top = base + tex
    rim = 1 - smoothstep(0, 8, d)
    # tiles horizontally only: wrap along x, not y
    gy = np.gradient(ndimage.gaussian_filter(np.minimum(d, 8), 1.0, mode=("nearest", "wrap")), axis=0)
    top -= rim**1.5 * 0.10
    top += np.clip(-gy * 1.2, -0.6, 0.6) * rim * 0.10  # light from above: top edges catch light
    mortar = 0.27 + noise(rng, (h, w), 1.5) * 0.02
    lum = mortar * (1 - stone_mask) + top * stone_mask
    # grime toward the bottom + a few vertical streaks
    grime = smoothstep(60, 128, ys) * 0.10
    streaks = np.clip(noise(rng, (1, w), 10)[0] - 0.5, 0, None) * 0.03
    lum -= grime + streaks[None, :] * smoothstep(40, 128, ys)
    # top lip shadow (where the wall top overhangs)
    lum -= (1 - smoothstep(0, 10, ys)) * 0.06
    return to_image(lum)
