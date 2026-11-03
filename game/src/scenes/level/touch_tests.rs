use super::*;
use crate::direction::Dir4;
use crate::grid::Grid;

fn scene_from_csv(csv: &str) -> LevelScene {
    let mut scene = LevelScene::new(&crate::levels::all()[0]);
    scene.game = Game::new(Grid::from_csv(csv));
    scene.after_rewind();
    scene
}

fn player_input(player: Player, action: Option<PlayerInput>) -> FrameInput {
    let mut players = EnumMap::default();
    players[player] = action;
    FrameInput {
        dt: 0.02,
        meta: Vec::new(),
        players,
        nav: None,
        pointer: Vec::new(),
        wheel: 0.0,
        any: action.is_some_and(|input| input.ready),
    }
}

#[test]
fn touch_repeat_survives_the_second_hero_grace_period_without_requeueing() {
    let mut scene = scene_from_csv("▲,.,.,.\n.,.,.,.\n△,.,.,.\n#,#,#,#\nR,.,.,.");
    let repeated = PlayerInput {
        action: Action::Move(Dir4::East),
        synced: false,
        fresh: false,
        ready: true,
    };
    scene.queue_player_input(&player_input(Player::Player1, Some(repeated)), false);
    scene.try_execute_pending();
    assert!(scene.game.state.action_history.is_empty());

    let waiting = player_input(
        Player::Player1,
        Some(PlayerInput {
            ready: false,
            ..repeated
        }),
    );
    for _ in 0..20 {
        scene.queue_player_input(&waiting, false);
        scene.advance_pending_timer(waiting.dt);
        let _ = scene.game.animate(waiting.dt);
        scene.try_execute_pending();
    }
    assert_eq!(
        scene.game.state.action_history,
        vec![vec![Action::Move(Dir4::East), Action::Stall]]
    );
    assert!(scene.pending.values().all(Option::is_none));
}

#[test]
fn touch_repeat_waits_for_animation_but_release_cancels_the_buffered_repeat() {
    for release in [false, true] {
        let mut scene = scene_from_csv("▲,.,.,.,.\n#,#,#,#,#\nR,.,.,.,.");
        scene.game.try_begin_actions(vec![Action::Move(Dir4::East)]);
        assert!(scene.game.is_animating());
        let repeated = PlayerInput {
            action: Action::Move(Dir4::East),
            synced: false,
            fresh: false,
            ready: true,
        };
        scene.queue_player_input(&player_input(Player::Player1, Some(repeated)), false);
        let waiting = player_input(
            Player::Player1,
            (!release).then_some(PlayerInput {
                ready: false,
                ..repeated
            }),
        );
        for _ in 0..30 {
            scene.queue_player_input(&waiting, false);
            scene.advance_pending_timer(waiting.dt);
            let _ = scene.game.animate(waiting.dt);
            scene.try_execute_pending();
        }
        assert_eq!(
            scene.game.state.action_history.len(),
            if release { 1 } else { 2 }
        );
        assert!(scene.pending.values().all(Option::is_none));
    }
}

#[test]
fn touch_repeat_cooldown_keeps_a_fresh_synchronized_move_queued() {
    let mut scene = scene_from_csv("▲,.,.\n.,.,.\n△,.,.");
    let fresh = PlayerInput {
        action: Action::Move(Dir4::East),
        synced: true,
        fresh: true,
        ready: true,
    };
    scene.queue_player_input(&player_input(Player::Player1, Some(fresh)), false);
    scene.queue_player_input(
        &player_input(
            Player::Player1,
            Some(PlayerInput {
                fresh: false,
                ready: false,
                ..fresh
            }),
        ),
        false,
    );
    scene.queue_player_input(&player_input(Player::Player1, None), false);
    scene.advance_pending_timer(1.0);
    scene.try_execute_pending();
    assert!(scene.pending[Player::Player1].is_some_and(|input| input.fresh && input.synced));
    assert!(scene.game.state.action_history.is_empty());
}

#[test]
fn touch_actions_follow_moves_and_undo() {
    let mut scene = scene_from_csv("▲,.,.");
    let initial = scene.touch_state();
    assert_eq!(initial.screen, TouchScreen::Playing);
    assert_eq!(initial.players, 1);
    assert_eq!(initial.actions, touch_ui::DIRECTION | touch_ui::WAIT);

    assert!(scene.game.apply_action(Action::Move(Dir4::East)));
    assert_eq!(
        scene.touch_state().actions,
        touch_ui::DIRECTION | touch_ui::WAIT | touch_ui::UNDO | touch_ui::RESTART
    );
    scene.game.undo();
    scene.after_rewind();
    assert_eq!(scene.touch_state(), initial);
}

#[test]
fn touch_dock_keeps_two_heroes_after_a_fatal_turn() {
    let mut scene = scene_from_csv("►,▷");
    let before = scene.touch_state();
    assert_eq!(before.players, 2);
    scene
        .game
        .begin_actions(&[Action::Move(Dir4::East), Action::Stall]);
    assert!(scene.game.is_animating());
    assert!(!scene.card_visible());
    let lost = scene.touch_state();
    assert_eq!(lost.screen, TouchScreen::Lost);
    assert_eq!(lost.room, before.room);
    assert_eq!(lost.players, 2);
    assert_eq!(scene.game.state.grid.find_players().len(), 1);
    assert_eq!(lost.actions, touch_ui::UNDO | touch_ui::RESTART);

    scene.game.undo();
    scene.after_rewind();
    assert_eq!(scene.touch_state(), before);
}

#[test]
fn touch_wait_stops_as_soon_as_the_winning_turn_resolves() {
    let mut scene = scene_from_csv("►,R");
    scene.game.begin_actions(&[Action::Move(Dir4::East)]);
    assert!(scene.game.is_animating());
    assert!(!scene.card_visible());
    let state = scene.touch_state();
    assert_eq!(state.screen, TouchScreen::Won);
    assert_eq!(state.actions, touch_ui::UNDO | touch_ui::RESTART);
    // Continue remains a deliberate tap on the game's outcome card.
    assert_eq!(state.actions & touch_ui::CONFIRM, 0);
}

#[test]
fn interrupting_touch_clears_buffered_moves_but_preserves_the_current_turn() {
    let mut scene = scene_from_csv(".,.,.\n.,▲,.\n.,.,.");
    scene.game.begin_actions(&[Action::Move(Dir4::East)]);
    scene
        .game
        .try_begin_actions(vec![Action::Move(Dir4::South)]);
    scene.pending[Player::Player1] = Some(PendingAction {
        action: Action::Move(Dir4::West),
        synced: true,
        fresh: true,
    });
    scene.pending_timer = 1.0;
    scene.restart_armed = Some(scene.time);
    let resolved = scene.game.state.grid.to_csv();
    let history = scene.game.state.action_history.clone();
    assert!(scene.game.is_animating());
    assert!(scene.game.state.queued_actions.is_some());

    scene.cancel_pending_input();

    assert!(scene.pending.values().all(Option::is_none));
    assert_eq!(scene.pending_timer, 0.0);
    assert!(scene.game.state.queued_actions.is_none());
    assert!(!scene.restart_pending());
    assert!(scene.game.is_animating());
    assert_eq!(scene.game.state.grid.to_csv(), resolved);
    assert_eq!(scene.game.state.action_history, history);
    while scene.game.is_animating() {
        let _ = scene.game.animate(1.0);
    }
    assert_eq!(scene.game.state.grid.to_csv(), resolved);
    assert_eq!(scene.game.state.action_history, history);
}

#[test]
fn room_tips_do_not_change_touch_dock_state() {
    let mut scene = scene_from_csv("▲,.,△");
    let before = scene.touch_state();
    scene.dialogue = Dialogue {
        text: "A room tip that appears while the puzzle stays in place.".to_string(),
        player: Some(Player::Player2),
        shown: 12.0,
    };
    assert_eq!(scene.touch_state(), before);
    scene.dialogue = Dialogue::default();
    assert_eq!(scene.touch_state(), before);
}
