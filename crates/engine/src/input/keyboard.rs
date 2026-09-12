use std::collections::HashSet;

use winit::keyboard::KeyCode;

#[derive(Debug, Default)]
pub struct KeyboardState {
    pub pressed_keys: HashSet<KeyCode>,
    pub just_pressed_keys: HashSet<KeyCode>,
    pub just_released_keys: HashSet<KeyCode>,
}