// https://sotrh.github.io/learn-wgpu/beginner/tutorial2-surface/

use crate::data::volume;

pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
}

//TODO: Seperate file for shader?
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

impl Gpu {
    pub fn new(window: std::sync::Arc<winit::window::Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            flags: wgpu::InstanceFlags::default(),
            backend_options: wgpu::BackendOptions::default(),
            //Use Default::default() to avoid missing InstanceDisplay type issues
            display: Default::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        });

        let surface = instance.create_surface(window.clone())?;

        // block_on safely executes the async calls during the one-time startup sequence
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .map_err(|e| anyhow::anyhow!("No compatible graphics adapter found: {}", e))?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("My Custom Compute/Render Device"),
                required_features: wgpu::Features::empty(), 
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(), 
                trace: wgpu::Trace::Off,
            },
        ))
        .map_err(|e| anyhow::anyhow!("Failed to request device/queue: {}", e))?;

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, 
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        
        surface.configure(&device, &config);

        let (render_pipeline, bind_group_layout, sampler) = Self::pipeline(&device);

        Ok(Self {
            device,
            queue,
            surface,
            config,
            render_pipeline,
            bind_group_layout,
            sampler,
        })
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    // Later split ui render and dicom render
    pub fn render(&mut self) -> Option<wgpu::SurfaceTexture> {
        let output = self.surface.get_current_texture();

        match output {
            wgpu::CurrentSurfaceTexture::Success(frame) => Some(frame),
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface.configure(&self.device, &self.config);
                Some(frame)
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                None
            }
            wgpu::CurrentSurfaceTexture::Timeout 
            | wgpu::CurrentSurfaceTexture::Occluded 
            | wgpu::CurrentSurfaceTexture::Validation => {
                None
            }
        }
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