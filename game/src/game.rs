use std::borrow::BorrowMut;

use serde::{Deserialize, Serialize};

use crate::direction::Dir4;
use crate::grid::{Cell, Grid, NoteText, Player};
use crate::position::Position;

mod animation;
mod cyborg_distance;
mod cyborg_rat;
mod explosion;
mod player;
mod rat;
mod zap;

/// Information about a player for movement resolution.
#[derive(Clone, Copy)]
pub struct PlayerInfo {
    pub pos: Position,
    pub dir: Dir4,
    /// True if the player moved (not just stalled) this turn.
    pub moved: bool,
    pub player: Player,
}

/// Animation phases (moving, zapping, each explosion wave) per second.
const MOVE_SPEED: f32 = 9.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayState {
    Playing,
    GameOver,
    Won,
}

#[derive(Clone, Copy)]
pub(crate) struct Moving {
    pub(crate) cell: Cell,
    pub(crate) from: Position,
    pub(crate) progress: f32,
    pub(crate) to: Position,
}

#[derive(Clone, Copy)]
pub(crate) struct Exploding {
    pub(crate) pos: Position,
    pub(crate) progress: f32,
}

#[derive(Clone, Copy)]
pub(crate) struct Zapping {
    pub(crate) pos: Position,
    pub(crate) progress: f32,
}

/// A notable change while resolving a turn, consumed by the presentation
/// layer for effects and sounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum GameEvent {
    /// A new turn begins; the `Moved` events that follow belong to it.
    Turn,
    /// An entity starts moving from `from` to `to` (equal when it turns in
    /// place or its move is blocked).
    Moved {
        entity: Cell,
        from: Position,
        to: Position,
    },
    /// A moving entity arrived at `pos`, replacing a non-empty cell (which it
    /// killed, destroyed or consumed).
    Arrived {
        pos: Position,
        entity: Cell,
        displaced: Cell,
    },
    /// A moving entity fell into the black hole at `pos`.
    Swallowed { pos: Position, entity: Cell },
    /// Both players tried to enter `pos`, clearing its non-empty contents.
    Contested { pos: Position, cleared: Cell },
    /// The explosive at `pos` detonated, destroying `center` (the explosive
    /// itself, or whatever had entered its cell).
    Exploded { pos: Position, center: Cell },
    /// A blast destroyed `cell` at `pos`.
    Blasted { pos: Position, cell: Cell },
    /// A signalled trigger at `pos` turned into a wall.
    Zapped { pos: Position, digit: u8 },
    /// A zap turned the empty cell at `pos` into a wall.
    WallRaised { pos: Position },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum Action {
    Move(Dir4),
    Stall,
}

impl From<Action> for String {
    fn from(action: Action) -> String {
        match action {
            Action::Move(dir) => serde_plain::to_string(&dir).unwrap(),
            Action::Stall => "stall".to_string(),
        }
    }
}

impl TryFrom<String> for Action {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "stall" => Ok(Action::Stall),
            _ => serde_plain::from_str(&s)
                .map(Action::Move)
                .map_err(|_| format!("Unknown action: {s}")),
        }
    }
}

/// Handles move resolution and animation.
/// Used for both instant resolution and animated playback.
#[derive(Clone)]
pub(crate) struct MoveHandler<G = Grid> {
    /// Grid being modified (also used for rendering during animation).
    pub(crate) grid: G,
    /// A cell both players tried to enter. Players stay put, but the cell's
    /// normal on-enter effects resolve after player movement.
    pub(crate) contested_cell: Option<Position>,
    /// Movement animations in progress.
    pub(crate) moving: Vec<Moving>,
    /// Zap animations in progress.
    pub(crate) zapping: Vec<Zapping>,
    /// Trigger numbers that have been activated and need processing.
    pub(crate) triggered_numbers: Vec<u8>,
    /// Explosion animations in progress.
    pub(crate) exploding: Vec<Exploding>,
    /// Explosions queued for the next wave.
    pub(crate) pending_explosions: Vec<Position>,
    /// Events recorded since they were last drained.
    pub(crate) events: Vec<GameEvent>,
}

impl<G: BorrowMut<Grid>> MoveHandler<G> {
    pub(crate) fn new(grid: G) -> Self {
        Self {
            grid,
            contested_cell: None,
            moving: Vec::new(),
            zapping: Vec::new(),
            triggered_numbers: Vec::new(),
            exploding: Vec::new(),
            pending_explosions: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Check if there's anything to animate.
    pub(crate) fn is_empty(&self) -> bool {
        self.moving.is_empty()
            && self.zapping.is_empty()
            && self.exploding.is_empty()
            && self.triggered_numbers.is_empty()
            && self.pending_explosions.is_empty()
    }

    fn begin_move(&mut self, moving: Moving) {
        let grid = self.grid.borrow_mut();
        *grid.at_mut(moving.from) = Cell::Empty;
        self.moving.push(moving);
        self.events.push(GameEvent::Moved {
            entity: moving.cell,
            from: moving.from,
            to: moving.to,
        });
        let dest_entity = grid.at_mut(moving.to);
        if !matches!(*dest_entity, Cell::BlackHole) {
            // The grid changes will get overwritten when we replace the grid with the previous one.
            // This is just for sequential blocking checks.
            *dest_entity = moving.cell;
        }
    }
}

/// Core game state without animation.
#[derive(Clone)]
pub struct GameState {
    pub(crate) grid: Grid,
    pub(crate) initial_grid: Grid,
    pub(crate) history: Vec<Grid>,
    pub(crate) action_history: Vec<Vec<Action>>,
    pub(crate) queued_actions: Option<Vec<Action>>,
    /// Tracks which player moved last (for note priority). None if no moves yet.
    /// If both moved in sync, this is Some(Player1) (P1 priority).
    pub(crate) last_acting_player: Option<Player>,
}

/// Game wrapper combining state and move handling.
#[derive(Clone)]
pub struct Game {
    pub(crate) state: GameState,
    /// Animation state. When Some, render from handler.grid.
    /// state.grid always has the final resolved state.
    pub(crate) animation: Option<MoveHandler>,
}

impl GameState {
    pub(crate) fn player_count(&self) -> usize {
        Self::count_players(&self.initial_grid)
    }

    pub(crate) fn new(grid: Grid) -> Self {
        Self {
            initial_grid: grid.clone(),
            grid: grid.clone(),
            history: vec![grid],
            action_history: Vec::new(),
            queued_actions: None,
            last_acting_player: None,
        }
    }

    /// Current position of the given player, if it's still on the grid.
    pub(crate) fn player_position(&self, player: Player) -> Option<Position> {
        self.grid
            .find_players()
            .into_iter()
            .find(|p| p.player == player)
            .map(|p| p.pos)
    }

    /// The note a player is standing on, and which player.
    /// Priority: if both players are on notes, prefer whichever moved last.
    /// If both moved in sync, prefer P1.
    pub(crate) fn standing_on_note(&self) -> Option<(Player, &NoteText)> {
        let note_under = |player| {
            let note = self.grid.get_note(self.player_position(player)?)?;
            Some((player, note))
        };
        let p1_note = note_under(Player::Player1);
        let p2_note = note_under(Player::Player2);

        match (p1_note, p2_note) {
            (Some(_), Some(_)) => {
                // Both on notes: use last_acting_player preference (defaults to P1 if None)
                if self.last_acting_player == Some(Player::Player2) {
                    p2_note
                } else {
                    p1_note
                }
            }
            (Some(_), None) => p1_note,
            (None, Some(_)) => p2_note,
            (None, None) => None,
        }
    }

    pub(crate) fn initial_has_rats(&self) -> bool {
        self.initial_grid
            .entries()
            .any(|(_, cell)| matches!(cell, Cell::Rat(_) | Cell::CyborgRat(_)))
    }

    fn count_players(grid: &Grid) -> usize {
        grid.entries()
            .filter(|(_, cell)| matches!(cell, Cell::Player(..)))
            .count()
    }

    /// Compute play state from grid: GameOver if any player died, Won if no rats (and started with rats).
    pub(crate) fn play_state(&self) -> PlayState {
        // GameOver if any player has died
        let initial_player_count = Self::count_players(&self.initial_grid);
        let current_player_count = Self::count_players(&self.grid);
        if current_player_count < initial_player_count {
            return PlayState::GameOver;
        }

        // Check for win condition (no rats left)
        let has_rats = self
            .grid
            .entries()
            .any(|(_, cell)| matches!(cell, Cell::Rat(_) | Cell::CyborgRat(_)));

        if !has_rats && self.initial_has_rats() {
            PlayState::Won
        } else {
            PlayState::Playing
        }
    }
}

impl Game {
    pub(crate) fn new(grid: Grid) -> Self {
        Self {
            state: GameState::new(grid),
            animation: None,
        }
    }

    pub(crate) fn restart(&mut self) {
        self.state.grid = self.state.initial_grid.clone();
        self.state.history = vec![self.state.grid.clone()];
        self.state.action_history.clear();
        self.animation = None;
        self.state.queued_actions = None;
        self.state.last_acting_player = None;
    }

    pub(crate) fn undo(&mut self) {
        if self.state.history.len() > 1 {
            self.state.history.pop();
            self.state.action_history.pop();
            self.state.grid = self.state.history.last().unwrap().clone();
            self.animation = None;
            self.state.queued_actions = None;
            self.state.last_acting_player = None;
        }
    }

    pub(crate) fn is_animating(&self) -> bool {
        self.animation.is_some()
    }

    pub(crate) fn begin_actions(&mut self, actions: &[Action]) {
        let prev_grid = self.state.grid.clone();

        // Handler #1: resolve instantly
        if !self.apply_actions(actions) {
            return;
        }

        // Track which player(s) moved for note priority
        let moving_players: Vec<Player> = [Player::Player1, Player::Player2]
            .into_iter()
            .zip(actions.iter())
            .filter_map(|(player, action)| matches!(action, Action::Move(_)).then_some(player))
            .collect();

        if moving_players.len() == 1 {
            self.state.last_acting_player = Some(moving_players[0]);
        } else {
            // Both moved or neither moved → prefer P1
            self.state.last_acting_player = Some(Player::Player1);
        }

        // Handler #2: for animation
        let mut animator = MoveHandler::new(prev_grid);
        animator.events.push(GameEvent::Turn);
        animator.do_player_moves(actions);

        if !animator.is_empty() {
            self.animation = Some(animator);
        }
    }

    pub(crate) fn try_begin_actions(&mut self, actions: Vec<Action>) {
        if self.is_animating() {
            if self.state.queued_actions.is_none() {
                self.state.queued_actions = Some(actions);
            }
        } else {
            self.begin_actions(&actions);
        }
    }

    /// Apply an input immediately without animation (for editor replay)
    pub fn apply_action(&mut self, m: Action) -> bool {
        self.apply_actions(&[m])
    }

    /// Apply multiple player actions immediately without animation.
    pub fn apply_actions(&mut self, actions: &[Action]) -> bool {
        let play_state = self.state.play_state();

        if play_state != PlayState::Playing {
            return false;
        }

        // Check that at least one player exists and at least one action is provided
        let mut resolver = MoveHandler::new(&mut self.state.grid);
        if resolver.find_players().is_empty() {
            return false;
        }

        resolver.do_player_moves(actions);
        resolver.resolve_all();

        self.state.history.push(self.state.grid.clone());
        self.state.action_history.push(actions.to_vec());

        true
    }

    pub(crate) fn grid_width(&self) -> usize {
        self.state.grid.width()
    }

    pub(crate) fn grid_height(&self) -> usize {
        self.state.grid.height()
    }
}

#[cfg(test)]
mod tests;
