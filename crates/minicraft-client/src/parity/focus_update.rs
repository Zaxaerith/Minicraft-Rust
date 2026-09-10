//! Shared production clock and Updater policy, driven without real-time sleeps.

use crate::core::{game_loop::FixedTickAccumulator, updater::Updater};
use minicraft_common::parity::{
    Observation, PARITY_SCHEMA, ParityCommand, ParitySnapshot, ParityTrace,
};

pub fn focus_update_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("focus_update")?;
    let mut clock = FixedTickAccumulator::default();
    let mut updater = Updater::default();
    let mut calendar_ticks = 0;
    let mut input_ticks = 0;
    trace
        .commands
        .iter()
        .enumerate()
        .map(|(tick, command)| {
            let ParityCommand::FocusFrame {
                elapsed_nanos,
                focused,
                has_display,
            } = *command
            else {
                return Err("non-focus command in focus_update trace".into());
            };
            let updates = clock.advance(std::time::Duration::from_nanos(elapsed_nanos));
            for _ in 0..updates {
                let plan = updater.plan_tick(focused, has_display);
                calendar_ticks += u64::from(plan.advance_calendar);
                input_ticks += u64::from(plan.dispatch_input);
            }
            Ok(ParitySnapshot {
                schema: PARITY_SCHEMA,
                system: trace.system.clone(),
                tick: tick as u64,
                observation: Observation::FocusUpdate {
                    updates: updates as u64,
                    calendar_ticks,
                    input_ticks,
                    paused: updater.paused(),
                },
            })
        })
        .collect()
}
