//! Named physical keys accepted by settings JSON (platform-agnostic).
pub fn key_name_exists(name: &str) -> bool {
    matches!(
        name,
        "A" | "B" | "C" | "D" | "E" | "F" | "G" | "H" | "I" | "J" | "K" | "L" | "M" | "N" | "O"
            | "P" | "Q" | "R" | "S" | "T" | "U" | "V" | "W" | "X" | "Y" | "Z" | "0" | "1" | "2"
            | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "UP" | "DOWN" | "LEFT" | "RIGHT" | "ENTER"
            | "SPACE" | "ESCAPE" | "LEFT SHIFT" | "RIGHT SHIFT" | "LEFT CTRL" | "RIGHT CTRL"
            | "TAB" | "BACKSPACE"
    )
}
