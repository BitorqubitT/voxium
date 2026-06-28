use std::fs::Metadata;

pub struct MetaData {
    pub patient_id: Option<String>,
    pub patient_name: Option<String>,
    pub patient_weight: Option<String>,
    pub pixel_representation: Option<u16>,
    pub bits_allocated: Option<u16>,
    pub photometric_interpretation: Option<String>,
}

impl Default for MetaData {
    fn default() -> Self {
        MetaData {
            patient_id: None,
            patient_name: None,
            patient_weight: None,
            pixel_representation: None,
            bits_allocated: None,
            photometric_interpretation: None,
        }
    }
}