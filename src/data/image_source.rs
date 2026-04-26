use image::DynamicImage;

use crate::data::image::ImageData;
use crate::data::volume::VolumeCpu;
use crate::data::volume::VolumeGpu;

pub enum ImageSource {
    Single(SingleImage),
    Volume(VolumeData),
}

pub struct SingleImage {
    pub texture: ImageData,
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
        let texture = ImageData::from_image(ctx, new_image);
        ImageSource::Single(SingleImage {
            texture,
        })
    }

    pub fn create_volume()
}


