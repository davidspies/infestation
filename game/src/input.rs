use enum_map::{Enum, EnumMap};
use macroquad::input::utils::{register_input_subscriber, repeat_all_miniquad_input};
use macroquad::miniquad;
use macroquad::prelude::*;
use quad_gamepad::{ControllerType, GamepadAxis, GamepadButton, GamepadContext};

use crate::direction::Dir4;
use crate::game::Action;
use crate::grid::Player;

#[derive(Debug, Clone, Copy, Enum)]
enum Gamepad {
    Gamepad1,
    Gamepad2,
}

impl Gamepad {
    fn index(self) -> usize {
        match self {
            Gamepad::Gamepad1 => 0,
            Gamepad::Gamepad2 => 1,
        }
    }
}

const REPEAT_DELAY: f32 = 0.2;
const STICK_THRESHOLD: f32 = 0.5;
/// How far the mouse may move between press and release for it to still
/// count as a click.
const CLICK_SLOP: f32 = 30.0;

/// [0]=arrows, [1]=WASD, [2..4]=gamepads, [4..6]=touch heroes.
const NUM_SOURCES: usize = 6;

const ARROW_SYNC: KeyCode = KeyCode::RightShift;
const WASD_SYNC: KeyCode = KeyCode::LeftShift;

/// Shared with js/touch-controls.js. Touch has its own held state and fresh
/// edges: it never impersonates or releases a physical keyboard key.
#[derive(Clone, Copy, Default)]
struct TouchInput {
    down: u32,
    pressed: u32,
}

const TOUCH_WAIT: u32 = 1 << 10;
const TOUCH_UNDO: u32 = 1 << 11;
const TOUCH_RESTART: u32 = 1 << 12;
const TOUCH_EXIT: u32 = 1 << 13;
const TOUCH_CONFIRM: u32 = 1 << 14;

impl TouchInput {
    fn from_snapshot(snapshot: u32) -> Self {
        Self {
            down: snapshot & 0x7fff,
            pressed: (snapshot >> 16) & 0x7fff,
        }
    }

    fn down(self, mask: u32) -> bool {
        self.down & mask != 0
    }

    fn pressed(self, mask: u32) -> bool {
        self.pressed & mask != 0
    }

    fn direction_mask(player: Player, dir: Dir4) -> u32 {
        let offset = if player == Player::Player1 { 0 } else { 4 };
        1 << (offset
            + match dir {
                Dir4::North => 0,
                Dir4::South => 1,
                Dir4::West => 2,
                Dir4::East => 3,
            })
    }

    fn synced(self, player: Player) -> bool {
        self.down(if player == Player::Player1 {
            1 << 8
        } else {
            1 << 9
        })
    }

    #[must_use]
    fn poll_direction(
        self,
        player: Player,
        held: &mut EnumMap<Dir4, f32>,
        dt: f32,
        repeat_delay: f32,
    ) -> Option<PlayerInput> {
        let mut result = None;
        for (dir, held) in held.iter_mut() {
            let mask = Self::direction_mask(player, dir);
            let down = self.down(mask);
            let fresh = self.pressed(mask);
            update_input_suppression(down, fresh, held);
            if !down {
                *held = 0.0;
                continue;
            }
            if *held == SUPPRESSED {
                continue;
            }
            let ready = if fresh {
                *held = 0.0;
                true
            } else {
                *held += dt;
                if *held >= repeat_delay {
                    // A delayed frame emits only one repeat. Start a full new
                    // interval instead of catching up with a burst of moves.
                    *held = 0.0;
                    true
                } else {
                    false
                }
            };
            result = result.or(Some(PlayerInput {
                action: Action::Move(dir),
                synced: self.synced(player),
                fresh,
                ready,
            }));
        }
        result
    }
}

pub(crate) fn arrow_key(dir: Dir4) -> KeyCode {
    match dir {
        Dir4::North => KeyCode::Up,
        Dir4::South => KeyCode::Down,
        Dir4::East => KeyCode::Right,
        Dir4::West => KeyCode::Left,
    }
}

pub(crate) fn wasd_key(dir: Dir4) -> KeyCode {
    match dir {
        Dir4::North => KeyCode::W,
        Dir4::South => KeyCode::S,
        Dir4::East => KeyCode::D,
        Dir4::West => KeyCode::A,
    }
}

fn dpad_button(dir: Dir4) -> GamepadButton {
    match dir {
        Dir4::North => GamepadButton::DPadUp,
        Dir4::South => GamepadButton::DPadDown,
        Dir4::East => GamepadButton::DPadRight,
        Dir4::West => GamepadButton::DPadLeft,
    }
}

/// Which device the player is using, for showing matching button prompts.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub(crate) enum InputHints {
    #[default]
    Keyboard,
    Controller(ControllerType),
    Touch,
}

/// A player action with metadata from input polling.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PlayerInput {
    pub(crate) action: Action,
    pub(crate) synced: bool,
    /// Whether this was triggered by a fresh key press (not a held-repeat).
    pub(crate) fresh: bool,
    /// A held direction remains present between touch repeat ticks. Only a
    /// ready input may enqueue a new move; physical inputs are always ready.
    pub(crate) ready: bool,
}

impl PlayerInput {
    /// Wait applies to both heroes unless they have a move ready this frame.
    pub(crate) fn add_wait(actions: &mut EnumMap<Player, Option<Self>>, fresh: bool) {
        let stall = Self {
            action: Action::Stall,
            synced: false,
            fresh,
            ready: true,
        };
        for (_, action) in actions.iter_mut() {
            if action.is_none_or(|input| !input.ready) {
                *action = Some(stall);
            }
        }
    }
}

/// A meta input action (not per-player).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum MetaInput {
    Undo,
    Restart,
    Exit,
    /// Confirm action from a specific player.
    Confirm(Player),
}

/// The left mouse button pressed, dragged or released.
#[derive(Debug, Clone, Copy)]
pub(crate) enum PointerEvent {
    Down(Vec2),
    Moved(Vec2),
    Up { start: Vec2, end: Vec2 },
}

impl PointerEvent {
    /// Where a click landed: a release near where the press began.
    pub(crate) fn click(self) -> Option<Vec2> {
        let PointerEvent::Up { start, end } = self else {
            return None;
        };
        (start.distance(end) < CLICK_SLOP).then_some(start)
    }
}

/// Sums a frame's mouse wheel events, replayed from miniquad's input
/// subscriber (`mouse_wheel()` keeps only the last).
#[derive(Default)]
struct WheelCollector {
    /// Vertical wheel movement, in the platform's units.
    wheel: f32,
}

impl miniquad::EventHandler for WheelCollector {
    fn update(&mut self) {}

    fn draw(&mut self) {}

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        self.wheel += y;
    }
}

/// Wheel units per notch: browsers report pixels, desktops whole notches.
const WHEEL_UNITS_PER_NOTCH: f32 = if cfg!(target_arch = "wasm32") {
    100.0
} else {
    1.0
};

/// Pointer input for one frame.
pub(crate) struct PointerInput {
    pub(crate) events: Vec<PointerEvent>,
    /// Mouse wheel (or trackpad) scrolling, in notches; positive scrolls up.
    pub(crate) wheel: f32,
}

/// Tracks held state for input repeat, and mouse presses.
pub(crate) struct InputState {
    /// Per-source directional held timers; physical and touch sources are separate.
    held_move: [EnumMap<Dir4, f32>; NUM_SOURCES],
    touch: TouchInput,
    /// Stall held timer for spacebar.
    held_stall_space: f32,
    /// Stall held timers per gamepad.
    held_stall_gp: EnumMap<Gamepad, f32>,
    /// Meta action held timers.
    held_undo: f32,
    held_confirm: EnumMap<Player, f32>,
    /// Per-gamepad analog stick edge detection.
    stick_active: EnumMap<Gamepad, EnumMap<Dir4, bool>>,
    /// Directional held timers for menu/map navigation, from any source.
    held_nav: EnumMap<Dir4, f32>,
    /// Where the left mouse button went down, while it's held.
    mouse_start: Option<Vec2>,
    wheel_subscriber: usize,
    /// The device most recently used.
    hints: InputHints,
    /// Whether any input has been seen yet (before that, hints are guessed).
    used_any: bool,
}

impl InputState {
    pub(crate) fn new() -> Self {
        Self {
            held_move: Default::default(),
            touch: TouchInput::default(),
            held_stall_space: 0.0,
            held_stall_gp: EnumMap::default(),
            held_undo: 0.0,
            held_confirm: EnumMap::default(),
            stick_active: EnumMap::default(),
            held_nav: EnumMap::default(),
            mouse_start: None,
            wheel_subscriber: register_input_subscriber(),
            hints: InputHints::Keyboard,
            used_any: false,
        }
    }

    /// The device the player last used. Before any input, a connected
    /// controller if there is one.
    pub(crate) fn hints(&self, gamepad: &GamepadContext) -> InputHints {
        #[cfg(target_arch = "wasm32")]
        match crate::touch_ui::input_mode() {
            crate::touch_ui::InputMode::Touch => return InputHints::Touch,
            crate::touch_ui::InputMode::Keyboard => return InputHints::Keyboard,
            crate::touch_ui::InputMode::Controller => {}
        }
        match self.hints {
            InputHints::Keyboard if !self.used_any => {
                if let Some(gp) = gamepad.gamepad(0)
                    && gp.is_connected()
                {
                    InputHints::Controller(gp.controller_type())
                } else {
                    InputHints::Keyboard
                }
            }
            hints => hints,
        }
    }

    /// Note which device was used this frame.
    pub(crate) fn track_device(&mut self, gamepad: &GamepadContext) {
        // The browser chooses the instruction mode; keep the last real pad
        // family for its matching glyphs. Mode never determines input ownership.
        #[cfg(not(target_arch = "wasm32"))]
        if get_last_key_pressed().is_some() || is_mouse_button_pressed(MouseButton::Left) {
            self.hints = InputHints::Keyboard;
            self.used_any = true;
        }
        for index in 0..2 {
            let Some(gp) = gamepad.gamepad(index) else {
                continue;
            };
            let pressed = ALL_BUTTONS.iter().any(|&b| gp.is_button_pressed(b));
            let stick = [GamepadAxis::LeftX, GamepadAxis::LeftY]
                .iter()
                .any(|&a| gp.axis(a).abs() > STICK_THRESHOLD);
            if gp.is_connected() && (pressed || stick) {
                self.hints = InputHints::Controller(gp.controller_type());
                self.used_any = true;
            }
        }
    }

    /// Consume the browser's held state and fresh touch edges once per frame,
    /// including frames blocked by a page overlay.
    pub(crate) fn sample_touch_input(&mut self, snapshot: u32) {
        self.touch = TouchInput::from_snapshot(snapshot);
    }

    /// Reset held timers, suppressing carried-over holds until release or a fresh press.
    /// This prevents key presses from "carrying over" across level transitions.
    pub(crate) fn reset(&mut self) {
        // A cancelled browser touch may leave its emulated mouse button down.
        // Forget the old gesture so it cannot resume dragging after an interruption.
        self.mouse_start = None;
        for source in &mut self.held_move {
            for (_, held) in source.iter_mut() {
                *held = SUPPRESSED;
            }
        }
        self.held_stall_space = SUPPRESSED;
        for (_, held) in self.held_stall_gp.iter_mut() {
            *held = SUPPRESSED;
        }
        self.held_undo = SUPPRESSED;
        for (_, held) in self.held_confirm.iter_mut() {
            *held = SUPPRESSED;
        }
        for (_, held) in self.held_nav.iter_mut() {
            *held = SUPPRESSED;
        }
        // Set all stick directions as active so edge-detection won't fire until released.
        for (_, dirs) in self.stick_active.iter_mut() {
            for (_, active) in dirs.iter_mut() {
                *active = true;
            }
        }
    }

    /// Poll meta actions (undo, restart, exit, confirm) from keyboard + any gamepad.
    pub(crate) fn poll_meta_inputs(&mut self, gamepad: &GamepadContext, dt: f32) -> Vec<MetaInput> {
        let mut inputs = Vec::new();

        // Restart (R / LB / LT on any gamepad)
        if is_key_pressed(KeyCode::R)
            || self.touch.pressed(TOUCH_RESTART)
            || any_gp_pressed_multi(
                gamepad,
                &[GamepadButton::LeftShoulder, GamepadButton::LeftTrigger],
            )
        {
            inputs.push(MetaInput::Restart);
        }

        // Undo with repeat (U / X / Y on any gamepad)
        let undo_down = is_key_down(KeyCode::U)
            || self.touch.down(TOUCH_UNDO)
            || any_gp_down_multi(gamepad, &[GamepadButton::West, GamepadButton::North]);
        let undo_pressed = is_key_pressed(KeyCode::U)
            || self.touch.pressed(TOUCH_UNDO)
            || any_gp_pressed_multi(gamepad, &[GamepadButton::West, GamepadButton::North]);
        if input_repeat(undo_down, undo_pressed, &mut self.held_undo, dt) {
            inputs.push(MetaInput::Undo);
        }

        // Per-player confirm with repeat: Space/Enter → both, gp0 → P1, gp1 → P2
        let space_down = is_key_down(KeyCode::Space)
            || is_key_down(KeyCode::Enter)
            || self.touch.down(TOUCH_WAIT | TOUCH_CONFIRM);
        let space_pressed = is_key_pressed(KeyCode::Space)
            || is_key_pressed(KeyCode::Enter)
            || self.touch.pressed(TOUCH_WAIT | TOUCH_CONFIRM);
        let confirm_sources: [(bool, bool, Player); 4] = [
            (space_down, space_pressed, Player::Player1),
            (space_down, space_pressed, Player::Player2),
            (
                gp_btn_down_multi(
                    gamepad,
                    Gamepad::Gamepad1.index(),
                    &[GamepadButton::South, GamepadButton::East],
                ),
                gp_btn_pressed_multi(
                    gamepad,
                    Gamepad::Gamepad1.index(),
                    &[GamepadButton::South, GamepadButton::East],
                ),
                Player::Player1,
            ),
            (
                gp_btn_down_multi(
                    gamepad,
                    Gamepad::Gamepad2.index(),
                    &[GamepadButton::South, GamepadButton::East],
                ),
                gp_btn_pressed_multi(
                    gamepad,
                    Gamepad::Gamepad2.index(),
                    &[GamepadButton::South, GamepadButton::East],
                ),
                Player::Player2,
            ),
        ];
        for (down, pressed, player) in confirm_sources {
            if input_repeat(down, pressed, &mut self.held_confirm[player], dt) {
                inputs.push(MetaInput::Confirm(player));
            }
        }

        // Exit (Escape / Start on any gamepad)
        if is_key_pressed(KeyCode::Escape)
            || self.touch.pressed(TOUCH_EXIT)
            || any_gp_pressed(gamepad, GamepadButton::Start)
        {
            inputs.push(MetaInput::Exit);
        }

        inputs
    }

    /// Poll all input sources and return per-player actions.
    ///
    /// With one player, every source controls them. With two, WASD and the
    /// second controller are P2's, and the arrow keys are P1's unless the
    /// first controller is connected (then they're P2's).
    pub(crate) fn poll_player_actions(
        &mut self,
        gamepad: &GamepadContext,
        dt: f32,
        player_count: usize,
        touch_repeat_delay: f32,
    ) -> EnumMap<Player, Option<PlayerInput>> {
        let controller_connected = gamepad
            .gamepad(Gamepad::Gamepad1.index())
            .is_some_and(|g| g.is_connected());
        let (arrow_player, second) = keyboard_players(player_count, controller_connected);

        let mut result: EnumMap<Player, Option<PlayerInput>> = EnumMap::default();

        // Source 0: Arrow keys → arrow_player
        if let Some((action, fresh)) = poll_keys_dir(arrow_key, &mut self.held_move[0], dt) {
            let synced = is_key_down(ARROW_SYNC);
            result[arrow_player] = Some(PlayerInput {
                action,
                synced,
                fresh,
                ready: true,
            });
        }

        // Source 1: WASD → P2
        if let Some((action, fresh)) = poll_keys_dir(wasd_key, &mut self.held_move[1], dt) {
            let synced = is_key_down(WASD_SYNC);
            result[second] = Some(PlayerInput {
                action,
                synced,
                fresh,
                ready: true,
            });
        }

        // Each touch direction identifies its hero even with a controller
        // connected. It cannot change ownership of a physical key already held.
        for (source, player, target) in [
            (4, Player::Player1, Player::Player1),
            (5, Player::Player2, second),
        ] {
            if let Some(action) = self.touch.poll_direction(
                player,
                &mut self.held_move[source],
                dt,
                touch_repeat_delay,
            ) {
                // A touch hold waiting for its next repeat must not mask a
                // ready physical key controlling the same hero.
                if action.ready || result[target].is_none() {
                    result[target] = Some(action);
                }
            }
        }

        // Spacebar or touch Wait stalls both players (not synced).
        if let Some(fresh) = input_held(
            is_key_down(KeyCode::Space) || self.touch.down(TOUCH_WAIT),
            is_key_pressed(KeyCode::Space) || self.touch.pressed(TOUCH_WAIT),
            &mut self.held_stall_space,
            dt,
        ) {
            PlayerInput::add_wait(&mut result, fresh);
        }

        // Source 2: Gamepad 1 → always P1
        if let Some((action, fresh)) = poll_gamepad_action(
            gamepad,
            Gamepad::Gamepad1.index(),
            &mut self.held_move[2],
            &mut self.held_stall_gp[Gamepad::Gamepad1],
            &mut self.stick_active[Gamepad::Gamepad1],
            dt,
        ) {
            let synced = gp_btn_down(
                gamepad,
                Gamepad::Gamepad1.index(),
                GamepadButton::RightShoulder,
            );
            result[Player::Player1] = Some(PlayerInput {
                action,
                synced,
                fresh,
                ready: true,
            });
        }

        // Source 3: Gamepad 2 → P2
        if let Some((action, fresh)) = poll_gamepad_action(
            gamepad,
            Gamepad::Gamepad2.index(),
            &mut self.held_move[3],
            &mut self.held_stall_gp[Gamepad::Gamepad2],
            &mut self.stick_active[Gamepad::Gamepad2],
            dt,
        ) {
            let synced = gp_btn_down(
                gamepad,
                Gamepad::Gamepad2.index(),
                GamepadButton::RightShoulder,
            );
            result[second] = Some(PlayerInput {
                action,
                synced,
                fresh,
                ready: true,
            });
        }

        result
    }

    /// A direction for moving through menus and the map, from any device,
    /// repeating while held.
    pub(crate) fn poll_nav(&mut self, gamepad: &GamepadContext, dt: f32) -> Option<Dir4> {
        let mut result = None;
        for (dir, held) in self.held_nav.iter_mut() {
            let touch_direction = TouchInput::direction_mask(Player::Player1, dir)
                | TouchInput::direction_mask(Player::Player2, dir);
            let down = is_key_down(arrow_key(dir))
                || is_key_down(wasd_key(dir))
                || self.touch.down(touch_direction)
                || (0..2).any(|i| {
                    gp_btn_down(gamepad, i, dpad_button(dir)) || stick_points(gamepad, i, dir)
                });
            let pressed = is_key_pressed(arrow_key(dir))
                || is_key_pressed(wasd_key(dir))
                || self.touch.pressed(touch_direction)
                || (0..2).any(|i| gp_btn_pressed(gamepad, i, dpad_button(dir)));
            if input_repeat_nav(down, pressed, held, dt) {
                result = result.or(Some(dir));
            }
        }
        result
    }

    /// Poll the left mouse button and the wheel.
    pub(crate) fn poll_pointer(&mut self) -> PointerInput {
        let mut collector = WheelCollector::default();
        repeat_all_miniquad_input(&mut collector, self.wheel_subscriber);

        let pos = Vec2::from(mouse_position());
        let mut events = Vec::new();
        if is_mouse_button_pressed(MouseButton::Left) {
            self.mouse_start = Some(pos);
            events.push(PointerEvent::Down(pos));
        } else if let Some(start) = self.mouse_start {
            if is_mouse_button_down(MouseButton::Left) {
                events.push(PointerEvent::Moved(pos));
            } else {
                self.mouse_start = None;
                events.push(PointerEvent::Up { start, end: pos });
            }
        }

        PointerInput {
            events,
            wheel: collector.wheel / WHEEL_UNITS_PER_NOTCH,
        }
    }
}

/// Physical keyboard routing is independent of the displayed input prompts.
fn keyboard_players(player_count: usize, controller_connected: bool) -> (Player, Player) {
    let second = if player_count < 2 {
        Player::Player1
    } else {
        Player::Player2
    };
    let arrows = if controller_connected {
        second
    } else {
        Player::Player1
    };
    (arrows, second)
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

// --- Shared polling functions ---

/// Poll keyboard keys for a directional action. Returns `(action, fresh)`.
fn poll_keys_dir(
    key_for: fn(Dir4) -> KeyCode,
    held_move: &mut EnumMap<Dir4, f32>,
    dt: f32,
) -> Option<(Action, bool)> {
    poll_direction(held_move, dt, |dir| {
        let key = key_for(dir);
        (is_key_down(key), is_key_pressed(key))
    })
}

#[must_use]
fn poll_direction(
    held_move: &mut EnumMap<Dir4, f32>,
    dt: f32,
    state: impl Fn(Dir4) -> (bool, bool),
) -> Option<(Action, bool)> {
    for (dir, held) in held_move.iter_mut() {
        let (down, pressed) = state(dir);
        if let Some(fresh) = input_held(down, pressed, held, dt) {
            return Some((Action::Move(dir), fresh));
        }
    }

    None
}

/// Poll a gamepad for a directional action or stall. Returns `(action, fresh)`.
fn poll_gamepad_action(
    gp: &GamepadContext,
    index: usize,
    held_move: &mut EnumMap<Dir4, f32>,
    held_stall: &mut f32,
    stick_active: &mut EnumMap<Dir4, bool>,
    dt: f32,
) -> Option<(Action, bool)> {
    // D-pad with repeat
    for (dir, held) in held_move.iter_mut() {
        let btn = dpad_button(dir);
        let down = gp_btn_down(gp, index, btn);
        let pressed = gp_btn_pressed(gp, index, btn);
        if let Some(fresh) = input_held(down, pressed, held, dt) {
            return Some((Action::Move(dir), fresh));
        }
    }

    // Left analog stick (edge-triggered, no repeat) — always fresh
    let stick_y = gp_stick_value(gp, index, GamepadAxis::LeftY);
    let stick_x = gp_stick_value(gp, index, GamepadAxis::LeftX);

    let stick_dir = if stick_y.abs() > stick_x.abs() {
        if stick_y > 0.0 {
            Some(Dir4::North)
        } else if stick_y < 0.0 {
            Some(Dir4::South)
        } else {
            None
        }
    } else if stick_x < 0.0 {
        Some(Dir4::West)
    } else if stick_x > 0.0 {
        Some(Dir4::East)
    } else {
        None
    };

    let mut result = None;
    if let Some(dir) = stick_dir
        && !stick_active[dir]
    {
        result = Some((Action::Move(dir), true));
    }
    for (dir, active) in stick_active.iter_mut() {
        *active = stick_dir == Some(dir);
    }
    if result.is_some() {
        return result;
    }

    // Stall (South/East buttons)
    let stall_down =
        gp_btn_down(gp, index, GamepadButton::South) || gp_btn_down(gp, index, GamepadButton::East);
    let stall_pressed = gp_btn_pressed(gp, index, GamepadButton::South)
        || gp_btn_pressed(gp, index, GamepadButton::East);
    if let Some(fresh) = input_held(stall_down, stall_pressed, held_stall, dt) {
        return Some((Action::Stall, fresh));
    }

    None
}

// --- Gamepad helpers (parameterized by index) ---

fn gp_btn_down(gp: &GamepadContext, index: usize, btn: GamepadButton) -> bool {
    gp.gamepad(index).is_some_and(|g| g.is_button_down(btn))
}

fn gp_btn_pressed(gp: &GamepadContext, index: usize, btn: GamepadButton) -> bool {
    gp.gamepad(index).is_some_and(|g| g.is_button_pressed(btn))
}

fn gp_stick_value(gp: &GamepadContext, index: usize, axis: GamepadAxis) -> f32 {
    let v = gp.gamepad(index).map(|g| g.axis(axis)).unwrap_or(0.0);
    if v.abs() > STICK_THRESHOLD { v } else { 0.0 }
}

/// Check if a button is down on any gamepad (0 or 1).
fn any_gp_down(gp: &GamepadContext, btn: GamepadButton) -> bool {
    (0..2).any(|i| gp_btn_down(gp, i, btn))
}

/// Check if a button was pressed on any gamepad (0 or 1).
fn any_gp_pressed(gp: &GamepadContext, btn: GamepadButton) -> bool {
    (0..2).any(|i| gp_btn_pressed(gp, i, btn))
}

fn gp_btn_down_multi(gp: &GamepadContext, index: usize, btns: &[GamepadButton]) -> bool {
    btns.iter().any(|&btn| gp_btn_down(gp, index, btn))
}

fn gp_btn_pressed_multi(gp: &GamepadContext, index: usize, btns: &[GamepadButton]) -> bool {
    btns.iter().any(|&btn| gp_btn_pressed(gp, index, btn))
}

/// Check if any of multiple buttons are down on any gamepad.
fn any_gp_down_multi(gp: &GamepadContext, btns: &[GamepadButton]) -> bool {
    btns.iter().any(|&btn| any_gp_down(gp, btn))
}

/// Check if any of multiple buttons were pressed on any gamepad.
fn any_gp_pressed_multi(gp: &GamepadContext, btns: &[GamepadButton]) -> bool {
    btns.iter().any(|&btn| any_gp_pressed(gp, btn))
}

// --- Utilities ---

const UNDO_REPEAT_RATE: f32 = 0.05;

/// Sentinel value: a carried-over hold is suppressed until release or a fresh press.
const SUPPRESSED: f32 = -1.0;

fn update_input_suppression(down: bool, pressed: bool, held: &mut f32) {
    // A touch cancellation is acknowledged on the next game frame. A new tap
    // may already have arrived by then, without an intervening poll observing
    // the released key. Its fresh edge starts a new repeat delay; an old hold
    // with no new edge must remain suppressed across scene changes.
    if *held == SUPPRESSED && (!down || pressed) {
        *held = 0.0;
    }
}

/// Returns `Some(true)` for a fresh press, `Some(false)` for a held-repeat, `None` for no input.
fn input_held(down: bool, pressed: bool, held: &mut f32, dt: f32) -> Option<bool> {
    update_input_suppression(down, pressed, held);
    if *held == SUPPRESSED {
        return None;
    }
    if down {
        *held += dt;
        if pressed {
            Some(true)
        } else if *held > REPEAT_DELAY {
            Some(false)
        } else {
            None
        }
    } else {
        *held = 0.0;
        None
    }
}

fn input_repeat(down: bool, pressed: bool, held: &mut f32, dt: f32) -> bool {
    update_input_suppression(down, pressed, held);
    if *held == SUPPRESSED {
        return false;
    }
    if down {
        *held += dt;
        pressed || (*held > REPEAT_DELAY && *held % UNDO_REPEAT_RATE < dt)
    } else {
        *held = 0.0;
        false
    }
}

/// Whether gamepad `index`'s left stick points in `dir`.
fn stick_points(gp: &GamepadContext, index: usize, dir: Dir4) -> bool {
    let x = gp_stick_value(gp, index, GamepadAxis::LeftX);
    let y = gp_stick_value(gp, index, GamepadAxis::LeftY);
    match dir {
        Dir4::North => y > x.abs(),
        Dir4::South => -y > x.abs(),
        Dir4::East => x > y.abs(),
        Dir4::West => -x > y.abs(),
    }
}

/// Menus and the map wait longer before repeating than movement does: one
/// step there jumps a whole level, and controller presses tend to be held
/// longer than key taps.
const NAV_REPEAT_DELAY: f32 = 0.4;
const NAV_REPEAT_RATE: f32 = 0.18;

/// Like `input_repeat`, but repeating at a comfortable menu-scrolling rate.
/// Stick deflection counts as a press when it starts.
fn input_repeat_nav(down: bool, pressed: bool, held: &mut f32, dt: f32) -> bool {
    update_input_suppression(down, pressed, held);
    if *held == SUPPRESSED {
        return false;
    }
    if down {
        let first = *held == 0.0;
        *held += dt;
        let repeats_before = ((*held - dt - NAV_REPEAT_DELAY) / NAV_REPEAT_RATE).floor();
        let repeats_now = ((*held - NAV_REPEAT_DELAY) / NAV_REPEAT_RATE).floor();
        pressed || first || (*held > NAV_REPEAT_DELAY && repeats_now > repeats_before)
    } else {
        *held = 0.0;
        false
    }
}

const ALL_BUTTONS: [GamepadButton; 17] = [
    GamepadButton::South,
    GamepadButton::East,
    GamepadButton::West,
    GamepadButton::North,
    GamepadButton::LeftShoulder,
    GamepadButton::RightShoulder,
    GamepadButton::LeftTrigger,
    GamepadButton::RightTrigger,
    GamepadButton::Select,
    GamepadButton::Start,
    GamepadButton::LeftStick,
    GamepadButton::RightStick,
    GamepadButton::DPadUp,
    GamepadButton::DPadDown,
    GamepadButton::DPadLeft,
    GamepadButton::DPadRight,
    GamepadButton::Home,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_heroes_keep_their_identity_with_an_idle_controller_connected() {
        let p1 = TouchInput::direction_mask(Player::Player1, Dir4::East);
        let p2 = TouchInput::direction_mask(Player::Player2, Dir4::North);
        let bits = p1 | p2 | (1 << 9);
        let touch = TouchInput::from_snapshot(bits | (bits << 16));
        let mut p1_held = EnumMap::default();
        let mut p2_held = EnumMap::default();
        let first = touch
            .poll_direction(Player::Player1, &mut p1_held, 1.0 / 60.0, 0.3)
            .unwrap();
        let second = touch
            .poll_direction(Player::Player2, &mut p2_held, 1.0 / 60.0, 0.3)
            .unwrap();
        assert_eq!(first.action, Action::Move(Dir4::East));
        assert!(!first.synced);
        assert!(first.fresh);
        assert_eq!(second.action, Action::Move(Dir4::North));
        assert!(second.synced);
        assert!(second.fresh);
        // A physical arrow still belongs to P2. Showing touch instructions
        // cannot reassign it or merge its held state with the touch P1 source.
        assert_eq!(
            keyboard_players(2, true),
            (Player::Player2, Player::Player2)
        );
    }

    #[test]
    fn physical_keyboard_routing_and_solo_controls_keep_the_original_behavior() {
        assert_eq!(
            keyboard_players(2, true),
            (Player::Player2, Player::Player2)
        );
        assert_eq!(
            keyboard_players(2, false),
            (Player::Player1, Player::Player2)
        );
        for connected in [false, true] {
            assert_eq!(
                keyboard_players(1, connected),
                (Player::Player1, Player::Player1)
            );
        }
    }

    #[test]
    fn releasing_touch_does_not_release_a_held_physical_direction() {
        let mut physical = EnumMap::default();
        let mut touch_held = EnumMap::default();
        let key = |dir| (dir == Dir4::East, false);
        assert!(poll_direction(&mut physical, 0.1, key).is_none());
        let bit = TouchInput::direction_mask(Player::Player1, Dir4::East);
        let touch = TouchInput::from_snapshot(bit | (bit << 16));
        assert!(
            touch
                .poll_direction(Player::Player1, &mut touch_held, 0.1, 0.3)
                .is_some()
        );
        assert!(
            TouchInput::default()
                .poll_direction(Player::Player1, &mut touch_held, 0.1, 0.3)
                .is_none()
        );
        assert_eq!(
            poll_direction(&mut physical, 0.2, key),
            Some((Action::Move(Dir4::East), false))
        );
    }

    #[test]
    fn blocked_touch_edges_are_consumed_but_a_new_press_survives_suppression() {
        let bit = TouchInput::direction_mask(Player::Player1, Dir4::East);
        let mut held = EnumMap::from_fn(|_| SUPPRESSED);
        // The blocking frame consumed the initial edge and suppressed its hold.
        assert!(
            TouchInput::from_snapshot(bit)
                .poll_direction(Player::Player1, &mut held, 1.0, 0.3)
                .is_none()
        );
        // A release and a fresh press may both occur between polls. The
        // explicit edge survives even though the sampled held bit is unchanged.
        let fresh = TouchInput::from_snapshot(bit | (bit << 16));
        let action = fresh
            .poll_direction(Player::Player1, &mut held, 1.0 / 60.0, 0.3)
            .unwrap();
        assert_eq!(action.action, Action::Move(Dir4::East));
        assert!(action.fresh);
        assert!(action.ready);
        let waiting = TouchInput::from_snapshot(bit)
            .poll_direction(Player::Player1, &mut held, 0.1, 0.3)
            .unwrap();
        assert!(!waiting.ready);
        assert!(!waiting.fresh);
    }

    #[test]
    fn touch_repeat_delay_applies_to_the_first_and_every_subsequent_repeat() {
        let bit = TouchInput::direction_mask(Player::Player1, Dir4::East);
        for delay in [0.1, 0.3, 0.6] {
            let mut held = EnumMap::default();
            let first = TouchInput::from_snapshot(bit | (bit << 16))
                .poll_direction(Player::Player1, &mut held, 1.0 / 60.0, delay)
                .unwrap();
            assert!(first.ready && first.fresh);

            let hold = TouchInput::from_snapshot(bit);
            for _ in 0..3 {
                let waiting = hold
                    .poll_direction(Player::Player1, &mut held, delay - 0.01, delay)
                    .unwrap();
                assert!(!waiting.ready && !waiting.fresh);
                let repeated = hold
                    .poll_direction(Player::Player1, &mut held, 0.011, delay)
                    .unwrap();
                assert!(repeated.ready && !repeated.fresh);
            }
        }
    }

    #[test]
    fn a_slow_frame_does_not_leave_touch_repeats_to_catch_up() {
        let bit = TouchInput::direction_mask(Player::Player1, Dir4::East);
        let mut held = EnumMap::default();
        let hold = TouchInput::from_snapshot(bit);
        assert!(
            hold.poll_direction(Player::Player1, &mut held, 3.0, 0.3)
                .unwrap()
                .ready
        );
        assert!(
            !hold
                .poll_direction(Player::Player1, &mut held, 0.29, 0.3)
                .unwrap()
                .ready
        );
        assert!(
            hold.poll_direction(Player::Player1, &mut held, 0.011, 0.3)
                .unwrap()
                .ready
        );
    }

    #[test]
    fn fresh_touch_taps_and_direction_changes_restart_the_repeat_delay() {
        let east = TouchInput::direction_mask(Player::Player1, Dir4::East);
        let north = TouchInput::direction_mask(Player::Player1, Dir4::North);
        let mut held = EnumMap::default();
        held[Dir4::East] = 0.29;

        // A new edge resets the delay even if release and press were sampled
        // together, and even if that frame itself took longer than the delay.
        let fresh = TouchInput::from_snapshot(east | (east << 16))
            .poll_direction(Player::Player1, &mut held, 1.0, 0.3)
            .unwrap();
        assert!(fresh.ready && fresh.fresh);
        assert_eq!(held[Dir4::East], 0.0);

        let changed = TouchInput::from_snapshot(north | (north << 16))
            .poll_direction(Player::Player1, &mut held, 0.05, 0.3)
            .unwrap();
        assert_eq!(changed.action, Action::Move(Dir4::North));
        assert!(changed.ready && changed.fresh);
        assert!(held.values().all(|&time| time == 0.0));

        assert!(
            TouchInput::default()
                .poll_direction(Player::Player1, &mut held, 0.05, 0.3)
                .is_none()
        );
        assert!(held.values().all(|&time| time == 0.0));
    }

    #[test]
    fn physical_movement_still_repeats_each_frame_after_its_original_delay() {
        let mut held = 0.0;
        assert_eq!(input_held(true, true, &mut held, 0.01), Some(true));
        assert_eq!(input_held(true, false, &mut held, 0.18), None);
        assert_eq!(input_held(true, false, &mut held, 0.02), Some(false));
        assert_eq!(input_held(true, false, &mut held, 0.01), Some(false));
        assert_eq!(input_held(true, false, &mut held, 0.01), Some(false));
    }

    /// How many navigation steps holding a direction for `seconds` gives.
    fn nav_steps(seconds: f32) -> usize {
        let dt = 1.0 / 60.0;
        let mut held = 0.0;
        let frames = (seconds / dt).round() as usize;
        (0..frames)
            .filter(|&i| input_repeat_nav(true, i == 0, &mut held, dt))
            .count()
    }

    #[test]
    fn an_ordinary_press_steps_once() {
        assert_eq!(nav_steps(0.1), 1);
        assert_eq!(nav_steps(0.35), 1);
    }

    #[test]
    fn holding_a_direction_keeps_stepping() {
        assert!(nav_steps(1.0) >= 3);
    }

    #[test]
    fn a_fresh_move_after_reset_is_not_swallowed() {
        let dt = 1.0 / 60.0;
        let mut held = SUPPRESSED;
        assert_eq!(input_held(true, false, &mut held, 1.0), None);
        assert_eq!(held, SUPPRESSED);
        // Release and a new press both arrived between two game frames.
        assert_eq!(input_held(true, true, &mut held, dt), Some(true));
        assert_eq!(input_held(true, false, &mut held, dt), None);
    }

    #[test]
    fn a_fresh_undo_or_confirm_after_reset_is_not_swallowed() {
        let dt = 1.0 / 60.0;
        let mut held = SUPPRESSED;
        assert!(!input_repeat(true, false, &mut held, 1.0));
        assert_eq!(held, SUPPRESSED);
        assert!(input_repeat(true, true, &mut held, dt));
        assert!(!input_repeat(true, false, &mut held, dt));
    }

    #[test]
    fn a_fresh_navigation_press_after_reset_is_not_swallowed() {
        let dt = 1.0 / 60.0;
        let mut held = SUPPRESSED;
        assert!(!input_repeat_nav(true, false, &mut held, 1.0));
        assert_eq!(held, SUPPRESSED);
        assert!(input_repeat_nav(true, true, &mut held, dt));
        assert!(!input_repeat_nav(true, false, &mut held, dt));
    }

    #[test]
    fn releasing_a_carried_over_hold_clears_suppression() {
        let dt = 1.0 / 60.0;
        let mut movement = SUPPRESSED;
        let mut repeat = SUPPRESSED;
        let mut navigation = SUPPRESSED;
        assert_eq!(input_held(false, false, &mut movement, dt), None);
        assert!(!input_repeat(false, false, &mut repeat, dt));
        assert!(!input_repeat_nav(false, false, &mut navigation, dt));
        assert_eq!((movement, repeat, navigation), (0.0, 0.0, 0.0));
    }
}
