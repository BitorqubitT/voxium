use wgpu::TexelCopyTextureInfo;
use wgpu::TexelCopyBufferLayout;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

pub struct VolumeCpu {
    pub data: Vec<u16>,
    pub width: usize,
    pub height: usize,
    pub depth: usize,
}

impl VolumeCpu {
    pub fn to_gpu(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> VolumeGpu {
      // only move it to gpu memory
        let texture_descriptor = wgpu::TextureDescriptor {
            label: Some("3d dicom texture"),
            size: wgpu::Extent3d {
                width: self.width as u32,
                height: self.height as u32,
                depth_or_array_layers: self.depth as u32
            },
            //TODO: check advantage of extra mips (performace?)
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::R16Uint,
            //TODO: https://gpuweb.github.io/gpuweb/#typedefdef-gputextureusageflags check
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[]
            };

        let texture = device.create_texture(&texture_descriptor);
        
        queue.write_texture(
            &texture,
            bytemuck::cast_slice(&self.data),
            wgpu::TexelCopyBufferLayout{
                offset: 0,
                bytes_per_row: (self.width * std::mem::size_of::<u16>()) as u32,
                rows_per_image: self.height as u32
            },
            wgpu::Extent3d {
                width: self.width as u32,
                height: self.height as u32,
                depth_or_array_layers: self.depth as u32
            }

        );
        //TODO: Write code to actually transfer the data to gpu memory
            //use queue
        
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("3d dicom sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
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