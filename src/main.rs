pub mod app;
pub use app::MyApp;
pub mod data;
pub mod dicom; // do i need these here?
pub mod viewer;
use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    eframe::run_native(
        "app",
        options,
        Box::new(|cc| {
            let device = cc.wgpu_render_state.as_ref().unwrap().device.clone();
            let queue = cc.wgpu_render_state.as_ref().unwrap().queue.clone();

            Ok(Box::new(MyApp {
                gpu: GpuContext { device, queue },
                ..Default::default()
            }))
        }),
    );
}
