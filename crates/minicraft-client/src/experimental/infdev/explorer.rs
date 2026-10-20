//! GUI adapter for the independent Infdev store. Disk work and terrain sampling
//! belong exclusively to one worker; rendering only reads bounded snapshots.
pub(super) mod render;
#[cfg(test)]
mod tests;
mod worker;

use super::{Cell, paged::Key};
use crate::{assets::Assets, gfx::Screen, io::input_event::Input};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::JoinHandle,
};

const VIEW_X: i64 = 11;
const VIEW_Y: i64 = 8;
const MOVE_TICKS: u8 = 6;

#[derive(Clone, Default)]
struct Snapshot {
    position: (i64, i64),
    depth: i8,
    layered: bool,
    chunks: BTreeMap<Key, Arc<[Cell]>>,
    status: String,
    ready: bool,
    save_failed: bool,
    survival: Option<super::survival::State>,
    action_serial: u64,
    entities: BTreeMap<Key, Arc<super::entities::Page>>,
}

impl Snapshot {
    fn cell(&self, x: i64, y: i64) -> Option<Cell> {
        self.chunks
            .get(&Key::at_depth(self.depth, x, y))?
            .get((x.rem_euclid(64) + y.rem_euclid(64) * 64) as usize)
            .copied()
    }
}

enum Command {
    Move(i32, i32),
    Edit(i32, i32),
    Stairs,
    Survival(super::survival::Action),
    Quit(Option<SyncSender<Result<(), String>>>),
}

enum Start {
    Create {
        root: PathBuf,
        name: String,
        seed: i64,
        layered: bool,
        survival: bool,
    },
    Open(PathBuf),
}

/// One GUI session. Keep this object alive after a failed `finish` so that the
/// worker retains dirty chunks and its exclusive lock while the user retries.
pub struct Explorer {
    commands: SyncSender<Command>,
    updates: Receiver<Snapshot>,
    thread: Option<JoinHandle<()>>,
    snapshot: Snapshot,
    closing: bool,
    quit_sent: bool,
    closed: bool,
    cooldown: u8,
    direction: (i32, i32),
    camera_offset: (i32, i32),
    ticks: u32,
    animation_enter_millis: i64,
}

impl Explorer {
    /// Returns immediately after starting the worker; initialization errors are
    /// displayed in `status`, without performing filesystem work on the GUI.
    pub fn create(root: PathBuf, name: String, seed: i64) -> Result<Self, String> {
        Self::start(Start::Create {
            root,
            name,
            seed,
            layered: false,
            survival: false,
        })
    }

    /// A separate v3 world; opening v2 never upgrades or regenerates its surface.
    pub fn create_layers(root: PathBuf, name: String, seed: i64) -> Result<Self, String> {
        Self::start(Start::Create {
            root,
            name,
            seed,
            layered: true,
            survival: false,
        })
    }

    pub fn create_survival(root: PathBuf, name: String, seed: i64) -> Result<Self, String> {
        Self::start(Start::Create {
            root,
            name,
            seed,
            layered: true,
            survival: true,
        })
    }
    pub(crate) fn action_serial(&self) -> u64 {
        self.snapshot.action_serial
    }
    pub(crate) fn survival_state(&self) -> Option<&super::survival::State> {
        self.snapshot.survival.as_ref()
    }
    pub(crate) fn diagnostic_death(&mut self) {
        let _ = self
            .commands
            .try_send(Command::Survival(super::survival::Action::DiagnosticDeath));
    }
    pub(crate) fn entity_pages(&self) -> impl Iterator<Item = &super::entities::Page> {
        self.snapshot.entities.values().map(|p| p.as_ref())
    }
    pub fn is_survival(&self) -> bool {
        self.snapshot.survival.is_some()
    }
    pub fn open(directory: PathBuf) -> Result<Self, String> {
        Self::start(Start::Open(directory))
    }

    fn start(start: Start) -> Result<Self, String> {
        let (commands, receive) = mpsc::sync_channel(1);
        let (send, updates) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("infdev-store".into())
            .spawn(move || worker::run(start, receive, send))
            .map_err(|error| error.to_string())?;
        Ok(Self {
            commands,
            updates,
            thread: Some(thread),
            snapshot: Snapshot {
                status: "OPENING / VALIDATING WORLD...".into(),
                ..Snapshot::default()
            },
            closing: false,
            quit_sent: false,
            closed: false,
            cooldown: 0,
            direction: (0, 1),
            camera_offset: (0, 0),
            ticks: 0,
            animation_enter_millis: crate::assets::animation_millis(),
        })
    }

    pub fn position(&self) -> (i64, i64) {
        self.snapshot.position
    }
    pub fn depth(&self) -> i8 {
        self.snapshot.depth
    }
    pub fn is_layered(&self) -> bool {
        self.snapshot.layered
    }
    pub fn status(&self) -> &str {
        &self.snapshot.status
    }
    pub fn is_ready(&self) -> bool {
        self.snapshot.ready && !self.closing
    }
    /// Number of snapshots held by the GUI, at most four 64x64 chunks.
    pub fn visible_chunk_count(&self) -> usize {
        self.snapshot.chunks.len()
    }
    pub fn visible_cell(&self, x: i64, y: i64) -> Option<Cell> {
        self.snapshot.cell(x, y)
    }

    fn poll(&mut self) {
        while let Ok(snapshot) = self.updates.try_recv() {
            let old = self.snapshot.position;
            let next = snapshot.position;
            if let Some(survival) = &snapshot.survival {
                self.camera_offset = (8 - survival.player.x, 8 - survival.player.y);
                self.direction = match survival.player.direction {
                    crate::world::Direction::Down => (0, 1),
                    crate::world::Direction::Up => (0, -1),
                    crate::world::Direction::Left => (-1, 0),
                    crate::world::Direction::Right => (1, 0),
                };
            } else if snapshot.depth != self.snapshot.depth {
                self.camera_offset = (0, 0);
            } else if let (Some(dx), Some(dy)) =
                (next.0.checked_sub(old.0), next.1.checked_sub(old.1))
                && (-1..=1).contains(&dx)
                && (-1..=1).contains(&dy)
            {
                self.camera_offset = (dx as i32 * 16, dy as i32 * 16);
            }
            if snapshot.save_failed {
                self.closing = false;
                self.quit_sent = false;
            }
            self.snapshot = snapshot;
        }
        if self.thread.as_ref().is_some_and(JoinHandle::is_finished) {
            let joined = self.thread.take().expect("worker handle").join();
            if joined.is_ok() && self.quit_sent {
                self.closed = true;
            } else {
                self.snapshot.ready = false;
                self.snapshot.status = "WORKER STOPPED UNEXPECTEDLY".into();
            }
        }
    }

    fn send(&self, command: Command) -> bool {
        match self.commands.try_send(command) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => false,
        }
    }

    pub fn tick(&mut self, input: &Input) {
        self.poll();
        self.ticks = self.ticks.wrapping_add(1);
        for offset in [&mut self.camera_offset.0, &mut self.camera_offset.1] {
            if self.snapshot.survival.is_some() {
                break;
            }
            *offset -= offset.signum() * offset.abs().min(4);
        }
        if input.exit
            && self
                .snapshot
                .survival
                .as_ref()
                .is_some_and(|s| s.panel != super::survival::Panel::Closed)
        {
            self.send(Command::Survival(super::survival::Action::Close));
            return;
        }
        if input.exit || (self.snapshot.save_failed && input.select) {
            self.request_save_quit();
        }
        if self.closing {
            if !self.quit_sent {
                self.quit_sent = self.send(Command::Quit(None));
            }
            return;
        }
        if !self.snapshot.ready {
            return;
        }
        if let Some(survival) = &self.snapshot.survival {
            use super::survival::{Action, Panel};
            let action = if input.menu {
                if survival.panel == Panel::Closed {
                    Action::Inventory
                } else {
                    Action::Close
                }
            } else if input.craft {
                Action::Crafting
            } else if survival.panel != Panel::Closed {
                if input.cursor_up_pressed || input.up_pressed {
                    Action::Cursor(-1)
                } else if input.cursor_down_pressed || input.down_pressed {
                    Action::Cursor(1)
                } else if input.attack || input.select {
                    Action::Confirm
                } else {
                    Action::Tick
                }
            } else if input.drop_one || input.drop_stack {
                Action::Drop(input.drop_stack)
            } else if input.pickup {
                Action::Pickup
            } else if input.attack {
                Action::Attack
            } else if input.select {
                Action::Interact
            } else if input.up {
                Action::Move(0, -1)
            } else if input.down {
                Action::Move(0, 1)
            } else if input.left {
                Action::Move(-1, 0)
            } else if input.right {
                Action::Move(1, 0)
            } else {
                Action::Tick
            };
            if !matches!(action, Action::Tick) && !self.send(Command::Survival(action)) {
                self.snapshot.status = "BUSY; RETRY AFTER LOADING".into();
            }
            return;
        }
        if self.cooldown > 0 {
            self.cooldown -= 1;
        }
        if input.pickup {
            if !self.send(Command::Stairs) {
                self.snapshot.status = "BUSY; RETRY STAIRS AFTER LOADING".into();
            }
            return;
        }
        if input.attack {
            if !self.send(Command::Edit(self.direction.0, self.direction.1)) {
                self.snapshot.status = "BUSY; RETRY EDIT AFTER LOADING".into();
            }
            return;
        }
        let movement = if input.up {
            Some((0, -1))
        } else if input.down {
            Some((0, 1))
        } else if input.left {
            Some((-1, 0))
        } else if input.right {
            Some((1, 0))
        } else {
            None
        };
        if let Some((dx, dy)) = movement {
            self.direction = (dx, dy);
            if self.cooldown == 0 && self.send(Command::Move(dx, dy)) {
                self.cooldown = MOVE_TICKS;
            }
        }
    }

    pub fn request_save_quit(&mut self) {
        self.closing = true;
    }

    /// True only after the successful save worker has exited and released its
    /// World/session lock. Failed saves leave the session alive for retry.
    pub fn has_finished(&self) -> bool {
        self.closed
    }

    /// Used by window close. Waiting is intentional here; all normal frames and
    /// Escape Save & Quit use nonblocking requests instead.
    pub fn finish(&mut self) -> Result<(), String> {
        self.poll();
        if self.closed {
            return Ok(());
        }
        if self.thread.is_none() {
            return Err(self.snapshot.status.clone());
        }
        // Escape may already have queued its asynchronous quit. Do not enqueue
        // another ack behind it: a successful first quit closes the receiver.
        while self.quit_sent {
            self.poll();
            if self.closed {
                return Ok(());
            }
            if self.snapshot.save_failed {
                return Err(self.snapshot.status.clone());
            }
            if self.thread.is_none() {
                return Err(self.snapshot.status.clone());
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let (ack, receive) = mpsc::sync_channel(1);
        self.commands
            .send(Command::Quit(Some(ack)))
            .map_err(|_| "Infdev worker disconnected".to_owned())?;
        match receive
            .recv()
            .map_err(|_| "Infdev save worker stopped".to_owned())?
        {
            Ok(()) => {
                self.quit_sent = true;
                if let Some(thread) = self.thread.take() {
                    thread
                        .join()
                        .map_err(|_| "Infdev worker panicked".to_owned())?;
                }
                self.closed = true;
                Ok(())
            }
            Err(error) => {
                self.closing = false;
                self.quit_sent = false;
                self.poll();
                Err(error)
            }
        }
    }

    pub fn render(&self, screen: &mut Screen, assets: &Assets) {
        render::draw(self, screen, assets);
    }
}

impl Drop for Explorer {
    fn drop(&mut self) {
        if !self.closed
            && let Err(error) = self.finish()
        {
            eprintln!("Infdev session could not save before drop: {error}");
        }
    }
}
