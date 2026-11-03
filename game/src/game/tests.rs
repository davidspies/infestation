use super::*;
use crate::grid::Grid;

fn game_from_csv(csv: &str) -> Game {
    Game::new(Grid::from_csv(csv))
}

fn player_pos(game: &Game) -> Position {
    game.state.grid.find_players()[0].pos
}

fn assert_board_eq(game: &Game, expected: &Grid) {
    assert_eq!(game.state.grid.bounds(), expected.bounds());
    // Comparing cells also checks rat facings, which CSV does not encode.
    assert_eq!(
        game.state.grid.entries().collect::<Vec<_>>(),
        expected.entries().collect::<Vec<_>>()
    );
}

#[test]
fn undo_restores_state() {
    let mut game = game_from_csv(".,.,.\n.,▼,.\n.,.,.");
    let initial = player_pos(&game);
    game.apply_action(Action::Move(Dir4::East));
    assert_ne!(player_pos(&game), initial);
    game.undo();
    assert_eq!(player_pos(&game), initial);
}

#[test]
fn restart_resets_game() {
    let mut game = game_from_csv(".,.,.\n.,▼,.\n.,.,.");
    let initial = player_pos(&game);
    game.apply_action(Action::Move(Dir4::East));
    game.apply_action(Action::Move(Dir4::South));
    game.restart();
    assert_eq!(player_pos(&game), initial);
    assert_eq!(game.state.history.len(), 1);
    assert!(game.state.action_history.is_empty());
    assert!(game.can_undo());
    assert!(!game.can_restart());
}

#[test]
fn undo_restart_restores_the_attempt_and_its_earlier_moves() {
    let mut game = game_from_csv(".,.,.\n.,▼,.\n.,.,.");
    let initial = game.state.grid.clone();
    assert!(game.apply_action(Action::Move(Dir4::East)));
    let after_first_move = game.state.grid.clone();
    assert!(game.apply_action(Action::Move(Dir4::South)));
    let before_restart = game.state.grid.clone();
    let actions_before_restart = game.state.action_history.clone();

    game.restart();
    game.undo();
    assert_board_eq(&game, &before_restart);
    assert_eq!(game.state.action_history, actions_before_restart);
    assert!(game.can_restart());

    game.undo();
    assert_board_eq(&game, &after_first_move);
    game.undo();
    assert_board_eq(&game, &initial);
    assert!(!game.can_undo());
}

#[test]
fn undo_walks_back_through_moves_and_multiple_restarts() {
    use Dir4::{East, North, South, West};
    let mut game = game_from_csv(".,.,.\n.,▼,.\n.,.,.");
    let mut previous_positions = Vec::new();

    for attempt in [vec![East, South], vec![North, West], vec![South]] {
        for direction in attempt {
            previous_positions.push((game.state.grid.clone(), game.state.action_history.clone()));
            assert!(game.apply_action(Action::Move(direction)));
        }
        previous_positions.push((game.state.grid.clone(), game.state.action_history.clone()));
        game.restart();
    }

    for (grid, actions) in previous_positions.into_iter().rev() {
        assert!(game.can_undo());
        game.undo();
        assert_board_eq(&game, &grid);
        assert_eq!(game.state.action_history, actions);
    }
    assert!(!game.can_undo());
    game.undo();
    assert_board_eq(&game, &game.state.initial_grid);
}

#[test]
fn new_moves_after_undoing_restart_extend_the_restored_attempt() {
    use Action::Move;
    use Dir4::{East, North, South, West};
    let mut game = game_from_csv(".,.,.\n.,▼,.\n.,.,.");
    assert!(game.apply_action(Move(East)));
    let after_first_move = game.state.grid.clone();
    assert!(game.apply_action(Move(South)));
    let before_restart = game.state.grid.clone();
    game.restart();
    assert!(game.apply_action(Move(West)));

    game.undo(); // The new attempt's westward move.
    game.undo(); // The restart itself.
    assert_board_eq(&game, &before_restart);
    assert!(game.apply_action(Move(North)));
    assert_eq!(
        game.state.action_history,
        vec![vec![Move(East)], vec![Move(South)], vec![Move(North)]]
    );
    game.undo();
    assert_board_eq(&game, &before_restart);
    game.undo();
    assert_board_eq(&game, &after_first_move);
    game.undo();
    assert!(!game.can_undo());
}

#[test]
fn redundant_restarts_do_not_add_undo_steps() {
    let mut game = game_from_csv("▲,.");
    game.restart();
    game.restart();
    assert!(!game.can_undo());

    assert!(game.apply_action(Action::Move(Dir4::East)));
    let before_restart = game.state.grid.clone();
    game.restart();
    game.restart();
    game.restart();
    game.undo();
    assert_board_eq(&game, &before_restart);
    game.undo();
    assert!(!game.can_undo());
}

#[test]
fn restart_after_wait_is_undoable_even_when_the_board_looks_unchanged() {
    let mut game = game_from_csv("▲");
    assert!(game.apply_action(Action::Stall));
    assert_board_eq(&game, &game.state.initial_grid);
    assert!(game.can_restart());
    game.restart();
    assert!(game.state.action_history.is_empty());
    game.undo();
    assert_eq!(game.state.action_history, vec![vec![Action::Stall]]);
    game.undo();
    assert!(!game.can_undo());
}

#[test]
fn undo_restart_restores_all_changed_cells_and_enemy_facings() {
    let mut game = game_from_csv("►,.,.,.,.\n.,=,.,w,.\nR,.,.,.,.\n.,.,C,.,O");
    let initial = game.state.grid.clone();
    assert!(game.apply_action(Action::Move(Dir4::East)));
    let before_restart = game.state.grid.clone();
    assert_ne!(
        before_restart.entries().collect::<Vec<_>>(),
        initial.entries().collect::<Vec<_>>()
    );
    game.restart();
    assert_board_eq(&game, &initial);
    game.undo();
    assert_board_eq(&game, &before_restart);
}

fn assert_solution_replays(game: &Game) {
    let encoded = crate::solution::encode_solution(
        "restart_test",
        &game.state.initial_grid.to_csv(),
        &game.state.action_history,
    );
    let decoded = crate::solution::decode_solution(&encoded);
    let mut replay = game_from_csv(&decoded.grid);
    for actions in decoded.actions {
        assert!(replay.apply_actions(&actions));
    }
    assert_board_eq(&replay, &game.state.grid);
    assert_eq!(replay.state.play_state(), game.state.play_state());
}

#[test]
fn undo_restart_restores_a_win_and_its_replayable_solution() {
    let mut game = game_from_csv("►,R");
    assert!(game.apply_action(Action::Stall));
    game.restart();
    assert!(game.apply_action(Action::Move(Dir4::East)));
    assert_eq!(game.state.play_state(), PlayState::Won);
    let winning_grid = game.state.grid.clone();
    let winning_actions = vec![vec![Action::Move(Dir4::East)]];
    assert_eq!(game.state.action_history, winning_actions);
    assert_solution_replays(&game);

    game.restart();
    assert_eq!(game.state.play_state(), PlayState::Playing);
    game.undo();
    assert_board_eq(&game, &winning_grid);
    assert_eq!(game.state.play_state(), PlayState::Won);
    assert_eq!(game.state.action_history, winning_actions);
    assert_solution_replays(&game);
    game.undo();
    assert_eq!(game.state.play_state(), PlayState::Playing);
    game.undo();
    assert_eq!(game.state.action_history, vec![vec![Action::Stall]]);
}

#[test]
fn undo_restart_does_not_resume_old_animations_or_buffered_moves() {
    use Action::Move;
    use Dir4::{East, North, South};
    let mut game = game_from_csv(".,.,.\n.,▼,.\n.,.,.");
    game.begin_actions(&[Move(East)]);
    game.try_begin_actions(vec![Move(South)]);
    assert!(game.is_animating());
    assert_eq!(game.state.queued_actions, Some(vec![Move(South)]));
    let before_restart = game.state.grid.clone();

    game.restart();
    assert!(!game.is_animating());
    assert!(game.state.queued_actions.is_none());
    game.undo();
    assert_board_eq(&game, &before_restart);
    assert!(!game.is_animating());
    assert!(game.state.queued_actions.is_none());
    assert!(game.animate(f32::INFINITY).is_empty());

    game.begin_actions(&[Move(North)]);
    assert!(!game.animate(f32::INFINITY).is_empty());
    assert_eq!(player_pos(&game), Position::new(2, 0));
    assert_eq!(
        game.state.action_history,
        vec![vec![Move(East)], vec![Move(North)]]
    );
}

#[test]
fn undo_restart_restores_which_hero_was_reading_a_note() {
    let mut grid = Grid::from_csv("▲,.,△");
    grid.insert_note(Position::new(0, 0), NoteText::Simple("First hero".into()));
    grid.insert_note(Position::new(1, 0), NoteText::Simple("Second hero".into()));
    let mut game = Game::new(grid);
    game.begin_actions(&[Action::Stall, Action::Move(Dir4::West)]);
    assert_eq!(game.state.standing_on_note().unwrap().0, Player::Player2);

    game.restart();
    game.undo();
    let (player, note) = game.state.standing_on_note().unwrap();
    assert_eq!(player, Player::Player2);
    assert_eq!(
        note.resolve(crate::input::InputHints::Keyboard, 0),
        "Second hero"
    );
    game.undo();
    assert_eq!(game.state.standing_on_note().unwrap().0, Player::Player1);
}

/// What killed a hero in the turn the actions start, as it plays out.
fn death_in_turn(csv: &str, actions: &[Action]) -> Option<Death> {
    let mut game = game_from_csv(csv);
    game.begin_actions(actions);
    game.animate(f32::INFINITY)
        .into_iter()
        .find_map(GameEvent::death)
}

#[test]
fn deaths_say_what_killed_the_hero() {
    for (csv, actions, cause) in fatal_turns() {
        assert_eq!(death_in_turn(csv, &actions), Some(cause));
    }
    assert_eq!(death_in_turn("▲,.", &[Action::Move(Dir4::East)]), None);
}

fn fatal_turns() -> [(&'static str, Vec<Action>, Death); 5] {
    use Action::{Move, Stall};
    use Dir4::{East, North, West};
    [
        (".,.,.\n.,▲,.\n.,R,.\n.,.,.", vec![Stall], Death::Bitten),
        ("O\n▲", vec![Move(North)], Death::Swallowed),
        ("X\n▲", vec![Move(North)], Death::Blasted),
        (".,1,.\n.,X,.\n1,▲,.", vec![Move(West)], Death::Blasted),
        ("►,▷", vec![Move(East), Stall], Death::FriendlyFire),
    ]
}

#[test]
fn undo_restart_restores_death_even_before_its_animation_finishes() {
    for (csv, actions, cause) in fatal_turns() {
        let mut game = game_from_csv(csv);
        game.begin_actions(&actions);
        assert_eq!(game.state.play_state(), PlayState::GameOver);
        assert_eq!(game.state.death, Some(cause));
        assert!(game.is_animating());
        let dead_grid = game.state.grid.clone();

        game.restart();
        assert_eq!(game.state.play_state(), PlayState::Playing);
        assert_eq!(game.state.death, None);
        game.undo();
        assert_board_eq(&game, &dead_grid);
        assert_eq!(game.state.play_state(), PlayState::GameOver);
        assert_eq!(game.state.death, Some(cause));
        assert!(!game.is_animating());

        game.undo();
        assert_board_eq(&game, &game.state.initial_grid);
        assert_eq!(game.state.play_state(), PlayState::Playing);
        assert_eq!(game.state.death, None);
    }
}
