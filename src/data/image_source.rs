use image::DynamicImage;

use crate::data::image::ImageData;
use crate::data::volume::VolumeCpu;

pub enum ImageSource {
    Single {
        texture: ImageData,
    },
    Volume {
        volume: VolumeCpu,
        // TODO: check if I should keep texture.
        // We store slices eventually to volume_data on gpu
        texture: ImageData,
        current_slice: i32,
    },
}

impl ImageSource {

    pub fn create_single(ctx: &egui::Context, image: DynamicImage) -> Self {
        let new_image = image.clone();
        let texture = ImageData::from_image(ctx, new_image);
        ImageSource::Single {
            texture
        }
    }

    pub fn create_volume(ctx: &egui::Context, volume: VolumeCpu) -> Self {
        let new_image = volume.slices[0].clone();
        let texture = ImageData::from_image(ctx, new_image);

        ImageSource::Volume {
            volume,
            texture,
            current_slice: 0,
        }
    }

    // TODO: This will be changed when we have gpu loading. 
    pub fn update_slice(&mut self, ctx: &egui::Context, delta:i32) {
        if let ImageSource::Volume { volume, texture, current_slice } = self {

        // Use this because % is not true modulo
        let next_slice = (*current_slice + delta).rem_euclid(volume.slices.len() as i32);
        //let next_slice = (*current_slice as i32 + delta) % volume.slices.len() as i32;

        *current_slice = next_slice;

        let new_image = volume.slices[next_slice as usize].clone();
        *texture = ImageData::from_image(ctx, new_image);
        
        }
    }

}