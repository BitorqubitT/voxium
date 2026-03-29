use crate::data::image_source::ImageSource;
use crate::data::volume::VolumeData;

use image::DynamicImage;

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
    pub source: Option<ImageSource>,
    pub transform: ViewTransform,
}

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

    // Put all gpu logic in the viewer
    pub fn load_volume(&mut self, ctx: &egui::Context, volume:VolumeData) {

        self.source = Some(ImageSource::create_volume(ctx, volume));

    }

    pub fn load_image(&mut self, ctx: &egui::Context, image:DynamicImage) {

        self.source = Some(ImageSource::create_single(ctx, image));

    }

    pub fn next_slice(&mut self, ctx: &egui::Context) {
        //TODO: c
        if let Some(ref mut image_source) = self.source {
            image_source.update_slice(ctx, 1);
        }
    }

    pub fn prev_slice(&mut self, ctx: &egui::Context) {
        //TODO: call update_slice
        if let Some(ref mut image_source) = self.source {
            image_source.update_slice(ctx, -1);
        }
    }


}