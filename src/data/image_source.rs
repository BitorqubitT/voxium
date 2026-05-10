use image::DynamicImage;

use crate::data::image::ImageData;
use crate::data::volume::VolumeCpu;
use crate::data::volume::VolumeGpu;

pub enum ImageSource {
    Single(ImageData),
    Volume(VolumeData),
}

pub struct VolumeData {
    pub cpu: Option<VolumeCpu>,
    pub gpu: Option<VolumeGpu>,
    pub current_slice: i32,
}

impl ImageSource {

    pub fn create_single(ctx: &egui::Context, image: DynamicImage) -> Self {
        // Dont think i need clone here
        let new_image = image.clone();
        let texture = ImageData::upload_texture(ctx, new_image);
        let size = texture.size_vec2();
        ImageSource::Single(ImageData{
            texture: texture,
            size: size,
        })
    }

    pub fn create_volume(volume: VolumeCpu) -> Self {
        // TODO add gpu later
        ImageSource::Volume(VolumeData {
            cpu: Some(volume),
            gpu: None,
            current_slice: 0,
        })
    }

}


