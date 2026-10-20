//! Entity pages share the terrain AVL implementation, object writer, checkpoint
//! publication and GC. No independent player/container save transaction exists.
use super::super::{
    entities::{Page, Position},
    survival::{self, Action, Panel},
};
use super::*;
use objects::Domain;

pub(super) struct CachedEntities {
    pub page: Page,
    pub dirty: bool,
    pub(super) touched: u64,
}

impl World {
    pub(super) fn read_entity_object(&self, key: Key, object: Object) -> Result<Page, String> {
        let bytes = objects::read(&self.directory, Domain::Entities, object)?;
        let page: Page = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        page.validate()?;
        if page.key != key
            || serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|e| e.to_string())?
                != serde_json::to_value(&page).map_err(|e| e.to_string())?
        {
            return Err("entity page key/unknown fields mismatch".into());
        }
        Ok(page)
    }
    pub(super) fn validate_entity_root(
        &mut self,
        root: Option<Object>,
    ) -> Result<HashSet<(Domain, Object)>, String> {
        let mut marked = HashSet::new();
        self.walk(root, None, None, 0, &mut marked, Domain::Entities)?;
        Ok(marked)
    }
    pub(crate) fn entity_page(&mut self, key: Key) -> Result<Page, String> {
        if !self.is_survival() || !key.valid(true) {
            return Err("entities require a valid Survival chunk".into());
        }
        if !self.entity_cache.contains_key(&key) {
            let page = match self.find(self.saved.entity_root, key)? {
                Some(o) => self.read_entity_object(key, o)?,
                None => Page::empty(key),
            };
            if self.entity_cache.len() == self.capacity {
                let oldest = *self
                    .entity_cache
                    .iter()
                    .min_by_key(|(_, c)| c.touched)
                    .expect("bounded nonempty cache")
                    .0;
                if self.entity_cache.values().any(|c| c.dirty) {
                    self.checkpoint()?;
                }
                self.entity_cache.remove(&oldest);
            }
            self.entity_cache.insert(
                key,
                CachedEntities {
                    page,
                    dirty: false,
                    touched: 0,
                },
            );
        }
        self.clock = self.clock.wrapping_add(1);
        let cached = self.entity_cache.get_mut(&key).expect("loaded");
        cached.touched = self.clock;
        Ok(cached.page.clone())
    }
    pub(super) fn write_entity_pages(&mut self) -> Result<Option<Object>, String> {
        let dirty: Vec<_> = self
            .entity_cache
            .iter()
            .filter(|(_, c)| c.dirty)
            .map(|(&k, c)| (k, c.page.clone()))
            .collect();
        if !dirty.is_empty() {
            let directory = self.directory.join("entities");
            match fs::create_dir(&directory) {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
                Err(e) => return Err(e.to_string()),
            }
            real_path(&directory, true)?;
        }
        let mut root = self.saved.entity_root;
        for (key, page) in dirty {
            page.validate()?;
            let object = objects::write(
                &self.directory,
                Domain::Entities,
                &serde_json::to_vec(&page).map_err(|e| e.to_string())?,
            )?;
            root = Some(self.insert(root, key, object, 0)?);
        }
        Ok(root)
    }
    pub(super) fn act_with_entities(&mut self, action: Action) -> Result<bool, String> {
        if self
            .survival()
            .is_some_and(|s| s.entity_rules != survival::EntityRules::Legacy)
        {
            return self.act_dynamic(action);
        }
        let mut state = self.survival.clone().ok_or("not a Survival world")?;
        // Mining's map write remains LAST: do not load an entity page after an
        // edited tile but before installing the matching inventory in memory.
        if !matches!(
            action,
            Action::Tick | Action::Interact | Action::DiagnosticDeath
        ) && state.player.health > 0
        {
            state.act(self, action)?;
            self.survival = Some(state);
            return Ok(false);
        }
        let was_dead = state.player.health == 0;
        let receipt = state.death_recorded;
        let (x, y) = self.position();
        let at = Position {
            depth: self.depth(),
            x,
            y,
        };
        let target = if state.player.health > 0 && matches!(action, Action::Interact) {
            let (x, y) = state.target(self)?;
            Position {
                depth: at.depth,
                x,
                y,
            }
        } else {
            at
        };
        let mut page = self.entity_page(target.key())?;
        let old = page.clone();
        let cell = self.cell_at(at.depth, at.x, at.y)?;
        let can_place =
            !survival::tile(cell).solid(cell.data) && !state.station_at(at.depth, at.x, at.y);
        if state.player.health == 0 {
            page.record_death(&mut state, at, can_place);
        }
        state.act(self, action)?;
        if state.player.health == 0 {
            page.record_death(&mut state, at, can_place);
        } else if matches!(action, Action::Tick) && state.panel == Panel::Closed {
            page.tick(at, &mut state.player.inventory);
        } else if !was_dead && matches!(action, Action::Interact) {
            page.interact(target, &mut state.player.inventory);
        }
        // Preflight ALL validation before installing either half. The page is
        // still local; eviction during reads above can publish only old state.
        state.validate()?;
        let ownership_changed = old.entities.len() != page.entities.len()
            || old.entities.iter().zip(&page.entities).any(|(a, b)| {
                let super::super::entities::Kind::DeathChest(a) = &a.kind else {
                    return true;
                };
                let super::super::entities::Kind::DeathChest(b) = &b.kind else {
                    return true;
                };
                a.inventory != b.inventory
            });
        let critical = ownership_changed || !receipt && state.death_recorded;
        if page != old {
            page.validate()?;
            state.entity_pages_present = true;
            let cache = self
                .entity_cache
                .get_mut(&target.key())
                .expect("page retained during terrain-only actions");
            cache.page = page;
            cache.dirty = true;
        }
        self.survival = Some(state);
        Ok(critical)
    }
}
