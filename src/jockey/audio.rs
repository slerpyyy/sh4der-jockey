use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Device;
use std::sync::{Arc, Mutex};

pub struct Audio {
    samples: Arc<Mutex<Vec<f32>>>,
}

impl Audio {
    pub fn new() -> Self {
        let samples = Arc::new(Mutex::new(Vec::new()));
        let mut this = Self { samples };
        this.connect();
        this
    }

    pub fn connect(&mut self) {
        let host = cpal::default_host();
        let devices = host.input_devices();

        let device = if let Ok(devices) = devices {
            let devices: Vec<Device> = devices.collect();
            let mut chosen_device = host.default_input_device().unwrap();
            for device in devices {
                let name = device.name().unwrap();
                if name.matches("VoiceMeeter").count() != 0 {
                    chosen_device = device;
                }
            }
            chosen_device
        } else {
            host.default_input_device().unwrap()
        };

        println!(
            "Connected to audio input device: {:?}",
            device.name().unwrap()
        );

        let mut supported_configs_range = device
            .supported_input_configs()
            .expect("error while querying configs");
        let supported_config = supported_configs_range
            .next()
            .expect("no supported config?!")
            .with_max_sample_rate();
        let sample_format = supported_config.sample_format();

        println!("Supported Config: {:?}", supported_config);

        let config = supported_config.config();
        let samples_p = self.samples.clone();

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device
                .build_input_stream(
                    &config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        // react to stream events and read or write stream data here.
                        let mut samples = samples_p.lock().unwrap();
                        samples.copy_from_slice(data);
                        println!("getting samples");
                    },
                    |err| {
                        // react to errors here.
                        println!("{:?}", err);
                    },
                )
                .expect("Failed to initialize audio input stream"),
            _ => todo!(),
        };
        stream.play().expect("Failed to play input stream");
    }

    pub fn get_samples() {}
}
