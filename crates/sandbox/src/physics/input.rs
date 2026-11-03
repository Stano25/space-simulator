use engine::input::input::InputState;

use crate::physics::orbit::{TimeWarp, RAILS_THRESHOLD};

use bevy_ecs::prelude::*;
use winit::keyboard::KeyCode;


pub fn time_warp_input_system(
    input: Res<InputState>,
    mut warp: ResMut<TimeWarp>,
) {
    // '.' -> speed up time
    if input.is_key_pressed(KeyCode::Period) {
        warp.factor = (warp.factor * 1.5).min(100_000.0);
        println!("Time Warp: {:.1}x ({})", warp.factor, if warp.factor >= RAILS_THRESHOLD { "ON RAILS" } else { "PHYSICS" });
    }

    // ',' -> slow down time
    if input.is_key_pressed(KeyCode::Comma) {
        warp.factor = (warp.factor / 1.5).max(1.0);
        println!("Time Warp: {:.1}x ({})", warp.factor, if warp.factor >= RAILS_THRESHOLD { "ON RAILS" } else { "PHYSICS" });
    }

    // '/' -> 1x speed
    if input.is_key_pressed(KeyCode::Slash) {
        warp.factor = 1.0;
        println!("Time Warp: 1.0x (PHYSICS)");
    }
}
