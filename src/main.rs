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
            Event::WindowEvent { event, window_id } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => {
                        target.exit();
                    }

                    WindowEvent::Resized(size) => {
                        app.gpu.resize(size);
                        let _ = app.egui_winit.on_window_event(
                            &window,
                            &WindowEvent::Resized(size),
                        );

                        window.request_redraw();
                    }

                    WindowEvent::ScaleFactorChanged { .. } => {
                        window.request_redraw();
                    }

                    WindowEvent::RedrawRequested => {

                        //TODO: move most of this code to gpu/rs
                        let frame = match app.gpu.render() {
                            Some(frame) => frame,
                            None => return,
                        };

                        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

                        let mut encoder = app.gpu.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor {
                                label: Some("Main Encoder"),
                            },
                        );

                        // 1. INPUT & LAYOUT
                        let raw_input = app.egui_winit.take_egui_input(&window);

                        let egui_ctx = app.egui_ctx.clone(); 

                        let full_output = egui_ctx.run(raw_input, |ctx| {
                            app.ui(ctx);
                        });

                        let paint_jobs = egui_ctx.tessellate(
                            full_output.shapes,
                            full_output.pixels_per_point,
                        );

                        let screen_desc = egui_wgpu::ScreenDescriptor {
                            size_in_pixels: [
                                frame.texture.width(),
                                frame.texture.height(),
                            ],
                            pixels_per_point: egui_ctx.pixels_per_point(),
                        };

                        // 2. TEXTURE UPLOADS
                        // Do multiple passes because egui works this way? check this
                        for (id, delta) in &full_output.textures_delta.set {
                            app.egui_renderer.update_texture(
                                &app.gpu.device,
                                &app.gpu.queue,
                                *id,
                                delta,
                            );
                        }

                        // 3. BUFFER UPLOADS
                        app.egui_renderer.update_buffers(
                            &app.gpu.device,
                            &app.gpu.queue,
                            &mut encoder,
                            &paint_jobs,
                            &screen_desc,
                        );

                        // 4. RENDER PASS
                        {
                            let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("egui"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    depth_slice: None,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Load, 
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None,
                                occlusion_query_set: None,
                                timestamp_writes: None,
                                multiview_mask: None,
                            });

                            // Making it 'static so we can use it
                            let mut static_rpass = rpass.forget_lifetime();

                            app.egui_renderer.render(
                                &mut static_rpass,
                                &paint_jobs,
                                &screen_desc,
                            );
                        } 

                        // 5. CLEANUP &SUBMIT
                        for id in full_output.textures_delta.free {
                            app.egui_renderer.free_texture(&id);
                        }

                        app.gpu.queue.submit(Some(encoder.finish()));
                        frame.present();
                    }

                    _ => {
                        let _ = app.egui_winit.on_window_event(&window, &event);
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