//! Bounded experimental Survival session. Uses the existing Player, Inventory,
//! ToolItem and recipe transactions. Its immutable session object is published
//! with the chunk root, never as a separately saved player file.
use super::{Cell, Ground, paged::World};
use crate::{
    entity::furniture::FurnitureKind,
    item::{
        Inventory, ItemId, ItemStack, Recipe, RecipeOutput, RecipeView, ToolItem, ToolKind,
        ToolRecipe, recipe_views,
    },
    level::{
        GameMode,
        noise::JavaRandom,
        tile::{Tile, flower, ore, rock, tree},
    },
    world::{ActiveItem, Direction, PlayOptions, Player},
};
use serde::{Deserialize, Serialize};

const MAX_DROPS: usize = 512;
const MAX_STATIONS: usize = 64;

mod metabolism;

// A missing marker is the original v4 checkpoint: no hunger progression and
// keep inventory on death. Do not silently opt existing players into new rules.
#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum FoodRules {
    #[default]
    Legacy,
    JavaNormal,
}
impl FoodRules {
    fn is_legacy(&self) -> bool {
        *self == Self::Legacy
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum DeathRule {
    #[default]
    KeepInventory,
    ChestNormal,
}
impl DeathRule {
    fn is_keep(&self) -> bool {
        *self == Self::KeepInventory
    }
}
fn is_false(v: &bool) -> bool {
    !*v
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub(crate) enum StationKind {
    #[default]
    Workbench,
    Furnace,
    Anvil,
}
impl StationKind {
    fn is_workbench(&self) -> bool {
        *self == Self::Workbench
    }
    pub fn furniture(self) -> FurnitureKind {
        match self {
            Self::Workbench => FurnitureKind::Workbench,
            Self::Furnace => FurnitureKind::Furnace,
            Self::Anvil => FurnitureKind::Anvil,
        }
    }
    pub fn item(self) -> ItemId {
        match self {
            Self::Workbench => ItemId::Workbench,
            Self::Furnace => ItemId::Furnace,
            Self::Anvil => ItemId::Anvil,
        }
    }
    fn from_item(item: ItemId) -> Option<Self> {
        [Self::Workbench, Self::Furnace, Self::Anvil]
            .into_iter()
            .find(|k| k.item() == item)
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub(crate) enum Loot {
    Stack(ItemStack),
    Tool(ToolItem),
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Drop {
    pub depth: i8,
    pub x: i64,
    pub y: i64,
    pub loot: Loot,
    // Prevent dropping directly into one's own automatic pickup on the same tick.
    pub delay: u16,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Station {
    pub depth: i8,
    pub x: i64,
    pub y: i64,
    #[serde(default, skip_serializing_if = "StationKind::is_workbench")]
    pub kind: StationKind,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Panel {
    #[default]
    Closed,
    Inventory,
    Crafting(Option<StationKind>),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct State {
    #[serde(default, skip_serializing_if = "DeathRule::is_keep")]
    pub death_rule: DeathRule,
    #[serde(default, skip_serializing_if = "is_false")]
    pub entity_pages_present: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub death_recorded: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub death_retained: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death_site: Option<super::entities::Position>,
    #[serde(default, skip_serializing_if = "FoodRules::is_legacy")]
    pub food_rules: FoodRules,
    pub player: Player,
    pub spawn: (i64, i64),
    pub drops: Vec<Drop>,
    pub stations: Vec<Station>,
    random: JavaRandom,
    ticks: u64,
    #[serde(skip)]
    pub panel: Panel,
    #[serde(skip)]
    pub selection: usize,
}

#[derive(Clone, Copy)]
pub(crate) enum Action {
    Tick,
    DiagnosticDeath, // Only the explicit native-window diagnostic sends this.
    Move(i32, i32),
    Attack,
    Interact,
    Pickup,
    Inventory,
    Crafting,
    Cursor(i32),
    Confirm,
    Close,
    Drop(bool),
}

pub(crate) fn tile(cell: Cell) -> Tile {
    super::explorer::render::tile_for(cell)
}
fn ground(tile: Tile) -> Option<Ground> {
    Some(match tile {
        Tile::Dirt => Ground::Dirt,
        Tile::Grass => Ground::Grass,
        Tile::Sand => Ground::Sand,
        Tile::Rock => Ground::Rock,
        Tile::Hole => Ground::Hole,
        Tile::WoodFloor => Ground::WoodFloor,
        Tile::RawStone => Ground::RawStone,
        Tile::StoneFloor => Ground::StoneFloor,
        Tile::Flower230 => Ground::Flower,
        _ => return None,
    })
}

pub(crate) fn find_spawn(world: &mut World) -> Result<(i64, i64), String> {
    for radius in 0_i64..=32 {
        for x in -radius..=radius {
            for y in -radius..=radius {
                if x.abs().max(y.abs()) != radius {
                    continue;
                }
                let c = world.cell_at(0, x, y)?;
                if !tile(c).solid(c.data)
                    && !matches!(c.ground, Ground::Water | Ground::Lava | Ground::Hole)
                {
                    return Ok((x, y));
                }
            }
        }
    }
    Err("no safe natural Survival spawn within 32 tiles; terrain was not replaced".into())
}

pub(crate) fn recipes(station: Option<StationKind>) -> Vec<RecipeView> {
    recipe_views(station.map(StationKind::furniture))
        .into_iter()
        .filter(|r| match r.output {
            RecipeOutput::Stack(s) => matches!(
                s.item,
                ItemId::Workbench
                    | ItemId::Furnace
                    | ItemId::Anvil
                    | ItemId::Plank
                    | ItemId::StoneBrick
                    | ItemId::IronIngot
                    | ItemId::GoldIngot
                    | ItemId::Glass
                    | ItemId::GlassBottle
            ),
            RecipeOutput::Tool(t) => {
                matches!(t.kind, ToolKind::Axe | ToolKind::Pickaxe | ToolKind::Shovel)
            }
        })
        .collect()
}

impl State {
    pub fn new(seed: i64) -> Self {
        // Use the existing no-map constructor's canonical empty Player defaults;
        // this neither generates a fixed world nor loads any player's files.
        let template = crate::world::World::empty_for_java_import(
            seed,
            128,
            PlayOptions {
                difficulty: 1,
                mode: GameMode::Survival,
                score_minutes: 20,
                tutorials: false,
                quests: false,
                show_quests: false,
                custom_skin: false,
            },
        );
        Self {
            death_rule: DeathRule::ChestNormal,
            entity_pages_present: false,
            death_recorded: false,
            death_retained: false,
            death_site: None,
            food_rules: FoodRules::JavaNormal,
            player: template.player,
            spawn: (0, 0),
            drops: Vec::new(),
            stations: Vec::new(),
            random: JavaRandom::new(seed ^ 0x5355_5256_4956_414c),
            ticks: 0,
            panel: Panel::Closed,
            selection: 0,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let p = &self.player;
        if !(0..16).contains(&p.x)
            || !(0..16).contains(&p.y)
            || p.max_health != 10
            || p.health > p.max_health
            || p.stamina > 10
            || p.hunger > 10
            || p.inventory.capacity() != 27
            || p.inventory.used_slots() > 27
            || self.drops.len() > MAX_DROPS
            || self.stations.len() > MAX_STATIONS
        {
            return Err("invalid bounded Survival player/session".into());
        }
        if self.death_retained && !self.death_recorded
            || self
                .death_site
                .is_some_and(|p| !(-4..=0).contains(&p.depth))
        {
            return Err("invalid death receipt".into());
        }
        validate_inventory(&p.inventory)?;
        for drop in &self.drops {
            if !(-4..=0).contains(&drop.depth) || drop.delay > 60 {
                return Err("invalid dropped instance position/delay".into());
            }
            validate_loot(drop.loot)?;
        }
        let mut positions = std::collections::BTreeSet::new();
        for s in &self.stations {
            if !(-4..=0).contains(&s.depth) || !positions.insert((s.depth, s.x, s.y)) {
                return Err("invalid/duplicate workstation".into());
            }
        }
        match p.active_item {
            Some(ActiveItem::Tool(i)) if i >= p.inventory.tools().len() => {
                return Err("invalid active tool reference".into());
            }
            Some(ActiveItem::Stack(id)) if p.inventory.count(id) == 0 => {
                return Err("invalid active stack reference".into());
            }
            Some(ActiveItem::Slot { .. }) => {
                return Err("unsupported active instance reference in Survival v4".into());
            }
            _ => (),
        }
        Ok(())
    }

    pub fn station_at(&self, depth: i8, x: i64, y: i64) -> bool {
        self.stations
            .iter()
            .any(|s| (s.depth, s.x, s.y) == (depth, x, y))
    }
    fn facing(&self) -> (i64, i64) {
        match self.player.direction {
            Direction::Down => (0, 1),
            Direction::Up => (0, -1),
            Direction::Left => (-1, 0),
            Direction::Right => (1, 0),
        }
    }
    pub(super) fn target(&self, w: &World) -> Result<(i64, i64), String> {
        let (x, y) = w.position();
        let (dx, dy) = self.facing();
        x.checked_add(dx)
            .zip(y.checked_add(dy))
            .ok_or_else(|| "interaction outside i64 tiles".into())
    }
    fn tool(&self) -> Option<ToolItem> {
        match self.player.active_item {
            Some(ActiveItem::Tool(i)) => self.player.inventory.tools().get(i).copied(),
            _ => None,
        }
    }
    fn cleanup_active(&mut self) {
        if let Some(ActiveItem::Stack(id)) = self.player.active_item
            && self.player.inventory.count(id) == 0
        {
            self.player.active_item = None;
        }
    }
    fn charge_tool(&mut self, cost: u8) -> Option<ToolItem> {
        let Some(ActiveItem::Tool(i)) = self.player.active_item else {
            return None;
        };
        let t = *self.player.inventory.tools().get(i)?;
        if !self.player.inventory.tools_mut()[i].pay_durability() {
            return None;
        }
        self.player.stamina = self
            .player
            .stamina
            .saturating_sub(cost.saturating_sub(t.tier.level()));
        if self.player.inventory.tools()[i].is_depleted() {
            self.player.inventory.take_tool(i);
            self.player.active_item = None;
        }
        Some(t)
    }
    fn add_drop(&mut self, w: &World, position: (i64, i64), loot: Loot, delay: u16) {
        self.drops.push(Drop {
            depth: w.depth(),
            x: position.0,
            y: position.1,
            loot,
            delay,
        });
    }

    pub fn act(&mut self, w: &mut World, action: Action) -> Result<(), String> {
        if self.player.health == 0 {
            if matches!(action, Action::Confirm | Action::Interact) {
                let spawn = w.cell_at(0, self.spawn.0, self.spawn.1)?;
                if tile(spawn).solid(spawn.data)
                    || matches!(spawn.ground, Ground::Hole | Ground::Water | Ground::Lava)
                    || self.station_at(0, self.spawn.0, self.spawn.1)
                {
                    return Err("saved spawn obstructed; respawn refused".into());
                }
                w.visit_layer(0, self.spawn.0, self.spawn.1)?;
                self.player.x = 8;
                self.player.y = 8;
                self.death_recorded = false;
                self.death_retained = false;
                self.player.health = 10;
                self.player.stamina = 10;
                if self.food_rules == FoodRules::JavaNormal {
                    metabolism::reset(&mut self.player);
                }
            }
            return Ok(());
        }
        match action {
            Action::Tick => self.tick(w)?,
            Action::DiagnosticDeath => self.player.health = 0,
            Action::Inventory => {
                self.panel = Panel::Inventory;
                self.selection = 0;
            }
            Action::Crafting => {
                self.panel = Panel::Crafting(None);
                self.selection = 0;
            }
            Action::Close => self.panel = Panel::Closed,
            Action::Cursor(d) => {
                let n = match self.panel {
                    Panel::Inventory => self.player.inventory.used_slots(),
                    Panel::Crafting(b) => recipes(b).len(),
                    Panel::Closed => 0,
                };
                if n > 0 {
                    self.selection = (self.selection as i32 + d).rem_euclid(n as i32) as usize;
                }
            }
            Action::Confirm => self.confirm()?,
            Action::Drop(all) => self.drop_active(w, all)?,
            Action::Move(dx, dy) => {
                if self.panel == Panel::Closed {
                    self.move_player(w, dx, dy)?;
                }
            }
            Action::Attack => {
                if self.panel == Panel::Closed {
                    self.attack(w)?;
                }
            }
            Action::Interact => {
                let (x, y) = self.target(w)?;
                if let Some(station) = self
                    .stations
                    .iter()
                    .find(|s| (s.depth, s.x, s.y) == (w.depth(), x, y))
                {
                    self.panel = Panel::Crafting(Some(station.kind));
                    self.selection = 0;
                }
            }
            Action::Pickup => {
                let c = w.cell(w.position().0, w.position().1)?;
                if matches!(c.ground, Ground::StairsDown | Ground::StairsUp) {
                    super::stairs::travel(w)?;
                    self.player.x = 8;
                    self.player.y = 8;
                } else {
                    let p = self.target(w)?;
                    if let Some(i) = self
                        .stations
                        .iter()
                        .position(|s| (s.depth, s.x, s.y) == (w.depth(), p.0, p.1))
                    {
                        if self.drops.len() >= MAX_DROPS {
                            return Err("drop limit reached; collect existing items first".into());
                        }
                        let station = self.stations.remove(i);
                        self.add_drop(w, p, Loot::Stack(ItemStack::new(station.kind.item(), 1)), 0);
                    }
                }
            }
        }
        self.cleanup_active();
        Ok(())
    }

    fn tick(&mut self, w: &mut World) -> Result<(), String> {
        if self.panel != Panel::Closed {
            return Ok(());
        }
        self.ticks = self.ticks.wrapping_add(1);
        self.player.attack_time = self.player.attack_time.saturating_sub(1);
        self.player.hurt_time = self.player.hurt_time.saturating_sub(1);
        let (x, y) = w.position();
        let cell = w.cell(x, y)?;
        let swimming = cell.ground == Ground::Water;
        // Same Player stamina thresholds; menu time does not advance metabolism.
        if self.player.stamina == 0
            && self.player.stamina_recharge_delay == 0
            && self.player.stamina_recharge == 0
        {
            self.player.stamina_recharge_delay = 40;
        }
        if self.player.stamina_recharge_delay > 0 {
            self.player.stamina_recharge_delay -= 1;
        }
        if self.player.stamina_recharge_delay == 0 && !swimming {
            self.player.stamina_recharge = self.player.stamina_recharge.saturating_add(1);
        }
        if swimming {
            self.player.stamina_recharge = 0;
        }
        if self.player.stamina_recharge > 10 {
            self.player.stamina_recharge -= 10;
            self.player.stamina = (self.player.stamina + 1).min(10);
        }
        if self.food_rules == FoodRules::JavaNormal {
            metabolism::tick(&mut self.player, self.ticks);
        }
        if swimming && self.ticks.is_multiple_of(60) {
            if self.player.stamina > 0 {
                self.player.stamina -= 1;
            } else {
                self.hurt();
            }
        }
        if cell.ground == Ground::Lava && self.ticks.is_multiple_of(30) {
            self.hurt();
        }
        if self.player.health == 0 {
            return Ok(());
        } // Dead players cannot collect more loot.
        for d in &mut self.drops {
            d.delay = d.delay.saturating_sub(1);
            if d.delay == 0 && d.depth == w.depth() && d.x.abs_diff(x) <= 1 && d.y.abs_diff(y) <= 1
            {
                match &mut d.loot {
                    Loot::Stack(s) => s.count = self.player.inventory.add_stack(*s),
                    Loot::Tool(t) => {
                        if self.player.inventory.add_tool(*t).is_some() {
                            d.delay = u16::MAX;
                        }
                    }
                }
            }
        }
        self.drops.retain(|d| {
            d.delay != u16::MAX && !matches!(d.loot, Loot::Stack(ItemStack { count: 0, .. }))
        });
        Ok(())
    }
    fn hurt(&mut self) {
        if self.player.hurt_time == 0 {
            self.player.health = self.player.health.saturating_sub(1);
            self.player.hurt_time = 30;
        }
    }
    fn move_player(&mut self, w: &mut World, dx: i32, dy: i32) -> Result<(), String> {
        if !matches!((dx, dy), (-1 | 1, 0) | (0, -1 | 1)) {
            return Err("one cardinal pixel required".into());
        }
        self.player.direction = match (dx, dy) {
            (0, -1) => Direction::Up,
            (0, 1) => Direction::Down,
            (-1, 0) => Direction::Left,
            _ => Direction::Right,
        };
        let (gx, gy) = w.position();
        let (px, py) = (self.player.x + dx, self.player.y + dy);
        for (bx, by) in [(-4, -3), (4, -3), (-4, 3), (4, 3)] {
            let tx = gx
                .checked_add(i64::from((px + bx).div_euclid(16)))
                .ok_or("x overflow")?;
            let ty = gy
                .checked_add(i64::from((py + by).div_euclid(16)))
                .ok_or("y overflow")?;
            let c = w.cell(tx, ty)?;
            if tile(c).solid(c.data)
                || c.ground == Ground::Hole
                || self.station_at(w.depth(), tx, ty)
            {
                return Ok(());
            }
        }
        let x = gx
            .checked_add(i64::from(px.div_euclid(16)))
            .ok_or("x overflow")?;
        let y = gy
            .checked_add(i64::from(py.div_euclid(16)))
            .ok_or("y overflow")?;
        w.visit(x, y)?;
        self.player.x = px.rem_euclid(16);
        self.player.y = py.rem_euclid(16);
        self.player.walk_distance = self.player.walk_distance.wrapping_add(1);
        self.player.step_count = self.player.step_count.saturating_add(1);
        Ok(())
    }

    fn confirm(&mut self) -> Result<(), String> {
        match self.panel {
            Panel::Inventory => {
                self.player.active_item =
                    if let Some(s) = self.player.inventory.slots().get(self.selection) {
                        Some(ActiveItem::Stack(s.item))
                    } else {
                        let i = self
                            .selection
                            .saturating_sub(self.player.inventory.slots().len());
                        (i < self.player.inventory.tools().len()).then_some(ActiveItem::Tool(i))
                    };
                self.panel = Panel::Closed;
            }
            Panel::Crafting(b) => {
                let r = *recipes(b).get(self.selection).ok_or("no selected recipe")?;
                let success = match r.output {
                    RecipeOutput::Stack(s) => Recipe {
                        output: s,
                        costs: r.costs,
                    }
                    .craft(&mut self.player.inventory),
                    RecipeOutput::Tool(t) => {
                        let index = ToolRecipe {
                            output: t,
                            costs: r.costs,
                            tool_cost: r.tool_cost,
                        }
                        .craft(&mut self.player.inventory);
                        if let Some(i) = index {
                            self.player.active_item = Some(ActiveItem::Tool(i));
                        }
                        index.is_some()
                    }
                };
                if !success {
                    return Err("materials missing or inventory full; craft unchanged".into());
                }
            }
            Panel::Closed => (),
        }
        Ok(())
    }
    fn drop_active(&mut self, w: &World, all: bool) -> Result<(), String> {
        if self.drops.len() >= MAX_DROPS {
            return Err("drop limit reached; item retained".into());
        }
        let loot = match self.player.active_item {
            Some(ActiveItem::Stack(id)) => {
                let i = self
                    .player
                    .inventory
                    .slots()
                    .iter()
                    .position(|s| s.item == id)
                    .ok_or("selected stack missing")?;
                let s = self
                    .player
                    .inventory
                    .remove_slot(i, if all { u16::MAX } else { 1 })
                    .ok_or("selected stack empty")?;
                Loot::Stack(s)
            }
            Some(ActiveItem::Tool(i)) => {
                let t = self
                    .player
                    .inventory
                    .take_tool(i)
                    .ok_or("selected tool missing")?;
                self.player.active_item = None;
                Loot::Tool(t)
            }
            _ => return Ok(()),
        };
        self.add_drop(w, w.position(), loot, 60);
        Ok(())
    }

    fn attack(&mut self, w: &mut World) -> Result<(), String> {
        if self.player.attack_time > 0 || self.player.stamina == 0 {
            return Ok(());
        }
        // Food interacts with the player, regardless of the facing tile, drop
        // limit or whether another station is in front of them.
        if let Some(ActiveItem::Stack(id)) = self.player.active_item
            && let Some(feed) = id.food_value()
        {
            if self.player.hunger < 10 && self.player.inventory.remove(id, 1) {
                self.player.stamina = self
                    .player
                    .stamina
                    .saturating_sub(crate::item::food::FOOD_STAMINA_COST);
                self.player.hunger = self.player.hunger.saturating_add(feed).min(10);
                self.player.attack_time = 10;
            }
            return Ok(());
        }
        if self.drops.len() + 3 > MAX_DROPS {
            return Err("drop limit reached; collect existing items before mining".into());
        }
        let p = self.target(w)?;
        let c = w.cell(p.0, p.1)?;
        if self.station_at(w.depth(), p.0, p.1) {
            return Ok(());
        }
        if c.ground == Ground::ObsidianDoor {
            self.player.attack_time = 10;
            return w.set_cell(
                p.0,
                p.1,
                Cell {
                    data: c.data ^ 1,
                    ..c
                },
            );
        }
        // Read all fallible map data before changing any inventory/tool state.
        let mut next = c;
        let mut drops = Vec::new();
        if let Some(ActiveItem::Stack(id)) = self.player.active_item {
            if let Some(kind) = StationKind::from_item(id) {
                if self.stations.len() >= MAX_STATIONS {
                    return Err("workstation limit reached".into());
                }
                if !tile(c).solid(c.data)
                    && !matches!(
                        c.ground,
                        Ground::Water
                            | Ground::Lava
                            | Ground::Hole
                            | Ground::StairsDown
                            | Ground::StairsUp
                    )
                {
                    if !self.player.inventory.remove(id, 1) {
                        return Err("selected item missing".into());
                    }
                    self.stations.push(Station {
                        depth: w.depth(),
                        x: p.0,
                        y: p.1,
                        kind,
                    });
                    self.player.attack_time = 10;
                    return Ok(());
                }
                return Ok(());
            }
            if let Some((t, data)) = crate::item::tile_placement(id, tile(c)) {
                let Some(g) = ground(t) else {
                    return Err(
                        "this placement is not supported in Survival v4; item retained".into(),
                    );
                };
                if !self.player.inventory.remove(id, 1) {
                    return Err("selected item missing".into());
                }
                next = Cell { ground: g, data };
                self.player.attack_time = 10;
                return w.set_cell(p.0, p.1, next);
            }
        }
        self.player.attack_time = 10;
        self.player.stamina -= 1;
        self.player.stamina_recharge = 0;
        let tool = self.tool();
        let required = match c.ground {
            Ground::Tree => Some((ToolKind::Axe, 4)),
            Ground::Rock => Some((ToolKind::Pickaxe, 5)),
            Ground::IronOre | Ground::GoldOre | Ground::GemOre | Ground::Lapis => {
                Some((ToolKind::Pickaxe, 6))
            }
            Ground::Dirt | Ground::Grass | Ground::Sand => Some((ToolKind::Shovel, 4)),
            _ => None,
        };
        let used = required
            .filter(|(kind, _)| tool.is_some_and(|t| t.kind == *kind) && self.player.stamina > 0)
            .and_then(|(_, cost)| self.charge_tool(cost));
        let damage = if let Some(t) = used {
            t.terrain_damage(self.random.next_int(5) as u8)
        } else {
            if tool.is_some() {
                self.charge_tool(0);
            }
            (self.random.next_int(3) + 1) as u16
        };
        match c.ground {
            Ground::Tree | Ground::Rock => {
                // Infdev TreeTile.hurt: one real Apple on 1/100 of all hits,
                // including bare hands; not a guaranteed break reward.
                if c.ground == Ground::Tree && self.random.next_int(tree::APPLE_DROP_CHANCE) == 0 {
                    drops.push(ItemStack::new(ItemId::Apple, 1));
                }
                let max = if c.ground == Ground::Tree {
                    tree::MAX_HEALTH
                } else {
                    rock::MAX_HEALTH
                };
                let total = c.data.saturating_add(damage);
                if total >= max {
                    next = Cell {
                        ground: if c.ground == Ground::Tree {
                            Ground::Grass
                        } else {
                            Ground::Dirt
                        },
                        data: 0,
                    };
                    if c.ground == Ground::Tree {
                        drops.push(ItemStack::new(
                            ItemId::Wood,
                            (self.random.next_int(3) + 1) as u16,
                        ));
                        let acorns = self.random.next_int(3) as u16;
                        if acorns > 0 {
                            drops.push(ItemStack::new(ItemId::Acorn, acorns));
                        }
                    } else {
                        drops.push(ItemStack::new(
                            ItemId::Stone,
                            if used.is_some() {
                                (self.random.next_int(3) + 2) as u16
                            } else {
                                1
                            },
                        ));
                        if used.is_some() {
                            let coal = self.random.next_int(3) as u16;
                            if coal > 0 {
                                drops.push(ItemStack::new(ItemId::Coal, coal));
                            }
                        }
                    }
                } else {
                    next.data = total;
                }
            }
            Ground::IronOre | Ground::GoldOre | Ground::GemOre | Ground::Lapis
                if used.is_some() =>
            {
                let total = c.data.saturating_add(damage);
                let broken = total >= ore::max_health(self.random.next_int(10) as u16);
                next = if broken {
                    Cell {
                        ground: Ground::Dirt,
                        data: 0,
                    }
                } else {
                    Cell { data: total, ..c }
                };
                let count = ore::drop_count(self.random.next_int(2) as u16, broken);
                let id = match c.ground {
                    Ground::IronOre => ItemId::IronOre,
                    Ground::GoldOre => ItemId::GoldOre,
                    Ground::GemOre => ItemId::Gem,
                    _ => ItemId::Lapis,
                };
                if count > 0 {
                    drops.push(ItemStack::new(id, count));
                }
            }
            Ground::Dirt | Ground::Grass | Ground::Sand if used.is_some() => {
                next = Cell {
                    ground: if c.ground == Ground::Grass {
                        Ground::Dirt
                    } else {
                        Ground::Hole
                    },
                    data: 0,
                };
                if c.ground != Ground::Grass {
                    drops.push(ItemStack::new(
                        if c.ground == Ground::Sand {
                            ItemId::Sand
                        } else {
                            ItemId::Dirt
                        },
                        1,
                    ));
                }
            }
            Ground::Flower => {
                if let Some(id) = flower::harvested_item(Tile::Flower230, c.data) {
                    drops.push(ItemStack::new(id, 1));
                    next = Cell {
                        ground: Ground::Grass,
                        data: 0,
                    };
                }
            }
            _ => (),
        }
        for stack in drops {
            self.add_drop(w, p, Loot::Stack(stack), 0);
        }
        // The only map mutation is LAST. ensure() has already loaded this key;
        // nothing can checkpoint an edited tile with the pre-action inventory.
        if next != c {
            w.set_cell(p.0, p.1, next)?;
        }
        Ok(())
    }
}

pub(super) fn validate_inventory(i: &Inventory) -> Result<(), String> {
    i.validate_instances()?;
    for s in i.slots() {
        validate_loot(Loot::Stack(*s))?;
    }
    for t in i.tools() {
        validate_loot(Loot::Tool(*t))?;
    }
    Ok(())
}
fn validate_loot(loot: Loot) -> Result<(), String> {
    match loot {
        Loot::Stack(s) => {
            s.validate_instance()?;
            if s.count == 0 || s.count > crate::item::MAX_STACK {
                return Err("invalid stack count".into());
            }
        }
        Loot::Tool(t) => {
            if t.durability == 0
                || t.max_durability != ToolItem::new(t.kind, t.tier).max_durability
                || t.durability > t.max_durability
            {
                return Err("invalid tool instance".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
