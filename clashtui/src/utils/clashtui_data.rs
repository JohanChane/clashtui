use serde::{Deserialize, Serialize};
use std::fs::File;
use std::result::Result;
use std::error::Error;

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ClashTuiData {
    pub current_profile: String,
}

impl ClashTuiData {
    pub fn from_file(file_path: &str) -> Result<Self, Box<dyn Error>> {
        let f = File::open(file_path)?;
        Ok(serde_yaml::from_reader(f)?)
    }

    pub fn to_file(&self, file_path: &str) -> Result<(), Box<dyn Error>> {
        let f = File::create(file_path)?;
        Ok(serde_yaml::to_writer(f, self)?)
    }

    pub fn update_profile(&mut self, new_profile: &str) {
        self.current_profile = new_profile.to_string();
    }
}
