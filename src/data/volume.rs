use image::DynamicImage;
pub struct VolumeData {
    pub slices: Vec<DynamicImage>,
    pub width: usize,
    pub height: usize,
}

// add load volume here
impl VolumeData {
    // Put all gpu logic in the viewer
    pub fn load_volume(&mut self, ctx: &egui::Context, volume:VolumeData) {

        let current_slice = 0;
        let new_slice = volume.slices[current_slice].clone();

        //TODO: Use upload_image here
        let texture = ImageViewer::upload_texture(ctx, new_slice);
        let size = texture.size_vec2();

        self.source = Some(ImageSource::Volume {
            volume,
            texture: ImageData {
                texture,
                size,
            },
            current_slice,
        });

    }

    //TODO: Create update slice delta -1 or 1
    pub fn update_slice(&mut self, ui: &egui::Ui, delta:i32) {



    }


    pub fn next_slice(&mut self, ui: &egui::Ui) {

        if let Some(ImageSource::Volume { volume, texture, current_slice }) = &mut self.source {
            
            let next_slice = (*current_slice + 1) % volume.slices.len();

            *current_slice = next_slice;

            let new_image = volume.slices[next_slice].clone();
            let new_image = ImageViewer::upload_texture(ui.ctx(), new_image);
            let size = new_image.size_vec2();
            
            *texture = ImageData {
                texture: new_image.clone(),
                size,
            };
        }
    }

    pub fn prev_slice(&mut self, ui: &egui::Ui) {
        if let Some(ImageSource::Volume { volume, texture, current_slice }) = &mut self.source {
            if *current_slice != 0 {

                let next_slice = (*current_slice - 1) % volume.slices.len();

                *current_slice = next_slice;

                let new_image = volume.slices[next_slice].clone();
                let new_image = ImageViewer::upload_texture(ui.ctx(), new_image);
                let size = new_image.size_vec2();
                
                *texture = ImageData {
                    texture: new_image.clone(),
                    size,
                };
            }
        }
    }
}