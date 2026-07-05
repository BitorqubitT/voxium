use crate::data::image::ImageData;
use crate::data::image_source::ImageSource;
use crate::data::volume::VolumeGpu;
use crate::gpu::gpu::Gpu;
use wgpu::util::DeviceExt;

pub struct ViewTransform {
    pub zoom: f32,
    pub offset: egui::Vec2,
}

//TODO: remove these or create new file for it
impl Default for ViewTransform {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset: egui::Vec2::ZERO,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub inv_view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
}

pub struct RenderTarget3d {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

pub struct Raymarcher3d {
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
    pub transform: ViewTransform,
    pub render_target: Option<RenderTarget3d>,
    pub current_view_size: egui::Vec2,
    pub egui_texture_id: Option<egui::TextureId>,

    pub camera_pitch: f32,
    pub camera_yaw: f32,
    pub camera_buffer: wgpu::Buffer,
}

impl Raymarcher3d {
    pub fn new(device: &wgpu::Device) -> Self {
        let (pipeline, layout, sampler, pitch, yaw, buffer) = Raymarcher3d::pipeline(device);

        Self {
            render_pipeline: pipeline,
            bind_group_layout: layout,
            sampler,
            transform: ViewTransform::default(),
            render_target: None,
            current_view_size: egui::Vec2::ZERO,
            egui_texture_id: None,
            camera_pitch: pitch,
            camera_yaw: yaw,
            camera_buffer: buffer,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        source: Option<&ImageSource>,
        egui_renderer: &mut egui_wgpu::Renderer,
        gpu: &Gpu,
        windows_center: f32,
        window_width: f32,
        zoom: f32,
    ) {
        let source = match source {
            Some(src) => src,
            None => return,
        };
        let available_size = ui.available_size();

        match source {
            ImageSource::Single(single) => {
                self.render_image(ui, &single);
            }
            ImageSource::Volume(volume) => {
                if let Some(ref volume_gpu) = volume.gpu {
                    self.recreate_canvas(
                        &gpu.device,
                        available_size.x as u32,
                        available_size.y as u32,
                    );

                    let canvas_view = self
                        .render_target
                        .as_ref()
                        .map(|target| target.view.clone());

                    if let Some(_) = canvas_view {
                        // Triggers the 3D rendering pass safely
                        self.render_volume_3d(
                            ui,
                            egui_renderer,
                            gpu,
                            volume_gpu,
                            windows_center,
                            window_width,
                            zoom,
                        );
                    }
                } else {
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

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("egui_volume_render_target_3d"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.render_target = Some(RenderTarget3d {
            texture,
            view,
            width,
            height,
        });

        self.egui_texture_id = None;
    }

    fn render_image(&mut self, ui: &mut egui::Ui, image: &ImageData) {
        let image_size = image.size * self.transform.zoom;
        let available = ui.available_size();
        let (rect, _) = ui.allocate_exact_size(available, egui::Sense::drag());

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

    pub fn render_volume_3d(
        &mut self,
        ui: &mut egui::Ui,
        egui_renderer: &mut egui_wgpu::Renderer,
        gpu: &Gpu,
        volume_gpu: &VolumeGpu,
        windows_center: f32,
        window_width: f32,
        zoom_boy: f32,
    ) {
        let available_size = ui.available_size();
        self.current_view_size = available_size;

        // 1. Calculate the camera position in world space by rotating a back-vector
        let rotation = glam::Mat4::from_rotation_y(self.camera_yaw)
            * glam::Mat4::from_rotation_x(self.camera_pitch);
        
        let camera_pos = rotation.transform_point3(glam::Vec3::new(0.0, 0.0, zoom_boy));

        // 2. Build a proper View Matrix that looks at the center of the dataset (0, 0, 0)
        let view = glam::Mat4::look_at_lh(
            camera_pos,       
            glam::Vec3::ZERO, 
            glam::Vec3::Y,
        );

        // 3. Match the aspect ratio of your actual egui UI panel so it doesn't stretch
        let aspect_ratio = available_size.x / available_size.y;
        let proj = glam::Mat4::perspective_lh(45.0f32.to_radians(), aspect_ratio, 0.1, 10.0);

        // 4. Combine them and invert
        let inv_view_proj = (proj * view).inverse();

        let camera_data = CameraUniform {
            inv_view_proj: inv_view_proj.to_cols_array_2d(),
            camera_pos: [camera_pos.x, camera_pos.y, camera_pos.z, 1.0],
        };

        // 2. Safely updated with prefixed gpu configurations
        gpu.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera_data));

        // 3. Recreate the dynamic settings block matching WGSL alignments
        let settings_data = [0.0f32, windows_center, window_width, 0.0f32];
        let settings_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("raymarch_settings_uniform"),
                contents: bytemuck::cast_slice(&settings_data),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // 4. Extract target view safely out of Option
        let target_view = &self.render_target.as_ref().unwrap().view;

        // 5. Package bindings using the existing volume reference
        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("raymarch_bind_group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&volume_gpu.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: settings_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("raymarch_encoder"),
            });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("raymarch_render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_bind_group(0, &bind_group, &[]);
            rpass.draw(0..3, 0..1);
        }

        gpu.queue.submit(Some(encoder.finish()));

        if self.egui_texture_id.is_none() {
            if let Some(ref target) = self.render_target {
                let tex_id = egui_renderer.register_native_texture(
                    &gpu.device,
                    &target.view,
                    wgpu::FilterMode::Linear,
                );
                self.egui_texture_id = Some(tex_id);
            }
        }

        ui.vertical(|ui| {
            if let Some(tex_id) = self.egui_texture_id {
                let image_widget =
                    egui::Image::from_texture((tex_id, available_size)).sense(egui::Sense::drag());

                let response = ui.add(image_widget);

                if response.dragged() {
                    self.camera_yaw += response.drag_delta().x * 0.01;
                    self.camera_pitch += response.drag_delta().y * 0.01;
                }
            }
        });
    }

    pub fn pipeline(
        device: &wgpu::Device,
    ) -> (
        wgpu::RenderPipeline,
        wgpu::BindGroupLayout,
        wgpu::Sampler,
        f32,
        f32,
        wgpu::Buffer,
    ) {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("raymarch_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raymarch.wgsl").into()),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("raymarch_sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("raymarch_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
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

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("raymarch_pipeline_layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("raymarch_render_pipeline"),
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
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_uniform_buffer"),
            contents: bytemuck::bytes_of(&CameraUniform {
                inv_view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
                camera_pos: [0.0, 0.0, -2.0, 1.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        (
            pipeline,
            bind_group_layout,
            sampler,
            0.0,
            0.0,
            camera_buffer,
        )
    }
}
