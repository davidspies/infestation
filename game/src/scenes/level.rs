//! Playing a level: input handling, turning game events into effects and
//! sounds, win/lose flow, and the heads-up display.

use enum_map::EnumMap;
use macroquad::prelude::*;

use crate::audio::Sfx;
use crate::direction::Dir4;
use crate::game::{Action, Game, PlayState};
use crate::grid::{Cell, Player};
use crate::input::{MetaInput, PlayerInput, PointerEvent, TouchGesture};
use crate::levels::Level;
use crate::path::{NextMove, PathDrag, PathFollower};
use crate::position::Position;
use crate::progress::Progress;
use crate::render::board::{self, BoardLayout, BoardView, Facings};
use crate::render::fx::Fx;
use crate::render::palette::{self, Palette};
use crate::render::shapes::radial_gradient;
use crate::render::ui::ScreenLayout;
use crate::scenes::{Ctx, FrameInput, Request};
use crate::world_map::{Theme, WORLD_MAP};

mod director;
mod hud;

/// Grace period for the second player to input before a solo action fires.
const SYNC_GRACE_PERIOD: f32 = 0.064;
/// Characters per second for the dialogue typewriter.
const TYPE_SPEED: f32 = 60.0;
/// Seconds to press restart again to confirm it.
const RESTART_CONFIRM: f32 = 2.0;
/// Delay before the win/lose card appears, so the final blow can land.
const OUTCOME_DELAY: f32 = 0.55;

#[derive(Clone, Copy)]
struct PendingAction {
    action: Action,
    synced: bool,
    /// Whether this was from a fresh key press (not a held-repeat).
    /// Held-repeat pending is cleared when the key is released.
    fresh: bool,
}

/// Buttons on the HUD and the win/lose cards.
#[derive(Clone, Copy, PartialEq, Debug)]
enum UiAction {
    Wait,
    Undo,
    Restart,
    Menu,
    Continue,
    Solution,
}

/// Note text being typed out.
#[derive(Default)]
struct Dialogue {
    text: String,
    player: Option<Player>,
    shown: f32,
}

pub(crate) struct LevelScene {
    pub(crate) level: &'static Level,
    region: Option<&'static str>,
    palette: Palette,
    game: Game,
    facings: Facings,
    fx: Fx,
    /// Path being dragged out with a finger or mouse button held down.
    drag: Option<PathDrag>,
    /// Active paths the players are following.
    paths: EnumMap<Player, Option<PathFollower>>,
    /// Per-player buffered actions (sync or immediate).
    pending: EnumMap<Player, Option<PendingAction>>,
    /// Time elapsed since the first pending action was set, for the 2P grace period.
    pending_timer: f32,
    time: f32,
    /// When the level was won or lost (scene time), once animations settle.
    ended: Option<f32>,
    dialogue: Dialogue,
    /// When restart was first pressed, awaiting a second press.
    restart_armed: Option<f32>,
    /// Progress before this level was won, to animate what it opened.
    progress_before: Option<Progress>,
    /// Text of the solution, shown on request.
    pub(crate) solution: Option<String>,
}

impl LevelScene {
    pub(crate) fn new(level: &'static Level) -> Self {
        let map = &*WORLD_MAP;
        let theme = map.theme_of(level.name);
        let region = map
            .node_of_level(level.name)
            .map(|n| map.regions[map.nodes[n].region].name.as_str());
        let game = Game::new(level.grid.clone());
        let mut facings = Facings::default();
        facings.snap(&game.state.grid);
        Self {
            level,
            region,
            palette: palette::palette(theme),
            game,
            facings,
            fx: Fx::default(),
            drag: None,
            paths: EnumMap::default(),
            pending: EnumMap::default(),
            pending_timer: 0.0,
            time: 0.0,
            ended: None,
            dialogue: Dialogue::default(),
            restart_armed: None,
            progress_before: None,
            solution: None,
        }
    }

    pub(crate) fn theme(&self) -> Theme {
        WORLD_MAP.theme_of(self.level.name)
    }

    pub(crate) fn player_count(&self) -> usize {
        self.game.state.player_count()
    }

    /// Where the hero is on screen, for centering transitions.
    pub(crate) fn focus(&self, layout: &ScreenLayout) -> Vec2 {
        let board = self.board_layout(layout);
        self.game
            .state
            .player_position(Player::Player1)
            .map_or(board.slab_rect().center(), |p| board.space.cell_center(p))
    }

    fn board_layout(&self, layout: &ScreenLayout) -> BoardLayout {
        let m = 6.0 * layout.s;
        let area = Rect::new(
            layout.main.x + m,
            layout.main.y + m,
            layout.main.w - 2.0 * m,
            layout.main.h - 2.0 * m,
        );
        BoardLayout::fit(area, self.game.grid_width(), self.game.grid_height())
    }

    fn play_state(&self) -> PlayState {
        self.game.state.play_state()
    }

    /// Won or lost, with the final animation finished.
    fn outcome(&self) -> Option<PlayState> {
        let state = self.play_state();
        (state != PlayState::Playing && !self.game.is_animating()).then_some(state)
    }

    /// Whether the win/lose card is showing.
    fn card_visible(&self) -> bool {
        self.ended.is_some_and(|t| self.time - t > OUTCOME_DELAY)
    }

    pub(crate) fn restart(&mut self, ctx: &mut Ctx) {
        self.game.restart();
        self.after_rewind();
        self.fx.wash(Color::new(1.0, 1.0, 1.0, 0.35), 0.35);
        ctx.audio.play(Sfx::Restart);
    }

    fn undo(&mut self, ctx: &mut Ctx) {
        if self.game.state.history.len() > 1 {
            self.game.undo();
            self.after_rewind();
            self.fx.wash(Color::new(0.45, 0.6, 1.0, 0.18), 0.22);
            ctx.audio.play(Sfx::Undo);
        }
    }

    /// Reset presentation after undo/restart.
    fn after_rewind(&mut self) {
        self.pending = EnumMap::default();
        self.clear_paths();
        self.fx.clear();
        self.facings.snap(&self.game.state.grid);
        self.ended = None;
        self.restart_armed = None;
    }

    /// Drop the in-progress drag and all paths being followed.
    fn clear_paths(&mut self) {
        self.drag = None;
        self.paths = EnumMap::default();
    }

    pub(crate) fn update(
        &mut self,
        ctx: &mut Ctx,
        input: &FrameInput,
        layout: &ScreenLayout,
    ) -> Option<Request> {
        let dt = input.dt;
        self.time += dt;
        let board = self.board_layout(layout);
        let hud = self.hud(ctx, layout);

        // Touch/mouse: a drag starting on a player drags out a path for it
        // to follow; swipes move player 1; taps hit buttons.
        let mut swipe = None;
        let mut taps = Vec::new();
        for &event in &input.pointer {
            match event {
                PointerEvent::Down(pos) => self.pointer_down(board, pos),
                PointerEvent::Moved(pos) => self.pointer_moved(board, pos),
                PointerEvent::Up { .. } => {
                    if let Some(drag) = self.drag.take() {
                        let player = drag.player;
                        self.paths[player] = drag.into_follower();
                    } else {
                        match event.gesture() {
                            Some(TouchGesture::Swipe(dir)) => swipe = Some(dir),
                            Some(TouchGesture::Tap(pos)) => taps.push(pos),
                            None => {}
                        }
                    }
                }
            }
        }
        let mut actions: Vec<UiAction> = taps.iter().filter_map(|&p| hud.hit(p)).collect();
        for meta in &input.meta {
            let action = match meta {
                MetaInput::Undo => Some(UiAction::Undo),
                MetaInput::Restart => Some(UiAction::Restart),
                MetaInput::Exit => Some(UiAction::Menu),
                MetaInput::Confirm(_) if self.card_visible() => Some(match self.outcome() {
                    Some(PlayState::Won) => UiAction::Continue,
                    _ => UiAction::Undo,
                }),
                MetaInput::Confirm(_) => None,
            };
            actions.extend(action);
        }

        let mut request = None;
        let mut wait_tapped = false;
        for action in actions {
            match action {
                UiAction::Undo => self.undo(ctx),
                UiAction::Restart => {
                    if !self.action_enabled(UiAction::Restart) {
                        continue;
                    }
                    if self.restart_pending() || self.outcome().is_some() {
                        self.restart(ctx);
                    } else {
                        self.restart_armed = Some(self.time);
                        ctx.audio.play(Sfx::UiMove);
                    }
                }
                UiAction::Menu => request = Some(Request::Pause),
                UiAction::Continue => {
                    ctx.audio.play(Sfx::UiConfirm);
                    request = Some(Request::ToMap {
                        from_level: Some(self.level),
                        before: self.progress_before.take(),
                    });
                }
                UiAction::Wait => wait_tapped = true,
                UiAction::Solution => {
                    let csv = self.game.state.initial_grid.to_csv();
                    self.solution = Some(crate::solution::encode_solution(
                        self.level.name,
                        &csv,
                        &self.game.state.action_history,
                    ));
                }
            }
        }

        if self.play_state() == PlayState::Playing && request.is_none() {
            self.queue_player_input(input, swipe, wait_tapped);
        }
        self.try_execute_pending();
        self.advance_pending_timer(dt);

        self.fx.update(dt);
        self.facings.update(dt);
        if !self.fx.frozen() {
            let busy = self.pending.values().any(Option::is_some)
                || self.game.state.queued_actions.is_some();
            let events = self.game.animate(if busy { dt * 1.6 } else { dt });
            self.react(ctx, events);
        }
        self.try_execute_pending();
        self.update_outcome(ctx, layout);
        self.update_dialogue(ctx, dt);
        self.ambient(board);
        request
    }

    fn queue_player_input(&mut self, input: &FrameInput, swipe: Option<Dir4>, wait_tapped: bool) {
        let mut player_actions = input.players;
        if let Some(dir) = swipe {
            player_actions[Player::Player1] = Some(PlayerInput {
                action: Action::Move(dir),
                synced: false,
                fresh: true,
            });
        }
        if wait_tapped {
            let stall = PlayerInput {
                action: Action::Stall,
                synced: false,
                fresh: true,
            };
            for (_, action) in player_actions.iter_mut() {
                *action = action.or(Some(stall));
            }
        }
        self.feed_path_moves(&mut player_actions);
        let player_count = self.game.state.player_count();
        for (player, player_action) in player_actions.iter().take(player_count) {
            if let Some(input) = player_action {
                self.pending[player] = Some(PendingAction {
                    action: input.action,
                    synced: input.synced,
                    fresh: input.fresh,
                });
            } else if self.pending[player].is_some_and(|p| !p.fresh) {
                // Key was released — clear held-repeat pending so it doesn't
                // fire as a stale buffered move after animation finishes.
                self.pending[player] = None;
            }
        }
    }

    /// A finger/mouse press: stops any path being followed, and starts a
    /// path drag if it lands on a player.
    fn pointer_down(&mut self, board: BoardLayout, pos: Vec2) {
        self.clear_paths();
        if self.play_state() != PlayState::Playing {
            return;
        }
        let Some(cell) = board.screen_to_cell(pos) else {
            return;
        };
        if let Some((player, _)) = self.game.state.grid.at(cell).as_player() {
            self.drag = Some(PathDrag::new(player, cell));
        }
    }

    fn pointer_moved(&mut self, board: BoardLayout, pos: Vec2) {
        let Some(drag) = &mut self.drag else {
            return;
        };
        let Some(cell) = board.screen_to_cell(pos) else {
            return;
        };
        let grid = &self.game.state.grid;
        // Only walls are undrawable; anything else (planks included)
        // might be gone by the time the player gets there.
        drag.extend_to(cell, |p| grid.at(p) == Cell::Wall);
    }

    /// Feed the next move from each player's active path into this frame's
    /// actions. Manual input for a player abandons that player's path, as
    /// does a wall ahead or the player having strayed off the path.
    fn feed_path_moves(&mut self, player_actions: &mut EnumMap<Player, Option<PlayerInput>>) {
        for (player, follower) in self.paths.iter_mut() {
            let Some(active) = follower else {
                continue;
            };
            if player_actions[player].is_some() {
                *follower = None;
                continue;
            }
            if self.game.is_animating() || self.pending[player].is_some() {
                continue;
            }
            let next = self
                .game
                .state
                .player_position(player)
                .map(|pos| (pos, active.next_move(pos)));
            if let Some((pos, NextMove::Move(dir))) = next
                && !self.game.state.grid.at(pos + dir.delta()).blocks_player()
            {
                player_actions[player] = Some(PlayerInput {
                    action: Action::Move(dir),
                    synced: false,
                    fresh: true,
                });
            } else {
                *follower = None;
            }
        }
    }

    /// Advance the grace period timer whenever any pending action exists.
    fn advance_pending_timer(&mut self, dt: f32) {
        let player_count = self.game.state.player_count();
        let any_pending = self
            .pending
            .values()
            .take(player_count)
            .any(|p| p.is_some());
        if any_pending {
            self.pending_timer += dt;
        } else {
            self.pending_timer = 0.0;
        }
    }

    /// Try to execute pending actions if the trigger condition is met.
    fn try_execute_pending(&mut self) {
        if self.game.is_animating() {
            return;
        }
        let player_count = self.game.state.player_count();
        if player_count == 0 {
            return;
        }
        let players = &[Player::Player1, Player::Player2][..player_count];
        let any_pending = players.iter().any(|&p| self.pending[p].is_some());
        let all_pending = players.iter().all(|&p| self.pending[p].is_some());
        if !any_pending {
            return;
        }
        let any_synced = players
            .iter()
            .any(|&p| self.pending[p].is_some_and(|pa| pa.synced));
        if !all_pending {
            // Explicitly synced: wait indefinitely for all players
            if any_synced {
                return;
            }
            // In 2-player, grace period for natural simultaneous input
            if player_count > 1 && self.pending_timer < SYNC_GRACE_PERIOD {
                return;
            }
        }
        self.pending_timer = 0.0;
        let actions: Vec<Action> = players
            .iter()
            .map(|&p| {
                self.pending[p]
                    .take()
                    .map(|pa| pa.action)
                    .unwrap_or(Action::Stall)
            })
            .collect();
        self.restart_armed = None;
        self.game.try_begin_actions(actions);
    }

    /// Ghosts for explicitly synced (shift-held) preregistered moves: each
    /// shows the player translucently where it will end up (in place if the
    /// move is blocked).
    fn pending_ghosts(&self) -> Vec<(Position, Cell)> {
        self.pending
            .iter()
            .filter_map(|(player, pending)| {
                let Some(PendingAction {
                    action: Action::Move(dir),
                    synced: true,
                    ..
                }) = *pending
                else {
                    return None;
                };
                let pos = self.game.state.player_position(player)?;
                let dest = pos + dir.delta();
                let dest = if self.game.state.grid.at(dest).blocks_player() {
                    pos
                } else {
                    dest
                };
                Some((dest, Cell::Player(player, dir)))
            })
            .collect()
    }

    /// Paths to draw: the one being dragged out, plus what's left of each
    /// player's followed path.
    fn path_overlays(&self) -> Vec<(Player, Vec<Position>)> {
        let drag = self.drag.iter().map(|d| (d.player, d.cells.clone()));
        let following = self.paths.iter().filter_map(|(player, follower)| {
            let pos = self.game.state.player_position(player)?;
            Some((player, follower.as_ref()?.preview(pos)))
        });
        drag.chain(following).collect()
    }

    fn update_outcome(&mut self, ctx: &mut Ctx, layout: &ScreenLayout) {
        match (self.outcome(), self.ended) {
            (Some(state), None) => {
                self.ended = Some(self.time);
                if state == PlayState::Won {
                    if self.progress_before.is_none() {
                        self.progress_before = Some(ctx.progress.clone());
                    }
                    ctx.progress.complete(self.level.name);
                    ctx.audio.play(Sfx::Win);
                    self.confetti(layout);
                } else {
                    ctx.audio.play(Sfx::Lose);
                }
            }
            (None, Some(_)) => self.ended = None,
            _ => {}
        }
    }

    fn update_dialogue(&mut self, ctx: &mut Ctx, dt: f32) {
        let note = self
            .game
            .state
            .standing_on_note()
            .map(|(player, note)| (player, note.resolve(ctx.hints, ctx.gamepads)))
            .filter(|(_, text)| !text.is_empty());
        match note {
            Some((player, text)) if text != self.dialogue.text => {
                self.dialogue = Dialogue {
                    text: text.to_string(),
                    player: Some(player),
                    shown: 0.0,
                };
            }
            Some(_) => {
                let before = self.dialogue.shown as usize;
                self.dialogue.shown += TYPE_SPEED * dt;
                let now = self.dialogue.shown as usize;
                let len = self.dialogue.text.chars().count();
                if now > before && before < len && before.is_multiple_of(3) {
                    ctx.audio.play_at(Sfx::TextBlip, 0.55);
                }
            }
            None => self.dialogue = Dialogue::default(),
        }
    }

    pub(crate) fn draw(&self, ctx: &Ctx, layout: &ScreenLayout) {
        let sprites = ctx.sprites;
        let board = self.board_layout(layout);
        draw_backdrop(&self.palette, layout.main.center());
        let paths = self.path_overlays();
        let ghosts = self.pending_ghosts();
        board::draw(
            sprites,
            board,
            &BoardView {
                game: &self.game,
                facings: &self.facings,
                fx: &self.fx,
                palette: &self.palette,
                time: self.time,
                paths: &paths,
                ghosts: &ghosts,
            },
        );
        self.draw_intro(sprites, board, layout.s);
        if self.restart_pending() {
            self.draw_restart_prompt(ctx, board, layout.s);
        }
        self.hud(ctx, layout).draw(sprites, self, ctx, layout);
        if self.card_visible() {
            self.draw_outcome_card(ctx, layout);
        }
    }

    /// Whether a HUD action can be used right now.
    fn action_enabled(&self, action: UiAction) -> bool {
        match action {
            UiAction::Undo | UiAction::Restart => self.game.state.history.len() > 1,
            UiAction::Wait => self.play_state() == PlayState::Playing,
            UiAction::Menu | UiAction::Continue | UiAction::Solution => true,
        }
    }

    /// Whether restart was pressed once and is waiting for confirmation.
    fn restart_pending(&self) -> bool {
        self.restart_armed
            .is_some_and(|t| self.time - t < RESTART_CONFIRM)
    }
}

/// Behind the board: a soft pool of light in the region's colors.
pub(crate) fn draw_backdrop(palette: &Palette, center: Vec2) {
    clear_background(palette.backdrop.1);
    let r = vec2(screen_width(), screen_height()).length() * 0.62;
    radial_gradient(center, r, palette.backdrop.0, palette.backdrop.1);
}
