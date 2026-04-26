use dicom_object::{FileDicomObject, InMemDicomObject, open_file};
use dicom_pixeldata::{PixelDecoder};
use image::DynamicImage;
use dicom_dump::dump_file;
use dicom::dictionary_std::tags;
use std::{fs, path::PathBuf};
use crate::viewer::image_viewer::ImageViewer;
use crate::dicom::metadata::MetaData;
use crate::viewer::image_viewer::ViewTransform;
use crate::data::volume::VolumeCpu;
use crate::data::volume::VolumeGpu;
use crate::data::image_source::ImageSource;
// TODO: can do use crate::data::VolumeData;

pub struct MyApp {
    source: Option<ImageSource>,
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
            path: "data/1-1.dcm".into(),
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
        let source = match self.determine_file_type(){
            "dcm" => {
                let obj = open_file(&self.path)?;
                //dump_file(&obj)?;
                let image = self.convert_dicom_to_image(obj);
                // TODO: do i still want dynamicimage?
                ImageSource::create_single(ctx, image);

            }
            "dir" => {
                self.load_directory(ctx)?;
                self.get_meta_data()?;
                return Ok(());
            }

            _ => return Err("Unsupported file type".into()),
        }; 
        self.source = Some(source);
        Ok(())
    }

    fn convert_dicom_to_image(&self, obj: FileDicomObject<InMemDicomObject>) -> DynamicImage {
        let decoded = obj.decode_pixel_data()?;
        let data =  
        Ok(data)
    }

    fn load_directory(&mut self, ctx: &egui::Context) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: too much is happening in this function

        // ordering is especially import for 3d image
        let mut id_and_images = Vec::new();
        let mut expected_shape: Option<(usize, usize)> = None;

        //TODO: Check if tags are present and use the present one to order
        for file_name in fs::read_dir(&self.path)?.flatten(){
            let obj = open_file(file_name.path())?;

            //let instance_number = obj.element(tags::INSTANCE_NUMBER)?.to_str()?.to_string();
            let instance_number: i32 = obj
            .element(tags::INSTANCE_NUMBER)?
            .to_int()?; // important fix

            let rows: usize = obj.element(tags::ROWS)?.to_int()?;
            let cols: usize = obj.element(tags::COLUMNS)?.to_int()?;
            let image = self.convert_dicom_to_vec(obj)?;

            if image.len() != rows * cols {
                return Err("Pixel buffer size mismatch".into());
            }

            if let Some((expected_rows, expected_cols)) = expected_shape {
                if rows != expected_rows || cols != expected_cols {
                    return Err(format!(
                        "Mismatch shape: expected {} and {}. got {} and {}",
                        expected_rows, rows, expected_cols, cols
                    ).into());
                }
            } else {
                expected_shape = Some((rows, cols));
            }

            id_and_images.push((instance_number, image));

        }

        // correct numeric sort
        id_and_images.sort_by(|a, b| a.0.cmp(&b.0));

        let depth = id_and_images.len();
        let (height, width)  = expected_shape.ok_or("no image")?;

        let mut volume_data = Vec::new();
        for (_, image) in id_and_images{
            volume_data.extend(image);
        }
    
        // TODO: Should loading be used in this method? or put it in file opener
        let volume = VolumeCpu{

            data: volume_data,
            width: width as usize,
            height: height as usize,
            depth: depth as usize,
        };
        ImageSource::create_volume(ctx, volume);

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
                        self.path = r"D:\dataset\manifest-1771003632643\PSMA-PET-CT-Lesions\PSMA_a96814a79aa26c8f\08-16-2005-NA-PETCT whole-body PSMA-53100\4.000000-CT-10240".into();
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
            ui.label(format!(
                "Patient id: {}",
                self.meta_data.patient_id.as_deref().unwrap_or("N/A")
            ));
            ui.label(format!(
                "Patient height: {}",
                self.meta_data.patient_name.as_deref().unwrap_or("N/A")
            ));
            ui.label(format!(
                "Patient weight: {}",
                self.meta_data.patient_weight.as_deref().unwrap_or("N/A")
            ));
        });

        // Always centrapnel as last one.
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My dicom viewer");

            if ui.input(|i|i.key_pressed(egui::Key::N)) {
                self.viewer.next_slice(ctx);
            };
            if ui.input(|i|i.key_pressed(egui::Key::P)) {
                self.viewer.prev_slice(ctx);
            };

            self.viewer.ui(ui);

        });

    }
}
