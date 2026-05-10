use crate::data::image_source::ImageSource;
use crate::data::image::ImageData;
use crate::data::image_source::VolumeData;
use crate::data::volume;

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
    pub fn ui(&mut self, ui: &mut egui::Ui, source: Option<&ImageSource>) {
        
        let source = match source {
            Some(src) => src,
            None => return,
        };

        match source {
            ImageSource::Single(single) => {
                // Do we need another reference?
                self.render_image(ui, &single);
            }
            ImageSource::Volume(volume) => {
                self.render_volume(ui, volume);
            }
        }
    
       /* 
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
        */

    }

    fn render_image(&mut self, ui: &mut egui::Ui, image: &ImageData) {
        // i think i use draw code here?
        let image_size = image.size * self.transform.zoom;

        let available = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(available, egui::Sense::drag());

        let image_rect = egui::Rect::from_min_size(
            rect.center() - image_size * 0.5 + self.transform.offset,
            image_size,
        );

        ui.painter().image(
            image.texture.id(),
            image_rect,
            egui::Rect::from_min_max(
                egui::Pos2::new(0.0, 0.0),
                egui::Pos2::new(1.0, 1.0),
            ),
            egui::Color32::WHITE,
        );
    }

    fn render_volume(&mut self, ui: &mut egui::Ui, volume: &VolumeData) { 
    
        // this is the full 3d image
        let data: &Vec<u16> = &volume.cpu.as_ref().unwrap().data;

        // First just render one slice as image using gpu

        // 



    }

}