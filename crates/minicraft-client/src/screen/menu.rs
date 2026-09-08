//! Java Menu.tick / SelectEntry.tick for a vertical list of selectable entries.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MenuEvent {
    None,
    Moved,
    Activated,
    Closed,
}

pub fn tick_selectable(
    selection: &mut usize,
    count: usize,
    up: bool,
    down: bool,
    select: bool,
    exit: bool,
) -> MenuEvent {
    if exit {
        return MenuEvent::Closed;
    }
    if count == 0 {
        return MenuEvent::None;
    }
    let delta = i32::from(down) - i32::from(up);
    if delta != 0 {
        *selection = (*selection as i32 + delta).rem_euclid(count as i32) as usize;
        return MenuEvent::Moved;
    }
    if select {
        MenuEvent::Activated
    } else {
        MenuEvent::None
    }
}
