use serde_yaml::Value;
use std::path::{Path, PathBuf};

pub struct GlobalConfig {
    pub midi_devices: Vec<String>,
    pub audio_device: Option<String>,
}

impl GlobalConfig {
    pub fn new() -> Self {
        match Self::load_config() {
            Ok(x) => x,
            Err(e) => {
                println!("Failed to load config.yaml: {}", e);
                let midi_devices = vec![];
                let audio_device = None;
                Self {
                    midi_devices,
                    audio_device,
                }
            }
        }
    }

    pub fn load_config() -> Result<Self, String> {
        let mut path = std::env::current_dir().map_err(|e| e.to_string())?;
        println!("{:?}", path);
        path.push("config.yaml");
        println!("{:?}", path);
        let reader = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let object: Value = serde_yaml::from_reader(reader).map_err(|e| e.to_string())?;

        let mut midi_devices = vec![];

        match object.get("midi_devices") {
            Some(Value::Sequence(xs)) => {
                for val in xs {
                    match val.as_str() {
                        Some(s) => midi_devices.push(s.to_owned()),
                        None => {
                            return Err(format!(
                                "Expected midi_device name {:?} to be a string",
                                val
                            ))
                        }
                    }
                }
            }
            None => {}
            s => {
                return Err(format!(
                    "Expected midi_devices to be a list of strings, got: {:?}",
                    s
                ))
            }
        };

        let audio_device = match object.get("audio_device") {
            Some(Value::String(s)) => Some(s.clone()),
            None => None,
            s => {
                return Err(format!(
                    "Expected audio_device name to be a string, got: {:?}",
                    s
                ))
            }
        };

        Ok(Self {
            midi_devices,
            audio_device,
        })
    }
}
