use std::sync::Arc;

use tracing::debug;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

enum App {
    Loading,
    Ready {
        window: Arc<Window>
    },
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Self::Loading = self {
            let window_attributes = WindowAttributes::default()
                .with_title("WGPU Tutorial");

            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

            center_window(window.clone());

            event_loop.set_control_flow(ControlFlow::Wait);

            *self = Self::Ready {
                window
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        let Self::Ready {
            window,
            ..
        } = self
        else {
            return;
        };

        match event {
            WindowEvent::RedrawRequested => {
                debug!("Rendering");

                window.request_redraw();
            }
            WindowEvent::Resized(_) => {
                window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let mut app = App::Loading;

    let _ = event_loop.run_app(&mut app);
}

fn center_window(window: Arc<Window>) {
    if let Some(monitor) = window.current_monitor() {
        let screen_size = monitor.size();
        let window_size = window.outer_size();

        window.set_outer_position(winit::dpi::PhysicalPosition {
            x: screen_size.width.saturating_sub(window_size.width) as f64 / 2.0
                + monitor.position().x as f64,
            y: screen_size.height.saturating_sub(window_size.height) as f64 / 2.0
                + monitor.position().y as f64,
        });
    }
} 
