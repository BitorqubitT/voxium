pub struct VolumeCpu {
    pub data: Vec<u16>,
    pub width: usize,
    pub height: usize,
    pub depth: usize,
}

impl VolumeCpu {
    pub fn to_gpu(self, device: &wgpu::Device, queue: &wgpu::Queue) -> VolumeGpu {
        let size = wgpu::Extent3d {
            width: self.width as u32,
            height: self.height as u32,
            depth_or_array_layers: self.depth as u32,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Volume Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::R16Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&self.data),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some((self.width * 2) as u32), // u16 = 2 bytes
                rows_per_image: Some(self.height as u32),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D3),
            ..Default::default()
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Volume Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        VolumeGpu {
            texture,
            view,
            sampler,
        }
    }

    pub fn normalise(self) {
        //implement later
    }
}

pub struct VolumeGpu {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}