

pub struct VolumeCpu {

    pub data: Vec<u16>,
    pub width: usize,
    pub height: usize,
    pub depth: usize,

}




impl VolumeCpu {

    pub fn to_gpu(self) {





    }

    // Check if values are suitable for GPU
    pub fn normalise(self) {


    }


}


pub struct VolumeGpu {

    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,

}
