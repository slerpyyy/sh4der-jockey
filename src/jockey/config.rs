use std::fs::File;
use serde_yaml::Value;

pub struct Config {
    pub midi_devices: Vec<String>,
    pub audio_device: Option<String>,
    pub ndi_sources: Vec<String>,
    pub sound_file: Option<String>,
}

impl Config {
    pub fn new() -> Self {
        match Self::load() {
            Ok(x) => x,
            Err(e) => {
                println!("Failed to load config.yaml: {}", e);
                Self {
                    midi_devices: vec![],
                    audio_device: None,
                    ndi_sources: vec![],
                    sound_file: None,
                }
            }
        }
    }

    pub fn load() -> Result<Self, String> {
        let mut path = std::env::current_dir().map_err(|e| e.to_string())?;
        path.push("config.yaml");

        let reader = File::open(path).map_err(|e| e.to_string())?;
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

        let mut ndi_sources = vec![];
        match object.get("ndi_sources") {
            Some(Value::Sequence(xs)) => {
                for val in xs {
                    match val.as_str() {
                        Some(s) => ndi_sources.push(s.to_owned()),
                        None => {
                            return Err(format!(
                                "Expected NDI source name {:?} to be a string",
                                val
                            ))
                        }
                    }
                }
            }
            None => {}
            Some(s) => {
                return Err(format!(
                    "Expected ndi_sources to be a list of strings, got: {:?}",
                    s
                ))
            }
        };

        let sound_file = match object.get("sound_file") {
            Some(Value::String(s)) => Some(s.clone()),
            None => None,
            s => {
                return Err(format!(
                    "Expected sound_file to be a string, got: {:?}",
                    s
                ))
            }
        };

        Ok(Self {
            midi_devices,
            audio_device,
            ndi_sources,
            sound_file,
        })
    }
}
