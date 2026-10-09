use super::*;
use crate::grid::Grid;
fn game_from_csv(csv: &str) -> Game {
    Game::new(Grid::from_csv(csv))
}

fn player_pos(game: &Game) -> Position {
    game.state.grid.find_players()[0].pos
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
    use Action::{Move, Stall};
    use Dir4::{East, North, West};
    // A rat attacks from behind.
    let rat_behind = ".,.,.\n.,▲,.\n.,R,.\n.,.,.";
    assert_eq!(death_in_turn(rat_behind, &[Stall]), Some(Death::Bitten));
    assert_eq!(
        death_in_turn("O\n▲", &[Move(North)]),
        Some(Death::Swallowed)
    );
    // Stepping onto an explosive, or caught in a blast set off by a zap.
    assert_eq!(death_in_turn("X\n▲", &[Move(North)]), Some(Death::Blasted));
    let zapped = ".,1,.\n.,X,.\n1,▲,.";
    assert_eq!(death_in_turn(zapped, &[Move(West)]), Some(Death::Blasted));
    // One hero walks into the other's back.
    let heroes = "►,▷";
    assert_eq!(
        death_in_turn(heroes, &[Move(East), Stall]),
        Some(Death::FriendlyFire)
    );
    assert_eq!(death_in_turn("▲,.", &[Move(East)]), None);
}
