use dicom_object::{FileDicomObject, InMemDicomObject, open_file};
use dicom_pixeldata::{PixelDecoder};
use image::DynamicImage;
use dicom::dictionary_std::tags;
use std::{fs, path::PathBuf};
use crate::viewer::image_viewer::ImageViewer;
use crate::dicom::metadata::MetaData;
use crate::viewer::image_viewer::ViewTransform;
use crate::data::volume::VolumeCpu;
use crate::data::image_source::ImageSource;
use std::sync::Arc;
use winit::window::Window;
use anyhow::Result;
// TODO: can do use crate::data::VolumeData;

pub struct MyApp {
    source: Option<ImageSource>,
    gpu: Option<Gpu>,
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
            source: None,
            gpu: None,
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

// https://sotrh.github.io/learn-wgpu/beginner/tutorial2-surface/
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub config: wgpu::SurfaceConfiguration,
    pub window: Arc<Window>,
}

//Use anyhow in case no gpu detection?
impl Gpu {
    async fn new(window: Arc<Window>) -> anyhow::Result<Gpu> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        // select correct gpu
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps.formats[0];

        Ok(Self {
            device,
            queue,
            surface,
            config: wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width: size.width,
                height: size.height,
                desired_maximum_frame_latency: 2,
                view_formats: vec![],
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
            },
            window,
        })

        // ...
    }
}

impl MyApp {

    pub fn new() -> Self {
        Self {
            source: None,
            gpu: None,
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

    //TODO: other options? Could also set non default withh gpu
    //TODO: When do we use this? Do we create on app startup or only when needing it?
    pub fn set_gpu(&mut self, gpu: Gpu) {
        self.gpu = Some(gpu);
    }

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

        let bits_allocated: Option<u16> = obj
        .element(tags::BITS_ALLOCATED)
        .ok()
        .and_then(|e| e.to_int::<u16>().ok());

        let pixel_representation: Option<u16> = obj
        .element(tags::PIXEL_REPRESENTATION)
        .ok()
        .and_then(|e| e.to_int::<u16>().ok());

        let photometric_interpretation: Option<String> = obj
        .element(tags::PHOTOMETRIC_INTERPRETATION)
        .ok()
        .and_then(|e| e.to_str().ok())
        .map(|s| s.to_string());

        self.meta_data = MetaData {
            patient_id,
            patient_name,
            patient_weight,
            pixel_representation,
            bits_allocated,
            photometric_interpretation,
        };

        Ok(())

    }

    fn file_opener(&mut self, ctx: &egui::Context) -> Result<(), Box<dyn std::error::Error>> {
        match self.determine_file_type(){
            "dcm" => {
                let obj = open_file(&self.path)?;
                //dump_file(&obj)?;
                let image = self.convert_dicom_to_image(obj)?;
                // TODO: Should return this imagesource and put it in app 
                self.source = Some(ImageSource::create_single(ctx, image));
                //TODO: Load it to viewer here
            }
            "dir" => {
                self.get_meta_data()?;
                self.source = Some(self.load_directory()?);
                //TODO: Load it to viewer here
            }

            _ => return Err("Unsupported file type".into()),
        }; 
        Ok(())
    }

    fn convert_dicom_to_image(&self, obj: FileDicomObject<InMemDicomObject>) -> Result<DynamicImage, Box<dyn std::error::Error>> {
        let decoded = obj.decode_pixel_data()?;
        // Could remove param, because we are sure its just one image
        let image = decoded.to_dynamic_image(0)?; 
        Ok(image)
    }

    fn convert_dicom_to_vec(
        &self,
        obj: FileDicomObject<InMemDicomObject>
    ) -> Result<Vec<u16>, Box<dyn std::error::Error>> {
        //TODO: check the meta data and pick a good option

        let decoded = obj.decode_pixel_data()?;

        // we use data to skip lut processing
        let bytes = decoded.data(); 

        if bytes.len() % 2 != 0 {
            return Err("Invalid pixel buffer length for u16".into());
        }

        let pixels: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();

        Ok(pixels)
    }

    fn load_directory(&mut self) -> Result<ImageSource, Box<dyn std::error::Error>> {
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
        //todo; add ok
        return Ok(ImageSource::create_volume(volume));
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
                // TODO: eventually add these methods again
                //self.viewer.next_slice(ctx);
            };
            if ui.input(|i|i.key_pressed(egui::Key::P)) {
                //self.viewer.prev_slice(ctx);
            };
            // Here we give the source to viewer and let it decide what to do
            if let Some(gpu) = &self.gpu {
                self.viewer.ui(
                    ui,
                    self.source.as_ref(),
                    &gpu.device,
                    &gpu.queue,
                );
            } else {
                ui.label("(no GPU)");
            }
        });

    }
}
