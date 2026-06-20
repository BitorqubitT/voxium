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

    let mut app = MyApp::new(gpu, &window);

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
                            app.egui_winit.as_mut().unwrap().on_window_event(&window, &WindowEvent::Resized(size));
                        }

                        WindowEvent::RedrawRequested => {
                           // ----------------------------
                            // 0. GET FRAME
                            // ----------------------------
                            let frame = match app.gpu.render() {
                                Ok(frame) => frame,
                                Err(_) => return,
                            };

                            let view = frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            let mut encoder = app.gpu.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor {
                                    label: Some("Main Encoder"),
                                },
                            );

                            // ----------------------------
                            // 1. EGUI INPUT (modern API)
                            // ----------------------------
                            let raw_input = app
                                .egui_state
                                .take_egui_input(&window);

                            // ----------------------------
                            // 2. RUN UI
                            // ----------------------------
                            let full_output = app.egui_ctx.run(raw_input, |ctx| {
                                app.ui(ctx);
                            });

                            // ----------------------------
                            // 3. TESSELLATE UI
                            // ----------------------------
                            let paint_jobs = app.egui_ctx.tessellate(
                                full_output.shapes,
                                full_output.pixels_per_point,
                            );

                            // ----------------------------
                            // 4. RENDER EGUI
                            // ----------------------------
                            app.egui_renderer.render(
                                &mut encoder,
                                &view,
                                &paint_jobs,
                                &full_output.textures_delta,
                                &app.gpu.device,
                                &app.gpu.queue,
                            );

                            // ----------------------------
                            // 5. SUBMIT GPU WORK
                            // ----------------------------
                            app.gpu.queue.submit(Some(encoder.finish()));

                            // ----------------------------
                            // 6. PRESENT FRAME
                            // ----------------------------
                            frame.present();
                        } 
                        _ => {
                            let _ = app.egui_winit.as_mut().unwrap().on_window_event(&window, &event);
                        }
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