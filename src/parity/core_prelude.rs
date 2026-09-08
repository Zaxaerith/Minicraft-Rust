//! Headless probes around the shared production Updater prelude policy.
use minicraft_client::core::updater::{Updater, advance_score_time};
use minicraft_common::parity::{
    Observation, PARITY_SCHEMA, ParityCommand, ParitySnapshot, ParityTrace,
};

pub fn core_prelude_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("core_prelude")?;
    let mut updater = Updater::default();
    let mut calendar = 0;
    let mut score = 0;
    let mut over = false;
    trace
        .commands
        .iter()
        .enumerate()
        .map(|(tick, command)| {
            let mut saves = Vec::new();
            match *command {
                ParityCommand::CoreSetup {
                    paused,
                    autosave_ticks,
                    score_ticks,
                    game_over,
                } => {
                    updater = Updater::from_state(paused, autosave_ticks);
                    calendar = 0;
                    score = score_ticks;
                    over = game_over;
                }
                ParityCommand::CoreTick {
                    ticks,
                    focused,
                    has_display,
                    autosave,
                    alive,
                    score_mode,
                } => {
                    for _ in 0..ticks {
                        if updater.advance_autosave(autosave, over, alive) {
                            saves.push(format!("auto:{calendar}:{score}:{over}"));
                        }
                        let plan = updater.plan_tick(focused, has_display);
                        calendar += u64::from(plan.advance_calendar);
                        if advance_score_time(plan.advance_calendar, score_mode, over, &mut score) {
                            over = true;
                        }
                    }
                }
                ParityCommand::CoreSave => {
                    saves.push(format!("manual:{calendar}:{score}:{over}"));
                    updater.reset_autosave();
                }
                _ => return Err("non-core command in core_prelude trace".into()),
            }
            Ok(ParitySnapshot {
                schema: PARITY_SCHEMA,
                system: trace.system.clone(),
                tick: tick as u64,
                observation: Observation::CorePrelude {
                    autosave_ticks: updater.autosave_ticks(),
                    calendar_ticks: calendar,
                    score_ticks: score,
                    game_over: over,
                    paused: updater.paused(),
                    saves,
                },
            })
        })
        .collect()
}
