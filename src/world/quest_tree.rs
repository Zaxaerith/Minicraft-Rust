//! SeriesQuestViewerDisplay layout, cursor and raster rules from Java 2.2.4.
//! This adapter stays independent of World mutation and audio device delivery.

use std::collections::HashMap;

use crate::{
    content::QuestView,
    gfx::{Image, Screen, WIDTH},
    input::Input,
};

pub(super) const GREEN: u32 = 0x4DD44D;
const GRAY: u32 = 0x999999;
const RASTER_WIDTH: i32 = 256;
const RASTER_HEIGHT: i32 = 116;

#[derive(Debug, Clone, Default)]
pub(super) struct State {
    pub x_scroll: i32,
    pub y_scroll: i32,
    pub browsing: bool,
    // Runtime-only localized geometry; never serialized as gameplay data.
    pub labels: HashMap<String, String>,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Action {
    None,
    Moved,
    Open,
}

#[derive(Debug)]
struct Node {
    quest: usize,
    parent: Option<usize>,
    label: String,
    x: i32,
    y: i32,
    width: i32,
}

#[derive(Debug)]
pub(super) struct Tree {
    rows: Vec<Vec<Node>>,
}

impl Tree {
    pub fn new(quests: &[QuestView], label: impl Fn(&str) -> String) -> Self {
        let mut rows: Vec<Vec<Node>> = Vec::new();
        let mut remaining: Vec<_> = (0..quests.len()).collect();
        while !remaining.is_empty() {
            let indices: Vec<_> = remaining
                .iter()
                .copied()
                .filter(|&index| match &quests[index].parent {
                    None => rows.is_empty(),
                    Some(parent) => rows
                        .last()
                        .is_some_and(|row| row.iter().any(|node| quests[node.quest].id == *parent)),
                })
                .collect();
            // Broken resource data must not spin forever on orphans or cycles.
            if indices.is_empty() {
                break;
            }
            let mut x = 5;
            let mut row = Vec::new();
            for index in indices {
                remaining.retain(|&value| value != index);
                let label = label(&quests[index].id);
                let width = 6 + label.chars().count() as i32 * 8;
                row.push(Node {
                    quest: index,
                    parent: quests[index]
                        .parent
                        .as_ref()
                        .and_then(|id| quests.iter().position(|quest| quest.id == *id)),
                    label,
                    x,
                    y: 5 + rows.len() as i32 * 29,
                    width,
                });
                // Java initializes x=5 then translates by previous.right+5.
                x += width + 10;
            }
            // Java's identity HashSet sibling order is not stable across JVMs.
            // Preserve the resource/view order within a depth in this adapter.
            rows.push(row);
        }
        Self { rows }
    }

    pub fn first(&self) -> usize {
        self.rows
            .first()
            .and_then(|row| row.first())
            .map_or(0, |node| node.quest)
    }

    pub fn tick(&self, selection: &mut usize, state: &mut State, input: &Input) -> Action {
        let Some((mut y, mut x)) = self.rows.iter().enumerate().find_map(|(y, row)| {
            row.iter()
                .position(|node| node.quest == *selection)
                .map(|x| (y, x))
        }) else {
            return Action::None;
        };
        if input.shift {
            state.browsing = true;
            if input.down_pressed {
                state.y_scroll = state.y_scroll.saturating_add(3);
            } else if input.up_pressed {
                state.y_scroll = state.y_scroll.saturating_sub(3);
            } else if input.right_pressed {
                state.x_scroll = state.x_scroll.saturating_add(3);
            } else if input.left_pressed {
                state.x_scroll = state.x_scroll.saturating_sub(3);
            }
            return Action::None;
        }
        if state.browsing {
            self.rows[y][x].reveal(state);
            state.browsing = false;
        }
        // Java is an else-if chain: a blocked direction still consumes Select.
        if input.down_pressed {
            y = (y + 1).min(self.rows.len() - 1);
        } else if input.up_pressed {
            y = y.saturating_sub(1);
        } else if input.right_pressed {
            x = (x + 1).min(self.rows[y].len() - 1);
        } else if input.left_pressed {
            x = x.saturating_sub(1);
        } else if input.select {
            return Action::Open;
        }
        x = x.min(self.rows[y].len() - 1);
        let node = &self.rows[y][x];
        if *selection == node.quest {
            return Action::None;
        }
        *selection = node.quest;
        node.reveal(state);
        Action::Moved
    }

    pub fn render(
        &self,
        screen: &mut Screen,
        font: &Image,
        quests: &[QuestView],
        selection: usize,
        state: &State,
        empty_text: &str,
    ) {
        let mut raster = Screen::new();
        if self.rows.is_empty() {
            raster.text_colored(
                font,
                empty_text,
                RASTER_WIDTH / 2 - empty_text.chars().count() as i32 * 4,
                RASTER_HEIGHT / 2 - 4,
                GRAY,
            );
        }
        for node in self.rows.iter().flatten() {
            if let Some(parent) = node
                .parent
                .and_then(|index| self.rows.iter().flatten().find(|node| node.quest == index))
            {
                let color = if node.quest == selection {
                    0x5ACCCC
                } else if parent.quest == selection {
                    GREEN
                } else {
                    tint(GRAY, -1)
                };
                line(
                    (parent.x + parent.width / 2, parent.y + 7),
                    (node.x + node.width / 2, node.y + 7),
                    |x, y| {
                        if y >= parent.y + 14 && y <= node.y {
                            pixel(&mut raster, x - state.x_scroll, y - state.y_scroll, color);
                        }
                    },
                );
            }
        }
        for node in self.rows.iter().flatten() {
            let selected = node.quest == selection;
            let completed = quests[node.quest].completed;
            let text_color = if selected {
                if completed { tint(GREEN, 1) } else { 0xFFFFFF }
            } else {
                tint(GRAY, 1)
            };
            let border = if selected {
                if completed {
                    tint(GREEN, -1)
                } else {
                    tint(GRAY, 2)
                }
            } else {
                GRAY
            };
            let x = node.x - state.x_scroll;
            let y = node.y - state.y_scroll;
            raster.text_colored(font, &node.label, x + 3, y + 3, text_color);
            for dx in 0..node.width {
                pixel(&mut raster, x + dx, y, border);
                pixel(&mut raster, x + dx, y + 13, border);
            }
            for dy in 1..13 {
                pixel(&mut raster, x, y + dy, border);
                pixel(&mut raster, x + node.width - 1, y + dy, border);
            }
        }
        // Graphics.drawRect includes x+w/y+h; the raster overwrites its interior.
        screen.frame(15, 47, RASTER_WIDTH + 3, RASTER_HEIGHT + 3, 0xFFFFFF);
        for y in 0..RASTER_HEIGHT {
            for x in 0..RASTER_WIDTH {
                screen.rect(
                    16 + x,
                    48 + y,
                    1,
                    1,
                    raster.pixels()[x as usize + y as usize * WIDTH],
                );
            }
        }
    }
}

impl Node {
    fn reveal(&self, state: &mut State) {
        if state.x_scroll > self.x - 5 {
            state.x_scroll = self.x - 5;
        } else if state.x_scroll + RASTER_WIDTH < self.x + self.width + 5 {
            state.x_scroll = self.x + self.width + 5 - RASTER_WIDTH;
        }
        if state.y_scroll > self.y - 5 {
            state.y_scroll = self.y - 5;
        } else if state.y_scroll + RASTER_HEIGHT < self.y + 19 {
            state.y_scroll = self.y + 19 - RASTER_HEIGHT;
        }
    }
}

fn pixel(raster: &mut Screen, x: i32, y: i32, color: u32) {
    if (0..RASTER_WIDTH).contains(&x) && (0..RASTER_HEIGHT).contains(&y) {
        raster.rect(x, y, 1, 1, color);
    }
}

// Color.tint(..., true) decodes each byte as a base-6 palette entry, not RGB.
fn tint(color: u32, amount: i32) -> u32 {
    [0, 8, 16].into_iter().fold(0, |result, shift| {
        let value = ((color >> shift) & 255) as i32;
        let tinted = if value == 255 {
            255
        } else {
            ((value / 36 % 6 + amount).clamp(0, 5) * 36
                + (value / 6 % 6 + amount).clamp(0, 5) * 6
                + (value % 6 + amount).clamp(0, 5)) as u32
        };
        result | (tinted << shift)
    })
}

fn line(mut start: (i32, i32), mut end: (i32, i32), mut plot: impl FnMut(i32, i32)) {
    let low = (end.1 - start.1).abs() < (end.0 - start.0).abs();
    if (low && start.0 > end.0) || (!low && start.1 > end.1) {
        std::mem::swap(&mut start, &mut end);
    }
    let (major_start, major_end, minor_start, minor_end) = if low {
        (start.0, end.0, start.1, end.1)
    } else {
        (start.1, end.1, start.0, end.0)
    };
    let major_delta = major_end - major_start;
    let minor_delta = (minor_end - minor_start).abs();
    let step = if minor_end < minor_start { -1 } else { 1 };
    let mut error = 2 * minor_delta - major_delta;
    let mut minor = minor_start;
    for major in major_start..=major_end {
        if low {
            plot(major, minor);
        } else {
            plot(minor, major);
        }
        if error > 0 {
            minor += step;
            error += 2 * (minor_delta - major_delta);
        } else {
            error += 2 * minor_delta;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quests() -> Vec<QuestView> {
        [
            ("root", None),
            ("a", Some("root")),
            ("b", Some("root")),
            ("leaf", Some("a")),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (id, parent))| QuestView {
            index,
            id: id.into(),
            description: String::new(),
            parent: parent.map(str::to_owned),
            completed: false,
            has_requirements: false,
            criteria_completed: 0,
            criteria_total: 1,
        })
        .collect()
    }

    #[test]
    fn geometry_uses_java_padding_and_translation() {
        let tree = Tree::new(&quests(), str::to_owned);
        assert_eq!(
            (tree.rows[0][0].x, tree.rows[0][0].y, tree.rows[0][0].width),
            (5, 5, 38)
        );
        assert_eq!((tree.rows[1][1].x, tree.rows[1][1].y), (29, 34));
        assert_eq!(tree.rows[2][0].y, 63);
    }

    #[test]
    fn navigation_is_two_dimensional_bounded_and_clamps_columns() {
        let tree = Tree::new(&quests(), str::to_owned);
        let mut state = State::default();
        let mut selected = 0;
        let down = Input {
            down_pressed: true,
            ..Input::default()
        };
        let right = Input {
            right_pressed: true,
            ..Input::default()
        };
        assert_eq!(tree.tick(&mut selected, &mut state, &right), Action::None);
        assert_eq!(tree.tick(&mut selected, &mut state, &down), Action::Moved);
        assert_eq!(selected, 1);
        assert_eq!(tree.tick(&mut selected, &mut state, &right), Action::Moved);
        assert_eq!(selected, 2);
        assert_eq!(tree.tick(&mut selected, &mut state, &down), Action::Moved);
        assert_eq!(selected, 3);
        assert_eq!(tree.tick(&mut selected, &mut state, &down), Action::None);
        assert_eq!(
            tree.tick(
                &mut selected,
                &mut state,
                &Input {
                    select: true,
                    ..down
                }
            ),
            Action::None
        );
    }

    #[test]
    fn browsing_is_silent_and_release_reveals_localized_selection() {
        let tree = Tree::new(&quests(), |_| "X".repeat(40));
        let mut state = State::default();
        let mut selected = 0;
        assert_eq!(
            tree.tick(
                &mut selected,
                &mut state,
                &Input {
                    shift: true,
                    right_pressed: true,
                    select: true,
                    ..Input::default()
                }
            ),
            Action::None
        );
        assert_eq!((selected, state.x_scroll), (0, 3));
        tree.tick(&mut selected, &mut state, &Input::default());
        assert_eq!(state.x_scroll, 0); // Java's left-edge test takes priority.
        tree.tick(
            &mut selected,
            &mut state,
            &Input {
                down_pressed: true,
                ..Input::default()
            },
        );
        assert_eq!(state.x_scroll, 80); // 5 + (6+40*8) + 5 - 256.
    }

    #[test]
    fn vertical_reveal_uses_the_raster_height_and_five_pixel_margin() {
        let mut quests = quests();
        for index in 4..7 {
            let mut quest = quests[3].clone();
            quest.id = format!("depth{index}");
            quest.parent = Some(quests[index - 1].id.clone());
            quests.push(quest);
        }
        let tree = Tree::new(&quests, str::to_owned);
        let mut state = State::default();
        let mut selected = 0;
        for _ in 0..5 {
            tree.tick(
                &mut selected,
                &mut state,
                &Input {
                    down_pressed: true,
                    ..Input::default()
                },
            );
        }
        assert_eq!(selected, 6);
        assert_eq!(state.y_scroll, 53); // top150 + height14 + margin5 - raster116.
        for _ in 0..5 {
            tree.tick(
                &mut selected,
                &mut state,
                &Input {
                    up_pressed: true,
                    ..Input::default()
                },
            );
        }
        assert_eq!((selected, state.y_scroll), (0, 0));
    }

    #[test]
    fn malformed_parent_graph_terminates_without_inventing_roots() {
        let mut quests = quests();
        quests[0].parent = Some("leaf".into());
        assert!(Tree::new(&quests, str::to_owned).rows.is_empty());
        quests[0].parent = Some("missing".into());
        assert!(Tree::new(&quests, str::to_owned).rows.is_empty());
    }

    #[test]
    fn java_palette_tint_and_bresenham_ties_are_preserved() {
        assert_eq!(tint(GRAY, 1), 0xC4C4C4);
        assert_eq!(tint(GRAY, 2), 0xCBCBCB);
        assert_eq!(tint(GREEN, -1), 0x28A928);
        let mut points = Vec::new();
        line((0, 0), (4, 2), |x, y| points.push((x, y)));
        assert_eq!(points, [(0, 0), (1, 0), (2, 1), (3, 1), (4, 2)]);
    }
}
