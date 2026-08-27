use minifb::{Key, KeyRepeat, Window};

use crate::config::KeyBindings;

#[derive(Default)]
pub struct Input {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub up_pressed: bool,
    pub down_pressed: bool,
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub select: bool,
    pub exit: bool,
    pub attack: bool,
    pub menu: bool,
}

impl Input {
    pub fn poll(window: &Window, bindings: &KeyBindings) -> Self {
        let up = key_from_name(&bindings.up).unwrap_or(Key::W);
        let down = key_from_name(&bindings.down).unwrap_or(Key::S);
        let left = key_from_name(&bindings.left).unwrap_or(Key::A);
        let right = key_from_name(&bindings.right).unwrap_or(Key::D);
        let select = key_from_name(&bindings.select).unwrap_or(Key::Enter);
        let exit = key_from_name(&bindings.exit).unwrap_or(Key::Escape);
        let attack = key_from_name(&bindings.attack).unwrap_or(Key::C);
        let menu = key_from_name(&bindings.menu).unwrap_or(Key::X);
        Self {
            up: window.is_key_down(up) || window.is_key_down(Key::Up),
            down: window.is_key_down(down) || window.is_key_down(Key::Down),
            left: window.is_key_down(left) || window.is_key_down(Key::Left),
            right: window.is_key_down(right) || window.is_key_down(Key::Right),
            up_pressed: pressed(window, up) || pressed(window, Key::Up),
            down_pressed: pressed(window, down) || pressed(window, Key::Down),
            left_pressed: pressed(window, left) || pressed(window, Key::Left),
            right_pressed: pressed(window, right) || pressed(window, Key::Right),
            select: pressed(window, select),
            exit: pressed(window, exit),
            attack: pressed(window, attack),
            menu: pressed(window, menu),
        }
    }
}

fn pressed(window: &Window, key: Key) -> bool {
    window.is_key_pressed(key, KeyRepeat::No)
}

pub fn key_name(key: Key) -> Option<&'static str> {
    Some(match key {
        Key::A => "A",
        Key::B => "B",
        Key::C => "C",
        Key::D => "D",
        Key::E => "E",
        Key::F => "F",
        Key::G => "G",
        Key::H => "H",
        Key::I => "I",
        Key::J => "J",
        Key::K => "K",
        Key::L => "L",
        Key::M => "M",
        Key::N => "N",
        Key::O => "O",
        Key::P => "P",
        Key::Q => "Q",
        Key::R => "R",
        Key::S => "S",
        Key::T => "T",
        Key::U => "U",
        Key::V => "V",
        Key::W => "W",
        Key::X => "X",
        Key::Y => "Y",
        Key::Z => "Z",
        Key::Key0 => "0",
        Key::Key1 => "1",
        Key::Key2 => "2",
        Key::Key3 => "3",
        Key::Key4 => "4",
        Key::Key5 => "5",
        Key::Key6 => "6",
        Key::Key7 => "7",
        Key::Key8 => "8",
        Key::Key9 => "9",
        Key::Up => "UP",
        Key::Down => "DOWN",
        Key::Left => "LEFT",
        Key::Right => "RIGHT",
        Key::Enter => "ENTER",
        Key::Space => "SPACE",
        Key::Escape => "ESCAPE",
        Key::LeftShift => "LEFT SHIFT",
        Key::RightShift => "RIGHT SHIFT",
        Key::LeftCtrl => "LEFT CTRL",
        Key::RightCtrl => "RIGHT CTRL",
        Key::Tab => "TAB",
        Key::Backspace => "BACKSPACE",
        _ => return None,
    })
}

pub fn key_from_name(name: &str) -> Option<Key> {
    Some(match name {
        "A" => Key::A,
        "B" => Key::B,
        "C" => Key::C,
        "D" => Key::D,
        "E" => Key::E,
        "F" => Key::F,
        "G" => Key::G,
        "H" => Key::H,
        "I" => Key::I,
        "J" => Key::J,
        "K" => Key::K,
        "L" => Key::L,
        "M" => Key::M,
        "N" => Key::N,
        "O" => Key::O,
        "P" => Key::P,
        "Q" => Key::Q,
        "R" => Key::R,
        "S" => Key::S,
        "T" => Key::T,
        "U" => Key::U,
        "V" => Key::V,
        "W" => Key::W,
        "X" => Key::X,
        "Y" => Key::Y,
        "Z" => Key::Z,
        "0" => Key::Key0,
        "1" => Key::Key1,
        "2" => Key::Key2,
        "3" => Key::Key3,
        "4" => Key::Key4,
        "5" => Key::Key5,
        "6" => Key::Key6,
        "7" => Key::Key7,
        "8" => Key::Key8,
        "9" => Key::Key9,
        "UP" => Key::Up,
        "DOWN" => Key::Down,
        "LEFT" => Key::Left,
        "RIGHT" => Key::Right,
        "ENTER" => Key::Enter,
        "SPACE" => Key::Space,
        "ESCAPE" => Key::Escape,
        "LEFT SHIFT" => Key::LeftShift,
        "RIGHT SHIFT" => Key::RightShift,
        "LEFT CTRL" => Key::LeftCtrl,
        "RIGHT CTRL" => Key::RightCtrl,
        "TAB" => Key::Tab,
        "BACKSPACE" => Key::Backspace,
        _ => return None,
    })
}
