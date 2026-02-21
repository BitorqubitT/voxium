use eframe::egui;
use dicom_object::{DefaultDicomObject, FileDicomObject, InMemDicomObject, open_file, ReadError, AccessError};
use dicom_pixeldata::{PixelDecoder};
use image::DynamicImage;
use dicom_dump::dump_file;
use dicom::dictionary_std::tags;
use std::path::PathBuf;

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

pub struct ImageViewer {
    pub image: Option<ImageData>,
    pub transform: ViewTransform,
}

impl ImageViewer {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let Some(image) = &self.image else { return };

        let available = ui.available_size();
        let (rect, response) =
            ui.allocate_exact_size(available, egui::Sense::drag());

        // ---- DRAW ----
        // TODO: Create
        let scroll = ui.input(|i| i.raw_scroll_delta.y);

        if scroll != 0.0 {

            self.transform.zoom += scroll;



        }



        let image_size = image.size * self.transform.zoom;

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
}


struct MyApp {
    height: u32,
    image_size: f32,
    zoom_level: i32,
    image_path: PathBuf,
    viewer: ImageViewer,
}

enum LoadedImage {
    Dicom2d(DynamicImage),
    DicomVolume(DynamicImage),
    Tiff(DynamicImage),
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            height: 180,
            image_size: 30.,
            zoom_level: 100,
            image_path: "data/1-001.dcm".into(),
            viewer: ImageViewer {
                 image: None, 
                 transform: ViewTransform::default(), 
            },
        }
    }
}

impl MyApp {

    fn determine_file_type(&self) -> &str {
        if self.image_path.is_dir() {
            "dir"
        } else {
            self.image_path.extension().and_then(|ext| ext.to_str()).unwrap_or("")
        }
    }

    fn file_opener(&mut self) -> Result<LoadedImage, Box<dyn std::error::Error>> {
        let file_type_name = self.determine_file_type();
        match file_type_name {
            "dcm" => {
            let obj = open_file(&self.image_path)?;
            let image = self.convert_dicom_to_image(obj)?;
            Ok(LoadedImage::Dicom2d(image))

            }
           //TODO: add directory 

            "tiff" => {
                //TODO: Add support
                let image = image::open(&self.image_path)?;
                Ok(LoadedImage::Tiff(image))
            }

            //"dir" => {
             //   println!("should check multiple files");
            //}
            _ => Err("Unsupported file typ".into()),
            }
        }

    fn upload_image(&mut self, ctx: &egui::Context, image: LoadedImage) {
        let dynamic_image = match image {
            LoadedImage::Dicom2d(image) => image,
            LoadedImage::DicomVolume(image) => image,  
            LoadedImage::Tiff(image) => image,  
        };

        let texture = self.upload_texture(ctx, dynamic_image);
        let size = texture.size_vec2();

        self.viewer.image = Some(ImageData {
            texture,
            size,
        });

    }    

    fn upload_texture(&self, ctx: &egui::Context, image: DynamicImage) -> egui::TextureHandle{

        let size = [image.width() as usize, image.height() as usize];
        let rgba_buffer = image.to_rgba8();
        let pixels = rgba_buffer.as_raw();

        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            size,
            pixels,
        );

        ctx.load_texture("dicom_layer", color_image, Default::default())

    }

    fn convert_dicom_to_image(&self, obj: FileDicomObject<InMemDicomObject>) -> Result<DynamicImage, Box<dyn std::error::Error>> {
        let decoded = obj.decode_pixel_data()?;
        let image = decoded.to_dynamic_image(0)?;
        Ok(image)
    }

}

impl eframe::App for MyApp {

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        egui::TopBottomPanel::top("my_top_panel").show(ctx, |ui| {

            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Button 1").clicked() {
                        println!("loading file");
                        // TODO: Check name, maybe change it
                        match self.file_opener() {
                            Ok(image) => self.upload_image(ctx, image),
                            Err(e) => print!("Error: {}", e)
                        }
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
        });

        // Always centrapnel as last one.
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My dicom viewer");

            self.viewer.ui(ui);

            
        });
    }
}
