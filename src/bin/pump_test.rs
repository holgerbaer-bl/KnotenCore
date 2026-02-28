use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes};

struct InitApp {
    window: Option<Arc<Window>>,
}

impl ApplicationHandler for InitApp {
    fn resumed(&mut self, active_el: &ActiveEventLoop) {
        if self.window.is_none() {
            let win = active_el
                .create_window(WindowAttributes::default())
                .unwrap();
            self.window = Some(Arc::new(win));
        }
    }

    fn window_event(
        &mut self,
        _active_el: &ActiveEventLoop,
        _id: winit::window::WindowId,
        _event: WindowEvent,
    ) {
    }
}

fn main() {
    let mut event_loop = EventLoop::new().unwrap();
    let mut app = InitApp { window: None };
    use winit::platform::pump_events::EventLoopExtPumpEvents;

    // Pump events once to trigger resumed()
    event_loop.pump_app_events(Some(std::time::Duration::from_millis(10)), &mut app);

    if let Some(win) = app.window {
        println!("Window successfully created: {:?}", win.id());
    } else {
        println!("Failed to create window from pump_events");
    }
}
