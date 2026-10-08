"""Core drawing toolkit shared by all art modules.

Sprites are drawn with pycairo onto supersampled layers. A `Canvas` keeps a
premultiplied float RGBA buffer; each `Layer` is a cairo surface whose user
space is in *final sprite pixels* (the supersampling scale is applied for you).
When a layer is added to the canvas it can receive a chunky silhouette outline
(computed from the layer's alpha with a distance transform, so it hugs the union
of everything drawn on the layer) and a soft, rotation-invariant contact shadow
on whatever is underneath.
"""

import math
from contextlib import contextmanager

import cairo
import numpy as np
from PIL import Image
from scipy import ndimage

SS = 4  # supersampling factor used for all vector rendering


def hexc(h):
    h = h.lstrip("#")
    return tuple(int(h[i : i + 2], 16) / 255 for i in (0, 2, 4))


def mix(a, b, t):
    return tuple(x + (y - x) * t for x, y in zip(a, b))


def scale_rgb(c, k):
    return tuple(min(1.0, x * k) for x in c)


OUTLINE = hexc("#1d1726")
GOLD = hexc("#f2c14e")
GOLD_SHADE = hexc("#c08a2a")
GOLD_LIGHT = hexc("#fff0b0")
STEEL = hexc("#d9e2ec")
STEEL_SHADE = hexc("#8a99ab")
STEEL_DARK = hexc("#5d6b7e")
WHITE = (1.0, 1.0, 1.0)
BLACK = (0.0, 0.0, 0.0)

# Standard line weights (in 256-canvas pixels).
OUTLINE_W = 6.5
DETAIL_W = 3.0
DETAIL_ALPHA = 0.6


# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------


def ellipse(ctx, cx, cy, rx, ry=None, angle=0.0):
    ry = rx if ry is None else ry
    ctx.save()
    ctx.translate(cx, cy)
    ctx.rotate(angle)
    ctx.scale(rx, ry)
    ctx.new_sub_path()
    ctx.arc(0, 0, 1, 0, 2 * math.pi)
    ctx.close_path()
    ctx.restore()


def rounded_rect(ctx, x, y, w, h, r):
    r = min(r, w / 2, h / 2)
    ctx.new_sub_path()
    ctx.arc(x + w - r, y + r, r, -math.pi / 2, 0)
    ctx.arc(x + w - r, y + h - r, r, 0, math.pi / 2)
    ctx.arc(x + r, y + h - r, r, math.pi / 2, math.pi)
    ctx.arc(x + r, y + r, r, math.pi, 1.5 * math.pi)
    ctx.close_path()


def polygon(ctx, pts):
    ctx.move_to(*pts[0])
    for p in pts[1:]:
        ctx.line_to(*p)
    ctx.close_path()


def _catmull_segments(pts, closed, tension=1.0):
    n = len(pts)
    pts = [np.asarray(p, float) for p in pts]
    segs = []
    rng = range(n) if closed else range(n - 1)
    for i in rng:
        p0 = pts[(i - 1) % n] if (closed or i > 0) else pts[i]
        p1 = pts[i]
        p2 = pts[(i + 1) % n]
        p3 = pts[(i + 2) % n] if (closed or i + 2 < n) else pts[(i + 1) % n]
        c1 = p1 + (p2 - p0) * tension / 6
        c2 = p2 - (p3 - p1) * tension / 6
        segs.append((p1, c1, c2, p2))
    return segs


def smooth_path(ctx, pts, closed=True, tension=1.0):
    """Catmull-Rom spline through `pts` as cubic Beziers."""
    segs = _catmull_segments(pts, closed, tension)
    ctx.move_to(*segs[0][0])
    for _, c1, c2, p2 in segs:
        ctx.curve_to(*c1, *c2, *p2)
    if closed:
        ctx.close_path()


def line(x0, y0, x1, y1):
    """Path: a single straight segment."""
    return lambda c: (c.move_to(x0, y0), c.line_to(x1, y1))


def curve(pts, tension=1.0):
    """Path: an open Catmull-Rom curve through pts."""
    return lambda c: smooth_path(c, pts, closed=False, tension=tension)


def polyline(pts):
    """Path: open straight segments through pts."""

    def p(c):
        c.move_to(*pts[0])
        for q in pts[1:]:
            c.line_to(*q)

    return p


def cross_ticks(pts, step, n=100):
    """Evenly spaced stations along a polyline: yields (point, unit normal, u in [0,1])
    every `step` samples of an n-point resampling (used for tail rings / cable segments)."""
    p, _ = resample_polyline(pts, n)
    for i in range(step, n - step, step):
        d = p[i + 1] - p[i - 1]
        d = d / np.linalg.norm(d)
        yield p[i], np.array([-d[1], d[0]]), i / n


def erase(ctx, path, width=None):
    """Punch `path` out of what's already drawn (fill it, or stroke it if width is given)."""
    ctx.save()
    ctx.set_operator(cairo.OPERATOR_CLEAR)
    ctx.new_path()
    path(ctx)
    if width:
        ctx.set_line_width(width)
        ctx.stroke()
    else:
        ctx.fill()
    ctx.restore()


def mirror_x(pts, cx):
    """Given the right half of a symmetric outline (top to bottom), return the full loop.
    End points lying on the axis are not duplicated."""
    pts = list(pts)
    left = [(2 * cx - x, y) for x, y in reversed(pts)]
    if abs(pts[-1][0] - cx) < 1e-6:
        left = left[1:]
    if abs(pts[0][0] - cx) < 1e-6:
        left = left[:-1]
    return pts + left


def sample_spline(pts, n=200, closed=False, tension=1.0):
    """Points along a Catmull-Rom spline (used for tapered strokes)."""
    out = []
    for p1, c1, c2, p2 in _catmull_segments(pts, closed, tension):
        for t in np.linspace(0, 1, n, endpoint=False):
            u = 1 - t
            out.append(u**3 * p1 + 3 * u * u * t * c1 + 3 * u * t * t * c2 + t**3 * p2)
    if not closed:
        out.append(np.asarray(pts[-1], float))
    return np.array(out)


def resample_polyline(pts, n):
    pts = np.asarray(pts, float)
    seg = np.linalg.norm(np.diff(pts, axis=0), axis=1)
    s = np.concatenate([[0], np.cumsum(seg)])
    t = np.linspace(0, s[-1], n)
    return np.stack([np.interp(t, s, pts[:, 0]), np.interp(t, s, pts[:, 1])], axis=1), s[-1]


def taper_path(ctx, pts, widths, n=120):
    """Closed outline of a variable-width stroke along polyline `pts`.

    `widths` is a function of arc-length fraction u in [0,1] -> full width.
    Ends are rounded (semicircular caps).
    """
    p, _ = resample_polyline(pts, n)
    d = np.gradient(p, axis=0)
    d /= np.linalg.norm(d, axis=1, keepdims=True)
    nrm = np.stack([-d[:, 1], d[:, 0]], axis=1)
    u = np.linspace(0, 1, n)
    w = np.array([widths(x) for x in u]) / 2
    left = p + nrm * w[:, None]
    right = p - nrm * w[:, None]
    ctx.move_to(*left[0])
    for q in left[1:]:
        ctx.line_to(*q)
    a_end = math.atan2(d[-1, 1], d[-1, 0])
    ctx.arc(p[-1, 0], p[-1, 1], max(w[-1], 1e-3), a_end - math.pi / 2, a_end + math.pi / 2)
    for q in right[::-1]:
        ctx.line_to(*q)
    a0 = math.atan2(d[0, 1], d[0, 0])
    ctx.arc(p[0, 0], p[0, 1], max(w[0], 1e-3), a0 + math.pi / 2, a0 + 3 * math.pi / 2)
    ctx.close_path()
    return p, nrm, w


# ---------------------------------------------------------------------------
# Raster helpers
# ---------------------------------------------------------------------------


def surface_to_rgba(surface):
    """cairo ARGB32 surface -> premultiplied float RGBA array."""
    surface.flush()
    h, w = surface.get_height(), surface.get_width()
    buf = np.ndarray((h, surface.get_stride() // 4, 4), np.uint8, surface.get_data())[:, :w]
    bgra = buf.astype(np.float32) / 255
    return bgra[..., [2, 1, 0, 3]].copy()


def rgba_to_surface(rgba):
    """premultiplied float RGBA array -> (cairo surface, backing array)."""
    h, w = rgba.shape[:2]
    arr = np.ascontiguousarray((np.clip(rgba[..., [2, 1, 0, 3]], 0, 1) * 255 + 0.5).astype(np.uint8))
    surf = cairo.ImageSurface.create_for_data(memoryview(arr), cairo.FORMAT_ARGB32, w, h, w * 4)
    return surf, arr


def bbox(mask, pad, shape):
    ys, xs = np.nonzero(mask)
    if len(ys) == 0:
        return None
    y0 = max(0, ys.min() - pad)
    y1 = min(shape[0], ys.max() + pad + 1)
    x0 = max(0, xs.min() - pad)
    x1 = min(shape[1], xs.max() + pad + 1)
    return slice(y0, y1), slice(x0, x1)


def downsample(a, f):
    if f == 1:
        return a
    h, w = a.shape[:2]
    return a.reshape(h // f, f, w // f, f, *a.shape[2:]).mean(axis=(1, 3))


def blur(a, sigma, ss=1):
    """Gaussian blur with sigma in sprite pixels of an array at supersample `ss`."""
    if sigma <= 0:
        return a
    small = downsample(a, ss)
    b = ndimage.gaussian_filter(small, sigma, mode="constant")
    if ss == 1:
        return b
    return ndimage.zoom(b, ss, order=1, mode="nearest", grid_mode=True)[: a.shape[0], : a.shape[1]]


def over(src, dst):
    return src + dst * (1 - src[..., 3:4])


def premul(rgb, a):
    a = np.asarray(a, np.float32)
    return np.concatenate([np.asarray(rgb, np.float32) * a[..., None], a[..., None]], axis=-1)


def unpremultiply(rgba):
    a = rgba[..., 3:4]
    rgb = np.where(a > 1e-6, rgba[..., :3] / np.maximum(a, 1e-6), 0)
    return np.concatenate([np.clip(rgb, 0, 1), a], axis=-1)


def to_pil(rgba_straight):
    return Image.fromarray((np.clip(rgba_straight, 0, 1) * 255 + 0.5).astype(np.uint8), "RGBA")


def centered_grid(canvas):
    """Per-(supersampled)-pixel x, y offsets from the canvas centre, in sprite px."""
    ys, xs = np.mgrid[0 : canvas.H, 0 : canvas.W].astype(np.float64)
    return (xs + 0.5) / canvas.ss - canvas.w / 2, (ys + 0.5) / canvas.ss - canvas.h / 2


def smoothstep(e0, e1, x):
    t = np.clip((x - e0) / (e1 - e0), 0, 1)
    return t * t * (3 - 2 * t)


# ---------------------------------------------------------------------------
# Canvas & layers
# ---------------------------------------------------------------------------


class Layer:
    """A supersampled cairo surface; user space is in final sprite pixels."""

    def __init__(self, canvas):
        self.canvas = canvas
        self.ss = canvas.ss
        self.surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, canvas.W, canvas.H)
        self.ctx = cairo.Context(self.surface)
        self.ctx.scale(self.ss, self.ss)
        self.ctx.set_line_cap(cairo.LINE_CAP_ROUND)
        self.ctx.set_line_join(cairo.LINE_JOIN_ROUND)
        self._keep = []

    @contextmanager
    def clip(self, path):
        """Restrict drawing to `path` inside the with-block."""
        c = self.ctx
        c.save()
        c.new_path()
        path(c)
        c.clip()
        try:
            yield
        finally:
            c.restore()

    # -- basic fills -------------------------------------------------------
    def fill(self, path, color, alpha=1.0):
        c = self.ctx
        c.new_path()
        path(c)
        c.set_source_rgba(*color, alpha)
        c.fill()

    def stroke(self, path, color, width, alpha=1.0):
        c = self.ctx
        c.new_path()
        path(c)
        c.set_source_rgba(*color, alpha)
        c.set_line_width(width)
        c.stroke()

    def detail(self, path, width=DETAIL_W, alpha=DETAIL_ALPHA, color=OUTLINE):
        """Thin interior detail line in the outline colour."""
        self.stroke(path, color, width, alpha)

    def fill_pattern(self, path, pattern):
        c = self.ctx
        c.new_path()
        path(c)
        c.set_source(pattern)
        c.fill()

    def mask(self, path):
        """Coverage of `path` as a float array at supersampled resolution."""
        surf = cairo.ImageSurface(cairo.FORMAT_A8, self.canvas.W, self.canvas.H)
        c = cairo.Context(surf)
        c.set_matrix(self.ctx.get_matrix())
        path(c)
        c.fill()
        surf.flush()
        buf = np.ndarray((self.canvas.H, surf.get_stride()), np.uint8, surf.get_data())
        return buf[:, : self.canvas.W].astype(np.float32) / 255

    def fill_image(self, path, rgb_fn, alpha=1.0):
        """Fill `path` with per-pixel colour computed by rgb_fn(xs, ys, mask_crop) ->
        (h, w, 3) or (h, w, 4) straight colour, evaluated only on the path's bbox."""
        m = self.mask(path)
        bb = bbox(m > 0, 2, m.shape)
        if bb is None:
            return
        ys, xs = np.mgrid[bb[0], bb[1]].astype(np.float32)
        xs = (xs + 0.5) / self.ss
        ys = (ys + 0.5) / self.ss
        col = np.asarray(rgb_fn(xs, ys, m[bb]), np.float32)
        if col.shape[-1] == 3:
            a = np.full(col.shape[:2], alpha, np.float32)
        else:
            a = col[..., 3] * alpha
            col = col[..., :3]
        full = np.zeros((self.canvas.H, self.canvas.W, 4), np.float32)
        full[bb] = premul(col, a)
        surf, arr = rgba_to_surface(full)
        self._keep.append(arr)
        c = self.ctx
        m = c.get_matrix()
        c.identity_matrix()
        c.set_source_surface(surf, 0, 0)  # locked to device space
        c.set_matrix(m)
        c.new_path()
        path(c)
        c.fill()

    def fill_shaded(
        self,
        path,
        base,
        shade,
        light=None,
        rim=12.0,
        hl=None,
        hl_r=None,
        hl_amt=0.0,
        rim_pow=0.6,
        spec=None,
        alpha=1.0,
        bevel=0.0,
        bevel_light=None,
    ):
        """Overhead-lit fill: colour falls from `base` to `shade` near the silhouette
        (over `rim` px, measured with a distance transform so it follows any shape),
        plus an optional soft highlight blob of colour `light` centred at `hl`.
        `spec` = (x, y, rx, ry, strength) adds a soft white specular spot.
        `bevel` > 0 adds a directional (top-left light) bevel inside the rim zone:
        edges facing up-left brighten toward `bevel_light`, the others darken.
        Only for static, non-rotating art."""
        base, shade = np.array(base, np.float32), np.array(shade, np.float32)
        light = None if light is None else np.array(light, np.float32)
        bl = np.array(WHITE if bevel_light is None else bevel_light, np.float32)
        ss = self.ss

        def rgb(xs, ys, m):
            inside = m > 0.5
            d = ndimage.distance_transform_edt(inside) / ss
            t = np.clip(d / rim, 0, 1) ** rim_pow
            t = t * t * (3 - 2 * t)
            col = shade + (base - shade) * t[..., None]
            if bevel:
                ds = ndimage.gaussian_filter(np.minimum(d, rim), ss * 0.6)
                gy, gx = np.gradient(ds)
                nrm = np.hypot(gx, gy) + 1e-6
                # outward normal = -grad; facing light (up-left) => -(-gx - gy) > 0
                facing = (gx + gy) / nrm / math.sqrt(2)
                k = (1 - t) * bevel
                up = np.clip(facing, 0, 1) * k
                down = np.clip(-facing, 0, 1) * k
                col = col + (bl - col) * up[..., None]
                col = col * (1 - 0.5 * down[..., None])
            if light is not None and hl is not None:
                rx, ry = (hl_r, hl_r) if np.isscalar(hl_r) else hl_r
                r2 = ((xs - hl[0]) / rx) ** 2 + ((ys - hl[1]) / ry) ** 2
                g = np.exp(-r2 * 2.0) * hl_amt
                col = col + (light - col) * g[..., None]
            if spec is not None:
                sx, sy, srx, sry, k = spec
                r2 = ((xs - sx) / srx) ** 2 + ((ys - sy) / sry) ** 2
                g = smoothstep(1.0, 0.55, np.sqrt(r2)) * k
                col = col + (1 - col) * g[..., None]
            return col

        self.fill_image(path, rgb, alpha)

    def gradient_fill(self, path, stops, x0, y0, x1, y1, alpha=1.0):
        pat = cairo.LinearGradient(x0, y0, x1, y1)
        for off, col in stops:
            pat.add_color_stop_rgba(off, *col[:3], alpha if len(col) == 3 else col[3] * alpha)
        self.fill_pattern(path, pat)

    def radial_fill(self, path, stops, cx, cy, r, alpha=1.0, fx=None, fy=None, r0=0.0, sx=1.0, sy=1.0):
        pat = cairo.RadialGradient(cx if fx is None else fx, cy if fy is None else fy, r0, cx, cy, r)
        for off, col in stops:
            a = alpha if len(col) == 3 else col[3] * alpha
            pat.add_color_stop_rgba(off, *col[:3], a)
        if sx != 1.0 or sy != 1.0:
            m = cairo.Matrix()
            m.translate(cx, cy)
            m.scale(1 / sx, 1 / sy)
            m.translate(-cx, -cy)
            pat.set_matrix(m)
        self.fill_pattern(path, pat)

    def rgba(self):
        return surface_to_rgba(self.surface)


class Canvas:
    """Premultiplied float RGBA buffer at supersampled resolution."""

    def __init__(self, w, h=None, ss=SS):
        h = w if h is None else h
        self.w, self.h, self.ss = w, h, ss
        self.W, self.H = w * ss, h * ss
        self.rgba = np.zeros((self.H, self.W, 4), np.float32)

    def layer(self):
        return Layer(self)

    def add(self, layer, outline=0.0, color=OUTLINE, ao=0.0, ao_sigma=5.0, ao_offset=(0, 0), opacity=1.0, mode="over"):
        """Composite a layer. `outline` is the silhouette outline width in sprite px.
        `ao` darkens what's already on the canvas under a blurred copy of the
        layer's silhouette (a soft contact shadow; offset (0,0) = overhead light)."""
        src = layer.rgba() if isinstance(layer, Layer) else layer
        if outline > 0:
            src = with_outline(src, outline * self.ss, color)
        if ao > 0:
            a = src[..., 3]
            if ao_offset != (0, 0):
                a = ndimage.shift(a, (ao_offset[1] * self.ss, ao_offset[0] * self.ss), order=1)
            sh = np.clip(blur(a, ao_sigma, self.ss) * ao, 0, 1)
            self.rgba[..., :3] *= 1 - sh[..., None]
        if opacity != 1.0:
            src = src * opacity
        if mode == "over":
            self.rgba = over(src, self.rgba)
        elif mode == "add":
            self.rgba[..., :3] += src[..., :3]
            self.rgba[..., 3] = np.maximum(self.rgba[..., 3], 0) + src[..., 3] * (1 - self.rgba[..., 3])
            self.rgba = np.clip(self.rgba, 0, None)
        elif mode == "atop":  # only where the canvas already has content
            da = self.rgba[..., 3:4]
            self.rgba[..., :3] = src[..., :3] * da + self.rgba[..., :3] * (1 - src[..., 3:4])
        else:
            raise ValueError(mode)

    def glow(self, color, cx, cy, r, strength=1.0, mode="over", sx=1.0, sy=1.0):
        """Soft gaussian-ish glow blob (in sprite px)."""
        ys, xs = np.mgrid[0 : self.H, 0 : self.W].astype(np.float32)
        xs = (xs + 0.5) / self.ss
        ys = (ys + 0.5) / self.ss
        d2 = ((xs - cx) / (r * sx)) ** 2 + ((ys - cy) / (r * sy)) ** 2
        a = np.exp(-d2 * 3.0) * strength
        a = np.where(d2 < 1, a, 0) * smoothstep(1.0, 0.7, np.sqrt(d2))
        src = premul(color, np.clip(a, 0, 1))
        self.add(src, mode=mode)

    def image(self):
        small = downsample(self.rgba, self.ss)
        return to_pil(unpremultiply(np.clip(small, 0, 1)))


def with_outline(src, r, color):
    """Return src composited over a dilation of its silhouette by r (supersampled px)."""
    a = src[..., 3]
    inside = a > 0.5
    bb = bbox(inside, int(r) + 4, a.shape)
    if bb is None:
        return src
    d = ndimage.distance_transform_edt(~inside[bb])
    o = np.zeros_like(a)
    o[bb] = np.clip(r + 0.5 - d, 0, 1)
    o = np.maximum(o, a)
    under = premul(color, o)
    return over(src, under)


# ---------------------------------------------------------------------------
# Convenience shapes
# ---------------------------------------------------------------------------


def path_of(fn, *args, **kw):
    return lambda c: fn(c, *args, **kw)


def multi(*paths):
    def p(c):
        for q in paths:
            q(c)

    return p
