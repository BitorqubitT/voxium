use image::DynamicImage;
use crate::viewer::image_viewer::ImageViewer;
//TODO: check better way to import? since its in same dir
use crate::data::image_source::ImageSource;

pub struct ImageData {
    pub texture: egui::TextureHandle,
    pub size: egui::Vec2,
}

impl ImageData {
    //TODO:Change names
    pub fn upload_texture(ctx: &egui::Context, image: DynamicImage) -> egui::TextureHandle{

        let size = [image.width() as usize, image.height() as usize];
        let rgba_buffer = image.to_rgba8();
        let pixels = rgba_buffer.as_raw();

        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            size,
            pixels,
        );

        ctx.load_texture("dicom_layer", color_image, Default::default())

    }

    pub fn upload_image(&mut self, ctx: &egui::Context, image: DynamicImage) {

        let texture = ImageViewer::upload_texture(ctx, image);
        let size = texture.size_vec2();

        self.source = Some(ImageSource::Single {
        texture: ImageData {
            texture,
            size,
            },
        });
    }    
}