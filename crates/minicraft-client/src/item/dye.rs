//! Java `DyeItem` — dye colors used by wool / clothing / loom recipes.
use super::ItemId;

/// Java `DyeItem.DyeColor` packed colors (`0x1_RRGGBB`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DyeColor {
    Black = 0,
    Red = 1,
    Green = 2,
    Brown = 3,
    Blue = 4,
    Purple = 5,
    Cyan = 6,
    LightGray = 7,
    Gray = 8,
    Pink = 9,
    Lime = 10,
    Yellow = 11,
    LightBlue = 12,
    Magenta = 13,
    Orange = 14,
    White = 15,
}

impl DyeColor {
    pub const ALL: [Self; 16] = [
        Self::Black,
        Self::Red,
        Self::Green,
        Self::Brown,
        Self::Blue,
        Self::Purple,
        Self::Cyan,
        Self::LightGray,
        Self::Gray,
        Self::Pink,
        Self::Lime,
        Self::Yellow,
        Self::LightBlue,
        Self::Magenta,
        Self::Orange,
        Self::White,
    ];

    /// Java enum ordinal, checked before using saved furniture / recipe data.
    pub const fn from_index(index: u16) -> Option<Self> {
        match index {
            0 => Some(Self::Black),
            1 => Some(Self::Red),
            2 => Some(Self::Green),
            3 => Some(Self::Brown),
            4 => Some(Self::Blue),
            5 => Some(Self::Purple),
            6 => Some(Self::Cyan),
            7 => Some(Self::LightGray),
            8 => Some(Self::Gray),
            9 => Some(Self::Pink),
            10 => Some(Self::Lime),
            11 => Some(Self::Yellow),
            12 => Some(Self::LightBlue),
            13 => Some(Self::Magenta),
            14 => Some(Self::Orange),
            15 => Some(Self::White),
            _ => None,
        }
    }

    pub const fn dye_item(self) -> ItemId {
        match self {
            Self::Black => ItemId::BlackDye,
            Self::Red => ItemId::RedDye,
            Self::Green => ItemId::GreenDye,
            Self::Brown => ItemId::BrownDye,
            Self::Blue => ItemId::BlueDye,
            Self::Purple => ItemId::PurpleDye,
            Self::Cyan => ItemId::CyanDye,
            Self::LightGray => ItemId::LightGrayDye,
            Self::Gray => ItemId::GrayDye,
            Self::Pink => ItemId::PinkDye,
            Self::Lime => ItemId::LimeDye,
            Self::Yellow => ItemId::YellowDye,
            Self::LightBlue => ItemId::LightBlueDye,
            Self::Magenta => ItemId::MagentaDye,
            Self::Orange => ItemId::OrangeDye,
            Self::White => ItemId::WhiteDye,
        }
    }

    pub const fn from_dye_item(item: ItemId) -> Option<Self> {
        match item {
            ItemId::BlackDye => Some(Self::Black),
            ItemId::RedDye => Some(Self::Red),
            ItemId::GreenDye => Some(Self::Green),
            ItemId::BrownDye => Some(Self::Brown),
            ItemId::BlueDye => Some(Self::Blue),
            ItemId::PurpleDye => Some(Self::Purple),
            ItemId::CyanDye => Some(Self::Cyan),
            ItemId::LightGrayDye => Some(Self::LightGray),
            ItemId::GrayDye => Some(Self::Gray),
            ItemId::PinkDye => Some(Self::Pink),
            ItemId::LimeDye => Some(Self::Lime),
            ItemId::YellowDye => Some(Self::Yellow),
            ItemId::LightBlueDye => Some(Self::LightBlue),
            ItemId::MagentaDye => Some(Self::Magenta),
            ItemId::OrangeDye => Some(Self::Orange),
            ItemId::WhiteDye => Some(Self::White),
            _ => None,
        }
    }

    pub const fn wool_item(self) -> ItemId {
        match self {
            Self::Black => ItemId::BlackWool,
            Self::Red => ItemId::RedWool,
            Self::Green => ItemId::GreenWool,
            Self::Brown => ItemId::BrownWool,
            Self::Blue => ItemId::BlueWool,
            Self::Purple => ItemId::PurpleWool,
            Self::Cyan => ItemId::CyanWool,
            Self::LightGray => ItemId::LightGrayWool,
            Self::Gray => ItemId::GrayWool,
            Self::Pink => ItemId::PinkWool,
            Self::Lime => ItemId::LimeWool,
            Self::Yellow => ItemId::YellowWool,
            Self::LightBlue => ItemId::LightBlueWool,
            Self::Magenta => ItemId::MagentaWool,
            Self::Orange => ItemId::OrangeWool,
            Self::White => ItemId::Wool,
        }
    }

    pub const fn from_wool_item(item: ItemId) -> Option<Self> {
        match item {
            ItemId::BlackWool => Some(Self::Black),
            ItemId::RedWool => Some(Self::Red),
            ItemId::GreenWool => Some(Self::Green),
            ItemId::BrownWool => Some(Self::Brown),
            ItemId::BlueWool => Some(Self::Blue),
            ItemId::PurpleWool => Some(Self::Purple),
            ItemId::CyanWool => Some(Self::Cyan),
            ItemId::LightGrayWool => Some(Self::LightGray),
            ItemId::GrayWool => Some(Self::Gray),
            ItemId::PinkWool => Some(Self::Pink),
            ItemId::LimeWool => Some(Self::Lime),
            ItemId::YellowWool => Some(Self::Yellow),
            ItemId::LightBlueWool => Some(Self::LightBlue),
            ItemId::MagentaWool => Some(Self::Magenta),
            ItemId::OrangeWool => Some(Self::Orange),
            ItemId::Wool => Some(Self::White),
            _ => None,
        }
    }

    pub const fn bed_item(self) -> ItemId {
        match self {
            Self::Black => ItemId::BlackBed,
            Self::Red => ItemId::RedBed,
            Self::Green => ItemId::GreenBed,
            Self::Brown => ItemId::BrownBed,
            Self::Blue => ItemId::BlueBed,
            Self::Purple => ItemId::PurpleBed,
            Self::Cyan => ItemId::CyanBed,
            Self::LightGray => ItemId::LightGrayBed,
            Self::Gray => ItemId::GrayBed,
            Self::Pink => ItemId::PinkBed,
            Self::Lime => ItemId::LimeBed,
            Self::Yellow => ItemId::YellowBed,
            Self::LightBlue => ItemId::LightBlueBed,
            Self::Magenta => ItemId::MagentaBed,
            Self::Orange => ItemId::OrangeBed,
            Self::White => ItemId::WhiteBed,
        }
    }

    pub const fn from_bed_item(item: ItemId) -> Option<Self> {
        match item {
            ItemId::BlackBed => Some(Self::Black),
            ItemId::RedBed => Some(Self::Red),
            ItemId::GreenBed => Some(Self::Green),
            ItemId::BrownBed => Some(Self::Brown),
            ItemId::BlueBed => Some(Self::Blue),
            ItemId::PurpleBed => Some(Self::Purple),
            ItemId::CyanBed => Some(Self::Cyan),
            ItemId::LightGrayBed => Some(Self::LightGray),
            ItemId::GrayBed => Some(Self::Gray),
            ItemId::PinkBed => Some(Self::Pink),
            ItemId::LimeBed => Some(Self::Lime),
            ItemId::YellowBed => Some(Self::Yellow),
            ItemId::LightBlueBed => Some(Self::LightBlue),
            ItemId::MagentaBed => Some(Self::Magenta),
            ItemId::OrangeBed => Some(Self::Orange),
            ItemId::WhiteBed => Some(Self::White),
            _ => None,
        }
    }

    /// Java `DyeColor.color` field.
    pub const fn packed_color(self) -> u32 {
        match self {
            Self::Black => 0x1_1D_1D_21,
            Self::Red => 0x1_B0_2E_26,
            Self::Green => 0x1_5E_7C_16,
            Self::Brown => 0x0183_5432,
            Self::Blue => 0x1_3C_44_AA,
            Self::Purple => 0x1_89_32_B8,
            Self::Cyan => 0x1_16_9C_9C,
            Self::LightGray => 0x1_9D_9D_97,
            Self::Gray => 0x1_47_4F_52,
            Self::Pink => 0x1_F3_8B_AA,
            Self::Lime => 0x1_80_C7_1F,
            Self::Yellow => 0x1_FE_D8_3D,
            Self::LightBlue => 0x1_3A_B3_DA,
            Self::Magenta => 0x1_C7_4E_BD,
            Self::Orange => 0x1_F9_80_1D,
            Self::White => 0x1_F9_FF_FE,
        }
    }

    /// Java `MyUtils.capitalizeFully(color.toString().replace('_', ' '))`.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Black => "Black",
            Self::Red => "Red",
            Self::Green => "Green",
            Self::Brown => "Brown",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
            Self::Cyan => "Cyan",
            Self::LightGray => "Light Gray",
            Self::Gray => "Gray",
            Self::Pink => "Pink",
            Self::Lime => "Lime",
            Self::Yellow => "Yellow",
            Self::LightBlue => "Light Blue",
            Self::Magenta => "Magenta",
            Self::Orange => "Orange",
            Self::White => "White",
        }
    }

    /// Lowercase underscore form used for asset keys (`light_gray_dye`).
    pub const fn asset_stem(self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::Red => "red",
            Self::Green => "green",
            Self::Brown => "brown",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Cyan => "cyan",
            Self::LightGray => "light_gray",
            Self::Gray => "gray",
            Self::Pink => "pink",
            Self::Lime => "lime",
            Self::Yellow => "yellow",
            Self::LightBlue => "light_blue",
            Self::Magenta => "magenta",
            Self::Orange => "orange",
            Self::White => "white",
        }
    }
}

/// Legacy 2.2.4 color sources, retained for compatibility with old recipes.
/// 2.3.0 recipes use `DyeColor::dye_item` instead.
pub const fn dye_source_item(color: DyeColor) -> Option<ItemId> {
    match color {
        DyeColor::Red => Some(ItemId::Rose),
        DyeColor::Blue => Some(ItemId::Lapis),
        DyeColor::Green => Some(ItemId::Cactus),
        DyeColor::Yellow => Some(ItemId::Flower),
        DyeColor::Black => Some(ItemId::Coal),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java230_color_items_are_unique_and_inventory_round_trip() {
        let mut inventory = crate::item::Inventory::new(27);
        let mut items = std::collections::HashSet::new();
        for color in DyeColor::ALL {
            assert_eq!(DyeColor::from_index(color as u16), Some(color));
            assert_eq!(DyeColor::from_dye_item(color.dye_item()), Some(color));
            assert_eq!(DyeColor::from_wool_item(color.wool_item()), Some(color));
            assert_eq!(DyeColor::from_bed_item(color.bed_item()), Some(color));
            assert!(items.insert(color.dye_item()));
            assert_eq!(inventory.add(color.dye_item(), 3), 0);
            assert!(inventory.remove(color.dye_item(), 1));
        }
        assert_eq!(DyeColor::from_index(16), None);
        assert_eq!(DyeColor::from_bed_item(ItemId::Bed), None);
        let saved = serde_json::to_string(&inventory).unwrap();
        let loaded: crate::item::Inventory = serde_json::from_str(&saved).unwrap();
        for color in DyeColor::ALL {
            assert_eq!(loaded.count(color.dye_item()), 2);
        }
    }

    #[test]
    fn dye_colors_match_java_palette() {
        assert_eq!(DyeColor::ALL.len(), 16);
        assert_eq!(DyeColor::Black.packed_color(), 0x1_1D_1D_21);
        assert_eq!(DyeColor::White.packed_color(), 0x1_F9_FF_FE);
        assert_eq!(DyeColor::LightGray.display_name(), "Light Gray");
        assert_eq!(DyeColor::LightGray.asset_stem(), "light_gray");
    }
}
