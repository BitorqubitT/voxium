use eframe::egui;
use dicom_object::{DefaultDicomObject, FileDicomObject, InMemDicomObject, open_file, ReadError, AccessError};
use dicom_pixeldata::{PixelDecoder};
use image::DynamicImage;
use dicom_dump::dump_file;
use dicom::dictionary_std::tags;

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

struct MyApp {
    height: u32,
    image_size: f32,
    zoom_level: i32,
    image_path: String,
    dicom_image: Option<image::DynamicImage>,
    texture: Option<egui::TextureHandle>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            height: 180,
            image_size: 30.,
            zoom_level: 100,
            image_path: "data/1-001.dcm".to_owned(),
            dicom_image: None,
            texture: None,
        }
    }
}

impl MyApp {

    fn determine_file_type(&mut self) -> String {

        let parts: Vec<&str> = self.image_path.split(".").collect();
        let filetype = parts.last().unwrap_or(&"");
        println!("this is the type {:?}", filetype);
        filetype.to_string()
    }

    fn file_opener(&mut self, ctx: &egui::Context) -> String {
        let file_type_name = self.determine_file_type();
        match file_type_name.as_str() {
            "dcm"=> {"Opened dicom file".to_string();
            
            match self.load_dicom(ctx) {
                Ok(_) => "Opened dicom file".to_string(),
                Err(e) => format!("failed to open dicomm file: {}", e)
            }
            }
            "tiff"=>{println!("cmontiff");
            "Opened tiff file".to_string()
            }
            _ => {println!("Unsupported file type");
            "we dont support this type yet".to_string()
            }
        }
    }

    fn upload_dicom(&mut self, ctx: &egui::Context, image: DynamicImage) -> egui::TextureHandle{

        let size = [image.width() as usize, image.height() as usize];
        let rgba_buffer = image.to_rgba8();
        let pixels = rgba_buffer.as_raw();

        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            size,
            pixels,
        );

        // Only call this once
        ctx.load_texture("dicom_layer", color_image, Default::default())

    }

    fn load_dicom(&mut self, ctx: &egui::Context) -> Result<(), Box<dyn std::error::Error>> {

        //TODO: Dirty soltion, maybe create enum with r ead and access error.
        let obj = open_file(&self.image_path)?;

        let sop_class = obj.element_by_name("SOPClassUID")?.to_str()?;

        //let obj = self.load_dicom().expect("Failed to load Dicom");
        //dump_file(&obj);
        let image_as_array = self.convert_dicom_to_image(obj).expect("couldnt decode");

        // Load to gpu 

        // check if this went well 
        self.texture = Some(self.upload_dicom(ctx, image_as_array));

        Ok(())

    }

    fn convert_dicom_to_image(&self, obj: FileDicomObject<InMemDicomObject>) -> Result<DynamicImage, Box<dyn std::error::Error>> {
        //TODO: Should eventually check from meta data? what kind of image data it contains
        //TODO: Make a seperate one for non 2d data?

        // Check the type of decode ????? maybe get this from dicom meta data


        let decoded = obj.decode_pixel_data()?;
        // 0 is need because we give a stack of data? Multiple images? maaybe
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
                        self.file_opener(ctx);
                        //Check what file it is first
                        // let file = self.file_loader().expect("cant handle this format");
                        


                    }
                    if ui.button("Button 2").clicked() {
                        println!("Button 2 clicked");
                        self.image_size = 900.0;
                    }

                });
                ui.menu_button("Options", |ui| {
                    if ui.button("change ferris").clicked() {
                        println!("Options 1 clicked");
                        self.image_path = "file://assets/ferris2.png".to_owned();
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

          
            if let Some(texture) = &self.texture {
                ui.image(texture); 
                //ui.image(self.texture, self.texture.size_vec2());
            } else {
                ui.label("Waiting for DICOM upload...");
            }
            
        });
    }
}
