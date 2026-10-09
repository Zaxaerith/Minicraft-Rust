//! Java `util/MyUtils.java` — clamp / plural / capitalize helpers.
//!
//! `paginate` lives here because it is the shared text-wrapping utility used
//! by book pages and UI description wrapping (Java callers wrap via font metrics;
//! this char-grid form is the Rust host equivalent already used by menus).

/// Java `MyUtils.clamp`.
pub fn clamp(val: i32, min: i32, max: i32) -> i32 {
    if val > max {
        max
    } else if val < min {
        min
    } else {
        val
    }
}

/// Java `MyUtils.plural(num, word)` — always appends `s` when `num != 1`.
pub fn plural(num: i32, word: &str) -> String {
    let suffix = if num == 1 { "" } else { "s" };
    format!("{num} {word}{suffix}")
}

/// Java `MyUtils.capitalize` (Apache WordUtils.capitalize).
/// Capitalizes the first non-whitespace character after each whitespace run
/// using Unicode title case (ASCII title case == upper case).
pub fn capitalize(text: &str) -> String {
    if text.is_empty() {
        return text.to_owned();
    }
    let mut buffer = String::with_capacity(text.len());
    let mut capitalize_next = true;
    for ch in text.chars() {
        if ch.is_whitespace() {
            buffer.push(ch);
            capitalize_next = true;
        } else if capitalize_next {
            for titled in title_case(ch) {
                buffer.push(titled);
            }
            capitalize_next = false;
        } else {
            buffer.push(ch);
        }
    }
    buffer
}

/// Java `MyUtils.capitalizeFully` — lowercase then capitalize.
pub fn capitalize_fully(text: &str) -> String {
    if text.is_empty() {
        return text.to_owned();
    }
    capitalize(&text.to_lowercase())
}

/// Java `Character.toTitleCase` approximation via Unicode uppercase mapping.
fn title_case(ch: char) -> impl Iterator<Item = char> {
    ch.to_uppercase()
}

/// Wrap `text` into pages of `rows` lines, each line at most `columns` chars.
/// Empty paragraphs become empty lines; long words hard-split on the column bound.
pub fn paginate(text: &str, columns: usize, rows: usize) -> Vec<Vec<String>> {
    let mut lines = Vec::new();
    for paragraph in text.replace('\r', "").lines() {
        if paragraph.trim().is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            if word.chars().count() > columns {
                if !line.is_empty() {
                    lines.push(std::mem::take(&mut line));
                }
                let characters: Vec<char> = word.chars().collect();
                for chunk in characters.chunks(columns) {
                    lines.push(chunk.iter().collect());
                }
            } else if line.is_empty() {
                line.push_str(word);
            } else if line.chars().count() + 1 + word.chars().count() <= columns {
                line.push(' ');
                line.push_str(word);
            } else {
                lines.push(std::mem::take(&mut line));
                line.push_str(word);
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines.chunks(rows).map(|chunk| chunk.to_vec()).collect()
}

#[cfg(test)]
mod tests {
    use super::{capitalize, capitalize_fully, clamp, paginate, plural};

    #[test]
    fn clamp_matches_java() {
        assert_eq!(clamp(5, 0, 2), 2);
        assert_eq!(clamp(-1, 0, 2), 0);
        assert_eq!(clamp(1, 0, 2), 1);
    }

    #[test]
    fn plural_matches_java() {
        assert_eq!(plural(1, "item"), "1 item");
        assert_eq!(plural(2, "item"), "2 items");
        assert_eq!(plural(0, "item"), "0 items");
    }

    #[test]
    fn capitalize_matches_java_wordutils() {
        assert_eq!(capitalize("i am fine"), "I Am Fine");
        // Java WordUtils.capitalize only splits on whitespace — "aM.fine" is one word.
        assert_eq!(capitalize("i aM.fine"), "I AM.fine");
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize_fully("i aM.fine"), "I Am.fine");
        assert_eq!(capitalize_fully("LIGHT_GRAY"), "Light_gray");
    }

    #[test]
    fn paginate_wraps_like_existing_host() {
        assert_eq!(
            paginate("one two three four", 7, 2),
            vec![
                vec!["one two".to_owned(), "three".to_owned()],
                vec!["four".to_owned()]
            ]
        );
    }
}
