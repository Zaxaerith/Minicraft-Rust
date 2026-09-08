use minicraft_client::core::game::Game;
use minicraft_common::parity::{
    Observation, PARITY_SCHEMA, ParityCommand, ParitySnapshot, ParityTrace,
};

pub fn game_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("game")?;
    let mut game = Game::default();
    trace
        .commands
        .iter()
        .enumerate()
        .map(|(tick, command)| {
            match *command {
                ParityCommand::GameWorld {
                    time,
                    calendar,
                    paused,
                } => {
                    if calendar >= 64_800 {
                        return Err("calendar outside Java day".into());
                    }
                    game.enter_world(minicraft_client::core::game::ClockSnapshot::new(
                        time, calendar, paused,
                    ));
                }
                ParityCommand::GameTitle => game.enter_title(),
                ParityCommand::GameMenu { focused } => game.tick_menu(focused),
                _ => return Err("non-game command".into()),
            }
            Ok(ParitySnapshot {
                schema: PARITY_SCHEMA,
                system: trace.system.clone(),
                tick: tick as u64,
                observation: Observation::GameClock {
                    time: game.game_time(),
                    calendar: game.calendar_tick(),
                    paused: game.paused(),
                    ready: game.ready(),
                },
            })
        })
        .collect()
}
