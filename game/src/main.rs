use macroquad::window::{Conf, next_frame};
use quad_url::{easy_parse, get_program_parameters};

use infestation::app::App;

fn window_conf() -> Conf {
    Conf {
        window_title: "Infestation".to_string(),
        window_width: 1280,
        window_height: 800,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    }
}

/// Accepts a level name as a bare CLI argument (native) or a `level=<name>`
/// URL query parameter (web, surfaced by quad-url as `--level=<name>`).
#[macroquad::main(window_conf)]
async fn main() {
    let mut level_name: Option<String> = None;
    for param in get_program_parameters().iter().skip(1) {
        let name = match easy_parse(param) {
            Some(("level", Some(value))) => value,
            None => param.as_str(),
            _ => panic!("Unrecognized argument: {}", param),
        };
        if level_name.replace(name.to_string()).is_some() {
            panic!("Multiple levels specified");
        }
    }
    let mut app = App::load(level_name.as_deref()).await;
    while app.tick() {
        next_frame().await;
    }
}
