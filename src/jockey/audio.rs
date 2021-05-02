use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Device;
use std::sync::{Arc, Mutex};

use crate::util::RingBuffer;

pub enum Channels {
    None,
    Mono,
    Stereo,
}
pub struct Audio {
    l_samples: Arc<Mutex<RingBuffer<f32>>>,
    r_samples: Arc<Mutex<RingBuffer<f32>>>,
    stream: Option<cpal::Stream>,
    channels: Channels,
}

impl Audio {
    pub fn new() -> Self {
        let l_samples = Arc::new(Mutex::new(RingBuffer::new(8192)));
        let r_samples = Arc::new(Mutex::new(RingBuffer::new(8192)));
        let stream = None;
        let channels = Channels::None;
        let mut this = Self {
            l_samples,
            r_samples,
            stream,
            channels,
        };
        this.connect();
        this
    }

    pub fn connect(&mut self) {
        let host = cpal::default_host();
        println!("{:?}", cpal::available_hosts());
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

        println!("Supported Config: {:?}", supported_config);

        let config = device.default_input_config().unwrap().config();
        let sample_format = supported_config.sample_format();
        println!("Creating with config: {:?}", config);

        let channel_count = config.channels as usize;
        self.channels = match channel_count {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            _ => Channels::None,
        };

        // TODO: receive config for FFT buffer size

        let l_samples_p = self.l_samples.clone();
        let r_samples_p = self.r_samples.clone();

        let input_callback = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let sz = data.len() / (channel_count as usize);

            let mut l_samples_lock = l_samples_p.lock().unwrap();
            l_samples_lock.push_slice(&data[0..sz]);

            if channel_count > 1 {
                let mut r_samples_lock = r_samples_p.lock().unwrap();
                r_samples_lock.push_slice(&data[sz..2 * sz]);
            }
        };

        let stream = match sample_format {
            cpal::SampleFormat::F32 => device
                .build_input_stream(&config, input_callback, |err| {
                    // react to errors here.
                    println!("{:?}", err);
                })
                .expect("Failed to initialize audio input stream"),
            _ => todo!(),
        };
        stream.play().expect("Failed to play input stream");
        self.stream = Some(stream);
    }

    pub fn get_samples_build(&mut self) -> ([f32; 8192], [f32; 8192]) {
        let mut left = [0_f32; 8192];
        let mut right = [0_f32; 8192];
        self.get_samples(&mut left, &mut right);
        (left, right)
    }

    pub fn get_samples(&mut self, left: &mut [f32], right: &mut [f32]) {
        let l_samples_p = self.l_samples.clone();
        let l_samples = l_samples_p.lock().unwrap();
        l_samples.get_vec(left);

        match self.channels {
            Channels::Stereo => {
                let r_samples_p = self.r_samples.clone();
                let r_samples = r_samples_p.lock().unwrap();
                r_samples.get_vec(right);
            }
            _ => {}
        };
    }
}

impl Drop for Audio {
    fn drop(&mut self) {
        if let Some(stream) = &mut self.stream {
            drop(stream);
        }
    }
}
