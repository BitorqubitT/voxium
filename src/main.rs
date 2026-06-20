use std::sync::Arc;
pub mod app;
pub mod gpu;
pub mod viewer;
pub mod data;
pub mod dicom;
use crate::gpu::gpu::Gpu;
use crate::app::MyApp;

use winit::{
    event::*,
    event_loop::EventLoop,
    window::Window,
};

// stopped using eframe because it keeps causing issues with wgpu

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    // cant get alternative to work yet
    #[allow(deprecated)]
    let window = Arc::new(
        event_loop
            .create_window(
                Window::default_attributes()
                    .with_title("Voxium")
            )
            .unwrap()
    );

    let gpu = Gpu::new(window.clone()).unwrap();

    let mut app = MyApp::new(gpu);

    #[allow(deprecated)]
    event_loop
        .run(move |event, target| {

            match event {
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => {
                            target.exit();
                        }

                        WindowEvent::Resized(size) => {
                            app.gpu.resize(size);
                        }

                        WindowEvent::RedrawRequested => {
                            // render later
                            match app.gpu.render() {
                                Ok(_) => {}
                                Err(_) => {
                                    // handle error
                                }
                            }
                        }

                        _ => {}
                    }
                }

                Event::AboutToWait => {
                    window.request_redraw();
                }

                _ => {}
            }
        })
        .unwrap();
}