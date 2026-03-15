
pub struct ViewTransform {
    pub zoom: f32,
    pub offset: egui::Vec2,
}

impl Default for ViewTransform {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            offset: egui::Vec2::ZERO,
        }

    }
}

pub struct ImageViewer {
    source: Option<ImageSource>,
    transform: ViewTransform,
}

//TODO: move this to its own file
impl ImageViewer {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let texture = match &self.source {
            Some(ImageSource::Single { texture }) => texture,
            Some(ImageSource::Volume { texture, .. }) => texture,
            None => return,
        };
        
        let available = ui.available_size();
        let (rect, response) =
            ui.allocate_exact_size(available, egui::Sense::drag());

        if response.dragged() {
            self.transform.offset += response.drag_delta();
        }

        // Zooming
        let scroll = ui.input(|i| i.raw_scroll_delta.y);

        if scroll != 0.0 {
            if let Some(mouse_pos) = ui.input(|i| i.pointer.hover_pos()) {
                let old_zoom = self.transform.zoom;

                let zoom_factor = (1.0 + scroll * 0.001).clamp(0.1, 10.0);
                self.transform.zoom *= zoom_factor;

                let delta_zoom = self.transform.zoom / old_zoom;

        // TODO: Doesnt feel intuitive yet, maybe change offset calc
                self.transform.offset = (self.transform.offset - mouse_pos.to_vec2()) * delta_zoom
                        + mouse_pos.to_vec2();
            }
        }

        let image_size = texture.size * self.transform.zoom;

        let image_rect = egui::Rect::from_min_size(
            rect.center() - image_size * 0.5 + self.transform.offset,
            image_size,
        );

        ui.painter().image(
            texture.texture.id(),
            image_rect,
            egui::Rect::from_min_max(
                egui::Pos2::new(0.0, 0.0),
                egui::Pos2::new(1.0, 1.0),
            ),
            egui::Color32::WHITE,
        );
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

    //TODO: Move to utils
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

    // Put all gpu logic in the viewer
    pub fn load_volume(&mut self, ctx: &egui::Context, volume:VolumeData) {

        // Any other way to do this? 
        // We only clone one image, so not too bad.
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

    fn upload_image(&mut self, ctx: &egui::Context, image: DynamicImage) {

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