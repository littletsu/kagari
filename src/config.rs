use std::fs;
use std::io::ErrorKind;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub detection: DetectionConfig
}
#[derive(Serialize, Deserialize)]
pub struct DetectionConfig {
    pub energy: f32,
    pub sample_chunks_ms: u32
}

impl Config {
    pub fn write_to_file(path: &str, config: Config) -> Config {
        match fs::write(path, toml::to_string(&config).unwrap()) {
            Ok(()) => return config,
            Err(error) => {
                panic!("Error writing config file: {:?}", error);
            }
        };
    }

    pub fn from_file(path: &str, default_config: Config) -> Config {
        let contents = fs::read_to_string(path);
        let contents = match contents {
            Ok(contents) => contents,
            Err(error) => match error.kind() {
                ErrorKind::NotFound => return Config::write_to_file(path, default_config),
                other_error => {
                    panic!("Problem opening the file: {:?}", other_error)
                }
            }
        };
        let config: Config = toml::from_str(&contents).unwrap();
        config
    }
}