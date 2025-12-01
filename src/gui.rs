use glazer::winit::{event::WindowEvent, event_loop::ActiveEventLoop, window::Window};
use std::sync::Arc;

pub struct Egui {
    egui_glow: egui_glow::EguiGlow,
    window: Arc<Window>,
}

impl Egui {
    pub fn new(event_loop: &ActiveEventLoop, window: &Window, gl: &glazer::glow::Context) -> Self {
        let egui_glow = egui_glow::EguiGlow::new(
            event_loop,
            unsafe { Arc::from_raw(gl as *const _) },
            None,
            None,
            false,
        );

        Self {
            egui_glow,
            window: unsafe { Arc::from_raw(window as *const _) },
        }
    }

    pub fn handle_input(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.egui_glow.on_window_event(window, event).consumed
    }

    pub fn show(&mut self, ui: impl FnMut(&egui::Context)) {
        self.egui_glow.run(&self.window, ui);
    }

    pub fn paint(&mut self) {
        self.egui_glow.paint(&self.window);
    }
}
