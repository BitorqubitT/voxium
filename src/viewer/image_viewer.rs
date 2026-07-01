use egui::epaint::textures;

use crate::data::image_source::ImageSource;
use crate::data::image::ImageData;
use crate::data::image_source::VolumeData;
use crate::data::volume::VolumeGpu;

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


const SHADER_SRC: &str = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vertex_index << 1u) & 2) - 1.0;
    let y = f32(i32(vertex_index & 2u) - 1) * -1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return out;
}

// 1. Add a uniform block for settings
struct ViewSettings {
    slice_depth: f32,
    multiplier: f32, // We'll use this to multiply brightness!
};

@group(0) @binding(0) var t_volume: texture_3d<f32>;
@group(0) @binding(1) var s_volume: sampler;
@group(0) @binding(2) var<uniform> settings: ViewSettings; // Bound to slot 2
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_coords = vec3<f32>(in.uv, settings.slice_depth);
    let raw_val = textureSample(t_volume, s_volume, tex_coords).r;
    
    // --- DIAGNOSTIC VISUALIZATION ---
    
    // Test 1: Is the texture completely empty/zero? 
    // We will paint a dim blue tint across the whole canvas so you know the shader is actively drawing.
    var final_color = vec3<f32>(0.0, 0.0, 0.1); 

    if (raw_val < 0.0) {
        // Test 2: If values are negative (common in DICOM air pockets), paint them Bright Red
        final_color = vec3<f32>(abs(raw_val) * 0.1, 0.0, 0.0);
    } else if (raw_val > 0.0) {
        // Test 3: If values are positive, paint them Grayscale with your multiplier
        let bright = raw_val * settings.multiplier;
        final_color = vec3<f32>(bright, bright, bright);
    }
    
    return vec4<f32>(final_color, 1.0);
}
"#;

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
}

impl ImageViewer {
    pub fn ui(&mut self,
        //maybe remove option, we only call this when there is a source? 
              ui: &mut egui::Ui, 
              source: Option<&ImageSource>,
              egui_renderer: &mut egui_wgpu::Renderer,
              device: &wgpu::Device,
              queue: &wgpu::Queue, 
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
                        device, 
                 available_size.x as u32, 
                available_size.y as u32);

                    // Need clone, old situation: pass ref to self with an exlsuive mutable access. wont work
                    // image is 2d so cheap i guess
                    let canvas_view = self.render_target
                        .as_ref()
                        .map(|target| target.view.clone());


                    // Give internal 2d view to renderer
                    // Dont actually need to give volume_gpu
                    if let Some(view) = canvas_view {
                        self.render_volume(
                            ui, 
                            egui_renderer, 
                            device,
                            queue, 
                            &view,
                            volume_gpu,
                        );
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

    fn render_image(
        &mut self, 
        ui: &mut egui::Ui, 
        image: &ImageData) {
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
            egui::Rect::from_min_max(
                egui::Pos2::new(0.0, 0.0),
                egui::Pos2::new(1.0, 1.0),
            ),
            egui::Color32::WHITE,
        );
    }

    
    pub fn render_volume(
        &mut self, 
        ui: &mut egui::Ui, 
        egui_renderer: &mut egui_wgpu::Renderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        canvas_view: &wgpu::TextureView,
        volume_gpu: &VolumeGpu,
    ) { 
        ui.label("volume ready on gpu");
        let available_size = ui.available_size();
        self.current_view_size = available_size;

        // render pipeline to quickly test if everything is working.
        // TODO: spit this up
        if let Some(ref target) = self.render_target {
            
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

            //let settings_data = [
            //    0.5f32,  // slice_depth: 0.0 (front) to 1.0 (back). Let's peek right in the middle!
            //    50.0f32, // multiplier: Boosts visibility if values are extremely small decimals
            //];

            let settings_data = [
                0.01f32,  // Test near the front slice
                50.0f32,
            ];
            
            // TODO:  movethis
            use wgpu::util::DeviceExt;
            let settings_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("slice_settings_uniform"),
                contents: bytemuck::cast_slice(&settings_data),
                usage: wgpu::BufferUsages::UNIFORM,
            });

            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("slice_test_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
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

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("slice_test_bg"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&volume_gpu.view), // The 3D view
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: settings_buffer.as_entire_binding(),
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

            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("volume_slice_render_encoder"),
            });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("slice_test_pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &target.view, // Draw straight onto our flat 2D canvas view
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

                render_pass.set_pipeline(&render_pipeline);
                render_pass.set_bind_group(0, &bind_group, &[]);
                render_pass.draw(0..3, 0..1); // Triggers the vertex generator shader
            }

            queue.submit(std::iter::once(encoder.finish()));
        }

        // Pass the rendered view to egui
        if self.egui_texture_id.is_none() {
            if let Some(ref target) = self.render_target {
                let tex_id = egui_renderer.register_native_texture(
                    device, 
                    &target.view, // Use the real rendering destination
                    wgpu::FilterMode::Linear,
                );
                self.egui_texture_id = Some(tex_id);
            }
        }

        ui.vertical(|ui| {
            ui.label("Rendering via offscreen GPU Texture:");

            if let Some(tex_id) = self.egui_texture_id {
                let image_widget = egui::Image::from_texture((tex_id, available_size))
                    .sense(egui::Sense::drag());
                
                ui.add(image_widget);
            }
        });
    }

    fn render_volume2(
        &mut self, 
        ui: &mut egui::Ui, 
        egui_renderer: &mut egui_wgpu::Renderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue, // remove this after test
        canvas_view: &wgpu::TextureView,
        volume_gpu: &VolumeGpu,
    ) {

        if let Some(ref target) = self.render_target {
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("volume_slice_copy_encoder"),
            });
            
            let volume_size = volume_gpu.texture.size();
            // Using standard wgpu syntax for texture-to-texture copy definitions
            encoder.copy_texture_to_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &volume_gpu.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x: 0, y: 0, z: 0 }, // Front slice (Z = 0)
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &target.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: target.width.min(volume_size.width as u32),
                    height: target.height.min(volume_size.height as u32),
                    depth_or_array_layers: 1, // Only grab 1 slice deep from the 3D volume
                },
            );

            queue.submit(std::iter::once(encoder.finish()));
        }



        ui.label("volume ready on gpu");
        // Does this work? does available_size look at the window or not?
        let available_size = ui.available_size();
        self.current_view_size = available_size;

        // Register the canvas texture with egui
        // CANT PASS volume.view. egui expects 2d and volume is 3d.
        println!(" we hawt");
        if self.egui_texture_id.is_none() {
            // Give access to app.egui_renderei
            println!(" we hawt");
            let tex_id = egui_renderer.register_native_texture(
                device, 
                canvas_view, //volume view as 2d. 
                wgpu::FilterMode::Linear, // TODO: read this: https://gpuweb.github.io/gpuweb/#enumdef-gpumipmapfiltermode
            );
            self.egui_texture_id = Some(tex_id);
        }

        println!(" we hawt");
        ui.vertical(|ui| {
            ui.label("Rendering via offscreen GPU Texture:");

            // Rendered 3D volume slice
            if let Some(tex_id) = self.egui_texture_id {
                let image_widget = egui::Image::from_texture((tex_id, available_size))
                    .sense(egui::Sense::drag());
                println!("yasssssssssssssssssssssss");
                let response = ui.add(image_widget);
                // TODO: Interactions on the view for rotating etc
                if response.dragged() {
                    // self.handle_rotation(some delta);
                }
            }
        });
    }



}