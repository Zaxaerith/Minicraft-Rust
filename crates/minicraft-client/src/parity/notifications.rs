use crate::core::notifications::Notifications;
use minicraft_common::parity::{
    Observation, PARITY_SCHEMA, ParityCommand, ParitySnapshot, ParityTrace, RenderScene,
};

pub fn notification_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("notifications")?;
    let mut notes = Notifications::default();
    trace
        .commands
        .iter()
        .enumerate()
        .map(|(tick, command)| {
            let mut visible = None;
            match *command {
                ParityCommand::NotificationAdd { value, reset } => {
                    let message = format!("Note {value}");
                    if let Some(ticks) = reset {
                        notes.notify(message, ticks);
                    } else {
                        notes.add(message);
                    }
                }
                ParityCommand::NotificationTick { ticks } => {
                    for _ in 0..ticks {
                        notes.tick();
                    }
                }
                ParityCommand::NotificationRender { gui, permanent } => {
                    if gui {
                        visible = Some(notes.prepare_gui(permanent).to_vec());
                    }
                }
                ParityCommand::NotificationClear => notes.clear(),
                _ => return Err("non-notification command in notifications trace".into()),
            }
            Ok(ParitySnapshot {
                schema: PARITY_SCHEMA,
                system: trace.system.clone(),
                tick: tick as u64,
                observation: Observation::Notifications {
                    messages: notes.messages().to_vec(),
                    ticks: notes.ticks(),
                    ticking: notes.ticking(),
                    visible,
                },
            })
        })
        .collect()
}

pub(super) fn render_fixture(
    screen: &mut crate::gfx::Screen,
    assets: &crate::assets::Assets,
    locale: &crate::resource::localization::Localization,
    scene: RenderScene,
) {
    let mut notes = Notifications::default();
    for message in [
        "Old notice",
        "World Saved!",
        "Air Wizard Defeated!",
        "The Dungeon is now open!",
    ] {
        notes.add(message);
    }
    if scene == RenderScene::NotificationsWrapped {
        notes.clear();
        notes.add("This notification wraps across multiple lines without hiding the next message.");
        notes.add("Line one\nLine two");
        notes.add("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-long");
    }
    if scene == RenderScene::NotificationsExpired {
        notes.prepare_gui(false);
        for _ in 0..181 {
            notes.tick();
        }
    }
    screen.clear(0);
    crate::world::notifications::render_status(
        screen,
        assets,
        locale,
        &mut notes,
        (scene == RenderScene::StatusSaving).then_some(37),
        matches!(
            scene,
            RenderScene::StatusSleeping | RenderScene::StatusSaving
        ),
        matches!(
            scene,
            RenderScene::StatusSleeping | RenderScene::StatusSaving
        ),
        "ESCAPE",
    );
}
