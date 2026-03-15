use crate::data::image::ImageData;
use crate::data::volume::VolumeData;

pub enum ImageSource {
    Single {
        texture: ImageData,
    },
    Volume {
        volume: VolumeData,
        texture: ImageData,
        current_slice: usize,
    },
}
