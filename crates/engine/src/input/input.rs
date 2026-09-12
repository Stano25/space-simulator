use winit::keyboard::KeyCode;
use bevy_ecs::prelude::Resource;

use crate::input::keyboard::KeyboardState;

#[derive(Resource, Default, Debug)]
pub struct InputState {
    pub(crate) keyboard: KeyboardState,
}

impl InputState {   
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {   
        self.keyboard.pressed_keys.contains(&key)
    }

    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.keyboard.just_pressed_keys.contains(&key)
    }

    pub fn is_key_just_released(&self, key: KeyCode) -> bool {
        self.keyboard.just_released_keys.contains(&key)
    }

    pub(crate) fn clear_frame_states(&mut self) { // Clear the just pressed and just released states at the end of each frame (Only engine can call this)
        self.keyboard.just_pressed_keys.clear();
        self.keyboard.just_released_keys.clear();
    }
}