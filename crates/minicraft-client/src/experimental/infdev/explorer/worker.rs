use super::{Command, Snapshot, Start, VIEW_X, VIEW_Y};
use crate::experimental::infdev::{
    Cell, Ground,
    paged::{Key, World},
};
use std::{
    collections::BTreeSet,
    sync::{
        Arc,
        mpsc::{Receiver, SyncSender, TrySendError},
    },
    time::{Duration, Instant},
};

pub(super) fn advance_survival_deadline(next: &mut Instant, now: Instant) {
    // Retain fractional deadlines: Windows timeout rounding must not turn
    // a nominal 60 Hz loop into 30 Hz while the player waits for stamina.
    if now.saturating_duration_since(*next) > Duration::from_millis(100) {
        *next = now; // Bound catch-up after slow IO or a suspended process.
    }
    *next += Duration::from_nanos(1_000_000_000 / 60);
}

#[cfg(test)]
pub(super) fn needed(position: (i64, i64)) -> BTreeSet<Key> {
    needed_at(0, position)
}

pub(super) fn needed_at(depth: i8, position: (i64, i64)) -> BTreeSet<Key> {
    let mut keys = BTreeSet::new();
    for dx in [-VIEW_X, 0, VIEW_X] {
        for dy in [-VIEW_Y, 0, VIEW_Y] {
            if let (Some(x), Some(y)) = (position.0.checked_add(dx), position.1.checked_add(dy)) {
                keys.insert(Key::at_depth(depth, x, y));
            }
        }
    }
    keys
}

fn load_chunk(world: &mut World, key: Key) -> Result<Arc<[Cell]>, String> {
    let origin_x = key.x.checked_mul(64).ok_or("chunk x overflow")?;
    let origin_y = key.y.checked_mul(64).ok_or("chunk y overflow")?;
    let mut cells = Vec::with_capacity(4096);
    for y in 0..64 {
        for x in 0..64 {
            cells.push(world.cell_at(key.depth, origin_x + x, origin_y + y)?);
        }
    }
    Ok(cells.into())
}

fn open(start: Start) -> Result<World, String> {
    match start {
        Start::Create {
            root,
            name,
            seed,
            layered,
            survival,
        } => {
            std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
            if survival {
                World::create_survival(&root, &name, seed, 16)
            } else if layered {
                World::create_layers(&root, &name, seed, 16)
            } else {
                World::create(&root, &name, seed, 16)
            }
        }
        Start::Open(directory) => World::open(&directory, 16),
    }
}

pub(super) fn run(start: Start, receive: Receiver<Command>, send: SyncSender<Snapshot>) {
    let (mut world, mut state) = match open(start) {
        Ok(world) => {
            let status = if world.recovered_backup() {
                "BACKUP RECOVERED; GC HOLD RETAINED"
            } else {
                "LOADING VISIBLE CHUNKS..."
            };
            let snapshot = Snapshot {
                position: world.position(),
                depth: world.depth(),
                layered: world.is_layered(),
                survival: world.survival().cloned(),
                status: status.into(),
                ..Snapshot::default()
            };
            (Some(world), snapshot)
        }
        Err(error) => (
            None,
            Snapshot {
                status: format!("OPEN FAILED: {error}"),
                ..Snapshot::default()
            },
        ),
    };
    let mut pending = Some(state.clone());
    let mut failed_load = false;
    let mut next_survival_tick = Instant::now();
    loop {
        if let Some(update) = pending.take() {
            match send.try_send(update) {
                Ok(()) => (),
                Err(TrySendError::Full(update)) => pending = Some(update),
                Err(TrySendError::Disconnected(_)) => {
                    if let Some(world) = world.as_mut()
                        && let Err(error) = world.checkpoint()
                    {
                        eprintln!("Infdev abandoned session save failed: {error}");
                    }
                    return;
                }
            }
        }
        let command = match receive.recv_timeout(Duration::from_millis(3)) {
            Ok(command) => Some(command),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                if let Some(world) = world.as_mut()
                    && let Err(error) = world.checkpoint()
                {
                    eprintln!("Infdev disconnected session save failed: {error}");
                }
                return;
            }
        };
        // Commands can touch one destination chunk. Never add an additional
        // viewport load in that same iteration; GUI rendering has no IO at all.
        let touched_chunk = command
            .as_ref()
            .is_some_and(|c| !matches!(c, Command::Quit(_)));
        let mut changed = false;
        match command {
            Some(Command::Quit(ack)) => {
                let result = world.as_mut().map_or(Ok(()), World::checkpoint);
                if result.is_ok() {
                    // Release the exclusive store lock before notifying GUI.
                    drop(world.take());
                    if let Some(ack) = ack {
                        let _ = ack.send(Ok(()));
                    }
                    return;
                }
                let error = result.expect_err("failed save");
                state.status = format!("SAVE FAILED: {error}");
                state.save_failed = true;
                if let Some(ack) = ack {
                    let _ = ack.send(Err(error));
                }
                changed = true;
            }
            Some(command) => {
                if let Some(world) = world.as_mut() {
                    state.action_serial = state.action_serial.wrapping_add(1);
                    let result = match command {
                        Command::Survival(action) => world.survival_action(action),
                        Command::Move(dx, dy) => world.step(dx, dy),
                        Command::Stairs => crate::experimental::infdev::stairs::travel(world),
                        Command::Edit(dx, dy) => {
                            let (x, y) = world.position();
                            match (x.checked_add(i64::from(dx)), y.checked_add(i64::from(dy))) {
                                (Some(x), Some(y)) => {
                                    world.cell_at(world.depth(), x, y).and_then(|old| {
                                        // Deliberately limited, reversible probe: only
                                        // Grass <-> Flower ordinal 0. Other terrain is untouched.
                                        let new = match old {
                                            Cell {
                                                ground: Ground::Grass,
                                                data: 0,
                                            } if world.depth() == 0 => Cell {
                                                ground: Ground::Flower,
                                                data: 0,
                                            },
                                            Cell {
                                                ground: Ground::Flower,
                                                data: 0,
                                            } if world.depth() == 0 => Cell {
                                                ground: Ground::Grass,
                                                data: 0,
                                            },
                                            Cell {
                                                ground: Ground::Dirt,
                                                data: 0,
                                            } if world.depth() < 0 => Cell {
                                                ground: Ground::Rock,
                                                data: 0,
                                            },
                                            Cell {
                                                ground: Ground::Rock,
                                                data: 0,
                                            } if world.depth() < 0 => Cell {
                                                ground: Ground::Dirt,
                                                data: 0,
                                            },
                                            _ => {
                                                return Err(if world.depth() < 0 {
                                                    "EDIT ONLY DIRT / ROCK"
                                                } else {
                                                    "EDIT ONLY GRASS / OXEYE DAISY"
                                                }
                                                .into());
                                            }
                                        };
                                        world.set_cell_at(world.depth(), x, y, new)?;
                                        state.chunks.remove(&Key::at_depth(world.depth(), x, y));
                                        Ok(())
                                    })
                                }
                                _ => Err("EDIT COORDINATE OVERFLOW".into()),
                            }
                        }
                        Command::Quit(_) => unreachable!("quit handled above"),
                    };
                    if matches!(
                        command,
                        Command::Survival(crate::experimental::infdev::survival::Action::Attack)
                    ) {
                        state.chunks.clear();
                    }
                    state.survival = world.survival().cloned();
                    if world.depth() != state.depth {
                        state.chunks.clear();
                    }
                    state.position = world.position();
                    state.depth = world.depth();
                    let passive = matches!(
                        command,
                        Command::Survival(crate::experimental::infdev::survival::Action::Tick)
                    );
                    if !passive {
                        state.save_failed = false;
                    }
                    state.status = match result {
                        Ok(()) if passive => state.status.clone(),
                        Ok(()) => if world.is_survival() {
                            "EXPERIMENTAL SURVIVAL"
                        } else {
                            "FLIGHT EXPLORER / NOT SURVIVAL"
                        }
                        .into(),
                        Err(error) => format!("ACTION FAILED: {error}"),
                    };
                    failed_load = false;
                    changed = true;
                }
            }
            None => (),
        }
        if let Some(world) = world.as_mut() {
            if world.is_survival() && Instant::now() >= next_survival_tick {
                advance_survival_deadline(&mut next_survival_tick, Instant::now());
                match world.survival_action(crate::experimental::infdev::survival::Action::Tick) {
                    Ok(()) => state.survival = world.survival().cloned(),
                    Err(e) => state.status = format!("SURVIVAL TICK FAILED: {e}"),
                }
                changed = true;
            }
            let keys = needed_at(state.depth, state.position);
            state.chunks.retain(|key, _| keys.contains(key));
            if !touched_chunk
                && !failed_load
                && let Some(key) = keys
                    .iter()
                    .find(|key| !state.chunks.contains_key(key))
                    .copied()
            {
                match load_chunk(world, key) {
                    Ok(cells) => {
                        state.chunks.insert(key, cells);
                    }
                    Err(error) => {
                        state.status = format!("CHUNK LOAD FAILED: {error}");
                        failed_load = true;
                    }
                }
                changed = true;
            }
            state.ready = state.chunks.contains_key(&Key::at_depth(
                state.depth,
                state.position.0,
                state.position.1,
            ));
            if !state.save_failed
                && let Some(warning) = world.collection_warning()
            {
                state.status = format!("SAVED; GC DEFERRED: {warning}");
            }
        }
        if changed {
            pending = Some(state.clone());
        }
    }
}
