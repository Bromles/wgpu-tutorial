use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::GpuContext;
use crate::example::Example;

pub fn run<E: Example>(title: &str) {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app = App::<E>::Loading {
        title: title.to_owned(),
    };
    event_loop
        .run_app(&mut app)
        .expect("Failed to run event loop");
}

enum App<E: Example> {
    Loading {
        title: String,
    },
    Ready {
        window: Arc<Window>,
        ctx: Box<GpuContext>,
        example: Box<E>,
        need_resize: bool,
    },
}

impl<E: Example> ApplicationHandler for App<E> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Self::Loading { title } = self {
            let window_attrs = WindowAttributes::default()
                .with_title(title.clone())
                .with_visible(false);

            let window = Arc::new(
                event_loop
                    .create_window(window_attrs)
                    .expect("Failed to create window"),
            );

            center_window(&window);

            event_loop.set_control_flow(ControlFlow::Wait);

            let ctx = Box::new(GpuContext::new(window.clone()));
            let example = Box::new(E::init(&ctx));

            *self = Self::Ready {
                window,
                ctx,
                example,
                need_resize: false,
            };
        }

        let Self::Ready {
            window,
            ctx,
            example,
            ..
        } = self
        else {
            return;
        };

        render_frame(ctx, example.as_mut(), window);
        window.set_visible(true);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Self::Ready {
            window,
            ctx,
            example,
            need_resize,
        } = self
        else {
            return;
        };

        match event {
            WindowEvent::RedrawRequested => {
                if *need_resize {
                    let size = window.inner_size();
                    ctx.resize(size);
                    example.resize(ctx, size);
                    *need_resize = false;
                }

                render_frame(ctx, example.as_mut(), window);
                window.request_redraw();
            }
            WindowEvent::Resized(_) => {
                *need_resize = true;
                window.request_redraw();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let (PhysicalKey::Code(KeyCode::Escape), ElementState::Pressed) =
                    (event.physical_key, event.state)
                {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}

fn render_frame<E: Example>(ctx: &mut GpuContext, example: &mut E, window: &Window) {
    let Some((frame, view, mut encoder)) = ctx.acquire_frame() else {
        return;
    };

    example.render(ctx, &view, &mut encoder);

    ctx.queue.submit([encoder.finish()]);
    window.pre_present_notify();
    frame.present();
}

fn center_window(window: &Window) {
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
