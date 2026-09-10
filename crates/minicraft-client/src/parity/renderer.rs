use crate::core::renderer::{RenderHost, Renderer};
use minicraft_common::parity::{
    Observation, PARITY_SCHEMA, ParityCommand, ParitySnapshot, ParityTrace,
};

#[derive(Default)]
struct Probe(Vec<String>);
impl RenderHost for Probe {
    fn render_level(&mut self) {
        self.0.push("level".into());
    }
    fn render_gui(&mut self) {
        self.0.push("gui".into());
    }
    fn render_display(&mut self) {
        self.0.push("display".into());
    }
}
pub fn renderer_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("renderer")?;
    trace
        .commands
        .iter()
        .enumerate()
        .map(|(tick, command)| {
            let ParityCommand::RenderLayers { ready, gui } = *command else {
                return Err("non-renderer command".into());
            };
            let mut probe = Probe::default();
            Renderer::render(&mut probe, ready, gui);
            Ok(ParitySnapshot {
                schema: PARITY_SCHEMA,
                system: trace.system.clone(),
                tick: tick as u64,
                observation: Observation::RenderLayers { layers: probe.0 },
            })
        })
        .collect()
}
