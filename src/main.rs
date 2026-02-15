use eframe::egui;
use dicom_object::{DefaultDicomObject, open_file};
use dicom_pixeldata::{PixelDecoder};
use image::DynamicImage;

fn load_dicom() -> Result<DefaultDicomObject, dicom_object::ReadError> {
    open_file("data/RG3_J2KI.dcm")
}

fn convert_dicom_to_image(obj: DefaultDicomObject) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    //TODO: Should eventually check from meta data? what kind of image data it contains
    //TODO: Make a seperate one for non 2d data?
    let decoded = obj.decode_pixel_data()?;
    // 0 is need because we give a stack of data? Multiple images? maaybe
    let image = decoded.to_dynamic_image(0)?;
    Ok(image)
}

fn upload_dicom(ctx: &egui::Context, image: DynamicImage) -> egui::TextureHandle{

    let size = [image.width() as usize, image.height() as usize];
    
    // 1. Convert to RGBA8 buffer
    let rgba_buffer = image.to_rgba8();
    
    // 2. Access the underlying Vec<u8> and turn it into a slice
    let pixels = rgba_buffer.as_raw(); // This returns &Vec<u8>

    let color_image = egui::ColorImage::from_rgba_unmultiplied(
        size,
        pixels, // egui accepts &[u8] here
    );

    ctx.load_texture("dicom_layer", color_image, Default::default())

}


fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Voxium",
        options,
        Box::new(|cc| {
            // This gives us image support:
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
            image_path: "file://assets/ferris.png".to_owned(),
            dicom_image: None,
            texture: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        //let id = Id::new("my_side_panel");
        egui::TopBottomPanel::top("my_top_panel").show(ctx, |ui| {

            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Button 1").clicked() {
                        println!("loading file");
                        let obj = load_dicom().expect("Failed to load Dicom");
                        //dump_file(&obj);
                        let image_as_array = convert_dicom_to_image(obj).expect("couldnt decode");
                        let texture = Some(upload_dicom(ctx, image_as_array));

                        // TODO: implement loading file here
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

            // This runs at compile time, so it won't work with dynamic paths.
            //ui.add(egui::Image::new(egui::include_image!(image_location))
            //        .max_width(self.image_size)
            //        .corner_radius(10),
            //);

            //ui.add(egui::Image::from_uri(&self.image_path) // Use from_uri for dynamic paths
            //   .max_width(self.image_size)
            //    .corner_radius(10)
            //);
            if let Some(texture) = &self.texture {
                // We pass the handle itself. egui will automatically 
                // convert &TextureHandle into an ImageSource.
                ui.image(texture); 
                //ui.image(self.texture, self.texture.size_vec2());
            } else {
                ui.label("Waiting for DICOM upload...");
            }
            

        });
    }
}
