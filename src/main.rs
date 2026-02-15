use eframe::egui;
use dicom_object::{DefaultDicomObject, FileDicomObject, InMemDicomObject, open_file};

fn load_dicom() -> Result<DefaultDicomObject, dicom_object::ReadError> {
    open_file("data/RG3_J2KI.dcm")
}

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    
    let obj = load_dicom().expect("Failed to load Dicom");
    //dump_file(&obj);

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
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            height: 180,
            image_size: 30.,
            zoom_level: 100,
            image_path: "file://assets/ferris.png".to_owned(),
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
                        println!("Button 1 clicked");
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

            ui.add(egui::Image::from_uri(&self.image_path) // Use from_uri for dynamic paths
                .max_width(self.image_size)
                .corner_radius(10)
            );
        });
    }
}
