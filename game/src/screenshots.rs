//! Dev tool: renders scripted scenes to PNGs for visual review.
//! `SHOTS_DIR=/tmp/shots cargo test -r -p infestation screenshots -- --ignored --test-threads=1`
//! (opens a window briefly). `W`/`H` set the window size.

use enum_map::EnumMap;
use macroquad::prelude::*;

use crate::app::load_audio_for_tests;
use crate::direction::Dir4;
use crate::game::Action;
use crate::grid::Player;
use crate::input::{InputHints, PlayerInput};
use crate::levels;
use crate::progress::Progress;
use crate::render::ui::ScreenLayout;
use crate::scenes::level::LevelScene;
use crate::scenes::map::MapScene;
use crate::scenes::menu::Menu;
use crate::scenes::title::TitleScene;
use crate::scenes::{Ctx, FrameInput};
use crate::settings::Settings;
use crate::sprites::Sprites;

fn idle(dt: f32) -> FrameInput {
    FrameInput {
        dt,
        meta: Vec::new(),
        players: EnumMap::default(),
        nav: None,
        pointer: Vec::new(),
        any: false,
    }
}

fn moving(dir: Dir4) -> FrameInput {
    let mut input = idle(1.0 / 60.0);
    input.players[Player::Player1] = Some(PlayerInput {
        action: Action::Move(dir),
        synced: false,
        fresh: true,
    });
    input
}

/// Tiny levels that each show off one kind of effect.
static FX_LEVELS: std::sync::LazyLock<Vec<levels::Level>> = std::sync::LazyLock::new(|| {
    [
        ("fx_kill", ".,.,.\n.,R,.\n.,▲,.\n.,.,.\n"),
        (
            "fx_chain",
            "R,X,.,X,R\n.,.,1,.,.\n.,.,.,.,.\n.,.,1,.,.\n.,.,▲,.,.\n",
        ),
        ("fx_hole", "R,.,.,.\n.,O,.,.\n.,.,.,.\n.,.,.,▲\n"),
        ("fx_death", ".,.,.\n.,▲,.\n.,R,.\n.,.,.\n"),
        ("fx_plank", "R,=,.,.\n.,.,.,.\nw,.,.,.\n▲,.,.,.\n"),
    ]
    .into_iter()
    .map(|(name, csv)| levels::Level {
        name,
        display_name: name.to_string(),
        grid: crate::grid::Grid::from_csv(csv),
    })
    .collect()
});

/// The shortest single-player solution to a level, by breadth-first search.
fn solve(level: &levels::Level) -> Vec<Action> {
    use std::collections::{HashSet, VecDeque};
    let actions = [
        Action::Move(Dir4::North),
        Action::Move(Dir4::South),
        Action::Move(Dir4::East),
        Action::Move(Dir4::West),
        Action::Stall,
    ];
    let start = crate::game::Game::new(level.grid.clone());
    let mut seen = HashSet::from([start.state.grid.to_csv()]);
    let mut queue = VecDeque::from([(start, Vec::new())]);
    while let Some((game, path)) = queue.pop_front() {
        for action in actions {
            let mut next = game.clone();
            next.apply_action(action);
            let mut path = path.clone();
            path.push(action);
            match next.state.play_state() {
                crate::game::PlayState::Won => return path,
                crate::game::PlayState::GameOver => continue,
                crate::game::PlayState::Playing => {}
            }
            if seen.insert(next.state.grid.to_csv()) {
                queue.push_back((next, path));
            }
        }
    }
    panic!("{} has no solution", level.name);
}

fn save(sprites: &Sprites, dir: &str, name: &str) {
    sprites.finish_frame();
    get_screen_data().export_png(&format!("{dir}/{name}.png"));
}

#[test]
#[ignore]
fn screenshots() {
    let w: i32 = std::env::var("W")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1280);
    let h: i32 = std::env::var("H")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(800);
    let conf = Conf {
        window_title: "shots".into(),
        window_width: w,
        window_height: h,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    };
    // Music loads from `assets/` relative to the repository root.
    std::env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/..")).unwrap();
    macroquad::Window::from_config(conf, async {
        let dir = std::env::var("SHOTS_DIR").expect("set SHOTS_DIR");
        let only = std::env::var("ONLY").ok();
        let want = |name: &str| {
            only.as_deref()
                .is_none_or(|o| o.split(',').any(|p| name.starts_with(p)))
        };
        let sprites = Sprites::load().await;
        let mut audio = load_audio_for_tests(&sprites).await;
        let mut settings = Settings {
            music_volume: 0.0,
            sfx_volume: 0.0,
            ..Settings::default()
        };
        audio.music_volume = 0.0;
        audio.sfx_volume = 0.0;
        let hints = match std::env::var("HINTS").as_deref() {
            Ok("touch") => InputHints::Touch,
            Ok("pad") => InputHints::Controller(quad_gamepad::ControllerType::PlayStation),
            _ => InputHints::Keyboard,
        };

        macro_rules! ctx {
            ($progress:expr) => {
                Ctx {
                    sprites: &sprites,
                    audio: &mut audio,
                    progress: $progress,
                    settings: &mut settings,
                    hints,
                    gamepads: 0,
                }
            };
        }

        // Title
        if want("title") {
            let mut title = TitleScene::new();
            for _ in 0..150 {
                title.update(&idle(1.0 / 60.0));
            }
            let mut progress = Progress::default();
            let ctx = ctx!(&mut progress);
            let layout = ScreenLayout::current();
            title.draw(&ctx, &layout);
            save(&sprites, &dir, "title");
            next_frame().await;
        }

        // Levels
        for (shot, level, moves) in [
            ("level_rats", "rats", vec![]),
            ("level_webs", "webs", vec![]),
            ("level_triggers", "triggers", vec![]),
            ("level_coop", "cooperation/cooperation", vec![]),
            ("level_lab", "cyborg_rats/fakeout", vec![]),
            ("level_tinderbox", "tinderbox", vec![]),
            ("level_holes", "blackhole_v2", vec![]),
            (
                "level_more_rats_moved",
                "more_rats",
                vec![Dir4::North, Dir4::North],
            ),
        ] {
            if !want(shot) {
                continue;
            }
            let mut progress = Progress::with_completed(["rats"]);
            let mut ctx = ctx!(&mut progress);
            let mut scene = LevelScene::new(levels::get_level(level).unwrap());
            let layout = ScreenLayout::current();
            for _ in 0..170 {
                scene.update(&mut ctx, &idle(1.0 / 60.0), &layout);
            }
            for dir in moves {
                scene.update(&mut ctx, &moving(dir), &layout);
                for _ in 0..30 {
                    scene.update(&mut ctx, &idle(1.0 / 60.0), &layout);
                }
            }
            scene.draw(&ctx, &layout);
            save(&sprites, &dir, shot);
            next_frame().await;
        }

        // A kill, mid-animation: in Rats, walk up the corridor toward the rat.
        if want("anim") {
            let mut progress = Progress::default();
            let mut ctx = ctx!(&mut progress);
            let mut scene = LevelScene::new(levels::get_level("rats").unwrap());
            let layout = ScreenLayout::current();
            for _ in 0..170 {
                scene.update(&mut ctx, &idle(1.0 / 60.0), &layout);
            }
            for (i, step) in [Dir4::East, Dir4::North, Dir4::North]
                .into_iter()
                .enumerate()
            {
                scene.update(&mut ctx, &moving(step), &layout);
                for f in 0..40 {
                    scene.update(&mut ctx, &idle(1.0 / 60.0), &layout);
                    if i == 2 && f == 3 {
                        scene.draw(&ctx, &layout);
                        save(&sprites, &dir, "anim_mid");
                        next_frame().await;
                    }
                }
            }
            scene.draw(&ctx, &layout);
            save(&sprites, &dir, "anim_after");
            next_frame().await;
        }

        // Frame cost on the heaviest levels (CPU: update, draw, flush).
        if want("perf") {
            for name in [
                "cooperation/blocked_v2",
                "cyborg_rats/ai_takeover",
                "tinderbox",
            ] {
                let mut progress = Progress::default();
                let mut ctx = ctx!(&mut progress);
                let mut scene = LevelScene::new(levels::get_level(name).unwrap());
                let layout = ScreenLayout::current();
                let frames = 120;
                let start = std::time::Instant::now();
                for _ in 0..frames {
                    scene.update(&mut ctx, &idle(1.0 / 60.0), &layout);
                    scene.draw(&ctx, &layout);
                    sprites.finish_frame();
                    // Submit to the GPU without presenting (presenting can
                    // block on vsync while the screen is idle).
                    unsafe { get_internal_gl() }.flush();
                }
                let per = start.elapsed().as_secs_f64() * 1000.0 / frames as f64;
                println!("PERF {name}: {per:.2} ms/frame");
            }
        }

        // Effects, frame by frame.
        for (level, first) in FX_LEVELS.iter().zip([
            Some(Dir4::North),
            Some(Dir4::North),
            None,
            None,
            Some(Dir4::North),
        ]) {
            if !want(level.name) {
                continue;
            }
            let mut progress = Progress::default();
            let mut ctx = ctx!(&mut progress);
            let mut scene = LevelScene::new(level);
            let layout = ScreenLayout::current();
            for _ in 0..150 {
                scene.update(&mut ctx, &idle(1.0 / 60.0), &layout);
            }
            let input = match first {
                Some(dir) => moving(dir),
                None => {
                    let mut wait = idle(1.0 / 60.0);
                    wait.players[Player::Player1] = Some(PlayerInput {
                        action: Action::Stall,
                        synced: false,
                        fresh: true,
                    });
                    wait
                }
            };
            scene.update(&mut ctx, &input, &layout);
            let mut frame = 0;
            for shot in [2, 6, 10, 16, 24, 40] {
                while frame < shot {
                    scene.update(&mut ctx, &idle(1.0 / 60.0), &layout);
                    frame += 1;
                }
                scene.draw(&ctx, &layout);
                save(&sprites, &dir, &format!("{}_{shot:02}", level.name));
                next_frame().await;
            }
        }

        // Restart asks for a second press.
        if want("restart_prompt") {
            let mut progress = Progress::default();
            let mut ctx = ctx!(&mut progress);
            let mut scene = LevelScene::new(levels::get_level("planks").unwrap());
            let layout = ScreenLayout::current();
            for _ in 0..150 {
                scene.update(&mut ctx, &idle(1.0 / 60.0), &layout);
            }
            scene.update(&mut ctx, &moving(Dir4::East), &layout);
            for _ in 0..30 {
                scene.update(&mut ctx, &idle(1.0 / 60.0), &layout);
            }
            let mut restart = idle(1.0 / 60.0);
            restart.meta = vec![crate::input::MetaInput::Restart];
            scene.update(&mut ctx, &restart, &layout);
            scene.draw(&ctx, &layout);
            save(&sprites, &dir, "restart_prompt");
            next_frame().await;
        }

        // Pause menu over a level.
        if want("menu") {
            let mut progress = Progress::default();
            let mut ctx = ctx!(&mut progress);
            let mut scene = LevelScene::new(levels::get_level("planks").unwrap());
            let layout = ScreenLayout::current();
            for _ in 0..170 {
                scene.update(&mut ctx, &idle(1.0 / 60.0), &layout);
            }
            let mut menu = Menu::new(true);
            let mut nav = idle(1.0 / 60.0);
            nav.nav = Some(Dir4::South);
            let still = idle(1.0 / 60.0);
            for i in 0..30 {
                menu.update(&mut ctx, if i == 0 { &nav } else { &still }, &layout);
            }
            scene.draw(&ctx, &layout);
            menu.draw(&ctx, &layout);
            save(&sprites, &dir, "menu");
            next_frame().await;
        }

        // The whole flow through the app: title, map, a level won, and
        // back to the map as the next level opens.
        if want("flow") {
            let flow_settings = Settings {
                music_volume: 0.0,
                sfx_volume: 0.0,
                ..Settings::default()
            };
            let flow_audio = load_audio_for_tests(&sprites).await;
            let mut app = crate::app::App::new(
                sprites.clone(),
                flow_audio,
                flow_settings,
                Progress::default(),
                None,
            );
            let mut confirm = idle(1.0 / 60.0);
            confirm.meta = vec![crate::input::MetaInput::Confirm(Player::Player1)];
            confirm.any = true;
            let still = idle(1.0 / 60.0);
            macro_rules! run {
                ($input:expr, $frames:expr) => {
                    for _ in 0..$frames {
                        app.step($input, hints, 0);
                    }
                };
            }
            macro_rules! shot {
                ($name:expr) => {{
                    app.draw(&ScreenLayout::current(), hints, 0);
                    get_screen_data().export_png(&format!("{dir}/{}.png", $name));
                    next_frame().await;
                }};
            }
            run!(&still, 60);
            app.step(&confirm, hints, 0);
            run!(&still, 15);
            shot!("flow_1_iris");
            run!(&still, 60);
            shot!("flow_2_map");
            app.step(&confirm, hints, 0);
            run!(&still, 120);
            shot!("flow_3_level");
            for action in solve(levels::get_level("rats").unwrap()) {
                let mut input = idle(1.0 / 60.0);
                input.players[Player::Player1] = Some(PlayerInput {
                    action,
                    synced: false,
                    fresh: true,
                });
                app.step(&input, hints, 0);
                run!(&still, 20);
            }
            run!(&still, 60);
            shot!("flow_4_won");
            app.step(&confirm, hints, 0);
            run!(&still, 75);
            shot!("flow_5_reveal");
            run!(&still, 60);
            let mut right = idle(1.0 / 60.0);
            right.nav = Some(Dir4::East);
            app.step(&right, hints, 0);
            run!(&still, 12);
            shot!("flow_6_walk");
            let mut exit = idle(1.0 / 60.0);
            exit.meta = vec![crate::input::MetaInput::Exit];
            app.step(&exit, hints, 0);
            run!(&still, 20);
            shot!("flow_7_pause");
        }

        // Chains to a locked level: all intact, then one shattering just
        // after its level is cleared.
        let cellar = ["rats", "more_rats", "webs", "planks", "blackhole_v2"];
        for (shot, extra, before_extra, at, frames) in [
            ("map_chains", vec![], None, "triggers", 120),
            (
                "map_chain_break",
                vec!["triggers", "explosives", "explosives2"],
                Some(vec!["triggers", "explosives"]),
                "explosives2",
                40,
            ),
        ] {
            if !want(shot) {
                continue;
            }
            let mut progress = Progress::with_completed(cellar.iter().copied().chain(extra));
            let before =
                before_extra.map(|b| Progress::with_completed(cellar.iter().copied().chain(b)));
            let from = before.as_ref().map(|_| levels::get_level(at).unwrap());
            let mut scene = MapScene::new(&progress, Some(at), from, before.as_ref());
            let mut ctx = ctx!(&mut progress);
            let layout = ScreenLayout::current();
            for _ in 0..frames {
                scene.update(&mut ctx, &idle(1.0 / 60.0), &layout);
            }
            scene.draw(&ctx, &layout);
            save(&sprites, &dir, shot);
            next_frame().await;
        }

        // World map: fresh, mid-way, and just after clearing the cellar.
        for (shot, completed, before) in [
            ("map_new", vec![], None),
            (
                "map_powder",
                vec![
                    "rats",
                    "more_rats",
                    "webs",
                    "planks",
                    "blackhole_v2",
                    "triggers",
                ],
                None,
            ),
            (
                "map_reveal",
                vec!["rats", "more_rats", "webs", "planks", "blackhole_v2"],
                Some(vec!["rats", "more_rats", "webs", "planks"]),
            ),
            (
                "map_hall",
                vec![
                    "rats",
                    "more_rats",
                    "webs",
                    "planks",
                    "blackhole_v2",
                    "triggers",
                    "triggering_explosives_v3",
                    "order_of_operations_new_v2",
                    "chase",
                ],
                None,
            ),
        ] {
            if !want(shot) {
                continue;
            }
            let mut progress = Progress::with_completed(completed.iter().copied());
            let before = before.map(|b| Progress::with_completed(b.iter().copied()));
            let from = completed.last().map(|l| levels::get_level(l).unwrap());
            let mut scene = MapScene::new(&progress, None, from, before.as_ref());
            let mut ctx = ctx!(&mut progress);
            let layout = ScreenLayout::current();
            let frames = if shot == "map_reveal" { 75 } else { 200 };
            for _ in 0..frames {
                scene.update(&mut ctx, &idle(1.0 / 60.0), &layout);
            }
            scene.draw(&ctx, &layout);
            save(&sprites, &dir, shot);
            next_frame().await;
        }
        miniquad::window::order_quit();
    });
}
