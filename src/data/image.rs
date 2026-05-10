use image::DynamicImage;

pub struct ImageData {
    pub texture: egui::TextureHandle,
    pub size: egui::Vec2,
}

impl ImageData {
    // TODO: is from image still useful?
  pub fn from_image(ctx: &egui::Context, image: DynamicImage) -> Self {
        let texture = Self::upload_texture(ctx, image);
        let size = texture.size_vec2();

        Self { texture, size }
    }

    pub fn upload_texture(ctx: &egui::Context, image: DynamicImage) -> egui::TextureHandle {

        let size = [image.width() as usize, image.height() as usize];
        let rgba_buffer = image.to_rgba8();
        let pixels = rgba_buffer.as_raw();

        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            size,
            pixels,
        );

        ctx.load_texture(
            "dicom_layer",
            color_image,
            egui::TextureOptions::default(),
        )
    }

}