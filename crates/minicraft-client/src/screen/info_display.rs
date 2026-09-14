//! Java `InfoDisplay.java` — play-time / score info popup.
use crate::gfx::color;

/// Java InfoDisplay frame builder constants.
pub const SPACING: i32 = 4;
/// Java MinicraftImage.boxWidth for the TOP_LEFT anchor.
pub const ANCHOR: (i32, i32) = (8, 8);

/// Java InfoDisplay.getTimeString: hours+minutes, else minutes+seconds.
/// `game_time` is Updater.gameTime; `norm_speed` is Updater.normSpeed.
pub fn time_string(game_time: u64, norm_speed: u32) -> String {
    let norm_speed = norm_speed.max(1) as u64;
    let seconds_total = game_time / norm_speed;
    let minutes_total = seconds_total / 60;
    let hours = minutes_total / 60;
    let minutes = (minutes_total % 60) as u32;
    let seconds = (seconds_total % 60) as u32;
    if hours > 0 {
        format!("{hours}h{:02}m", minutes)
    } else {
        format!("{minutes}m {seconds:02}s")
    }
}

/// Separator line used above and below the info body.
pub const SEPARATOR: &str = "----------------------------";

/// Java body rows: separator, time, score, separator, exit help.
pub fn body_lines(time: &str, score: i32, exit_help: &str) -> Vec<String> {
    vec![
        SEPARATOR.to_owned(),
        format!("Time: {time}"),
        format!("Score: {score}"),
        SEPARATOR.to_owned(),
        exit_help.to_owned(),
    ]
}

/// InfoDisplay title color is WHITE (frame titles default YELLOW when framed).
pub const TITLE_COLOR: u32 = color::YELLOW_RGB;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_string_matches_java_formatting() {
        // 0 game-time ticks → 0m 00s
        assert_eq!(time_string(0, 60), "0m 00s");
        // 60 norm-speed ticks = 1 second
        assert_eq!(time_string(60, 60), "0m 01s");
        // 60*60 ticks = 1 minute
        assert_eq!(time_string(60 * 60, 60), "1m 00s");
        // 3 hours
        assert_eq!(time_string(3 * 60 * 60 * 60, 60), "3h00m");
        // padding for single-digit minutes/seconds
        assert_eq!(time_string(60 * 65, 60), "1m 05s");
    }

    #[test]
    fn body_lines_keep_java_order() {
        let lines = body_lines("1m 00s", 42, "select/exit");
        assert_eq!(lines[0], SEPARATOR);
        assert_eq!(lines[1], "Time: 1m 00s");
        assert_eq!(lines[2], "Score: 42");
        assert_eq!(lines[3], SEPARATOR);
        assert_eq!(lines[4], "select/exit");
    }
}
