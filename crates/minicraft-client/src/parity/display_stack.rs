//! Lifecycle probes use the SAME stack as the real application. This oracle
//! adapter does not implement a second copy of the queue/parent algorithm.

use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use crate::{
    core::{
        display_stack::{DisplayStack, LifecycleEvent},
        game::Game,
    },
    screen::display::{Display, DisplayId, ParentPolicy},
};
use minicraft_common::parity::{
    DisplayAction, Observation, PARITY_SCHEMA, ParityCommand, ParitySnapshot, ParityTrace,
};

struct Probe {
    name: String,
    root: bool,
    selection: u32,
    names: Rc<RefCell<BTreeMap<DisplayId, String>>>,
    events: Rc<RefCell<Vec<String>>>,
}

impl Display for Probe {
    fn parent_policy(&self) -> ParentPolicy {
        if self.root {
            ParentPolicy::Root
        } else {
            ParentPolicy::Inherit
        }
    }
    fn init(&mut self, parent: Option<DisplayId>) {
        let parent = parent
            .map(|id| self.names.borrow()[&id].clone())
            .unwrap_or_else(|| "-".into());
        self.events
            .borrow_mut()
            .push(format!("init:{}:{parent}", self.name));
    }
    fn on_exit(&mut self) {
        self.events.borrow_mut().push(format!("exit:{}", self.name));
    }
}

pub fn display_stack_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("display_stack")?;
    let mut stack = DisplayStack::<Probe>::default();
    let names = Rc::new(RefCell::new(BTreeMap::new()));
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut snapshots = Vec::new();
    let mut serial = 0;
    let mut game = Game::default();
    for (tick, command) in trace.commands.iter().enumerate() {
        let ParityCommand::DisplayInput { action } = command else {
            return Err("non-display command in display_stack trace".into());
        };
        let mut sounds = Vec::new();
        match action {
            DisplayAction::World
            | DisplayAction::Title
            | DisplayAction::Pause
            | DisplayAction::Child => {
                let kind = match action {
                    DisplayAction::Title => "title",
                    DisplayAction::Pause => "pause",
                    DisplayAction::World => "world",
                    _ => "child",
                };
                let name = format!("{kind}{serial}");
                serial += 1;
                let id = stack.open(Probe {
                    name: name.clone(),
                    root: *action != DisplayAction::Child,
                    selection: 0,
                    names: names.clone(),
                    events: events.clone(),
                });
                names.borrow_mut().insert(id, name);
            }
            DisplayAction::Clear => stack.clear(),
            DisplayAction::Update => stack.update_observed(|event, probe| {
                if event == LifecycleEvent::Init {
                    if probe.name.starts_with("title") {
                        game.enter_title();
                    }
                    if probe.name.starts_with("world") {
                        game.enter_world(crate::core::game::ClockSnapshot::new(0, 0, true));
                    }
                }
                let hook = if event == LifecycleEvent::Init {
                    "init"
                } else {
                    "exit"
                };
                events.borrow_mut().push(format!(
                    "observed:{hook}:{}:{}",
                    probe.name,
                    game.ready()
                ));
            }),
            DisplayAction::SelectNext => {
                if let Some(display) = stack.current_mut() {
                    display.selection += 1;
                }
            }
            DisplayAction::ExitZero
            | DisplayAction::ExitOne
            | DisplayAction::ExitTwo
            | DisplayAction::ExitMany => {
                let depth = match action {
                    DisplayAction::ExitZero => 0,
                    DisplayAction::ExitOne => 1,
                    DisplayAction::ExitTwo => 2,
                    _ => 100,
                };
                if stack.exit(depth) {
                    sounds.push("craft".into());
                }
            }
        }
        let mut parents = Vec::new();
        let mut cursor = stack.requested_id();
        while let Some(id) = cursor {
            parents.push(stack.get(id).unwrap().name.clone());
            cursor = stack.parent_id(id);
        }
        snapshots.push(ParitySnapshot {
            schema: PARITY_SCHEMA,
            system: trace.system.clone(),
            tick: tick as u64,
            observation: Observation::Display {
                ready: game.ready(),
                current: stack.current().map(|display| display.name.clone()),
                requested: stack.requested().map(|display| display.name.clone()),
                current_selection: stack.current().map_or(0, |display| display.selection),
                parents,
                events: std::mem::take(&mut *events.borrow_mut()),
                sounds,
            },
        });
    }
    Ok(snapshots)
}
