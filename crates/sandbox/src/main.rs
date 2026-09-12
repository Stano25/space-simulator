use engine::app::app::App;
use engine::window::window::WindowConfig;

fn main() {
    App::new()
        .set_window(WindowConfig {
            title: "Space Simulator".to_string(),
            width: 1280,
            height: 720,
        })
        .run();
}
