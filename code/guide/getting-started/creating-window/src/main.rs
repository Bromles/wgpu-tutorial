#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use tracing::debug;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

// #region appstate
enum App {
    Loading,
    Ready { window: Arc<Window> },
}
// #endregion appstate

impl ApplicationHandler for App {
    // #region appsetup
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Self::Loading = self {
            let window_attributes = WindowAttributes::default().with_title("WGPU Tutorial");

            let window = Arc::new(
                event_loop
                    .create_window(window_attributes)
                    .expect("Failed to create window"),
            );

            center_window(window.clone());

            *self = Self::Ready { window }
        }
    }
    // #endregion appsetup

    // #region apploop
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Self::Ready { window, .. } = self else {
            return;
        };

        match event {
            WindowEvent::RedrawRequested => {
                debug!("Rendering");

                window.request_redraw();
            }
            WindowEvent::Resized(_) => {
                debug!("Resized");

                window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {}
        }
    }
    // #endregion apploop
}

// #region main
fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let mut app = App::Loading;

    event_loop
        .run_app(&mut app)
        .expect("Failed to run event loop");
}
// #endregion main

// #region centerwindow
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
// #endregion centerwindow
