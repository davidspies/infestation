"""UI icons: 128x128 flat white glyphs (tinted by the engine), 12px round-capped
strokes, content kept inside a 12px margin."""

import math

import cairo

from artkit import Canvas, ellipse, erase, polygon, rounded_rect, smooth_path

W = 12.0  # stroke weight
WHITE = (1.0, 1.0, 1.0)


def icon(draw):
    cv = Canvas(128)
    L = cv.layer()
    c = L.ctx
    c.set_source_rgb(*WHITE)
    c.set_line_width(W)
    c.set_line_cap(cairo.LINE_CAP_ROUND)
    c.set_line_join(cairo.LINE_JOIN_ROUND)
    draw(c)
    cv.add(L)
    return cv.image()


cut = erase


def arrowhead(c, x, y, angle, size=22, spread=0.62):
    """Filled rounded arrowhead with its tip at (x, y) pointing along `angle`."""
    pts = [
        (x, y),
        (x - size * math.cos(angle - spread), y - size * math.sin(angle - spread)),
        (x - size * math.cos(angle + spread), y - size * math.sin(angle + spread)),
    ]
    c.new_path()
    polygon(c, pts)
    c.save()
    c.set_line_width(7)
    c.stroke_preserve()
    c.restore()
    c.fill()


def arc_arrow(c, cx, cy, r, a0, a1, clockwise):
    """Stroke an arc from a0 to a1 (radians, cairo convention) and put a head at a1."""
    c.new_path()
    # stop the shaft short of the head so the round cap doesn't poke out
    trim = 12 / r
    if clockwise:
        c.arc(cx, cy, r, a0, a1 - trim)
    else:
        c.arc_negative(cx, cy, r, a0, a1 + trim)
    c.stroke()
    tx, ty = cx + r * math.cos(a1), cy + r * math.sin(a1)
    tangent = a1 + (math.pi / 2 if clockwise else -math.pi / 2)
    # nudge the tip along the tangent so the head sits on the arc
    arrowhead(c, tx + 7 * math.cos(tangent), ty + 7 * math.sin(tangent), tangent, size=24)


def undo(c):
    arc_arrow(c, 66, 70, 34, math.radians(35), math.radians(185), clockwise=False)


def restart(c):
    arc_arrow(c, 64, 66, 36, math.radians(-60), math.radians(235), clockwise=True)


def wait(c):
    c.new_path()
    rounded_rect(c, 30, 16, 68, 12, 6)
    rounded_rect(c, 30, 100, 68, 12, 6)
    c.fill()
    c.new_path()
    c.move_to(40, 28)
    c.curve_to(40, 52, 58, 56, 58, 64)
    c.curve_to(58, 72, 40, 76, 40, 100)
    c.line_to(88, 100)
    c.curve_to(88, 76, 70, 72, 70, 64)
    c.curve_to(70, 56, 88, 52, 88, 28)
    c.close_path()
    c.set_line_width(9)
    c.stroke()
    # sand: a small heap below, a trickle, a sliver above
    c.new_path()
    c.move_to(46, 96)
    c.curve_to(52, 82, 76, 82, 82, 96)
    c.close_path()
    c.fill()
    c.new_path()
    rounded_rect(c, 61.5, 62, 5, 26, 2.5)
    c.fill()
    c.new_path()
    c.move_to(52, 40)
    c.line_to(76, 40)
    c.curve_to(74, 48, 66, 52, 64, 54)
    c.curve_to(62, 52, 54, 48, 52, 40)
    c.close_path()
    c.fill()


def map_(c):
    xs = [16, 44, 84, 112]
    top = [26, 16, 26, 16]
    bot = [112, 102, 112, 102]
    c.new_path()
    c.move_to(xs[0], top[0])
    for x, y in zip(xs[1:], top[1:]):
        c.line_to(x, y)
    for x, y in reversed(list(zip(xs, bot))):
        c.line_to(x, y)
    c.close_path()
    c.fill()
    # creases + dotted route + X, cut out
    for x, t, b in zip(xs[1:3], top[1:3], bot[1:3]):
        cut(c, lambda c, x=x, t=t, b=b: (c.move_to(x, t - 4), c.line_to(x, b + 4)), width=5)
    route = [(26, 92), (40, 76), (58, 82), (72, 62), (88, 56)]
    c.save()
    c.set_operator(cairo.OPERATOR_CLEAR)
    c.set_dash([0.1, 9.5])
    c.set_line_width(6)
    c.new_path()
    smooth_path(c, route, closed=False)
    c.stroke()
    c.restore()
    cut(c, lambda c: (c.move_to(88, 36), c.line_to(100, 48), c.move_to(100, 36), c.line_to(88, 48)), width=5.5)


def menu(c):
    for y in (34, 64, 94):
        c.new_path()
        c.move_to(24, y)
        c.line_to(104, y)
        c.stroke()


def gear(c):
    cx, cy = 64, 64
    teeth = 8
    c.new_path()
    for i in range(teeth * 2):
        a0 = (i - 0.5) / (teeth * 2) * 2 * math.pi
        a1 = (i + 0.5) / (teeth * 2) * 2 * math.pi
        r = 50 if i % 2 == 0 else 38
        # slightly tapered teeth
        da = 0.06 if i % 2 == 0 else -0.06
        p0 = (cx + r * math.cos(a0 + da), cy + r * math.sin(a0 + da))
        p1 = (cx + r * math.cos(a1 - da), cy + r * math.sin(a1 - da))
        if i == 0:
            c.move_to(*p0)
        else:
            c.line_to(*p0)
        c.line_to(*p1)
    c.close_path()
    c.save()
    c.set_line_width(5)
    c.stroke_preserve()
    c.restore()
    c.fill()
    cut(c, lambda c: ellipse(c, cx, cy, 16))


def music(c):
    # beamed pair of eighth notes
    for x, y in ((42, 92), (88, 84)):
        c.new_path()
        ellipse(c, x, y, 15, 11, -0.35)
        c.fill()
    c.new_path()
    c.move_to(54, 90)
    c.line_to(54, 30)
    c.move_to(100, 82)
    c.line_to(100, 22)
    c.set_line_width(9)
    c.stroke()
    c.new_path()
    polygon(c, [(50, 26), (104, 14), (104, 34), (50, 46)])
    c.fill()


def speaker(c):
    c.new_path()
    polygon(c, [(16, 50), (34, 50), (60, 26), (60, 102), (34, 78), (16, 78)])
    c.save()
    c.set_line_width(8)
    c.stroke_preserve()
    c.restore()
    c.fill()


def sound(c):
    speaker(c)
    for r in (20, 38):
        c.new_path()
        c.arc(64, 64, r, -0.75, 0.75)
        c.set_line_width(10)
        c.stroke()


def mute(c):
    speaker(c)
    c.set_line_width(11)
    c.new_path()
    c.move_to(80, 48)
    c.line_to(108, 80)
    c.move_to(108, 48)
    c.line_to(80, 80)
    c.stroke()


def lock(c):
    c.new_path()
    c.arc(64, 52, 24, math.pi, 0)
    c.line_to(88, 62)
    c.move_to(40, 62)
    c.line_to(40, 52)
    c.set_line_width(12)
    c.stroke()
    c.new_path()
    rounded_rect(c, 24, 56, 80, 58, 12)
    c.fill()
    cut(c, lambda c: ellipse(c, 64, 78, 8))
    cut(c, lambda c: polygon(c, [(60, 80), (68, 80), (71, 98), (57, 98)]))


def check(c):
    c.set_line_width(16)
    c.new_path()
    c.move_to(24, 66)
    c.line_to(52, 94)
    c.line_to(104, 36)
    c.stroke()


def star_pts(cx, cy, r0, r1, n=5, rot=-math.pi / 2):
    pts = []
    for i in range(2 * n):
        r = r0 if i % 2 == 0 else r1
        a = rot + i * math.pi / n
        pts.append((cx + r * math.cos(a), cy + r * math.sin(a)))
    return pts


def star(c):
    c.new_path()
    polygon(c, star_pts(64, 68, 48, 21))
    c.save()
    c.set_line_width(8)
    c.stroke_preserve()
    c.restore()
    c.fill()


def play(c):
    c.new_path()
    polygon(c, [(38, 24), (100, 64), (38, 104)])
    c.save()
    c.set_line_width(14)
    c.stroke_preserve()
    c.restore()
    c.fill()


def close(c):
    c.set_line_width(14)
    c.new_path()
    c.move_to(32, 32)
    c.line_to(96, 96)
    c.move_to(96, 32)
    c.line_to(32, 96)
    c.stroke()


def skull(c):
    c.new_path()
    ellipse(c, 64, 54, 42, 38)
    c.fill()
    c.new_path()
    rounded_rect(c, 38, 70, 52, 34, 10)
    c.fill()
    cut(c, lambda c: ellipse(c, 47, 58, 11, 12))
    cut(c, lambda c: ellipse(c, 81, 58, 11, 12))
    cut(c, lambda c: polygon(c, [(64, 70), (57, 82), (71, 82)]))
    for x in (54, 64, 74):
        cut(c, lambda c, x=x: (c.move_to(x, 92), c.line_to(x, 106)), width=4.5)


def sword(c):
    c.save()
    c.translate(64, 64)
    c.rotate(math.radians(45))
    # blade (pointing up-right after rotation)
    c.new_path()
    polygon(c, [(-9, -50), (0, -62), (9, -50), (9, 14), (-9, 14)])
    c.save()
    c.set_line_width(4)
    c.stroke_preserve()
    c.restore()
    c.fill()
    c.new_path()
    rounded_rect(c, -28, 14, 56, 13, 6.5)
    c.fill()
    c.new_path()
    rounded_rect(c, -6, 26, 12, 22, 4)
    c.fill()
    c.new_path()
    ellipse(c, 0, 51, 9)
    c.fill()
    c.restore()
    c.save()
    c.translate(64, 64)
    c.rotate(math.radians(45))
    cut(c, lambda c: (c.move_to(0, -46), c.line_to(0, 8)), width=3.5)
    c.restore()


def rat(c):
    c.save()
    c.translate(64, 64)
    c.scale(0.88, 0.88)
    c.translate(-64, -62)
    rat_head(c)
    c.restore()


def rat_head(c):
    # front view rat head: big round ears, round head, pointed snout down
    for sx in (-1, 1):
        c.new_path()
        ellipse(c, 64 + sx * 32, 40, 24, 24)
        c.fill()
    c.new_path()
    smooth_path(c, [(64, 110), (84, 94), (98, 70), (92, 48), (64, 38), (36, 48), (30, 70), (44, 94)], tension=0.9)
    c.fill()
    for sx in (-1, 1):
        cut(c, lambda c, sx=sx: ellipse(c, 64 + sx * 34, 38, 12, 12))
        cut(c, lambda c, sx=sx: ellipse(c, 64 + sx * 15, 68, 6.5, 7.5))
    cut(c, lambda c: ellipse(c, 64, 98, 8, 6))
    c.set_line_width(4)
    for sx in (-1, 1):
        for dy in (-6, 4):
            c.new_path()
            c.move_to(64 + sx * 26, 90 + dy * 0.5)
            c.line_to(64 + sx * 50, 88 + dy * 1.6)
            c.stroke()


def players(c):
    # back figure (right), cut a gap, then front figure (left)
    c.new_path()
    ellipse(c, 84, 40, 17)
    c.fill()
    c.new_path()
    c.move_to(56, 104)
    c.curve_to(56, 72, 70, 64, 84, 64)
    c.curve_to(98, 64, 114, 72, 114, 104)
    c.close_path()
    c.fill()
    # gap around the front figure
    c.save()
    c.set_operator(cairo.OPERATOR_CLEAR)
    c.set_line_width(14)
    c.new_path()
    ellipse(c, 48, 46, 20)
    c.stroke()
    c.new_path()
    c.move_to(12, 108)
    c.curve_to(12, 76, 30, 70, 48, 70)
    c.curve_to(66, 70, 84, 76, 84, 108)
    c.stroke()
    c.restore()
    c.new_path()
    ellipse(c, 48, 46, 20)
    c.fill()
    c.new_path()
    c.move_to(14, 112)
    c.curve_to(14, 80, 30, 72, 48, 72)
    c.curve_to(66, 72, 82, 80, 82, 112)
    c.close_path()
    c.fill()


def tray(c):
    c.new_path()
    c.move_to(20, 64)
    c.line_to(20, 104)
    c.line_to(108, 104)
    c.line_to(108, 64)
    c.stroke()


def export(c):
    tray(c)
    c.new_path()
    c.move_to(64, 84)
    c.line_to(64, 34)
    c.stroke()
    arrowhead(c, 64, 16, -math.pi / 2, size=26)


def import_(c):
    tray(c)
    c.new_path()
    c.move_to(64, 20)
    c.line_to(64, 62)
    c.stroke()
    arrowhead(c, 64, 84, math.pi / 2, size=26)


def trophy(c):
    c.new_path()
    c.move_to(34, 18)
    c.line_to(94, 18)
    c.curve_to(94, 54, 82, 72, 64, 74)
    c.curve_to(46, 72, 34, 54, 34, 18)
    c.close_path()
    c.fill()
    c.set_line_width(9)
    for sx in (-1, 1):
        c.new_path()
        x = 64 + sx * 30
        c.move_to(x, 26)
        c.curve_to(x + sx * 24, 24, x + sx * 22, 54, x - sx * 4, 56)
        c.stroke()
    c.new_path()
    rounded_rect(c, 57, 70, 14, 22, 3)
    c.fill()
    c.new_path()
    rounded_rect(c, 36, 92, 56, 16, 6)
    c.fill()
    cut(c, lambda c: polygon(c, star_pts(64, 42, 13, 6)))


def chevron(c):
    c.set_line_width(16)
    c.new_path()
    c.move_to(48, 24)
    c.line_to(86, 64)
    c.line_to(48, 104)
    c.stroke()


def door(c):
    # arched frame
    c.new_path()
    c.move_to(28, 108)
    c.line_to(28, 54)
    c.arc(64, 54, 36, math.pi, 0)
    c.line_to(100, 108)
    c.set_line_width(10)
    c.stroke()
    # door leaf ajar
    c.new_path()
    c.move_to(40, 108)
    c.line_to(40, 56)
    c.arc(64, 56, 24, math.pi, 1.5 * math.pi)
    c.line_to(72, 32)
    c.line_to(72, 108)
    c.close_path()
    c.fill()
    cut(c, lambda c: ellipse(c, 62, 76, 5))
    c.new_path()
    c.move_to(20, 110)
    c.line_to(108, 110)
    c.set_line_width(8)
    c.stroke()


NAMES = {
    "undo": undo,
    "restart": restart,
    "wait": wait,
    "map": map_,
    "menu": menu,
    "gear": gear,
    "music": music,
    "sound": sound,
    "mute": mute,
    "lock": lock,
    "check": check,
    "star": star,
    "play": play,
    "close": close,
    "skull": skull,
    "sword": sword,
    "rat": rat,
    "players": players,
    "export": export,
    "import": import_,
    "trophy": trophy,
    "chevron": chevron,
    "door": door,
}

SPRITES = {f"icon_{n}": (lambda f=f: icon(f)) for n, f in NAMES.items()}
