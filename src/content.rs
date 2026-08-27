use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub enum Book {
    Instructions,
    GameGuide,
    About,
    Credits,
}

impl Book {
    pub const ALL: [Self; 4] = [
        Self::Instructions,
        Self::GameGuide,
        Self::About,
        Self::Credits,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Self::Instructions => "INSTRUCTIONS",
            Self::GameGuide => "GAME GUIDE",
            Self::About => "ABOUT",
            Self::Credits => "CREDITS",
        }
    }

    fn text(self) -> &'static str {
        match self {
            Self::Instructions => include_str!("../assets/assets/books/instructions.txt"),
            Self::GameGuide => include_str!("../assets/assets/books/game_guide.txt"),
            Self::About => include_str!("../assets/assets/books/about.txt"),
            Self::Credits => include_str!("../assets/assets/books/credits.txt"),
        }
    }

    pub fn pages(self) -> Vec<Vec<String>> {
        paginate(self.text(), 32, 14)
    }
}

#[derive(Debug, Clone)]
pub struct Achievement {
    pub id: String,
    pub description: String,
    pub score: u32,
}

pub fn achievements() -> Result<Vec<Achievement>, String> {
    let value: Value = serde_json::from_str(include_str!("../assets/resources/achievements.json"))
        .map_err(|error| format!("cannot parse bundled achievements: {error}"))?;
    value
        .as_array()
        .ok_or_else(|| "bundled achievements must be an array".to_owned())?
        .iter()
        .map(|entry| {
            Ok(Achievement {
                id: entry["id"]
                    .as_str()
                    .ok_or_else(|| "achievement id is missing".to_owned())?
                    .to_owned(),
                description: entry["desc"]
                    .as_str()
                    .ok_or_else(|| "achievement description is missing".to_owned())?
                    .to_owned(),
                score: entry["score"].as_u64().unwrap_or(0) as u32,
            })
        })
        .collect()
}

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
    use super::{Book, achievements, paginate};

    #[test]
    fn bundled_content_is_valid_and_paginated() {
        assert_eq!(achievements().unwrap().len(), 17);
        assert!(Book::GameGuide.pages().len() > 1);
        let pages = paginate("one two three four", 7, 2);
        assert_eq!(pages, vec![vec!["one two", "three"], vec!["four"]]);
    }
}
