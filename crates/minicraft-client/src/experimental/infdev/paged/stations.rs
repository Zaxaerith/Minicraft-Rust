//! The original workstation actions operate on one target. Project that single
//! entity into the local action candidate, then move it back before validation
//! or publication. No session copy of paged furniture is ever committed.
use super::super::{
    entities::{Kind, Page, Position},
    survival::{State, Station, StationKind, StationRules},
};
use super::*;

pub(super) fn contains(pages: &BTreeMap<Key, Page>, at: Position) -> bool {
    kind(pages.get(&at.key()), at).is_some()
}
fn kind(page: Option<&Page>, at: Position) -> Option<StationKind> {
    page?.entities.iter().find_map(|e| {
        if e.x == at.x.rem_euclid(64) as u8
            && e.y == at.y.rem_euclid(64) as u8
            && let Kind::Station(k) = e.kind
        {
            return Some(k);
        }
        None
    })
}
impl World {
    pub(crate) fn paged_station_at(
        &mut self,
        depth: i8,
        x: i64,
        y: i64,
    ) -> Result<Option<StationKind>, String> {
        if !self
            .survival()
            .is_some_and(|s| s.station_rules == StationRules::PagedV1)
        {
            return Ok(None);
        }
        let at = Position { depth, x, y };
        Ok(kind(Some(&self.entity_page(at.key())?), at))
    }
}
pub(super) fn project(
    s: &mut State,
    pages: &BTreeMap<Key, Page>,
    at: Position,
) -> Result<(), String> {
    if s.station_rules != StationRules::PagedV1 {
        return Ok(());
    }
    if !s.stations.is_empty() {
        return Err("paged workstations cannot reside in session".into());
    }
    if let Some(k) = kind(pages.get(&at.key()), at) {
        s.stations.push(Station {
            depth: at.depth,
            x: at.x,
            y: at.y,
            kind: k,
        });
    }
    Ok(())
}
pub(super) fn collect(
    s: &mut State,
    pages: &mut BTreeMap<Key, Page>,
    at: Position,
) -> Result<(), String> {
    if s.station_rules != StationRules::PagedV1 {
        return Ok(());
    }
    if s.stations.len() > 1 {
        return Err("unexpected workstation action outbox".into());
    }
    let candidate = s.stations.pop();
    let old = kind(pages.get(&at.key()), at);
    if let Some(c) = &candidate
        && (c.depth, c.x, c.y) != (at.depth, at.x, at.y)
    {
        return Err("workstation destination mismatch".into());
    }
    let next = candidate.map(|c| c.kind);
    if old == next {
        return Ok(());
    }
    let page = pages
        .get_mut(&at.key())
        .ok_or("unreserved workstation page")?;
    page.entities.retain(|e| {
        !(matches!(e.kind, Kind::Station(_))
            && e.x == at.x.rem_euclid(64) as u8
            && e.y == at.y.rem_euclid(64) as u8)
    });
    if let Some(k) = next {
        page.insert(at, Kind::Station(k), None)?;
    }
    Ok(())
}
