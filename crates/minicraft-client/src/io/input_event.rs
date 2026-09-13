//! Pure input snapshot consumed by World tick (platform sampling stays outside).
#[derive(Default, Clone)]
pub struct Input {
    /// Controller-only edges: up/down/left/right/select/exit/attack/menu/pickup.
    pub controller_edges: [bool; 9],
    pub toggle_hud: bool,
    pub screenshot: bool,
    pub fullscreen: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub up_pressed: bool,
    pub down_pressed: bool,
    pub left_pressed: bool,
    pub right_pressed: bool,
    /// Java cursor-up/down/left/right only (WASD is player move, not menus).
    pub cursor_up_pressed: bool,
    pub cursor_down_pressed: bool,
    pub cursor_left_pressed: bool,
    pub cursor_right_pressed: bool,
    pub select: bool,
    pub exit: bool,
    pub attack: bool,
    pub menu: bool,
    pub craft: bool,
    pub pickup: bool,
    /// Java SEARCHER-BAR / PAGE-UP / PAGE-DOWN menu actions.
    pub search: bool,
    pub page_up: bool,
    pub page_down: bool,
    pub world_copy: bool,
    pub world_rename: bool,
    pub world_delete: bool,
    pub backspace: bool,
    pub text: Vec<char>,
}

impl Input {
    pub fn latch(&mut self, latest: Self) {
        for (pending, latest) in self
            .controller_edges
            .iter_mut()
            .zip(latest.controller_edges)
        {
            *pending |= latest;
        }
        self.toggle_hud |= latest.toggle_hud;
        self.screenshot |= latest.screenshot;
        self.fullscreen |= latest.fullscreen;
        self.shift = latest.shift;
        self.ctrl = latest.ctrl;
        self.alt = latest.alt;
        self.up = latest.up;
        self.down = latest.down;
        self.left = latest.left;
        self.right = latest.right;
        self.up_pressed |= latest.up_pressed;
        self.down_pressed |= latest.down_pressed;
        self.left_pressed |= latest.left_pressed;
        self.right_pressed |= latest.right_pressed;
        self.cursor_up_pressed |= latest.cursor_up_pressed;
        self.cursor_down_pressed |= latest.cursor_down_pressed;
        self.cursor_left_pressed |= latest.cursor_left_pressed;
        self.cursor_right_pressed |= latest.cursor_right_pressed;
        self.select |= latest.select;
        self.exit |= latest.exit;
        self.attack |= latest.attack;
        self.menu |= latest.menu;
        self.craft |= latest.craft;
        self.pickup |= latest.pickup;
        self.search |= latest.search;
        self.page_up |= latest.page_up;
        self.page_down |= latest.page_down;
        self.world_copy |= latest.world_copy;
        self.world_rename |= latest.world_rename;
        self.world_delete |= latest.world_delete;
        self.backspace |= latest.backspace;
        self.text.extend(latest.text);
    }

    pub fn take_tick(&mut self) -> Self {
        let tick = self.clone();
        self.controller_edges.fill(false);
        self.toggle_hud = false;
        self.screenshot = false;
        self.fullscreen = false;
        self.up_pressed = false;
        self.down_pressed = false;
        self.left_pressed = false;
        self.right_pressed = false;
        self.cursor_up_pressed = false;
        self.cursor_down_pressed = false;
        self.cursor_left_pressed = false;
        self.cursor_right_pressed = false;
        self.select = false;
        self.exit = false;
        self.attack = false;
        self.menu = false;
        self.craft = false;
        self.pickup = false;
        self.search = false;
        self.page_up = false;
        self.page_down = false;
        self.world_copy = false;
        self.world_rename = false;
        self.world_delete = false;
        self.backspace = false;
        self.text.clear();
        tick
    }
}
