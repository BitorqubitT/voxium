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

}