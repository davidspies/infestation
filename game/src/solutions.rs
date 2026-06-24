use crate::direction::Dir4;
use crate::game::Action;

pub(crate) struct LevelSolution {
    pub(crate) level_name: &'static str,
    players: usize,
    moves: &'static str,
}

impl LevelSolution {
    const fn new(level_name: &'static str, players: usize, moves: &'static str) -> Self {
        Self {
            level_name,
            players,
            moves,
        }
    }

    pub(crate) fn actions(&self) -> Vec<Vec<Action>> {
        parse_actions(self.moves, self.players)
            .unwrap_or_else(|e| panic!("invalid solution for {}: {e}", self.level_name))
    }
}

pub(crate) const SOLUTIONS: &[LevelSolution] = &[
    LevelSolution::new(
        "blackhole_v2",
        1,
        "vvv>>>>>><<<<<<^^^^^^^^vvvvvvvv>>>>>>><<<<<<<^^^^^^^^^^^^^^^^^>>>><<vv>>>>vv>",
    ),
    LevelSolution::new(
        "cooperation/coop_world_v3",
        2,
        "^^ ^> ^^ ^^ ^^ ^^ ^^ ^> ^^ v> <> ^> ^> ^> ^> ^> ^> >v <. ^v >< v< .> .^ ^^ ^^",
    ),
    LevelSolution::new(
        "cooperation/cooperation",
        2,
        "^^ ^> ^^ ^^ ^< <^ ^> ^> ^> <> <v <> <> <> <> ^> >> >> <> << .< <v vv vv v< v< vv vv",
    ),
    LevelSolution::new(
        "cooperation/handoff",
        2,
        "vv >> >> >< >< >^ ^^ .v .> .> .v .> v> <. v. ^. >. ^< .< .< .^ .^ .^ .< .< .> .v .> .> .v .> .v v> v. >. >. >. >. ^. ^. ^. >. ^. ^. ^. ^. <. ^. <. <. v. ^< >< >> v> >> <> ^v v^ v< >< v< v< v^ << vv vv v^ << <v <> << ^^ << << <^ <> <^ .< ^^ v< >< ^> >< v^ >^ >^ >^ >^ >^ >^ >^ ^^ ^^ ^^ >^ ^^ ^^ ^^ <^ <^ v^ <^ v^ v^ v^",
    ),
    LevelSolution::new(
        "cooperation/tug_of_war",
        2,
        "^v vv >< vv vv vv vv <> <> <> <> <> >v << vv >> vv vv << >> << >> ^^ <^ ^^ >^ <v ^> ^< >^ >^ ^> ^^ ^< >< >< ^^ ^< ^> ^v ^^ ^^ ^^ ^^ ^^ >^ >^ ^^ ^^ v^ v^ <^ v^ v^ v^ v^ v^ v^ v^ v^ v^ v^ ^^ ^^ >^ >^ ^^ ^^ ^^ ^^ ^< ^< ^^ ^^ ^> ^> ^> ^> ^v ^v ^> ^> ^> ^^ ^^ ^> ^v ^v ^v ^v ^v ^v ^< ^< ^< ^< ^v ^v",
    ),
    LevelSolution::new(
        "cyborg_rats/cyborg_rats",
        1,
        "^>>^^<<<>>>^^<<<>>>^^<<<>>>^^<<<",
    ),
    LevelSolution::new(
        "cyborg_rats/fakeout",
        1,
        "^^v......>><v<v................<<<><<<<<><>><>^>^>v<<vvvvvvvvvv>^>^^>>>>>>>>>>",
    ),
    LevelSolution::new(
        "cyborg_rats/stalemate",
        1,
        "^>>^>^>>vv<vv><^^>^^>>>..............><<<<<<<<<",
    ),
    LevelSolution::new(
        "cyborg_rats/unguided",
        1,
        "v<<<<<<<<<<<<vvvvvvvvvvvvvv>vv>><^<^^^^^^^^^^^^^^^>>>>>>>>>>>>>>>^vvvvvvvvvvvvvv>>vvvv<<<<^^<<<<<<^^>>>>>>",
    ),
    LevelSolution::new(
        "explosives",
        1,
        "vvv<<^^^^<<vvvvvv>>>>vv<<<<<>>>>>^^<<<<^^^^^^^^>>>>>>>",
    ),
    LevelSolution::new(
        "explosives2",
        1,
        "^<vvvvvv>>>>><<<<<^^^^^^^^vvvvvvvv><^v^^^^^^^^>>>>>>>",
    ),
    LevelSolution::new("gimmicks/crushkill", 2, "<> <> <> <> >^ ^v ^v ^< ^< ^<"),
    LevelSolution::new("gimmicks/platform", 1, "vvv>vvv..^^.^^^^^^^^^<>>>>vvv"),
    LevelSolution::new("gimmicks/robotic_cheese", 2, ">> ^> ^. ^. <. <^ ^^"),
    LevelSolution::new(
        "guidance",
        1,
        "<<<<<<<<<<<vvvvvvvvvvvvvvvv>>>>>>>>><^>>>>><<<<<<<<<<<<<<^^^^^^<v<v>^^^^^^^^^^^>>>>>^^^^^vv<<<<<vvvvvvvvvvvvvvv>>>>>>>^^>>>>>>",
    ),
    LevelSolution::new(
        "intro",
        1,
        "^^^>>>>vvv<>^^^<<<<^^^^^^<<vvvvv<v<vvv^^^>^^^^^^>>>^>>",
    ),
    LevelSolution::new(
        "limited2",
        1,
        ">>>>>>>>>>>>>><<<<<<<<<<<<<...<>.>>>>>>>>>>>>><<<<<<<<<<<<<...<>....>>>>>>>>>>>>><<<<<<<<<<<<<...<>>>>>>><<<<<<<<<<<^^^^^^^^^^^^>>>>>>>>^^^^^^^^^>>>>",
    ),
    LevelSolution::new(
        "lock_in",
        1,
        "^^>vvv<vv<<v<vv>v<^^^^^vvvvvv^^>>^>>>>>^^^^^^^^^^^v<v><<<^^<<<<<<<<<<",
    ),
    LevelSolution::new("more_rats", 1, "v<<><><><><>>^^^^^^<<><><><<<"),
    LevelSolution::new(
        "no_retreat",
        1,
        "v<^<<<<^<>^^>>>^^<<^<<<<>>>.<<<><^^>>>>>>>>^^<<<<<<<<^^>>>>>>>>>>>>^>>>>^>>>",
    ),
    LevelSolution::new("old_levels/old_levels", 1, "vvvv<<<<<"),
    LevelSolution::new(
        "old_levels/on_the_clock",
        1,
        "^>><vvvvv<v^^^^^>>v>>^<^^^^^^^>>>vvvvvvvvv^^^>>>><<>>vvv>>v>>>>>>vv<<<<<<<<<vv^^>>>>>>>>>^^<<<<<<^<^^^<<<vvv<<<<v<vvvv>>><<<<<<",
    ),
    LevelSolution::new(
        "old_levels/order_of_operations",
        1,
        "<^v^^<^^^>^^<vv^vv^^^^^^^^^^>^v>vv<vvvvvvvv>>>>>^^^^^^^^^^^^^^^>>>>>vvvvvvv>vvvvvvv>>vv",
    ),
    LevelSolution::new(
        "old_levels/overstep",
        1,
        "vvvvvv>>>>>>>>>v>><>><<^v^>>>^>^>^^^^^^^^^^^<<<<<>>>v>v>vvvvvvvvvvv<<<><<<<^<<<<^^<<^<<<^^<<^^^^^^>>>>>>>v>>>^^<<vvv<<>^^<<<<<vvv<<vvv>>vvvv<<<vv^^>>^^^^>^>>v^^^^^^^>>",
    ),
    LevelSolution::new(
        "order_of_operations_new_v2",
        1,
        "v>>>>^>^^^>>v>.....>>>v<vvvvvvvvvvvv",
    ),
    LevelSolution::new(
        "planks",
        1,
        ">>>><<<<<<<<<<^^^^^^<vvvvvv>>>>>>>>>>>>^^^^>^^^vvv<vvvv<<<<<<<<<<<<^^^^^^^>>>v>vvvv>>>>>>^^<^^^<<<vv",
    ),
    LevelSolution::new("rats", 1, "v<<^^^>>>><>^"),
    LevelSolution::new(
        "synchronicity",
        1,
        ">>^v>>vvv<<v^^^^<v^v^v^^^vv>vvv>vv>><<^^>>>>>>^^^^",
    ),
    LevelSolution::new(
        "tinderbox",
        1,
        "^>^^^^^<<<vv<<>.<>>>>^>v<<>>>v<v.><^v>><^^v<>^^<..v.^v^<v.>.vv^v<v..v^<^<<<<.vv^<^v>>",
    ),
    LevelSolution::new(
        "trapped_rat",
        1,
        "vvv^v.>><>>>>^^^^^^<vvvvvv<<>>^^^^^^<vvvvvv<<>>^^^^^^<vvvvvv<<>>^^^^^^^^^^<",
    ),
    LevelSolution::new(
        "trapped_rat2_v2",
        1,
        "<^^^v><^v><^v><^^<<<vvvvvv<^^^^^^><><vvvvvv<^^^^^^><><vvvvvv<^^^^^^><><^^^^^^>",
    ),
    LevelSolution::new(
        "triggering_explosives_v3",
        1,
        "<vvvv<<vvvv<<v^>>^^^^>>^^^^>>vvvv>>vvv^^^<^^><^<^<<^^vv<<^<<vvv>^^^>><>v",
    ),
    LevelSolution::new(
        "triggers",
        1,
        "vvv>>^^>>vv>>>>^>^^<^<<<^^>>>>^^<<<>>^^<^^<<vvv<v<vvvv<<<<<^^^^^^^>>>vv",
    ),
    LevelSolution::new(
        "triggers2",
        1,
        "^^>^>^^^vvv<v<<<<<^^^^><><vvvv>>>>>^>^^^^^vvvvv>>^^^^>>v>v>vvvvv><^^^^^<^<^^><<<><vvvvv<<<v<vvvvv>>v>>>>",
    ),
    LevelSolution::new("webs", 1, "<<^^>>>^>^^<<^>>>>>>>>>vvvvv<><><><<<^^"),
    LevelSolution::new(
        "world",
        2,
        "^^ << <^ ^^ ^^ v^ v^ <^ <^ ^^ ^^ v^ v^ >^ >^ >^ >^ >v >^ ^^ ^^ v^ v^ v^ v^ >^ >^ <^ <^ ^^ ^^ >^ >^ ^^ ^^ v^ v^ <^ <^ <^ <^ ^^ ^^ ^^ ^^ <^ <^ <^ <^ >^ ^^ ^^ ^^ ^^ >^ >^ v^ v^ ^^ ^^ >^ >^ v^ v^ ^^ ^^ ^^ ^^ ^^ ^^ >^ >^ >^ v^ vv ^< ^< ^^ ^^ ^> ^^ ^^ ^v ^v ^< ^< ^^ ^^ ^v ^v ^< ^< ^^ ^^ ^v ^v ^v ^< ^< ^^ ^^ ^^",
    ),
];

fn parse_actions(moves: &str, players: usize) -> Result<Vec<Vec<Action>>, String> {
    assert!(players > 0);

    if players == 1 {
        return moves
            .chars()
            .filter(|c| !c.is_whitespace())
            .map(|c| parse_action(c).map(|action| vec![action]))
            .collect();
    }

    moves
        .split_whitespace()
        .map(|turn| {
            let actions = turn
                .chars()
                .map(parse_action)
                .collect::<Result<Vec<_>, _>>()?;
            if actions.len() != players {
                return Err(format!(
                    "turn {turn:?} has {} actions, expected {players}",
                    actions.len()
                ));
            }
            Ok(actions)
        })
        .collect()
}

fn parse_action(c: char) -> Result<Action, String> {
    match c {
        '^' | '↑' => Ok(Action::Move(Dir4::North)),
        'v' | '↓' => Ok(Action::Move(Dir4::South)),
        '>' | '→' => Ok(Action::Move(Dir4::East)),
        '<' | '←' => Ok(Action::Move(Dir4::West)),
        '.' => Ok(Action::Stall),
        _ => Err(format!("unknown move character {c:?}")),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeSet, HashSet};

    use crate::game::{Game, PlayState};
    use crate::grid::Cell;
    use crate::levels;

    use super::*;

    const KNOWN_UNSOLVED_MASTER_LEVELS: &[&str] = &[
        "chase",
        "cooperation/blocked_v2",
        "cyborg_rats/ai_takeover",
        "release",
        "reload_v3",
        "tinderrectangle",
    ];

    const FORK_ONLY_SOLUTIONS: &[&str] = &[
        "claude/remote_detonator",
        "claude/roach_motel",
        "claude/sacrifice",
        "claude/stampede",
        "claude/web_lair",
    ];

    #[test]
    fn catalog_levels_are_unique_and_exist_on_master() {
        let mut seen = BTreeSet::new();

        for solution in SOLUTIONS {
            assert!(seen.insert(solution.level_name), "duplicate solution");
            assert!(
                levels::get_level(solution.level_name).is_some(),
                "{} is not a current master level",
                solution.level_name
            );
        }
    }

    #[test]
    fn catalog_coverage_matches_known_fork_status() {
        let solved = SOLUTIONS
            .iter()
            .map(|solution| solution.level_name)
            .collect::<BTreeSet<_>>();
        let master = levels::LEVEL_DATA
            .iter()
            .map(|(level_name, _, _)| *level_name)
            .collect::<BTreeSet<_>>();

        let missing = master.difference(&solved).copied().collect::<Vec<_>>();
        assert_eq!(missing, KNOWN_UNSOLVED_MASTER_LEVELS);

        for level_name in FORK_ONLY_SOLUTIONS {
            assert!(
                levels::get_level(level_name).is_none(),
                "{level_name} should not be treated as a master level"
            );
        }
    }

    #[test]
    fn catalog_solutions_win_current_master_levels() {
        for solution in SOLUTIONS {
            let level = levels::get_level(solution.level_name).unwrap();
            let mut game = Game::new(level.grid.clone(), HashSet::new());

            let player_count = level
                .grid
                .entries()
                .filter(|(_, cell)| matches!(cell, Cell::Player(..)))
                .count();
            assert_eq!(player_count, solution.players, "{}", solution.level_name);

            for actions in solution.actions() {
                assert!(
                    game.apply_actions(&actions),
                    "{} rejected actions {:?}",
                    solution.level_name,
                    actions
                );
            }

            assert_eq!(
                game.state.play_state(),
                PlayState::Won,
                "{}",
                solution.level_name
            );
        }
    }
}
