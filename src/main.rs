use eframe::egui;
use crate::egui::Id;

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
    name: String,
    height: u32,
    image_size: f32,
    zoom_level: i32,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "Arthur".to_owned(),
            height: 180,
            image_size: 30.,
            zoom_level: 100,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        //let id = Id::new("my_side_panel");
        egui::TopBottomPanel::top("my_top_panel").show(ctx, |ui| {
            ui.menu_button("Menu", |ui| {
                ui.label("Button 1");
                ui.label("Button 2");
                ui.label("Button 3");
            });
            ui.menu_button("Options", |ui| {
                ui.label("Options 1");
                ui.label("Options 2");
            });
        }); 

        egui::SidePanel::left("my_side_panel").show(ctx, |ui| {
            ui.heading("Left panel");
            ui.label("Add more widgets here.");
            ui.add(egui::Slider::new(&mut self.height, 140..=220).text("height"));
            ui.add(egui::Slider::new(&mut self.image_size, 50.0..=400.0).text("image size"));
            ui.add(egui::Slider::new(&mut self.zoom_level, 40..=150).text("zoom level"));
        });

        // Always centrapnel as last one.
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My dicom viewer");

            ui.add(egui::Image::new(egui::include_image!("../assets/ferris.png"))
                    .max_width(self.image_size)
                    .corner_radius(10),
            );


        });
    
    }
}
