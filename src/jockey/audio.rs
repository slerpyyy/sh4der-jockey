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
    pub l_signal: Vec<f32>,
    pub r_signal: Vec<f32>,
    l_samples: Arc<Mutex<RingBuffer<f32>>>,
    r_samples: Arc<Mutex<RingBuffer<f32>>>,
    _stream: Option<cpal::Stream>,
    channels: Channels,
}

impl Audio {
    pub fn new() -> Self {
        let size = 8192;
        let l_samples = Arc::new(Mutex::new(RingBuffer::new(size)));
        let r_samples = Arc::new(Mutex::new(RingBuffer::new(size)));
        let _stream = None;
        let channels = Channels::None;
        let mut l_signal = Vec::with_capacity(size);
        l_signal.resize(size, 0_f32);
        let mut r_signal = Vec::with_capacity(size);
        r_signal.resize(size, 0_f32);
        let mut this = Self {
            l_signal,
            r_signal,
            l_samples,
            r_samples,
            _stream,
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
            {
                let mut l_samples_lock = l_samples_p.lock().unwrap();
                for x in data.iter().step_by(channel_count) {
                    l_samples_lock.push(x);
                }
            }

            if channel_count > 1 {
                let mut r_samples_lock = r_samples_p.lock().unwrap();
                for x in data.iter().skip(1).step_by(channel_count) {
                    r_samples_lock.push(x);
                }
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

        self._stream = Some(stream);
    }

    pub fn update_samples(&mut self) {
        let l_samples_p = self.l_samples.clone();
        let l_samples = l_samples_p.lock().unwrap();
        l_samples.copy_to_slice(&mut self.l_signal);

        if let Channels::Stereo = self.channels {
            let r_samples_p = self.r_samples.clone();
            let r_samples = r_samples_p.lock().unwrap();
            r_samples.copy_to_slice(&mut self.r_signal);
        };
    }

    #[allow(dead_code)]
    pub fn get_samples(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.update_samples();
        left.copy_from_slice(&self.l_signal);
        right.copy_from_slice(&self.r_signal);
    }
}
