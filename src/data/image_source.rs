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

    pub fn create_single(texture) -> Self {
        ImageSource::Single {
            texture
        }
    }

    pub fn create_volume(ui, volume) -> Self {
        let new_image = volume.slices[0].clone();
        let texture = ImageData::upload_texture(ui.ctx(), new_image);
        let size= texture.size_vec2();

        ImageSource::Volume {
            volume,
            texture: ImageData {texture, size},
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