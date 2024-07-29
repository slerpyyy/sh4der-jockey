use anyhow::{format_err, Result};
use serde_yaml::Value;

#[derive(Debug, Default, Clone)]
pub struct Config {
    pub midi_devices: Vec<String>,
    pub audio_device: Option<String>,
}

impl Config {
    pub fn load_or_default() -> Self {
        match Self::load() {
            Ok(config) => config,
            Err(err) => {
                log::warn!("Failed to load config.yaml: {err}");
                Default::default()
            }
        }
    }

    pub fn load() -> Result<Self> {
        let mut file_path = std::env::current_dir()?;
        file_path.push("config.yaml");

        let reader = std::fs::File::open(file_path)?;
        let object: Value = serde_yaml::from_reader(reader)?;

        let mut midi_devices = Vec::new();

        match object.get("midi_devices") {
            Some(Value::Sequence(xs)) => {
                for val in xs {
                    match val.as_str() {
                        Some(s) => midi_devices.push(s.to_owned()),
                        None => {
                            return Err(format_err!(
                                "Expected midi_device name {:?} to be a string",
                                val
                            ));
                        }
                    }
                }
            }
            None => {}
            s => {
                return Err(format_err!(
                    "Expected midi_devices to be a list of strings, got: {:?}",
                    s
                ))
            }
        };

        let audio_device = match object.get("audio_device") {
            Some(Value::String(s)) => Some(s.clone()),
            None => None,
            s => {
                return Err(format_err!(
                    "Expected audio_device name to be a string, got: {:?}",
                    s
                ))
            }
        };

        let mut ndi_sources = Vec::new();
        match object.get("ndi_sources") {
            Some(Value::Sequence(xs)) => {
                for val in xs {
                    match val.as_str() {
                        Some(s) => ndi_sources.push(s.to_owned()),
                        None => {
                            return Err(format_err!(
                                "Expected NDI source name {:?} to be a string",
                                val
                            ))
                        }
                    }
                }
            }
            None => {}
            Some(s) => {
                return Err(format_err!(
                    "Expected ndi_sources to be a list of strings, got: {:?}",
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
