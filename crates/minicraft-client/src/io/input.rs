//! Java InputHandler getKey / ORKey / CompoundedKey evaluation.
#[derive(Clone, Copy, Default)]
pub struct KeyState {
    pub down: bool,
    pub clicked: bool,
}

/// PhysicalKey's queued presses and sticky repeat, advanced by logical ticks.
#[derive(Default)]
pub struct PhysicalKey {
    presses: i32,
    absorbs: i32,
    sticky: bool,
    state: KeyState,
}

impl PhysicalKey {
    pub fn state(&self) -> KeyState {
        self.state
    }
    /// Platform held-state synchronization must not synthesize a press event.
    pub fn set_down(&mut self, down: bool) {
        self.state.down = down;
    }
    pub fn toggle(&mut self, down: bool) {
        self.state.down = down;
        if down && !self.sticky {
            self.presses = self.presses.wrapping_add(1);
        }
    }
    pub fn tick(&mut self) {
        if self.absorbs < self.presses {
            self.absorbs = self.absorbs.wrapping_add(1);
            if self.presses.wrapping_sub(self.absorbs) > 3 {
                self.absorbs = self.presses.wrapping_sub(3);
            }
            self.state.clicked = true;
        } else {
            self.sticky = if self.sticky {
                self.state.down
            } else {
                self.presses > 3
            };
            self.state.clicked = self.sticky;
            self.presses = 0;
            self.absorbs = 0;
        }
    }
    pub fn release(&mut self) {
        *self = Self::default();
    }
}

pub fn binding_state(binding: &str, mut physical: impl FnMut(&str) -> KeyState) -> KeyState {
    let binding = binding.to_ascii_uppercase();
    let mut result = KeyState::default();
    for alternative in binding.split('|').filter(|value| !value.is_empty()) {
        let mut down = true;
        let mut clicked = false;
        let mut count = 0;
        for name in alternative.split('-') {
            let key = physical(name);
            down &= key.down;
            clicked |= key.clicked;
            count += 1;
        }
        result.down |= down;
        // A single PhysicalKey may retain a click after release. Compound
        // keys require every constituent down, per Java CompoundedKey.
        result.clicked |= clicked && (count == 1 || down);
    }
    result
}

/// getMappedKey wraps even a single physical key in CompoundedKey. The `+`
/// operator combines simultaneously held mappings, not a temporal sequence.
pub fn mapped_state(binding: &str, mut physical: impl FnMut(&str) -> KeyState) -> KeyState {
    let mut result = KeyState::default();
    for alternative in binding.split('|').filter(|part| !part.is_empty()) {
        let mut down = true;
        let mut clicked = false;
        for member in alternative.split('+') {
            let state = binding_state(member, &mut physical);
            down &= state.down;
            clicked |= state.clicked;
        }
        result.down |= down;
        result.clicked |= down && clicked;
    }
    result
}

/// InputHandler.getMappedKey action substitution and OR/compound precedence.
/// Reject recursive alias cycles rather than exhausting the Rust stack.
pub fn action_state(
    query: &str,
    mapping: impl Fn(&str) -> Option<String>,
    mut physical: impl FnMut(&str) -> KeyState,
) -> KeyState {
    fn resolve(
        query: &str,
        mapping: &impl Fn(&str) -> Option<String>,
        physical: &mut impl FnMut(&str) -> KeyState,
        depth: usize,
    ) -> KeyState {
        if depth > 32 {
            return KeyState::default();
        }
        let query = query.to_ascii_uppercase();
        let expanded = mapping(&query).unwrap_or(query);
        if expanded.contains('|') {
            let mut result = KeyState::default();
            for alternative in expanded.split('|') {
                let state = resolve(alternative, mapping, physical, depth + 1);
                result.down |= state.down;
                result.clicked |= state.clicked;
            }
            return result;
        }
        let mut down = true;
        let mut clicked = false;
        for member in expanded.split('+') {
            let binding = mapping(member).unwrap_or_else(|| member.to_owned());
            let state = binding_state(&binding, &mut *physical);
            down &= state.down;
            clicked |= state.clicked;
        }
        KeyState {
            down,
            clicked: down && clicked,
        }
    }
    resolve(query, &mapping, &mut physical, 0)
}
