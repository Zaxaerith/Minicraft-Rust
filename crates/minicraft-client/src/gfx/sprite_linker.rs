//! Category/key sheet ownership corresponding to Java SpriteLinker.
use super::{
    Image,
    sprite::{Px, Sprite},
};
use std::collections::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpriteType {
    Item,
    Gui,
    Tile,
    Entity,
}

#[derive(Default)]
pub struct SpriteLinker {
    sheets: HashMap<SpriteType, HashMap<String, Image>>,
}

impl SpriteLinker {
    pub fn set(&mut self, category: SpriteType, key: &str, image: Image) {
        self.sheets
            .entry(category)
            .or_default()
            .insert(key.to_owned(), image);
    }
    pub fn sheet(&self, category: SpriteType, key: &str) -> Option<&Image> {
        self.sheets.get(&category)?.get(key)
    }
    pub fn sheet_mut(&mut self, category: SpriteType, key: &str) -> Option<&mut Image> {
        self.sheets.get_mut(&category)?.get_mut(key)
    }
    pub fn reset(&mut self) {
        self.sheets.clear();
    }
    pub fn clear_skins(&mut self) {
        if let Some(entities) = self.sheets.get_mut(&SpriteType::Entity) {
            entities.retain(|key, _| !key.starts_with("skin."));
        }
    }
    pub fn missing(&self, category: SpriteType) -> Option<&Image> {
        let key = match category {
            SpriteType::Item => "missing_item",
            SpriteType::Tile => "missing_tile",
            SpriteType::Entity => "missing_entity",
            SpriteType::Gui => return None,
        };
        self.sheet(category, key)
    }
}

/// Resolve at draw time so replaced resource sheets cannot leave stale pointers.
pub struct LinkedSprite {
    pub category: SpriteType,
    pub key: String,
    pub x: usize,
    pub y: usize,
    pub width: usize,
    pub height: usize,
    pub mirror: u8,
    pub flip: u8,
    pub white_tint: Option<u32>,
}

impl LinkedSprite {
    pub fn resolve<'a>(&self, linker: &'a SpriteLinker) -> Option<Sprite<'a>> {
        let original = linker.sheet(self.category, &self.key);
        let sheet = original.or_else(|| linker.missing(self.category))?;
        let fallback = original.is_none();
        let w = if fallback || self.width == 0 {
            sheet.width / 8
        } else {
            self.width
        };
        let h = if fallback || self.height == 0 {
            sheet.height / 8
        } else {
            self.height
        };
        let rows = (0..h)
            .map(|r| {
                (0..w)
                    .map(|c| Px {
                        sheet,
                        sheet_x: if fallback {
                            c
                        } else {
                            self.x + if self.flip & 1 != 0 { w - 1 - c } else { c }
                        },
                        sheet_y: if fallback {
                            r
                        } else {
                            self.y + if self.flip & 2 != 0 { h - 1 - r } else { r }
                        },
                        mirror: if fallback { 0 } else { self.mirror },
                    })
                    .collect()
            })
            .collect();
        Some(Sprite {
            rows,
            white_tint: if fallback { None } else { self.white_tint },
        })
    }
}
