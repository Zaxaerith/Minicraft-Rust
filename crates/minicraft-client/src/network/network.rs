//! Java `network/Network.java` 1:1.
//!
//! Entity-id helpers scan the same level list Java walks via `Game.levels`.
//! Release lookup keeps the GitHub payload selection and the `VersionInfo`
//! shape; the live HTTP call itself is intentionally out of scope (no HTTP
//! client in this crate) — callers supply the releases array or the fallback.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::sync::Mutex;

use crate::entity::Entity;
use crate::level::noise::JavaRandom;
use crate::saveload::Version;
use crate::world::Level;

/// Java `Game.VERSION` used on the Network error path. The multiplayer wire
/// keeps the separate `GAME_VERSION` label (`2.2.4-rust`).
pub const LOCAL_RELEASE_VERSION: &str = "2.2.4";

/// Java `minicraft.core.VersionInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionInfo {
    pub version: Version,
    pub release_url: String,
    pub release_name: String,
}

impl VersionInfo {
    /// Java `new VersionInfo(Version, String, String)`.
    pub fn new(
        version: Version,
        release_url: impl Into<String>,
        release_name: impl Into<String>,
    ) -> Self {
        Self {
            version,
            release_url: release_url.into(),
            release_name: release_name.into(),
        }
    }

    /// Java `new VersionInfo(JSONObject)` from one GitHub release object.
    ///
    /// Java always does `tag_name.substring(1)` to drop the leading `v`.
    pub fn from_github_release(release: &serde_json::Value) -> Option<Self> {
        let tag = release.get("tag_name")?.as_str()?;
        let version = Version::parse(tag.get(1..).unwrap_or(""));
        Some(Self {
            version,
            release_url: release.get("html_url")?.as_str()?.to_owned(),
            release_name: release.get("name")?.as_str()?.to_owned(),
        })
    }
}

static LATEST_VERSION: Mutex<Option<VersionInfo>> = Mutex::new(None);

/// Java `Network.getLatestVersion()`.
pub fn get_latest_version() -> Option<VersionInfo> {
    LATEST_VERSION.lock().unwrap().clone()
}

/// Store a resolved release (success path or explicit override).
pub fn set_latest_version(info: VersionInfo) {
    *LATEST_VERSION.lock().unwrap() = Some(info);
}

/// Java catch / non-200 path: `new VersionInfo(VERSION, "", "")`.
pub fn store_local_version_fallback() {
    set_latest_version(VersionInfo::new(
        Version::parse(LOCAL_RELEASE_VERSION),
        "",
        "",
    ));
}

/// Java `Network.findLatestVersion` selection over a GitHub releases array.
///
/// Java walks indices `10..=0` for the highest `tag_name` (minus the leading
/// `v`) and then stores that release. The published Java loop condition
/// (`0 >= i`) is inverted so stock Java never enters the body and always keeps
/// index 0; this port implements the intended highest-of-first-11 scan.
pub fn select_latest_release(releases: &[serde_json::Value]) -> Option<VersionInfo> {
    let mut highest: Option<Version> = None;
    for release in releases.iter().take(11) {
        let tag = release.get("tag_name")?.as_str()?;
        let version = Version::parse(tag.get(1..).unwrap_or(""));
        let better = match highest {
            None => true,
            Some(best) => version.compare(&best) == Ordering::Greater,
        };
        if better {
            highest = Some(version);
        }
    }
    highest?;
    // Re-scan for the first release whose tag equals the winning version so
    // html_url / name come from the same object Java would have stored.
    for release in releases.iter().take(11) {
        let tag = release.get("tag_name").and_then(|value| value.as_str())?;
        let version = Version::parse(tag.get(1..).unwrap_or(""));
        if Some(version) == highest {
            return VersionInfo::from_github_release(release);
        }
    }
    None
}

/// Java `Network.getEntity(int eid)` — first match across every level.
pub fn get_entity<'a>(eid: i32, levels: impl IntoIterator<Item = &'a Level>) -> Option<&'a Entity> {
    if eid <= 0 {
        return None;
    }
    let candidate = eid as u64;
    levels
        .into_iter()
        .flat_map(|level| level.entities.entities())
        .find(|entity| entity.id == candidate)
}

/// Every entity id Java would see while scanning `Game.levels`.
pub fn collect_entity_ids<'a>(levels: impl IntoIterator<Item = &'a Level>) -> Vec<u64> {
    levels
        .into_iter()
        .flat_map(|level| level.entities.entities().iter().map(|entity| entity.id))
        .collect()
}

/// Java `Network.idIsAvailable(int eid)` — reserved 0, must be positive, unique.
pub fn id_is_available(eid: i32, occupied_ids: impl IntoIterator<Item = u64>) -> bool {
    if eid <= 0 {
        return false;
    }
    let candidate = eid as u64;
    !occupied_ids.into_iter().any(|id| id == candidate)
}

thread_local! {
    /// Java keeps a private static `Random` for entity ids.
    static ID_RANDOM: RefCell<JavaRandom> = RefCell::new(JavaRandom::new(0x4E57_524B));
}

/// Java `Network.generateUniqueEntityId()` over the caller's level list.
pub fn generate_unique_entity_id<'a>(levels: impl IntoIterator<Item = &'a Level>) -> i32 {
    let occupied = collect_entity_ids(levels);
    ID_RANDOM.with(|cell| {
        let mut random = cell.borrow_mut();
        generate_unique_entity_id_with(&mut random, &occupied)
    })
}

/// Java's retry loop with an injectable RNG (tests / deterministic worlds).
///
/// Java logs at the 1000th try and keeps going; the loop is unbounded here
/// exactly as in Java (`do { … } while (!idIsAvailable(eid))`).
pub fn generate_unique_entity_id_with(random: &mut JavaRandom, occupied: &[u64]) -> i32 {
    loop {
        let eid = random.next_i32();
        if id_is_available(eid, occupied.iter().copied()) {
            return eid;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::EntityArena;
    use crate::item::{ItemId, ItemStack};
    use crate::level::tile::Tile;
    use serde_json::json;

    fn empty_level() -> Level {
        Level {
            depth: 0,
            tiles: vec![Tile::Grass],
            data: vec![0],
            max_mob_count: 200,
            pending_spawns: Vec::new(),
            entities: EntityArena::default(),
        }
    }

    #[test]
    fn id_is_available_rejects_reserved_and_taken_ids() {
        assert!(!id_is_available(0, std::iter::empty::<u64>()));
        assert!(!id_is_available(-1, std::iter::empty::<u64>()));
        assert!(id_is_available(1, std::iter::empty::<u64>()));
        assert!(!id_is_available(7, [7u64]));
        assert!(id_is_available(8, [7u64]));
    }

    #[test]
    fn get_entity_scans_every_level_like_java() {
        let mut surface = empty_level();
        let mut cave = empty_level();
        let id = surface
            .entities
            .spawn_item(ItemStack::new(ItemId::Apple, 1), 8, 8);
        let eid = i32::try_from(id).expect("test ids fit in i32");
        assert!(get_entity(eid, [&surface, &cave]).is_some());
        assert!(get_entity(999_999, [&surface, &cave]).is_none());
        assert!(get_entity(0, [&surface, &cave]).is_none());
        let _ = cave
            .entities
            .spawn_item(ItemStack::new(ItemId::Key, 1), 1, 1);
        assert_eq!(collect_entity_ids([&surface, &cave]).len(), 2);
    }

    #[test]
    fn generate_unique_entity_id_skips_taken_values() {
        // java.util.Random(0).nextInt() is negative, so the reserved-range
        // rule alone forces another draw.
        assert_eq!(JavaRandom::new(0).next_i32(), -1_155_484_576);
        let mut retry = JavaRandom::new(0);
        let generated = generate_unique_entity_id_with(&mut retry, &[]);
        assert!(generated > 0);

        // Occupied positive ids from seed 1's early draws must be skipped.
        let mut probe = JavaRandom::new(1);
        let mut taken = Vec::new();
        for _ in 0..8 {
            let value = probe.next_i32();
            if value > 0 {
                taken.push(value as u64);
            }
        }
        let mut generator = JavaRandom::new(1);
        let unique = generate_unique_entity_id_with(&mut generator, &taken);
        assert!(unique > 0);
        assert!(!taken.contains(&(unique as u64)));
    }

    #[test]
    fn version_info_parses_github_release_like_java() {
        let release = json!({
            "tag_name": "v2.2.4",
            "html_url": "https://example.invalid/tag/2.2.4",
            "name": "2.2.4"
        });
        let info = VersionInfo::from_github_release(&release).unwrap();
        assert_eq!(info.version, Version::parse("2.2.4"));
        assert_eq!(info.release_url, "https://example.invalid/tag/2.2.4");
        assert_eq!(info.release_name, "2.2.4");
    }

    #[test]
    fn select_latest_release_picks_highest_of_first_eleven() {
        let releases = vec![
            json!({"tag_name": "v2.2.3", "html_url": "u0", "name": "n0"}),
            json!({"tag_name": "v2.3.0", "html_url": "u1", "name": "n1"}),
            json!({"tag_name": "v2.2.4", "html_url": "u2", "name": "n2"}),
        ];
        let info = select_latest_release(&releases).unwrap();
        assert_eq!(info.version, Version::parse("2.3.0"));
        assert_eq!(info.release_url, "u1");
        store_local_version_fallback();
        assert_eq!(
            get_latest_version().unwrap().version,
            Version::parse(LOCAL_RELEASE_VERSION)
        );
        set_latest_version(info);
        assert_eq!(get_latest_version().unwrap().release_name, "n1");
    }
}
