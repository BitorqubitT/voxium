use egui::epaint::textures;

use crate::data::image_source::ImageSource;
use crate::data::image::ImageData;
use crate::data::image_source::VolumeData;
use crate::data::volume::VolumeGpu;

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

// Want to render to a output canvas in imageviewer. (the we give id and paint in egui i think, check this)
pub struct ImageViewer {
    // TODO: Remove this, because it will keep giving ownership problems. App owns source.
    //pub source: Option<ImageSource>,
    pub transform: ViewTransform,

    pub render_target: Option<wgpu::Texture>,
    pub egui_texture_id: Option<egui::TextureId>,
    pub current_view_size: egui::Vec2,
}

impl ImageViewer {
    pub fn ui(&mut self,
        //maybe remove option, we only call this when there is a source? 
              ui: &mut egui::Ui, 
              source: Option<&ImageSource>,
              egui_renderer: &mut egui_wgpu::Renderer,
              device: &wgpu::Device, 
              ) {
        
        let source = match source {
            Some(src) => src,
            None => return,
        };

        match source {
            ImageSource::Single(single) => {
                //TODO: check if we want to keep calling this. Or is there a better way
                self.render_image(ui, &single);
            }
            // TODO: What do i want here? Just check if volume.gpu is present and then run render_volume using self.
            ImageSource::Volume(volume) => {
                if let Some(ref volume_gpu) = volume.gpu {
                    self.render_volume(ui, 
                                    volume_gpu, 
                                    egui_renderer, 
                                    device
                                );
                } else {
                    // TODO: add some code to fill center
                    println!("volume.gpu is empty :O");
                }
            }
        }

        let available = ui.available_size();
        let (rect, response) = ui.allocate_exact_size(
            available, 
            egui::Sense::drag(),
        );

    }

    fn render_image(
        &mut self, 
        ui: &mut egui::Ui, 
        image: &ImageData) {
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

    fn render_volume(
        &mut self, 
        ui: &mut egui::Ui, 
        volume: &VolumeGpu,
        egui_renderer: &mut egui_wgpu::Renderer,
        device: &wgpu::Device,
    ) { 
    
        ui.label("volume ready on gpu");
        let available_size = ui.available_size();
        self.current_view_size = available_size;

        // Register the canvas texture with egui
        if self.egui_texture_id.is_none() {
            // Give access to app.egui_renderei
            let tex_id = egui_renderer.register_native_texture(
                device, 
                &volume.view, 
                wgpu::FilterMode::Linear, // TODO: read this: https://gpuweb.github.io/gpuweb/#enumdef-gpumipmapfiltermode
            );
            self.egui_texture_id = Some(tex_id);
        }

        ui.vertical(|ui| {
            ui.label("Rendering via offscreen GPU Texture:");

            // Rendered 3D volume slice
            if let Some(tex_id) = self.egui_texture_id {
                let image_widget = egui::Image::from_texture((tex_id, available_size))
                    .sense(egui::Sense::drag());
                
                let response = ui.add(image_widget);
                // TODO: Interactions on the view for rotating etc
                if response.dragged() {
                    // self.handle_rotation(some delta);
                }
            }
        });
    }

}