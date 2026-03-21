use image::DynamicImage;
pub struct VolumeData {
    pub slices: Vec<DynamicImage>,
    pub width: usize,
    pub height: usize,
}
