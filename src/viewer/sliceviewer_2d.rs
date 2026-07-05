use crate::data::image::ImageData;
use crate::data::image_source::ImageSource;
use crate::data::volume::VolumeGpu;
use crate::gpu::gpu::Gpu;
use wgpu::util::DeviceExt;

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
pub struct SliceViewer2d {
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
    pub transform: ViewTransform,
    pub render_target: Option<RenderTarget2d>,
    pub current_view_size: egui::Vec2,
    pub egui_texture_id: Option<egui::TextureId>,
}

impl SliceViewer2d {
    pub fn new(device: &wgpu::Device) -> Self {
        let (render_pipeline, bind_group_layout, sampler) = SliceViewer2d::pipeline(device);

        Self {
            render_pipeline,
            bind_group_layout,
            sampler,
            transform: ViewTransform::default(),
            render_target: None,
            current_view_size: egui::Vec2::ZERO,
            egui_texture_id: None,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        source: Option<&ImageSource>,
        egui_renderer: &mut egui_wgpu::Renderer,
        gpu: &Gpu,
        window_center: f32,
        window_width: f32,
        current_slice: f32,
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
                        self.render_volume_2d(ui, egui_renderer, gpu, volume_gpu, window_center, current_slice, window_width);
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

    pub fn render_volume_2d(
        &mut self,
        ui: &mut egui::Ui,
        egui_renderer: &mut egui_wgpu::Renderer,
        gpu: &Gpu,
        volume_gpu: &VolumeGpu,
        window_center: f32,
        current_slice_depth: f32,
        window_width: f32,

    ) {
        ui.label("volume ready on gpu");
        let available_size = ui.available_size();
        self.current_view_size = available_size;

        // render pipeline to quickly test if everything is working.
        // TODO: spit this up
        if let Some(ref target) = self.render_target {
            // 1. Recreate the small uniform buffer for the modified slice depth frame data
            let settings_data = [
                current_slice_depth,
                window_center,
                window_width,
                0.0f32,
            ];

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
                layout: &self.bind_group_layout, // Using cached layout
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&volume_gpu.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler), // Can also be stored in Gpu
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

                render_pass.set_pipeline(&self.render_pipeline);
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

    pub fn pipeline(
        device: &wgpu::Device,
    ) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout, wgpu::Sampler) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("slice_test_shader"),
            //source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
            source: wgpu::ShaderSource::Wgsl(include_str!("sliceviewer_2d.wgsl").into()),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("slice_test_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("slice_test_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D3, // Crucial: It's 3D!
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group_layout_ref = &bind_group_layout;

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("slice_test_pipeline_layout"),
            bind_group_layouts: &[Some(bind_group_layout_ref)],
            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("slice_test_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb, // Canvas Target Format
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            cache: None,
            multiview_mask: None,
        });

        (render_pipeline, bind_group_layout, sampler)
    }
}
