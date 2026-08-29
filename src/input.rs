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
    pub pickup: bool,
    pub backspace: bool,
    pub text: Vec<char>,
}

impl Input {
    pub fn poll(
        window: &Window,
        bindings: &KeyBindings,
        raw_keys: &[Key],
        gamepad: &mut Gamepad,
    ) -> Self {
        let up = key_from_name(&bindings.up).unwrap_or(Key::W);
        let down = key_from_name(&bindings.down).unwrap_or(Key::S);
        let left = key_from_name(&bindings.left).unwrap_or(Key::A);
        let right = key_from_name(&bindings.right).unwrap_or(Key::D);
        let select = key_from_name(&bindings.select).unwrap_or(Key::Enter);
        let exit = key_from_name(&bindings.exit).unwrap_or(Key::Escape);
        let attack = key_from_name(&bindings.attack).unwrap_or(Key::C);
        let menu = key_from_name(&bindings.menu).unwrap_or(Key::X);
        let pickup = key_from_name(&bindings.pickup).unwrap_or(Key::V);
        let shifted = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
        let text = raw_keys
            .iter()
            .copied()
            .filter_map(|key| text_character(key, shifted))
            .collect();
        let pad = gamepad.poll();
        Self {
            up: window.is_key_down(up) || window.is_key_down(Key::Up) || pad.up,
            down: window.is_key_down(down) || window.is_key_down(Key::Down) || pad.down,
            left: window.is_key_down(left) || window.is_key_down(Key::Left) || pad.left,
            right: window.is_key_down(right) || window.is_key_down(Key::Right) || pad.right,
            up_pressed: pressed(window, up) || pressed(window, Key::Up) || pad.up_pressed,
            down_pressed: pressed(window, down) || pressed(window, Key::Down) || pad.down_pressed,
            left_pressed: pressed(window, left) || pressed(window, Key::Left) || pad.left_pressed,
            right_pressed: pressed(window, right)
                || pressed(window, Key::Right)
                || pad.right_pressed,
            select: pressed(window, select) || pad.select,
            exit: pressed(window, exit) || pad.exit,
            attack: pressed(window, attack) || pad.attack,
            menu: pressed(window, menu) || pad.menu,
            pickup: pressed(window, pickup) || pad.pickup,
            backspace: pressed(window, Key::Backspace),
            text,
        }
    }
}

pub struct Gamepad {
    previous_buttons: u16,
    previous_directions: u8,
    backend: gamepad_platform::Backend,
}

impl Default for Gamepad {
    fn default() -> Self {
        Self {
            previous_buttons: 0,
            previous_directions: 0,
            backend: gamepad_platform::Backend::new(),
        }
    }
}

impl Gamepad {
    fn poll(&mut self) -> GamepadInput {
        let state = self.backend.state();
        self.apply(state)
    }

    fn apply(&mut self, state: RawGamepadState) -> GamepadInput {
        let directions = u8::from(state.up)
            | (u8::from(state.down) << 1)
            | (u8::from(state.left) << 2)
            | (u8::from(state.right) << 3);
        let previous = self.previous_directions;
        let pressed = directions & !previous;
        let newly_pressed = state.buttons & !self.previous_buttons;
        self.previous_buttons = state.buttons;
        self.previous_directions = directions;
        GamepadInput {
            up: state.up,
            down: state.down,
            left: state.left,
            right: state.right,
            up_pressed: pressed & 1 != 0,
            down_pressed: pressed & 2 != 0,
            left_pressed: pressed & 4 != 0,
            right_pressed: pressed & 8 != 0,
            select: newly_pressed & gamepad_platform::A != 0,
            exit: newly_pressed & gamepad_platform::B != 0,
            attack: newly_pressed & gamepad_platform::X != 0,
            menu: newly_pressed & (gamepad_platform::Y | gamepad_platform::START) != 0,
            pickup: newly_pressed & gamepad_platform::LEFT_SHOULDER != 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Gamepad, RawGamepadState, gamepad_platform};

    #[test]
    fn controller_buttons_and_directions_are_edge_triggered() {
        let mut gamepad = Gamepad::default();
        let first = gamepad.apply(RawGamepadState {
            buttons: gamepad_platform::A | gamepad_platform::X,
            up: true,
            ..RawGamepadState::default()
        });
        assert!(first.up && first.up_pressed && first.select && first.attack);

        let held = gamepad.apply(RawGamepadState {
            buttons: gamepad_platform::A | gamepad_platform::X,
            up: true,
            ..RawGamepadState::default()
        });
        assert!(held.up);
        assert!(!held.up_pressed && !held.select && !held.attack);

        gamepad.apply(RawGamepadState::default());
        let pressed_again = gamepad.apply(RawGamepadState {
            buttons: gamepad_platform::A,
            right: true,
            ..RawGamepadState::default()
        });
        assert!(pressed_again.right_pressed && pressed_again.select);
    }
}

#[derive(Default)]
struct GamepadInput {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    up_pressed: bool,
    down_pressed: bool,
    left_pressed: bool,
    right_pressed: bool,
    select: bool,
    exit: bool,
    attack: bool,
    menu: bool,
    pickup: bool,
}

#[derive(Default)]
struct RawGamepadState {
    buttons: u16,
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

#[cfg(windows)]
mod gamepad_platform {
    use std::{ffi::c_void, sync::OnceLock};

    use super::RawGamepadState;

    pub const LEFT_SHOULDER: u16 = 0x0100;
    pub const A: u16 = 0x1000;
    pub const B: u16 = 0x2000;
    pub const X: u16 = 0x4000;
    pub const Y: u16 = 0x8000;
    pub const START: u16 = 0x0010;

    const DPAD_UP: u16 = 0x0001;
    const DPAD_DOWN: u16 = 0x0002;
    const DPAD_LEFT: u16 = 0x0004;
    const DPAD_RIGHT: u16 = 0x0008;
    const STICK_THRESHOLD: i16 = 12_000;

    #[repr(C)]
    #[derive(Default)]
    struct XInputGamepad {
        buttons: u16,
        left_trigger: u8,
        right_trigger: u8,
        thumb_lx: i16,
        thumb_ly: i16,
        thumb_rx: i16,
        thumb_ry: i16,
    }

    #[repr(C)]
    #[derive(Default)]
    struct XInputState {
        packet_number: u32,
        gamepad: XInputGamepad,
    }

    type XInputGetState = unsafe extern "system" fn(u32, *mut XInputState) -> u32;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryA(name: *const u8) -> *mut c_void;
        fn GetProcAddress(module: *mut c_void, name: *const u8) -> *mut c_void;
    }

    fn get_state_function() -> Option<XInputGetState> {
        static FUNCTION: OnceLock<Option<XInputGetState>> = OnceLock::new();
        *FUNCTION.get_or_init(|| {
            for library in [b"xinput1_4.dll\0".as_slice(), b"xinput9_1_0.dll\0"] {
                let module = unsafe { LoadLibraryA(library.as_ptr()) };
                if module.is_null() {
                    continue;
                }
                let address = unsafe { GetProcAddress(module, c"XInputGetState".as_ptr().cast()) };
                if !address.is_null() {
                    return Some(unsafe {
                        std::mem::transmute::<*mut c_void, XInputGetState>(address)
                    });
                }
            }
            None
        })
    }

    pub struct Backend;

    impl Backend {
        pub const fn new() -> Self {
            Self
        }

        pub fn state(&mut self) -> RawGamepadState {
            let Some(get_state) = get_state_function() else {
                return RawGamepadState::default();
            };
            let mut state = XInputState::default();
            let connected = unsafe { get_state(0, &mut state) } == 0;
            if !connected {
                return RawGamepadState::default();
            }
            let pad = state.gamepad;
            RawGamepadState {
                buttons: pad.buttons,
                up: pad.buttons & DPAD_UP != 0 || pad.thumb_ly > STICK_THRESHOLD,
                down: pad.buttons & DPAD_DOWN != 0 || pad.thumb_ly < -STICK_THRESHOLD,
                left: pad.buttons & DPAD_LEFT != 0 || pad.thumb_lx < -STICK_THRESHOLD,
                right: pad.buttons & DPAD_RIGHT != 0 || pad.thumb_lx > STICK_THRESHOLD,
            }
        }
    }
}

#[cfg(not(windows))]
mod gamepad_platform {
    use sdl2::{
        EventPump, GameControllerSubsystem, Sdl,
        controller::{Axis, Button, GameController},
    };

    use super::RawGamepadState;

    pub const LEFT_SHOULDER: u16 = 0x0100;
    pub const A: u16 = 0x1000;
    pub const B: u16 = 0x2000;
    pub const X: u16 = 0x4000;
    pub const Y: u16 = 0x8000;
    pub const START: u16 = 0x0010;

    const STICK_THRESHOLD: i16 = 12_000;

    pub struct Backend(Option<SdlController>);

    struct SdlController {
        _sdl: Sdl,
        subsystem: GameControllerSubsystem,
        event_pump: EventPump,
        controller: Option<GameController>,
    }

    impl Backend {
        pub fn new() -> Self {
            Self(SdlController::new())
        }

        pub fn state(&mut self) -> RawGamepadState {
            self.0
                .as_mut()
                .map_or_else(RawGamepadState::default, SdlController::state)
        }
    }

    impl SdlController {
        fn new() -> Option<Self> {
            let sdl = sdl2::init().ok()?;
            let subsystem = sdl.game_controller().ok()?;
            let _ = subsystem.load_mappings_from_read(std::io::Cursor::new(include_bytes!(
                concat!(env!("CARGO_MANIFEST_DIR"), "/assets/gamecontrollerdb.txt")
            )));
            let event_pump = sdl.event_pump().ok()?;
            let controller = open_first(&subsystem);
            Some(Self {
                _sdl: sdl,
                subsystem,
                event_pump,
                controller,
            })
        }

        fn state(&mut self) -> RawGamepadState {
            self.event_pump.pump_events();
            if self
                .controller
                .as_ref()
                .is_none_or(|controller| !controller.attached())
            {
                self.controller = open_first(&self.subsystem);
            }
            let Some(controller) = &self.controller else {
                return RawGamepadState::default();
            };
            let mut buttons = 0;
            buttons |= u16::from(controller.button(Button::A)) * A;
            buttons |= u16::from(controller.button(Button::B)) * B;
            buttons |= u16::from(controller.button(Button::X)) * X;
            buttons |= u16::from(controller.button(Button::Y)) * Y;
            buttons |= u16::from(controller.button(Button::Start)) * START;
            buttons |= u16::from(controller.button(Button::LeftShoulder)) * LEFT_SHOULDER;
            RawGamepadState {
                buttons,
                up: controller.button(Button::DPadUp)
                    || controller.axis(Axis::LeftY) < -STICK_THRESHOLD,
                down: controller.button(Button::DPadDown)
                    || controller.axis(Axis::LeftY) > STICK_THRESHOLD,
                left: controller.button(Button::DPadLeft)
                    || controller.axis(Axis::LeftX) < -STICK_THRESHOLD,
                right: controller.button(Button::DPadRight)
                    || controller.axis(Axis::LeftX) > STICK_THRESHOLD,
            }
        }
    }

    fn open_first(subsystem: &GameControllerSubsystem) -> Option<GameController> {
        (0..subsystem.num_joysticks().ok()?)
            .find(|&index| subsystem.is_game_controller(index))
            .and_then(|index| subsystem.open(index).ok())
    }
}

fn text_character(key: Key, shifted: bool) -> Option<char> {
    let name = key_name(key)?;
    if name.len() == 1 {
        let mut character = name.chars().next()?;
        if !shifted {
            character = character.to_ascii_lowercase();
        }
        return Some(character);
    }
    (key == Key::Space).then_some(' ')
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
