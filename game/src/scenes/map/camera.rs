//! Where the map view looks, and how closely.

use macroquad::prelude::*;

use crate::render::fx::BoardSpace;
use crate::render::ui::ScreenLayout;

/// Closest zoom, relative to the default.
const MAX_ZOOM: f32 = 1.6;
/// Zoom change per notch of the mouse wheel.
const ZOOM_PER_NOTCH: f32 = 1.15;
/// How quickly the view glides to where it's headed (per second).
const GLIDE_RATE: f32 = 4.0;

/// The map view. It follows the hero until the player zooms or drags it,
/// and goes back to following them when they walk with keys or a pad.
pub(super) struct Camera {
    /// Map point at the center of the view, in cells.
    center: Vec2,
    /// Cell size relative to the default.
    zoom: f32,
    follow: bool,
    /// Map size in cells.
    map_size: Vec2,
}

impl Camera {
    pub(super) fn new(center: Vec2, map_size: Vec2) -> Self {
        Self {
            center,
            zoom: 1.0,
            follow: true,
            map_size,
        }
    }

    /// Where map points land on screen.
    pub(super) fn space(&self, layout: &ScreenLayout) -> BoardSpace {
        let cell = default_cell(layout) * self.zoom;
        BoardSpace {
            origin: layout.main.center() - self.center * cell,
            cell,
        }
    }

    /// Zoom by `notches` of the mouse wheel (positive zooms in), keeping
    /// the map point under the screen point `at` where it is.
    pub(super) fn zoom_at(&mut self, notches: f32, at: Vec2, layout: &ScreenLayout) {
        let before = self.space(layout);
        let point = (at - before.origin) / before.cell;
        self.zoom =
            (self.zoom * ZOOM_PER_NOTCH.powf(notches)).clamp(self.min_zoom(layout), MAX_ZOOM);
        let after = self.space(layout);
        self.center += (after.to_screen(point) - at) / after.cell;
        self.follow = false;
    }

    /// Drag the map by `delta` screen pixels.
    pub(super) fn pan(&mut self, delta: Vec2, layout: &ScreenLayout) {
        self.center -= delta / self.space(layout).cell;
        self.follow = false;
    }

    /// Go back to following the hero.
    pub(super) fn follow(&mut self) {
        self.follow = true;
    }

    /// Glide toward the hero or, once the player has moved the view, back
    /// over the map if it's been dragged past the edge.
    pub(super) fn update(&mut self, dt: f32, hero: Vec2, layout: &ScreenLayout) {
        // The window may have grown since the last zoom.
        self.zoom = self.zoom.clamp(self.min_zoom(layout), MAX_ZOOM);
        let target = if self.follow {
            hero
        } else {
            self.within_map(layout)
        };
        self.center += (target - self.center) * (1.0 - (-dt * GLIDE_RATE).exp());
    }

    /// Zoomed out just far enough to show the whole map (or the default
    /// zoom, if that already does).
    fn min_zoom(&self, layout: &ScreenLayout) -> f32 {
        let fit = (layout.main.size() / self.map_size).min_element() / default_cell(layout);
        fit.min(1.0)
    }

    /// The nearest center that keeps the view over the map, or that centers
    /// the map when the view is wider than it.
    fn within_map(&self, layout: &ScreenLayout) -> Vec2 {
        let half_view = layout.main.size() / (2.0 * self.space(layout).cell);
        let half_map = self.map_size / 2.0;
        self.center.clamp(
            half_view.min(half_map),
            (self.map_size - half_view).max(half_map),
        )
    }
}

/// Cell size at the default zoom, in pixels.
fn default_cell(layout: &ScreenLayout) -> f32 {
    if layout.portrait() {
        (layout.main.w / 15.0).clamp(22.0, 52.0)
    } else {
        (layout.main.h / 23.0).clamp(24.0, 60.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout() -> ScreenLayout {
        ScreenLayout {
            main: Rect::new(0.0, 0.0, 900.0, 800.0),
            panel: Rect::new(900.0, 0.0, 380.0, 800.0),
            top: None,
            s: 1.0,
        }
    }

    fn camera() -> Camera {
        Camera::new(vec2(20.0, 20.0), vec2(47.0, 49.0))
    }

    /// Where the map point `p` is on screen.
    fn on_screen(camera: &Camera, p: Vec2) -> Vec2 {
        camera.space(&layout()).to_screen(p)
    }

    #[test]
    fn zooming_keeps_the_point_under_the_cursor() {
        let mut camera = camera();
        let cursor = vec2(700.0, 200.0);
        let space = camera.space(&layout());
        let under_cursor = (cursor - space.origin) / space.cell;
        camera.zoom_at(2.0, cursor, &layout());
        assert!(on_screen(&camera, under_cursor).distance(cursor) < 0.01);
        camera.zoom_at(-5.0, cursor, &layout());
        assert!(on_screen(&camera, under_cursor).distance(cursor) < 0.01);
    }

    #[test]
    fn zooming_all_the_way_out_shows_the_whole_map() {
        let mut camera = camera();
        camera.zoom_at(-100.0, vec2(100.0, 100.0), &layout());
        for _ in 0..600 {
            camera.update(1.0 / 60.0, vec2(5.0, 5.0), &layout());
        }
        let main = layout().main;
        let (top_left, bottom_right) = (
            on_screen(&camera, Vec2::ZERO),
            on_screen(&camera, vec2(47.0, 49.0)),
        );
        assert!(main.contains(top_left + Vec2::ONE) && main.contains(bottom_right - Vec2::ONE));
        // Fitted snugly, not shrunk any further.
        assert!(bottom_right.y - top_left.y > main.h - 1.0);
    }

    #[test]
    fn dragging_moves_the_map_with_the_pointer() {
        let mut camera = camera();
        let p = vec2(21.0, 19.0);
        let before = on_screen(&camera, p);
        camera.pan(vec2(30.0, -12.0), &layout());
        assert!(on_screen(&camera, p).distance(before + vec2(30.0, -12.0)) < 0.01);
    }

    #[test]
    fn walking_with_keys_brings_the_view_back_to_the_hero() {
        let mut camera = camera();
        let hero = vec2(30.0, 10.0);
        camera.pan(vec2(400.0, 0.0), &layout());
        for _ in 0..120 {
            camera.update(1.0 / 60.0, hero, &layout());
        }
        let center = layout().main.center();
        assert!(
            on_screen(&camera, hero).distance(center) > 50.0,
            "a dragged view stays put"
        );
        camera.follow();
        for _ in 0..600 {
            camera.update(1.0 / 60.0, hero, &layout());
        }
        assert!(on_screen(&camera, hero).distance(center) < 1.0);
    }

    #[test]
    fn a_view_dragged_off_the_map_glides_back() {
        let mut camera = camera();
        camera.pan(vec2(5000.0, 5000.0), &layout());
        for _ in 0..600 {
            camera.update(1.0 / 60.0, vec2(20.0, 20.0), &layout());
        }
        let main = layout().main;
        assert!(
            main.contains(on_screen(&camera, vec2(1.0, 1.0))),
            "the map's corner is in view"
        );
        assert!(
            on_screen(&camera, Vec2::ZERO).x > main.x - 1.0,
            "and nothing past it"
        );
    }
}
