//! Separate experimental namespace; ordinary WorldRecord/Survival never enter here.
use std::{fs, path::PathBuf};

use crate::{
    assets::Assets,
    experimental::infdev::{explorer::Explorer, paged::NAMESPACE},
    gfx::{HEIGHT, Screen, WIDTH, font},
    io::Input,
    resource::localization::Localization,
    screen::{
        entry::Entry,
        menu::{Menu, MenuBuilder, MenuEvent},
        rel_pos::RelPos,
    },
};

pub(super) fn title_menu(localization: &Localization, selection: usize) -> Menu {
    // Keep the original Java title builder/Oracle unchanged. Only the live
    // application adds this independent experimental destination.
    let mut entries = vec![Entry::blank(), Entry::blank()];
    for (i, key) in super::TITLE_KEYS.iter().enumerate() {
        if i == 5 {
            entries.push(Entry::select("Infdev (Experimental)", false));
        }
        entries.push(Entry::select(localization.text(key), false));
    }
    MenuBuilder::new(false, 0, RelPos::CENTER, entries)
        .set_positioning((WIDTH as i32 / 2, HEIGHT as i32 * 3 / 5), RelPos::CENTER)
        .set_selection(selection as i32)
        .create_menu()
}

pub(super) fn runtime_title(
    localization: &Localization,
    _world_count: usize,
    selection: usize,
) -> Menu {
    title_menu(localization, selection)
}

/// The window may already be closed; a Windows subsystem executable has no
/// console on which to report a failed save. Keep the worker alive for retry.
pub(super) fn retry_save_dialog(error: &str) -> bool {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(
            window: *mut std::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            kind: u32,
        ) -> i32;
    }
    let text: Vec<u16> = format!("Infdev save failed:\n{error}\n\nRetry keeps this session alive. Cancel exits with only the last successful checkpoint saved.")
        .encode_utf16().chain(Some(0)).collect();
    let caption: Vec<u16> = "Minicraft Infdev - Save error"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // MB_RETRYCANCEL | MB_ICONERROR, IDRETRY.
    unsafe { MessageBoxW(std::ptr::null_mut(), text.as_ptr(), caption.as_ptr(), 0x15) == 4 }
}

struct Record {
    name: String,
    directory: PathBuf,
}

pub(super) enum Action {
    None,
    Title,
    Start(Box<Explorer>),
}

pub(super) struct Browser {
    root: PathBuf,
    records: Vec<Record>,
    menu: Menu,
    creating: bool,
    error: Option<String>,
}

fn real_directory(path: &std::path::Path) -> Result<(), String> {
    use std::os::windows::fs::MetadataExt;
    let m = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !m.is_dir() || m.file_attributes() & 0x400 != 0 {
        return Err("Experimental directory must be a real folder, not a link".into());
    }
    Ok(())
}

impl Browser {
    pub fn new(root: PathBuf) -> Self {
        let mut browser = Self {
            root,
            records: Vec::new(),
            menu: Menu::default(),
            creating: false,
            error: None,
        };
        browser.refresh();
        browser
    }

    fn list(&self) -> Result<Vec<Record>, String> {
        if !self.root.try_exists().map_err(|e| e.to_string())? {
            return Ok(Vec::new());
        }
        real_directory(&self.root)?;
        let namespace = self.root.join(NAMESPACE);
        if !namespace.try_exists().map_err(|e| e.to_string())? {
            return Ok(Vec::new());
        }
        real_directory(&namespace)?;
        let mut records = Vec::new();
        for entry in fs::read_dir(namespace).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || real_directory(&entry.path()).is_err() {
                continue;
            }
            // Include damaged worlds so opening reports their actual error;
            // do not hide them or replace them with a newly generated world.
            records.push(Record {
                name,
                directory: entry.path(),
            });
        }
        records.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(records)
    }

    fn refresh(&mut self) {
        match self.list() {
            Ok(records) => {
                self.records = records;
                self.error = None;
            }
            Err(error) => {
                self.records.clear();
                self.error = Some(error);
            }
        }
        self.creating = false;
        let mut entries = vec![
            Entry::select("New Experimental World", false),
            Entry::select("Refresh", false),
        ];
        entries.extend(self.records.iter().map(|r| {
            let label = if r.name.chars().count() > 28 {
                format!("{}...", r.name.chars().take(25).collect::<String>())
            } else {
                r.name.clone()
            };
            Entry::select(label, false)
        }));
        self.menu = MenuBuilder::new(true, 2, RelPos::CENTER, entries)
            .set_title("Infdev Worlds")
            .set_display_length(7)
            .set_positioning((WIDTH as i32 / 2, 90), RelPos::CENTER)
            .create_menu();
    }

    fn new_form(&mut self) {
        let mut seed = Entry::input("Seed: ", 20);
        if let Entry::Input { user_input, .. } = &mut seed {
            *user_input = "42".into();
        }
        self.menu = MenuBuilder::new(
            true,
            8,
            RelPos::LEFT,
            vec![
                Entry::input("Name: ", 24),
                seed,
                Entry::select("Create", false),
                Entry::select("Back", false),
            ],
        )
        .set_title("New Experimental World")
        .set_positioning((WIDTH as i32 / 2, 86), RelPos::CENTER)
        .create_menu();
        self.creating = true;
        self.error = None;
    }

    pub fn tick(&mut self, input: &Input) -> Action {
        if input.exit {
            if self.creating {
                self.refresh();
                return Action::None;
            }
            return Action::Title;
        }
        let events = self.menu.tick(&super::menu_input_from(input));
        if !events.contains(&MenuEvent::Activated) {
            return Action::None;
        }
        if self.creating {
            match self.menu.selection {
                2 => {
                    let value = |index| match &self.menu.entries[index] {
                        Entry::Input { user_input, .. } => user_input.clone(),
                        _ => String::new(),
                    };
                    let name = value(0);
                    let seed = value(1).parse::<i64>();
                    let result = seed
                        .map_err(|_| "Seed must be a signed 64-bit integer".to_owned())
                        .and_then(|seed| {
                            if name.trim().is_empty() {
                                return Err("Enter a world name".into());
                            }
                            Explorer::create(self.root.clone(), name, seed)
                        });
                    match result {
                        Ok(explorer) => return Action::Start(Box::new(explorer)),
                        Err(error) => self.error = Some(error),
                    }
                }
                3 => self.refresh(),
                _ => (),
            }
        } else {
            match self.menu.selection {
                0 => self.new_form(),
                1 => self.refresh(),
                index => {
                    if let Some(record) = self.records.get(index - 2) {
                        match Explorer::open(record.directory.clone()) {
                            Ok(explorer) => return Action::Start(Box::new(explorer)),
                            Err(error) => self.error = Some(error),
                        }
                    }
                }
            }
        }
        Action::None
    }

    pub fn render(&self, screen: &mut Screen, assets: &Assets) {
        screen.clear(0x101820);
        font::draw_centered(
            screen,
            assets.font(),
            "EXPERIMENTAL - NOT SURVIVAL",
            8,
            0xffcc55,
        );
        self.menu.render(screen, assets.hud(), assets.font());
        if let Some(error) = &self.error {
            for (i, line) in error
                .chars()
                .collect::<Vec<_>>()
                .chunks(34)
                .take(4)
                .enumerate()
            {
                screen.text_colored(
                    assets.font(),
                    &line.iter().collect::<String>(),
                    8,
                    145 + i as i32 * 8,
                    0xff7777,
                );
            }
        } else {
            font::draw_centered(screen, assets.font(), "Arrows / Enter / Esc", 178, 0xaaaaaa);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn live_title_has_an_independent_experimental_action_and_keeps_java_builder() {
        let l = Localization::load("en-us", &[]);
        let menu = title_menu(&l, 7);
        assert_eq!(menu.entries.len(), 9);
        assert!(
            matches!(&menu.entries[7], Entry::Select { text, .. } if text == "Infdev (Experimental)")
        );
        assert_eq!(
            crate::screen::title_display::build_menu(&l, 0, 2)
                .entries
                .len(),
            8
        );
        let (action, _) = super::super::tick_title_menu_event(
            &mut title_menu(&l, 7),
            &mut 0,
            &Input {
                select: true,
                ..Default::default()
            },
            0,
            &l,
        );
        assert!(matches!(action, super::super::Transition::Infdev));
    }

    #[test]
    fn browser_only_lists_its_namespace_and_cancel_never_modifies_fixed_worlds() {
        struct Owned(PathBuf);
        impl Drop for Owned {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }
        let p = std::env::temp_dir().join(format!(
            "minicraft-browser-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&p).unwrap();
        let owned = Owned(p);
        fs::create_dir(owned.0.join("Fixed")).unwrap();
        let fixed = owned.0.join("Fixed/state.json");
        fs::write(&fixed, b"fixed world sentinel").unwrap();
        fs::create_dir_all(owned.0.join(NAMESPACE).join("Saved")).unwrap();
        fs::create_dir(owned.0.join(NAMESPACE).join(".new-owned")).unwrap();
        let mut browser = Browser::new(owned.0.clone());
        assert_eq!(browser.records.len(), 1);
        assert_eq!(browser.records[0].name, "Saved");
        browser.tick(&Input {
            select: true,
            ..Default::default()
        });
        browser.menu.selection = 2;
        assert!(matches!(
            browser.tick(&Input {
                select: true,
                ..Default::default()
            }),
            Action::None
        ));
        assert!(browser.error.as_ref().unwrap().contains("name"));
        assert!(matches!(
            browser.tick(&Input {
                exit: true,
                ..Default::default()
            }),
            Action::None
        ));
        assert!(matches!(
            browser.tick(&Input {
                exit: true,
                ..Default::default()
            }),
            Action::Title
        ));
        assert_eq!(fs::read(fixed).unwrap(), b"fixed world sentinel");
        assert_eq!(fs::read_dir(owned.0.join(NAMESPACE)).unwrap().count(), 2);
    }
}
