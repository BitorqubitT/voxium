use egui::epaint::textures;

use crate::data::image::ImageData;
use crate::data::image_source::ImageSource;
use crate::data::image_source::VolumeData;
use crate::data::volume::VolumeGpu;
use crate::gpu::gpu::Gpu;

pub struct ViewTransform {
    pub zoom: f32,
    pub offset: egui::Vec2,
}

impl Default for ViewTransform {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset: egui::Vec2::ZERO,
        }
    }
}

// use these for offscreen 2d view.
pub struct RenderTarget2d {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

// Want to render to a output canvas in imageviewer. (the we give id and paint in egui i think, check this)
pub struct ImageViewer {
    // TODO: Remove this, because it will keep giving ownership problems. App owns source.
    //pub source: Option<ImageSource>,
    pub transform: ViewTransform,

    // TODO: check these again
    pub render_target: Option<RenderTarget2d>,
    pub current_view_size: egui::Vec2,
    pub egui_texture_id: Option<egui::TextureId>,
    pub current_slice_depth: f32,
}

impl ImageViewer {
    pub fn ui(
        &mut self,
        //maybe remove option, we only call this when there is a source?
        ui: &mut egui::Ui,
        source: Option<&ImageSource>,
        egui_renderer: &mut egui_wgpu::Renderer,
        gpu: &Gpu,
    ) {
        let source = match source {
            Some(src) => src,
            None => return,
        };
        let available_size = ui.available_size();

        match source {
            ImageSource::Single(single) => {
                //TODO: check if we want to keep calling this. Or is there a better way
                self.render_image(ui, &single);
            }
            // TODO: What do i want here? Just check if volume.gpu is present and then run render_volume using self.
            ImageSource::Volume(volume) => {
                if let Some(ref volume_gpu) = volume.gpu {
                    self.recreate_canvas(
                        &gpu.device,
                        available_size.x as u32,
                        available_size.y as u32,
                    );

                    // Need clone, old situation: pass ref to self with an exlsuive mutable access. wont work
                    // image is 2d so cheap i guess
                    let canvas_view = self
                        .render_target
                        .as_ref()
                        .map(|target| target.view.clone());

                    // Give internal 2d view to renderer
                    // Dont actually need to give volume_gpu
                    if let Some(view) = canvas_view {
                        self.render_volume(ui, egui_renderer, gpu, volume_gpu);
                    }
                } else {
                    // TODO: add some code to fill center
                    let (rect, _) = ui.allocate_exact_size(available_size, egui::Sense::hover());
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "Loading volume data",
                        egui::FontId::proportional(16.0),
                        ui.visuals().weak_text_color(),
                    );
                }
            }
        }
    }

    pub fn recreate_canvas(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if let Some(ref target) = self.render_target {
            if target.width == width && target.height == height {
                return;
            }
        }

        // 1. Create the virtual screen)
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("egui_volume_render_target_2d"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1, //2D image
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            // Diff type maybe?
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 3. Save it
        self.render_target = Some(RenderTarget2d {
            texture,
            view,
            width,
            height,
        });

        // Reset so the new updated texture handle on the next frame!
        self.egui_texture_id = None;
    }

    fn render_image(&mut self, ui: &mut egui::Ui, image: &ImageData) {
        let image_size = image.size * self.transform.zoom;

        let available = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(available, egui::Sense::drag());

        let image_rect = egui::Rect::from_min_size(
            rect.center() - image_size * 0.5 + self.transform.offset,
            image_size,
        );

        ui.painter().image(
            image.texture.id(),
            image_rect,
            egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }

    pub fn render_volume(
        &mut self,
        ui: &mut egui::Ui,
        egui_renderer: &mut egui_wgpu::Renderer,
        gpu: &Gpu,
        volume_gpu: &VolumeGpu,
    ) {
        ui.label("volume ready on gpu");
        let available_size = ui.available_size();
        self.current_view_size = available_size;

        ui.add(egui::Slider::new(&mut self.current_slice_depth, 0.0..=1.0).text("DICOM Slice"));

        // render pipeline to quickly test if everything is working.
        // TODO: spit this up
        if let Some(ref target) = self.render_target {
            // 1. Recreate the small uniform buffer for the modified slice depth frame data
            use wgpu::util::DeviceExt;
            println!("Rendering slice at depth: {}", self.current_slice_depth);
            let settings_data = [self.current_slice_depth, 50.0f32];

            let settings_buffer =
                gpu.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("slice_settings_uniform"),
                        contents: bytemuck::cast_slice(&settings_data),
                        usage: wgpu::BufferUsages::UNIFORM,
                    });

            // 2. Build the lightweight frame BindGroup linking to the cached layout
            let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("slice_test_bg"),
                layout: &gpu.bind_group_layout, // Using cached layout
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&volume_gpu.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&gpu.sampler), // Can also be stored in Gpu
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: settings_buffer.as_entire_binding(),
                    },
                ],
            });

            // 3. Execute the RenderPass using gpu.device and encoder
            let mut encoder = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("volume_slice_render_encoder"),
                });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Volume Slice Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target.view, // Draw onto our virtual offscreen target texture!
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });

                render_pass.set_pipeline(&gpu.render_pipeline); // 👈 Matched name to gpu.rs
                render_pass.set_bind_group(0, &bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }

            gpu.queue.submit(std::iter::once(encoder.finish()));
        }

        // Pass the rendered view to egui
        if self.egui_texture_id.is_none() {
            if let Some(ref target) = self.render_target {
                let tex_id = egui_renderer.register_native_texture(
                    &gpu.device,
                    &target.view, // Use the real rendering destination
                    wgpu::FilterMode::Linear,
                );
                self.egui_texture_id = Some(tex_id);
            }
        }

        ui.vertical(|ui| {
            ui.label("Rendering via offscreen GPU Texture:");

            if let Some(tex_id) = self.egui_texture_id {
                let image_widget =
                    egui::Image::from_texture((tex_id, available_size)).sense(egui::Sense::drag());

                ui.add(image_widget);
            }
        });
    }
}
