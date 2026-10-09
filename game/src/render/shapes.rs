//! Vector shape primitives, each built as a single mesh so translucent
//! shapes blend once (no double-darkened overlaps).

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use macroquad::models::{Mesh, Vertex, draw_mesh};
use macroquad::prelude::*;

fn vertex(p: Vec2, color: Color) -> Vertex {
    Vertex::new(p.x, p.y, 0.0, 0.0, 0.0, color)
}

fn draw_triangles(vertices: Vec<Vertex>, indices: Vec<u16>) {
    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: None,
    });
}

/// Segments for a smooth arc of the given radius and angle.
fn arc_segments(radius: f32, angle: f32) -> usize {
    ((radius.sqrt() * angle * 2.0) as usize).clamp(3, 64)
}

/// The outline of a rounded rectangle, clockwise from the top-left corner.
fn rounded_rect_path(rect: Rect, radius: f32) -> Vec<Vec2> {
    rounded_rect_path_n(rect, radius, arc_segments(radius, FRAC_PI_2))
}

/// As `rounded_rect_path` with `n` segments per corner, so concentric
/// outlines can be joined vertex for vertex.
fn rounded_rect_path_n(rect: Rect, radius: f32, n: usize) -> Vec<Vec2> {
    let r = radius.min(rect.w / 2.0).min(rect.h / 2.0).max(0.0);
    let corners = [
        (vec2(rect.x + r, rect.y + r), PI),
        (vec2(rect.right() - r, rect.y + r), PI * 1.5),
        (vec2(rect.right() - r, rect.bottom() - r), 0.0),
        (vec2(rect.x + r, rect.bottom() - r), FRAC_PI_2),
    ];
    corners
        .into_iter()
        .flat_map(|(center, start)| {
            (0..=n).map(move |i| {
                let a = start + FRAC_PI_2 * i as f32 / n as f32;
                center + vec2(a.cos(), a.sin()) * r
            })
        })
        .collect()
}

/// Fill a convex outline as a fan, coloring each vertex.
fn fill_fan(center: Vec2, path: &[Vec2], color_at: impl Fn(Vec2) -> Color) {
    let mut vertices = vec![vertex(center, color_at(center))];
    vertices.extend(path.iter().map(|&p| vertex(p, color_at(p))));
    let n = path.len() as u16;
    let indices = (0..n).flat_map(|i| [0, i + 1, (i + 1) % n + 1]).collect();
    draw_triangles(vertices, indices);
}

/// Fill the band between two closed outlines with matching vertex counts.
fn fill_band(outer: &[Vec2], inner: &[Vec2], outer_color: Color, inner_color: Color) {
    assert_eq!(outer.len(), inner.len());
    let n = outer.len() as u16;
    let vertices = outer
        .iter()
        .map(|&p| vertex(p, outer_color))
        .chain(inner.iter().map(|&p| vertex(p, inner_color)))
        .collect();
    let indices = (0..n)
        .flat_map(|i| {
            let j = (i + 1) % n;
            [i, j, n + i, j, n + j, n + i]
        })
        .collect();
    draw_triangles(vertices, indices);
}

pub(crate) fn rounded_rect(rect: Rect, radius: f32, color: Color) {
    fill_fan(rect.center(), &rounded_rect_path(rect, radius), |_| color);
}

/// A rounded rectangle shaded from `top` to `bottom`.
pub(crate) fn rounded_rect_gradient(rect: Rect, radius: f32, top: Color, bottom: Color) {
    fill_fan(rect.center(), &rounded_rect_path(rect, radius), |p| {
        let t = ((p.y - rect.y) / rect.h).clamp(0.0, 1.0);
        lerp_color(top, bottom, t)
    });
}

pub(crate) fn rounded_rect_outline(rect: Rect, radius: f32, thickness: f32, color: Color) {
    let half = thickness / 2.0;
    let n = arc_segments(radius + half, FRAC_PI_2);
    let outer = rounded_rect_path_n(
        Rect::new(
            rect.x - half,
            rect.y - half,
            rect.w + thickness,
            rect.h + thickness,
        ),
        radius + half,
        n,
    );
    let inner = rounded_rect_path_n(
        Rect::new(
            rect.x + half,
            rect.y + half,
            rect.w - thickness,
            rect.h - thickness,
        ),
        (radius - half).max(0.0),
        n,
    );
    fill_band(&outer, &inner, color, color);
}

/// A soft drop shadow: `color` under the rectangle fading to transparent
/// over `blur` pixels outside it.
pub(crate) fn soft_shadow(rect: Rect, radius: f32, blur: f32, color: Color) {
    let n = arc_segments(radius + blur, FRAC_PI_2);
    let inner = rounded_rect_path_n(rect, radius, n);
    let outer = rounded_rect_path_n(
        Rect::new(
            rect.x - blur,
            rect.y - blur,
            rect.w + blur * 2.0,
            rect.h + blur * 2.0,
        ),
        radius + blur,
        n,
    );
    fill_band(&outer, &inner, Color { a: 0.0, ..color }, color);
    rounded_rect(rect, radius, color);
}

fn circle_path(center: Vec2, radius: f32) -> Vec<Vec2> {
    let n = arc_segments(radius, TAU);
    (0..n)
        .map(|i| {
            let a = TAU * i as f32 / n as f32;
            center + vec2(a.cos(), a.sin()) * radius
        })
        .collect()
}

pub(crate) fn circle(center: Vec2, radius: f32, color: Color) {
    fill_fan(center, &circle_path(center, radius), |_| color);
}

/// A disc fading from `inner` at the center to `outer` at the rim.
pub(crate) fn radial_gradient(center: Vec2, radius: f32, inner: Color, outer: Color) {
    fill_fan(center, &circle_path(center, radius), |p| {
        if p == center { inner } else { outer }
    });
}

pub(crate) fn ring(center: Vec2, radius: f32, thickness: f32, color: Color) {
    let half = thickness / 2.0;
    let n = arc_segments(radius + half, TAU);
    let point = |i: usize, r: f32| {
        let a = TAU * i as f32 / n as f32;
        center + vec2(a.cos(), a.sin()) * r
    };
    let outer: Vec<_> = (0..n).map(|i| point(i, radius + half)).collect();
    let inner: Vec<_> = (0..n).map(|i| point(i, (radius - half).max(0.0))).collect();
    fill_band(&outer, &inner, color, color);
}

/// An elliptical ring: `radii` along and across `angle`, which rotates it
/// clockwise.
pub(crate) fn ellipse_ring(center: Vec2, radii: Vec2, angle: f32, thickness: f32, color: Color) {
    let n = arc_segments(radii.max_element(), TAU);
    let rotation = Vec2::from_angle(angle);
    let half = thickness / 2.0;
    let point = |i: usize, grow: f32| {
        let a = TAU * i as f32 / n as f32;
        center + rotation.rotate(vec2(a.cos() * (radii.x + grow), a.sin() * (radii.y + grow)))
    };
    let outer: Vec<_> = (0..n).map(|i| point(i, half)).collect();
    let inner: Vec<_> = (0..n).map(|i| point(i, -half)).collect();
    fill_band(&outer, &inner, color, color);
}

/// A thick polyline with round joins and caps.
pub(crate) fn polyline(points: &[Vec2], thickness: f32, color: Color) {
    let half = thickness / 2.0;
    let mut vertices = Vec::new();
    let mut indices: Vec<u16> = Vec::new();
    let mut add_quad = |a: Vec2, b: Vec2| {
        let dir = (b - a).normalize_or_zero();
        let side = vec2(-dir.y, dir.x) * half;
        let base = vertices.len() as u16;
        vertices.extend([a + side, b + side, b - side, a - side].map(|p| vertex(p, color)));
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    };
    for pair in points.windows(2) {
        add_quad(pair[0], pair[1]);
    }
    draw_triangles(vertices, indices);
    for &p in points {
        circle(p, half, color);
    }
}

/// A dashed polyline; `phase` (in pixels) slides the dashes along it.
pub(crate) fn dashed_polyline(
    points: &[Vec2],
    thickness: f32,
    dash: f32,
    gap: f32,
    phase: f32,
    color: Color,
) {
    let period = dash + gap;
    let mut travelled = -phase.rem_euclid(period);
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let len = a.distance(b);
        let dir = (b - a) / len.max(1e-6);
        // Dashes overlapping this segment start at travelled + k * period.
        let first = ((-travelled) / period).floor() as i32;
        let mut k = first;
        loop {
            let start = travelled + k as f32 * period;
            if start >= len {
                break;
            }
            let (s, e) = (start.max(0.0), (start + dash).min(len));
            if e > s {
                let (p, q) = (a + dir * s, a + dir * e);
                polyline(&[p, q], thickness, color);
            }
            k += 1;
        }
        travelled -= len;
    }
}

/// Cover the screen except for a circular hole (for iris transitions).
pub(crate) fn iris(center: Vec2, radius: f32, color: Color) {
    let far = vec2(screen_width(), screen_height()).length() * 2.0 + radius;
    let n = 96;
    let point = |i: usize, r: f32| {
        let a = TAU * i as f32 / n as f32;
        center + vec2(a.cos(), a.sin()) * r
    };
    let outer: Vec<_> = (0..n).map(|i| point(i, far)).collect();
    let inner: Vec<_> = (0..n).map(|i| point(i, radius.max(0.0))).collect();
    fill_band(&outer, &inner, color, color);
}

pub(crate) fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

/// `color` with its alpha multiplied by `alpha`.
pub(crate) fn faded(color: Color, alpha: f32) -> Color {
    Color {
        a: color.a * alpha,
        ..color
    }
}

/// A rounded rectangle filled with `texture`, where `uv_rect` gives the
/// texture coordinates of `rect`'s corners (beyond [0, 1] for repeating
/// textures).
pub(crate) fn textured_rounded_rect(
    texture: &Texture2D,
    rect: Rect,
    radius: f32,
    uv_rect: Rect,
    color: Color,
) {
    let uv = |p: Vec2| {
        vec2(
            uv_rect.x + (p.x - rect.x) / rect.w * uv_rect.w,
            uv_rect.y + (p.y - rect.y) / rect.h * uv_rect.h,
        )
    };
    let path = rounded_rect_path(rect, radius);
    let center = rect.center();
    let mut vertices = vec![Vertex::new(
        center.x,
        center.y,
        0.0,
        uv(center).x,
        uv(center).y,
        color,
    )];
    vertices.extend(
        path.iter()
            .map(|&p| Vertex::new(p.x, p.y, 0.0, uv(p).x, uv(p).y, color)),
    );
    let n = path.len() as u16;
    let indices = (0..n).flat_map(|i| [0, i + 1, (i + 1) % n + 1]).collect();
    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: Some(texture.clone()),
    });
}

/// A quad with a color gradient from edge `a`-`b` to edge `d`-`c`
/// (corners in order), for soft shading strips.
pub(crate) fn gradient_quad(corners: [Vec2; 4], start: Color, end: Color) {
    let [a, b, c, d] = corners;
    draw_triangles(
        vec![
            vertex(a, start),
            vertex(b, start),
            vertex(c, end),
            vertex(d, end),
        ],
        vec![0, 1, 2, 0, 2, 3],
    );
}

/// A rectangle shaded from `top` to `bottom`.
pub(crate) fn vertical_gradient(rect: Rect, top: Color, bottom: Color) {
    gradient_quad(
        [
            vec2(rect.x, rect.y),
            vec2(rect.right(), rect.y),
            vec2(rect.right(), rect.bottom()),
            vec2(rect.x, rect.bottom()),
        ],
        top,
        bottom,
    );
}
