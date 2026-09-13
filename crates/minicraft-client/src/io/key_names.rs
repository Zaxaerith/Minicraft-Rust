//! Named physical keys accepted by settings JSON (platform-agnostic).
//!
//! Mirrors the subset of Java `KeyEvent.VK_*` names that `InputHandler`
//! actually binds (letters, digits, arrows, function keys, editing keys).
pub fn key_name_exists(name: &str) -> bool {
    matches!(
        name,
        "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M" | "N" | "O"
            | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z" | "0" | "1" | "2"
            | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "UP" | "DOWN" | "LEFT" | "RIGHT" | "ENTER"
            | "SPACE" | "ESCAPE" | "LEFT SHIFT" | "RIGHT SHIFT" | "LEFT CTRL" | "RIGHT CTRL"
            | "TAB" | "BACKSPACE" | "DELETE" | "HOME" | "END" | "INSERT" | "PAGE_UP" | "PAGE_DOWN"
            | "F1" | "F2" | "F3" | "F4" | "F5" | "F6" | "F7" | "F8" | "F9" | "F10" | "F11"
            | "F12" | "F13" | "F14" | "F15" | "F16" | "F17" | "F18" | "F19" | "F20" | "F21"
            | "F22" | "F23" | "F24"
    )
}

#[cfg(test)]
mod tests {
    use super::key_name_exists;

    #[test]
    fn accepts_java_default_binding_keys() {
        for name in [
            "UP", "W", "DOWN", "S", "LEFT", "A", "RIGHT", "D", "ENTER", "ESCAPE", "C", "SPACE",
            "X", "E", "Z", "V", "P", "Q", "F", "R", "O", "L", "I", "F1", "F2", "F11", "PAGE_UP",
            "PAGE_DOWN",
        ] {
            assert!(key_name_exists(name), "{name}");
        }
    }

    #[test]
    fn rejects_unknown_tokens() {
        assert!(!key_name_exists(""));
        assert!(!key_name_exists("SHIFT-E"));
        assert!(!key_name_exists("NOT_A_KEY"));
    }
}
