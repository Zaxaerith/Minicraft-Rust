use minifb::{Key, KeyRepeat, Window};

use crate::io::KeyBindings;

/// The sole frame-to-tick buffer, including raw key-binding capture events.
#[derive(Default)]
pub struct InputBuffer {
    input: Input,
    raw_keys: Vec<Key>,
    keyboard: std::collections::BTreeMap<String, crate::io::input::PhysicalKey>,
    bindings: Option<KeyBindings>,
}

impl InputBuffer {
    pub fn frame(&mut self, focused: bool, input: Input, raw_keys: Vec<Key>) {
        if focused {
            self.input.latch(input);
            self.raw_keys.extend(raw_keys);
        } else {
            self.input = Input::default();
            self.raw_keys.clear();
            for key in self.keyboard.values_mut() {
                key.release();
            }
        }
    }

    pub fn tick(&mut self) -> (Input, Vec<Key>) {
        for key in self.keyboard.values_mut() {
            key.tick();
        }
        let mut input = self.input.take_tick();
        if let Some(bindings) = &self.bindings {
            let clicked = |binding: &str| {
                crate::io::input::action_state(
                    binding,
                    |action| bindings.mapping(action),
                    |name| {
                        self.keyboard
                            .get(name)
                            .map(|key| key.state())
                            .unwrap_or_default()
                    },
                )
                .clicked
            };
            // Keyboard action clicks come exclusively from PhysicalKey tick.
            // Retain independently latched controller edges, not frame previews.
            let pad = input.controller_edges;
            input.up_pressed = clicked("MOVE-UP|CURSOR-UP") || pad[0];
            input.down_pressed = clicked("MOVE-DOWN|CURSOR-DOWN") || pad[1];
            input.left_pressed = clicked("MOVE-LEFT|CURSOR-LEFT") || pad[2];
            input.right_pressed = clicked("MOVE-RIGHT|CURSOR-RIGHT") || pad[3];
            input.select = clicked("SELECT") || pad[4];
            input.exit = clicked("EXIT") || pad[5];
            input.attack = clicked("ATTACK") || pad[6];
            input.menu = clicked("MENU") || pad[7];
            input.craft = clicked("CRAFT");
            input.pickup = clicked("PICKUP") || pad[8];
            input.search = clicked("SEARCHER-BAR");
            input.page_up = clicked("PAGE-UP");
            input.page_down = clicked("PAGE-DOWN");
            input.backspace = clicked("BACKSPACE");
        }
        (input, std::mem::take(&mut self.raw_keys))
    }

    pub fn sample_keyboard(&mut self, window: &Window, bindings: &KeyBindings) {
        self.bindings = Some(bindings.clone());
        let name = |key| match key {
            Key::LeftShift | Key::RightShift => Some("SHIFT"),
            Key::LeftCtrl | Key::RightCtrl => Some("CTRL"),
            Key::LeftAlt | Key::RightAlt => Some("ALT"),
            _ => key_name(key),
        };
        let down: std::collections::BTreeSet<_> =
            window.get_keys().into_iter().filter_map(name).collect();
        for name in &down {
            self.keyboard
                .entry((*name).to_owned())
                .or_default()
                .set_down(true);
        }
        for (name, key) in &mut self.keyboard {
            if !down.contains(name.as_str()) {
                key.toggle(false);
            }
        }
        let fresh = window.get_keys_pressed(KeyRepeat::No);
        let shifted = down.contains("SHIFT");
        for key in window.get_keys_pressed(KeyRepeat::Yes) {
            // Initial text was already captured by the frame poll. Append
            // repeat events only, preserving their event order across frames.
            if !fresh.contains(&key)
                && let Some(character) = text_character(key, shifted)
            {
                self.input.text.push(character);
            }
            if let Some(name) = name(key) {
                let state = self.keyboard.entry(name.to_owned()).or_default();
                state.toggle(true);
                if !down.contains(name) {
                    state.toggle(false);
                }
            }
        }
    }
}

pub use crate::io::Input;

pub struct Gamepad {
    previous_buttons: u16,
    previous_directions: u8,
    previous_triggers: u8,
    backend: gamepad_platform::Backend,
}

impl Default for Gamepad {
    fn default() -> Self {
        Self {
            previous_buttons: 0,
            previous_directions: 0,
            previous_triggers: 0,
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
        let triggers = u8::from(state.left_trigger) | (u8::from(state.right_trigger) << 1);
        let trigger_pressed = triggers & !self.previous_triggers;
        let newly_pressed = state.buttons & !self.previous_buttons;
        self.previous_buttons = state.buttons;
        self.previous_directions = directions;
        self.previous_triggers = triggers;
        GamepadInput {
            up: state.up,
            down: state.down,
            left: state.left,
            right: state.right,
            up_pressed: pressed & 1 != 0,
            down_pressed: pressed & 2 != 0,
            // Java Display accepts both cursor directions and triggers when
            // moving between sibling menus such as player/chest.
            left_pressed: pressed & 4 != 0 || trigger_pressed & 1 != 0,
            right_pressed: pressed & 8 != 0 || trigger_pressed & 2 != 0,
            select: newly_pressed & gamepad_platform::A != 0,
            exit: newly_pressed & (gamepad_platform::B | gamepad_platform::START) != 0,
            // Java InputHandler.buttonMap: ATTACK=A, MENU=X, CRAFT=Y, PICKUP=LEFTBUMPER.
            // START is PAUSE (also treated as exit for open/close pause in this host).
            attack: newly_pressed & gamepad_platform::A != 0,
            menu: newly_pressed & gamepad_platform::X != 0,
            craft: newly_pressed & gamepad_platform::Y != 0,
            pickup: newly_pressed & gamepad_platform::LEFT_SHOULDER != 0,
        }
    }
}

fn binding_down(window: &Window, binding: &str) -> bool {
    binding_state(window, binding).down
}

fn binding_pressed(window: &Window, binding: &str) -> bool {
    binding_state(window, binding).clicked
}

fn binding_state(window: &Window, binding: &str) -> crate::io::input::KeyState {
    use crate::io::input::{KeyState, mapped_state};
    mapped_state(binding, |name| {
        let modifiers = match name {
            "SHIFT" => Some((Key::LeftShift, Key::RightShift)),
            "CTRL" => Some((Key::LeftCtrl, Key::RightCtrl)),
            "ALT" => Some((Key::LeftAlt, Key::RightAlt)),
            _ => None,
        };
        if let Some((left, right)) = modifiers {
            KeyState {
                down: window.is_key_down(left) || window.is_key_down(right),
                clicked: pressed(window, left) || pressed(window, right),
            }
        } else {
            key_name_from_key(name).map_or_else(KeyState::default, |key| KeyState {
                down: window.is_key_down(key),
                clicked: pressed(window, key),
            })
        }
    })
}

fn key_name_from_key(name: &str) -> Option<Key> {
    // Map settings key names back to minifb keys for frame preview.
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

/// Platform frame sample: minifb keyboard + gamepad → pure Input.
pub fn poll_input(
    window: &Window,
    bindings: &KeyBindings,
    raw_keys: &[Key],
    gamepad: &mut Gamepad,
) -> crate::io::Input {
    let shifted = window.is_key_down(Key::LeftShift) || window.is_key_down(Key::RightShift);
    let text = raw_keys
        .iter()
        .copied()
        .filter_map(|key| text_character(key, shifted))
        .collect();
    let pad = gamepad.poll();
    crate::io::Input {
        controller_edges: [
            pad.up_pressed,
            pad.down_pressed,
            pad.left_pressed,
            pad.right_pressed,
            pad.select,
            pad.exit,
            pad.attack,
            pad.menu,
            pad.pickup,
        ],
        shift: shifted,
        ctrl: window.is_key_down(Key::LeftCtrl) || window.is_key_down(Key::RightCtrl),
        alt: window.is_key_down(Key::LeftAlt) || window.is_key_down(Key::RightAlt),
        toggle_hud: binding_pressed(window, &bindings.toggle_hud),
        screenshot: binding_pressed(window, &bindings.screenshot),
        fullscreen: binding_pressed(window, &bindings.fullscreen),
        // Movement uses only the stored MOVE-* keymap (Java defaults UP|W etc.).
        // Overwriting a binding removes the arrow alias, so do not hardcode it.
        up: binding_down(window, &bindings.up) || pad.up,
        down: binding_down(window, &bindings.down) || pad.down,
        left: binding_down(window, &bindings.left) || pad.left,
        right: binding_down(window, &bindings.right) || pad.right,
        up_pressed: binding_pressed(window, &bindings.up) || pad.up_pressed,
        down_pressed: binding_pressed(window, &bindings.down) || pad.down_pressed,
        left_pressed: binding_pressed(window, &bindings.left) || pad.left_pressed,
        right_pressed: binding_pressed(window, &bindings.right) || pad.right_pressed,
        // Menus listen only to CURSOR-* (Java cursor-up/down = arrows only).
        cursor_up_pressed: binding_pressed(window, "UP") || pad.up_pressed,
        cursor_down_pressed: binding_pressed(window, "DOWN") || pad.down_pressed,
        cursor_left_pressed: binding_pressed(window, "LEFT") || pad.left_pressed,
        cursor_right_pressed: binding_pressed(window, "RIGHT") || pad.right_pressed,
        select: binding_pressed(window, &bindings.select) || pad.select,
        exit: binding_pressed(window, &bindings.exit) || pad.exit,
        attack: binding_pressed(window, &bindings.attack) || pad.attack,
        menu: binding_pressed(window, &bindings.menu) || pad.menu,
        craft: binding_pressed(window, &bindings.craft) || pad.craft,
        pickup: binding_pressed(window, &bindings.pickup) || pad.pickup,
        search: binding_pressed(window, &bindings.searcher_bar),
        page_up: binding_pressed(window, &bindings.page_up),
        page_down: binding_pressed(window, &bindings.page_down),
        world_copy: shifted && raw_keys.contains(&Key::C),
        world_rename: shifted && raw_keys.contains(&Key::R),
        world_delete: shifted && raw_keys.contains(&Key::D),
        backspace: pressed(window, Key::Backspace),
        text,
    }
}

#[cfg(test)]
mod tests {
    use crate::io::Input;
    use super::{Gamepad, InputBuffer, RawGamepadState, gamepad_platform};

    #[test]
    fn pressed_inputs_are_latched_until_the_next_fixed_tick() {
        let mut latch = Input::default();
        latch.latch(Input {
            up: true,
            attack: true,
            craft: true,
            text: vec!['A'],
            ..Input::default()
        });
        latch.latch(Input {
            up: true,
            ..Input::default()
        });
        let tick = latch.take_tick();
        assert!(tick.up && tick.attack && tick.craft);
        assert_eq!(tick.text, vec!['A']);
        let next = latch.take_tick();
        assert!(next.up);
        assert!(!next.attack);
        assert!(!next.craft);
        assert!(next.text.is_empty());
    }

    #[test]
    fn shift_is_held_across_ticks_but_not_latched_after_release() {
        let mut latch = Input::default();
        latch.latch(Input {
            shift: true,
            down_pressed: true,
            ..Input::default()
        });
        let first = latch.take_tick();
        assert!(first.shift && first.down_pressed);
        let held = latch.take_tick();
        assert!(held.shift && !held.down_pressed);
        latch.latch(Input::default());
        assert!(!latch.take_tick().shift);
    }

    #[test]
    fn controller_buttons_and_directions_are_edge_triggered() {
        let mut gamepad = Gamepad::default();
        let first = gamepad.apply(RawGamepadState {
            buttons: gamepad_platform::A | gamepad_platform::X,
            up: true,
            ..RawGamepadState::default()
        });
        // Java: A=select+attack, X=menu; directions from DPAD.
        assert!(first.up && first.up_pressed && first.select && first.attack && first.menu);

        let held = gamepad.apply(RawGamepadState {
            buttons: gamepad_platform::A | gamepad_platform::X,
            up: true,
            ..RawGamepadState::default()
        });
        assert!(held.up);
        assert!(!held.up_pressed && !held.select && !held.attack && !held.menu);

        gamepad.apply(RawGamepadState::default());
        let craft = gamepad.apply(RawGamepadState {
            buttons: gamepad_platform::Y,
            right: true,
            ..RawGamepadState::default()
        });
        assert!(craft.right_pressed && craft.craft && !craft.menu);

        gamepad.apply(RawGamepadState::default());
        let triggers = gamepad.apply(RawGamepadState {
            left_trigger: true,
            right_trigger: true,
            ..RawGamepadState::default()
        });
        assert!(triggers.left_pressed && triggers.right_pressed);
        let held_triggers = gamepad.apply(RawGamepadState {
            left_trigger: true,
            right_trigger: true,
            ..RawGamepadState::default()
        });
        assert!(!held_triggers.left_pressed && !held_triggers.right_pressed);
    }

    #[test]
    fn focus_loss_releases_all_physical_keys_like_java_release_all() {
        use crate::io::KeyBindings;
        let mut buffer = InputBuffer::default();
        buffer.bindings = Some(KeyBindings::default());
        buffer.keyboard.insert(
            "W".to_owned(),
            crate::io::input::PhysicalKey::default(),
        );
        buffer.keyboard.insert(
            "ESCAPE".to_owned(),
            crate::io::input::PhysicalKey::default(),
        );
        {
            let key = buffer.keyboard.get_mut("W").expect("W");
            key.toggle(true);
            key.tick();
        }
        {
            let key = buffer.keyboard.get_mut("ESCAPE").expect("ESC");
            key.toggle(true);
            key.tick();
        }
        assert!(buffer.keyboard["W"].state().down);
        assert!(buffer.keyboard["ESCAPE"].state().clicked);

        buffer.frame(false, Input::default(), Vec::new());
        assert!(!buffer.keyboard["W"].state().down);
        assert!(!buffer.keyboard["W"].state().clicked);
        assert!(!buffer.keyboard["ESCAPE"].state().down);
        assert!(!buffer.keyboard["ESCAPE"].state().clicked);

        let (input, raw) = buffer.tick();
        assert!(!input.up && !input.exit && raw.is_empty());
    }

    #[test]
    fn menu_search_and_page_keys_come_from_java_bindings() {
        use crate::io::KeyBindings;
        let mut buffer = InputBuffer::default();
        buffer.bindings = Some(KeyBindings::default());
        for name in ["F", "PAGE_UP", "PAGE_DOWN", "SHIFT"] {
            buffer
                .keyboard
                .insert(name.to_owned(), crate::io::input::PhysicalKey::default());
        }
        // sample_keyboard only toggles; InputBuffer.tick consumes the presses.
        for name in ["F", "PAGE_UP", "PAGE_DOWN", "SHIFT"] {
            let key = buffer.keyboard.get_mut(name).expect("key");
            key.toggle(true);
        }
        let (input, _) = buffer.tick();
        assert!(input.search, "SEARCHER-BAR default is SHIFT-F");
        assert!(input.page_up);
        assert!(input.page_down);
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
    craft: bool,
    pickup: bool,
}

#[derive(Default)]
struct RawGamepadState {
    buttons: u16,
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    left_trigger: bool,
    right_trigger: bool,
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
    const TRIGGER_THRESHOLD: u8 = 30;

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
                left_trigger: pad.left_trigger > TRIGGER_THRESHOLD,
                right_trigger: pad.right_trigger > TRIGGER_THRESHOLD,
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
            let mut mappings = std::io::Cursor::new(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/gamecontrollerdb.txt"
            )));
            let _ = subsystem.load_mappings_from_read(&mut mappings);
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
                left_trigger: controller.axis(Axis::TriggerLeft) > STICK_THRESHOLD,
                right_trigger: controller.axis(Axis::TriggerRight) > STICK_THRESHOLD,
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
