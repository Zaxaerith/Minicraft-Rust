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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn pressed_map(pressed: &[&str]) -> impl FnMut(&str) -> KeyState {
        let held: HashMap<String, KeyState> = pressed
            .iter()
            .map(|name| {
                (
                    (*name).to_owned(),
                    KeyState {
                        down: true,
                        clicked: true,
                    },
                )
            })
            .collect();
        move |name: &str| held.get(name).copied().unwrap_or_default()
    }

    #[test]
    fn physical_key_tick_matches_java_presses_absorbs_sticky() {
        let mut key = PhysicalKey::default();
        key.toggle(true);
        key.tick();
        assert!(key.state().down && key.state().clicked);
        key.tick();
        // Still down, but the single press was consumed.
        assert!(key.state().down && !key.state().clicked);
        key.toggle(false);
        key.tick();
        assert!(!key.state().down && !key.state().clicked);

        // Held long enough for sticky repeat (presses > 3), then one idle tick
        // to latch sticky the way Java does after the press queue drains.
        let mut held = PhysicalKey::default();
        for _ in 0..5 {
            held.toggle(true);
            held.tick();
        }
        held.tick();
        assert!(held.state().clicked, "sticky key keeps repeating");
        held.toggle(false);
        held.tick();
        assert!(!held.state().clicked);
    }

    #[test]
    fn release_clears_down_clicked_and_queues_like_java_release_all() {
        let mut key = PhysicalKey::default();
        key.toggle(true);
        key.tick();
        assert!(key.state().down && key.state().clicked);
        key.release();
        let state = key.state();
        assert!(!state.down && !state.clicked);
        key.tick();
        let state = key.state();
        assert!(!state.down && !state.clicked);
    }

    #[test]
    fn binding_state_ors_alternatives_and_ands_compound_modifiers() {
        let state = binding_state("UP|W", pressed_map(&["W"]));
        assert!(state.down && state.clicked);

        let neither = binding_state("UP|W", pressed_map(&["A"]));
        assert!(!neither.down && !neither.clicked);

        let compound = binding_state("SHIFT-E", pressed_map(&["SHIFT", "E"]));
        assert!(compound.down && compound.clicked);

        let half = binding_state("SHIFT-E", pressed_map(&["E"]));
        assert!(!half.down);
        // Single-key clicked may fire after release; compound requires all down.
        let released_mod = binding_state("SHIFT-E", |name| match name {
            "SHIFT" => KeyState::default(),
            "E" => KeyState {
                down: false,
                clicked: true,
            },
            _ => KeyState::default(),
        });
        assert!(!released_mod.down && !released_mod.clicked);
    }

    #[test]
    fn mapped_state_requires_down_for_clicked_like_java_compounded_key() {
        let single = mapped_state("W", |name| {
            if name == "W" {
                KeyState {
                    down: false,
                    clicked: true,
                }
            } else {
                KeyState::default()
            }
        });
        // getMappedKey wraps even one physical key in CompoundedKey.
        assert!(!single.down && !single.clicked);

        let plus = mapped_state("SHIFT+E", pressed_map(&["SHIFT", "E"]));
        assert!(plus.down && plus.clicked);
    }

    #[test]
    fn cursor_and_move_edges_stay_separated() {
        let bindings = crate::io::KeyBindings::default();
        let physical = |name: &str| {
            if name == "W" || name == "UP" {
                KeyState {
                    down: true,
                    clicked: true,
                }
            } else {
                KeyState::default()
            }
        };
        let movement = action_state("MOVE-UP", |action| bindings.mapping(action), physical);
        assert!(movement.down && movement.clicked);

        // CURSOR-UP is arrow-only: W must not satisfy a cursor query.
        let cursor_on_w = action_state(
            "CURSOR-UP",
            |action| bindings.mapping(action),
            |name| {
                if name == "W" {
                    KeyState {
                        down: true,
                        clicked: true,
                    }
                } else {
                    KeyState::default()
                }
            },
        );
        assert!(!cursor_on_w.down && !cursor_on_w.clicked);

        let cursor_on_arrow = action_state(
            "CURSOR-UP",
            |action| bindings.mapping(action),
            |name| {
                if name == "UP" {
                    KeyState {
                        down: true,
                        clicked: true,
                    }
                } else {
                    KeyState::default()
                }
            },
        );
        assert!(cursor_on_arrow.down && cursor_on_arrow.clicked);
    }

    #[test]
    fn remapped_move_drops_hardcoded_arrow_aliases() {
        let mut bindings = crate::io::KeyBindings::default();
        bindings.set(0, "I".to_owned());
        let arrow = |name: &str| {
            if name == "UP" {
                KeyState {
                    down: true,
                    clicked: true,
                }
            } else {
                KeyState::default()
            }
        };
        let remapped = action_state("MOVE-UP", |action| bindings.mapping(action), arrow);
        assert!(!remapped.down, "UP is no longer bound after overwrite");
        let letter = action_state(
            "MOVE-UP",
            |action| bindings.mapping(action),
            |name| {
                if name == "I" {
                    KeyState {
                        down: true,
                        clicked: true,
                    }
                } else {
                    KeyState::default()
                }
            },
        );
        assert!(letter.down && letter.clicked);
    }
}
