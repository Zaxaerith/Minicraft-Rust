//! Java Renderer.render phase ordering; concrete drawing remains in adapters.
pub trait RenderHost {
    fn render_level(&mut self);
    fn render_gui(&mut self);
    fn render_display(&mut self);
}

pub struct Renderer;
impl Renderer {
    pub fn render(host: &mut impl RenderHost, ready: bool, render_gui: bool) {
        if ready {
            host.render_level();
            if render_gui {
                host.render_gui();
            }
        }
        host.render_display();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn display_is_always_last_and_gui_requires_both_gates() {
        #[derive(Default)]
        struct Probe(Vec<&'static str>);
        impl RenderHost for Probe {
            fn render_level(&mut self) {
                self.0.push("level");
            }
            fn render_gui(&mut self) {
                self.0.push("gui");
            }
            fn render_display(&mut self) {
                self.0.push("display");
            }
        }
        for (ready, gui, expected) in [
            (false, false, vec!["display"]),
            (false, true, vec!["display"]),
            (true, false, vec!["level", "display"]),
            (true, true, vec!["level", "gui", "display"]),
        ] {
            let mut probe = Probe::default();
            Renderer::render(&mut probe, ready, gui);
            assert_eq!(probe.0, expected);
        }
    }
}
