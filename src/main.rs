use eframe::egui;
use dicom_object::{FileDicomObject, InMemDicomObject, open_file, ReadError, AccessError};
use dicom_pixeldata::{PixelDecoder};
use image::DynamicImage;
use dicom_dump::dump_file;
use dicom::dictionary_std::tags;
use std::{fs, path::PathBuf};

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Voxium",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<MyApp>::default())
        }),
    )
}

pub struct ImageData {
    pub texture: egui::TextureHandle,
    pub size: egui::Vec2,
}

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

pub struct VolumeData {
    pub slices: Vec<DynamicImage>,
    pub width: usize,
    pub height: usize,
}

enum ImageSource {
    Single {
        texture: ImageData,
    },
    Volume {
        volume: VolumeData,
        texture: ImageData,
        current_slice: usize,
    },
}

pub struct MetaData {
    pub patient_id: Option<String>,
    pub patient_name: Option<String>,
    pub patient_weight: Option<String>,
}

struct ImageViewer {
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

struct MyApp {
    height: u32,
    image_size: f32,
    zoom_level: i32,
    path: PathBuf,
    viewer: ImageViewer,
    meta_data: MetaData,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            height: 180,
            image_size: 30.,
            zoom_level: 100,
            path: "data/MRBRAIN.dcm".into(),
            viewer: ImageViewer {
                source: None,
                transform:  ViewTransform::default(),
            },
            meta_data: MetaData {
                patient_id: None,
                patient_name: None,
                patient_weight: None,
            }
        }
    }
}

impl MyApp {

    fn determine_file_type(&self) -> &str {
        if self.path.is_dir() {
            "dir"
        } else {
            self.path.extension().and_then(|ext| ext.to_str()).unwrap_or("")
        }
    }

    fn get_meta_data(&mut self) -> Result<(), Box<dyn std::error::Error>>{
        // TODO: optimise later, keep repeating the check type part
        let obj = match self.determine_file_type(){
            "dcm" => open_file(&self.path)?,
            "dir" => {
                let entry = fs::read_dir(&self.path)?
                    .flatten()
                    .next()
                    .ok_or("No file in directory")?;

                open_file(entry.path())?
            }
            _ => return Err("Unsupported file type".into()),
        };

        let patient_id = obj
        .element(tags::IMAGE_POSITION_PATIENT)
        .ok()
        .and_then(|e| e.to_str().ok())
        .map(|s| s.to_string());
        
        let patient_name = obj
        .element(tags::PATIENT_NAME)
        .ok()
        .and_then(|e| e.to_str().ok())
        .map(|s| s.to_string());
        
        let patient_weight = obj
        .element(tags::PATIENT_WEIGHT)
        .ok()
        .and_then(|e| e.to_str().ok())
        .map(|s| s.to_string());
        
        self.meta_data = MetaData {
            patient_id,
            patient_name,
            patient_weight,
        };

        Ok(())

    }

    fn file_opener(&mut self, ctx: &egui::Context) -> Result<(), Box<dyn std::error::Error>> {
        match self.determine_file_type(){
            "dcm" => {
            let obj = open_file(&self.path)?;
            dump_file(&obj)?;
            let image = self.convert_dicom_to_image(obj)?;
            self.viewer.upload_image(ctx, image);
            
            }
            "tiff" => {
                //TODO: Add support
                let image = image::open(&self.path)?;
                self.viewer.upload_image(ctx, image);
                // better to propegate the error this way
                //let _ = self.get_meta_data();
            }
            "dir" => {
                //TODO: Maybe split this partly
                self.load_directory(ctx)?;
                self.get_meta_data()?;
            }
            _ => { 
                return Err("Unsupported file typ".into());
            }
        }
        Ok(())
    }

    fn convert_dicom_to_image(&self, obj: FileDicomObject<InMemDicomObject>) -> Result<DynamicImage, Box<dyn std::error::Error>> {
        let decoded = obj.decode_pixel_data()?;
        let image = decoded.to_dynamic_image(0)?;
        Ok(image)
    }

    fn load_directory(&mut self, ctx: &egui::Context) -> Result<(), Box<dyn std::error::Error>> {
        // load slice and sort slices
        // read the meta data to sort them then put them in vec
        
        let mut id_and_images = Vec::new();

        //TODO: Check if tags are present and use the present one to order
        //TODO: SHould have a check to see if its dicom
        for file_name in fs::read_dir(&self.path)?.flatten(){
            let obj = open_file(file_name.path())?;

            //let image_position = obj.element(tags::IMAGE_POSITION_PATIENT)?.to_str()?.to_string();
            let instance_number = obj.element(tags::INSTANCE_NUMBER)?.to_str()?.to_string();

            let image = self.convert_dicom_to_image(obj)?;

            id_and_images.push((instance_number, image))

        }

        id_and_images.sort_by(|a, b| a.0.cmp(&b.0));

        let images_vector: Vec<DynamicImage> = id_and_images.into_iter().map(|(_, b)| b).collect();

        let width = images_vector[0].width();
        let height = images_vector[0].height();

        let volume = VolumeData{
            slices: images_vector,
            width: width as usize,
            height: height as usize,
        };
        
        self.viewer.load_volume(ctx, volume);

        Ok(())

    }


}

impl eframe::App for MyApp {

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        egui::TopBottomPanel::top("my_top_panel").show(ctx, |ui| {

            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open a file").clicked() {
                        if let Err(e) = self.file_opener(ctx) {
                            eprintln!("Error loading file: {}", e);
                        }
                        ui.close();
                    }
                });
                ui.menu_button("File", |ui| {
                    if ui.button("Button 2 open directory").clicked() {
                        self.path = r"D:\dataset\manifest-1771003632643\PSMA-PET-CT-Lesions\PSMA_0ef9e2afd72f7483\08-27-2002-NA-PETCT whole-body PSMA-67604\2.000000-CT-96689".into();
                        if let Err(e) = self.file_opener(ctx) {
                            eprintln!("Error loading file: {}", e);
                        }
                        ui.close(); 
                    }
                });
                ui.menu_button("Options", |ui| {
                    if ui.button("change ferris").clicked() {
                        println!("Options 1 clicked");
                    }
                    ui.menu_button("More options", |ui|{
                        if ui.button("More Options 1").clicked() {
                            println!("Options 1 clicked");
                            ui.close();
                    }
                    ui.label("Options 2");
                    });
                });
            });
        }); 

        egui::SidePanel::left("my_side_panel").show(ctx, |ui| {
            ui.heading("Left panel");
            ui.label("Add more widgets here.");
            ui.add(egui::Slider::new(&mut self.height, 140..=220).text("height"));
            ui.add(egui::Slider::new(&mut self.image_size, 50.0..=900.0).text("image size"));
            ui.add(egui::Slider::new(&mut self.zoom_level, 40..=150).text("zoom level"));
            ui.label("Display information here.");
            ui.label("Display information here.");
            ui.label("Display information here.");
        });

        // Always centrapnel as last one.
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My dicom viewer");

            if ui.input(|i|i.key_pressed(egui::Key::N)) {
                self.viewer.next_slice(ui);
            };
            if ui.input(|i|i.key_pressed(egui::Key::P)) {
                self.viewer.prev_slice(ui);
            };

            self.viewer.ui(ui);

        });

    }
}
