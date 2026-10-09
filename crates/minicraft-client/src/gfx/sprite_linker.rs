//! Category/key sheet ownership corresponding to Java SpriteLinker.
use super::{
    Image, Screen,
    sprite::{Px, Sprite},
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
/// Dimensions are in 8px sheet cells, matching Java LinkedSprite.setSpriteDim.
#[derive(Clone, Debug)]
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
    pub fn new(category: SpriteType, key: impl Into<String>) -> Self {
        Self {
            category,
            key: key.into(),
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            mirror: 0,
            flip: 0,
            white_tint: None,
        }
    }

    pub fn set_sprite_dim(mut self, x: usize, y: usize, width: usize, height: usize) -> Self {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
        self
    }

    pub fn set_mirror(mut self, mirror: u8) -> Self {
        self.mirror = mirror;
        self
    }

    pub fn set_flip(mut self, flip: u8) -> Self {
        self.flip = flip;
        self
    }

    pub fn set_white_tint(mut self, color: u32) -> Self {
        self.white_tint = Some(color);
        self
    }

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

    /// Java Screen.render(x, y, LinkedSprite) / render(..., mirror, fullbright, color).
    // Keep the existing coordinate/render API without adding wrapper state.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        linker: &SpriteLinker,
        screen: &mut Screen,
        x: i32,
        y: i32,
        mirror: u8,
        full_bright: bool,
        color: u32,
    ) {
        if let Some(sprite) = self.resolve(linker) {
            sprite.render(screen, x, y, mirror, full_bright, color);
        }
    }

    /// Only the top 8px row (Java swimming half-body).
    pub fn render_half_top(
        &self,
        linker: &SpriteLinker,
        screen: &mut Screen,
        x: i32,
        y: i32,
        full_bright: bool,
    ) {
        if let Some(sprite) = self.resolve(linker)
            && let Some(row) = sprite.rows.first()
        {
            Sprite {
                rows: vec![row.clone()],
                white_tint: sprite.white_tint,
            }
            .render(screen, x, y, 0, full_bright, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gfx::Image;

    fn sheet(width: usize, height: usize) -> Image {
        Image::from_rgb(width, height, vec![0xff_ffff; width * height]).unwrap()
    }

    fn linker_with(key: &str, width: usize, height: usize) -> SpriteLinker {
        let mut linker = SpriteLinker::default();
        linker.set(SpriteType::Item, key, sheet(width, height));
        linker
    }

    #[test]
    fn resolve_defaults_to_full_sheet_cells() {
        let linker = linker_with("sword", 16, 8);
        let linked = LinkedSprite::new(SpriteType::Item, "sword");
        let sprite = linked.resolve(&linker).unwrap();
        assert_eq!(sprite.rows.len(), 1);
        assert_eq!(sprite.rows[0].len(), 2);
        assert_eq!(sprite.rows[0][0].sheet_x, 0);
        assert_eq!(sprite.rows[0][1].sheet_x, 1);
    }

    #[test]
    fn resolve_applies_flip_x_and_y_to_sheet_indices() {
        let linker = linker_with("icon", 16, 16);
        let linked = LinkedSprite::new(SpriteType::Item, "icon")
            .set_sprite_dim(1, 2, 2, 2)
            .set_flip(0b11);
        let sprite = linked.resolve(&linker).unwrap();
        // Java: xOffset = flipX ? w-1-c : c, plus the sprite origin.
        assert_eq!(sprite.rows[0][0].sheet_x, 1 + 1);
        assert_eq!(sprite.rows[0][0].sheet_y, 2 + 1);
        assert_eq!(sprite.rows[1][1].sheet_x, 1);
        assert_eq!(sprite.rows[1][1].sheet_y, 2);
    }

    #[test]
    fn resolve_carries_mirror_bits_onto_each_px() {
        let linker = linker_with("body", 8, 8);
        let linked = LinkedSprite::new(SpriteType::Item, "body").set_mirror(0b01);
        let sprite = linked.resolve(&linker).unwrap();
        assert_eq!(sprite.rows[0][0].mirror, 0b01);
    }

    #[test]
    fn missing_key_falls_back_to_missing_item_sheet() {
        let mut linker = linker_with("missing_item", 8, 8);
        linker.set(SpriteType::Tile, "missing_tile", sheet(8, 8));
        // No "ghost" sheet registered.
        let linked = LinkedSprite::new(SpriteType::Item, "ghost")
            .set_sprite_dim(3, 3, 1, 1)
            .set_white_tint(0xff_0000)
            .set_mirror(1);
        let sprite = linked.resolve(&linker).unwrap();
        // Fallback ignores configured dim/mirror/tint and uses the missing sheet.
        assert_eq!(sprite.rows.len(), 1);
        assert_eq!(sprite.rows[0].len(), 1);
        assert_eq!(sprite.rows[0][0].mirror, 0);
        assert!(sprite.white_tint.is_none());
    }

    #[test]
    fn gui_has_no_missing_sheet() {
        let linker = SpriteLinker::default();
        assert!(linker.missing(SpriteType::Gui).is_none());
        assert!(
            LinkedSprite::new(SpriteType::Gui, "font")
                .resolve(&linker)
                .is_none()
        );
    }

    #[test]
    fn clear_skins_only_drops_skin_prefix() {
        let mut linker = SpriteLinker::default();
        linker.set(SpriteType::Entity, "skin.paul", sheet(8, 8));
        linker.set(SpriteType::Entity, "player", sheet(8, 8));
        linker.clear_skins();
        assert!(linker.sheet(SpriteType::Entity, "skin.paul").is_none());
        assert!(linker.sheet(SpriteType::Entity, "player").is_some());
    }
}
