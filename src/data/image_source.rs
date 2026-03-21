use egui::Image;
use image::DynamicImage;

use crate::data::image::ImageData;
use crate::data::volume::VolumeData;
use crate::viewer::image_viewer::ImageViewer;

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

impl ImageSource {

    pub fn create_single(ctx: &egui::Context, image: DynamicImage) -> Self {
        let new_image = image.clone();
        let texture = ImageData::from_image(ctx, new_image);
        ImageSource::Single {
            texture
        }
    }

    pub fn create_volume(ctx: &egui::Context, volume: VolumeData) -> Self {
        let new_image = volume.slices[0].clone();
        let texture = ImageData::from_image(ctx, new_image);

        ImageSource::Volume {
            volume,
            texture,
            current_slice: 0,
        }
    }

    //TODO: Create update slice delta -1 or 1
    pub fn update_slice(&mut self, ui: &egui::Ui, delta:i32) {
        // Check should be done when calling this method?
        // TODO: Check where to put this. move to image_source
        // We use imagesource volume and not volumedata
        // volumedata is in imagesource
        if let ImageSource::Volume { volume, texture, current_slice } = self {

        let next_slice = (*current_slice + delta) % self.volume.slices.len();

        *current_slice = next_slice;

        let new_image = self.volume.slices[next_slice].clone();
        let new_image = ImageViewer::upload_texture(ui.ctx(), new_image);
        let size = new_image.size_vec2();
        
        *texture = ImageData {
            texture: new_image.clone(),
            size,
        };
        }
    }

}