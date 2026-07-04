use crate::data::image::ImageData;
use crate::data::image_source::ImageSource;
use crate::data::volume::VolumeGpu;
use crate::gpu::gpu::Gpu;
use wgpu::util::DeviceExt;



const SHADER_SRC: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct ViewSettings {
    slice_depth: f32,
    window_center: f32,
    window_width: f32,
    padding: f32,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    
    let x_u32 = (vertex_index << 1u) & 2u;
    let y_u32 = vertex_index & 2u;
    
    let x = f32(x_u32);
    let y = f32(y_u32);
    
    out.uv = vec2<f32>(x, y);
    
    // Map UV space (0..2) to WebGPU Clip Space (-1..1)
    out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    
    return out;
}

@group(0) @binding(0) var t_volume: texture_3d<u32>; // Unsigned Integer texture
@group(0) @binding(2) var<uniform> settings: ViewSettings;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size = textureDimensions(t_volume);
    
    // Clamp the coordinates to [0.0, 0.999] to guarantee we don't round up out of bounds
    let coords = vec3<i32>(
        i32(clamp(in.uv.x, 0.0, 0.999) * f32(tex_size.x)),
        i32(clamp(in.uv.y, 0.0, 0.999) * f32(tex_size.y)),
        i32(clamp(settings.slice_depth, 0.0, 0.999) * f32(tex_size.z))
    );
    
    // Load the raw voxel value directly
    let raw_val_u32 = textureLoad(t_volume, coords, 0).r;
    
    // We 1024 here to get true Hounsfield Units, and slicer values are standard:
    let raw_val = f32(raw_val_u32) - 1024.0;

    let half_width = settings.window_width / 2.0;
    let lower_bound = settings.window_center - half_width;
    
    // Scale the voxel value linearly between lower_bound and upper_bound
    let normalized_bright = (raw_val - lower_bound) / settings.window_width;
    
    let final_bright = clamp(normalized_bright, 0.0, 1.0);
    
    let final_color = vec3<f32>(final_bright, final_bright, final_bright);
    
    return vec4<f32>(final_color, 1.0);
}
"#;

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

// use these for offscreen 2d view.
//TODO: Change this
pub struct RenderTarget3d {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

// Want to render to a output canvas in imageviewer. (the we give id and paint in egui i think, check this)
pub struct Raymarcher3d {
    // TODO: Remove this, because it will keep giving ownership problems. App owns source.
    //pub source: Option<ImageSource>,
    // TODO: check these again
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
    pub transform: ViewTransform,
    pub render_target: Option<RenderTarget3d>,
    pub current_view_size: egui::Vec2,
    pub egui_texture_id: Option<egui::TextureId>,
    pub current_slice_depth: f32,
    pub window_center: f32,
    pub window_width: f32,
}

impl Raymarcher3d {
    pub fn new(device: &wgpu::Device) -> Self {
        let (render_pipeline, bind_group_layout, sampler) = Raymarcher3d::pipeline(device);

        Self {
            render_pipeline,
            bind_group_layout,
            sampler,
            transform: ViewTransform::default(),
            render_target: None,
            current_view_size: egui::Vec2::ZERO,
            egui_texture_id: None,
            current_slice_depth: 0.0,
            window_center: 40.0,
            window_width: 400.0,
        }
    }

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
                        self.render_volume_2d(ui, egui_renderer, gpu, volume_gpu);
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
            label: Some("egui_volume_render_target_3d"),
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
        self.render_target = Some(RenderTarget3d {
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
    ) {
        ui.label("volume ready on gpu");
        let available_size = ui.available_size();
        self.current_view_size = available_size;

        //TODO: Raymarching we need a second pipeline
        // need a camera
        // depending on camera angle we fire rays into the volume and we draw based on when they enter and leave
        //TODO: remove this from viewer code
        ui.add(egui::Slider::new(&mut self.current_slice_depth, 0.0..=1.0).text("DICOM Slice"));
        ui.add(
            egui::Slider::new(&mut self.window_center, -1000.0..=1000.0)
                .text("Window Center (Brightness)"),
        );
        ui.add(
            egui::Slider::new(&mut self.window_width, 1.0..=2000.0).text("Window Width (Contrast)"),
        );

        // render pipeline to quickly test if everything is working.
        // TODO: spit this up
        if let Some(ref target) = self.render_target {
            // 1. Recreate the small uniform buffer for the modified slice depth frame data
            //println!("Rendering slice at depth: {}", self.current_slice_depth);
            let settings_data = [
                self.current_slice_depth,
                self.window_center,
                self.window_width,
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
            source: wgpu::ShaderSource::Wgsl(SHADER_SRC.into()),
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
