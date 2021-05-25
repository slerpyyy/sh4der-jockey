use crate::util::RingBuffer;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Device;
use num_complex::Complex;
use rustfft::{Fft, FftPlanner};
use std::sync::{Arc, Mutex};

pub const AUDIO_SAMPLES: usize = 8192;

pub enum Channels {
    None,
    Mono,
    Stereo,
}

pub struct Audio {
    pub l_signal: Vec<f32>,
    pub r_signal: Vec<f32>,
    pub l_raw_spectrum: Vec<f32>,
    pub r_raw_spectrum: Vec<f32>,
    pub l_spectrum: Vec<f32>,
    pub r_spectrum: Vec<f32>,
    pub size: usize,
    pub nice_size: usize,
    pub volume: [f32; 3],
    l_fft: Vec<Complex<f32>>,
    r_fft: Vec<Complex<f32>>,
    l_samples: Arc<Mutex<RingBuffer<f32>>>,
    r_samples: Arc<Mutex<RingBuffer<f32>>>,
    _stream: Option<cpal::Stream>,
    channels: Channels,
    sample_freq: usize,
    fft: Arc<dyn Fft<f32>>,
}

impl Audio {
    pub fn new() -> Self {
        let size = AUDIO_SAMPLES;
        let spec_size = size / 2;
        let bands = 100;

        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(size);

        let mut this = Self {
            size,
            nice_size: bands,
            l_signal: vec![0.0; size],
            r_signal: vec![0.0; size],
            l_fft: vec![Complex::new(0.0, 0.0); size],
            r_fft: vec![Complex::new(0.0, 0.0); size],
            volume: [0f32; 3],
            l_raw_spectrum: vec![0.0; spec_size],
            r_raw_spectrum: vec![0.0; spec_size],
            l_spectrum: vec![0.0; bands],
            r_spectrum: vec![0.0; bands],
            l_samples: Arc::new(Mutex::new(RingBuffer::new(size))),
            r_samples: Arc::new(Mutex::new(RingBuffer::new(size))),
            _stream: None,
            channels: Channels::None,
            fft,
            sample_freq: 0,
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
        let sample_freq = config.sample_rate.0;
        self.sample_freq = sample_freq as _;

        self._stream = Some(stream);
    }

    pub fn update_samples(&mut self) {
        let l_samples_p = self.l_samples.clone();
        let l_samples = l_samples_p.lock().unwrap();
        l_samples.copy_to_slice(&mut self.l_signal);

        // calculate volume with RMS
        self.volume[1] = (self.l_signal.iter().fold(0f32, |acc, x| acc + x).powi(2)
            / l_samples.size as f32)
            .sqrt();

        if let Channels::Stereo = self.channels {
            let r_samples_p = self.r_samples.clone();
            let r_samples = r_samples_p.lock().unwrap();
            r_samples.copy_to_slice(&mut self.r_signal);
            self.volume[2] = (self.l_signal.iter().fold(0f32, |acc, x| acc + x).powi(2)
                / l_samples.size as f32)
                .sqrt();
            self.volume[0] = (self.volume[1] + self.volume[2]) / 2f32;
        } else {
            self.volume[0] = self.volume[1];
        };
    }

    pub fn update_fft(&mut self) {
        let left: Vec<_> = self
            .l_signal
            .iter()
            .map(|x| Complex::new(x.clone(), 0f32))
            .collect();

        let right: Vec<_> = self
            .r_signal
            .iter()
            .map(|x| Complex::new(x.clone(), 0f32))
            .collect();

        self.l_fft.copy_from_slice(&left);
        self.r_fft.copy_from_slice(&right);

        self.fft.process(&mut self.l_fft);
        self.fft.process(&mut self.r_fft);

        let mut left_spectrum: Vec<_> = self.l_fft.iter().map(|z| z.norm_sqr()).collect();
        let mut right_spectrum: Vec<_> = self.r_fft.iter().map(|z| z.norm_sqr()).collect();

        let cmp = |x: &&f32, y: &&f32| x.partial_cmp(y).unwrap();
        let max_left = left_spectrum.iter().max_by(cmp).unwrap().clone();
        let max_right = right_spectrum.iter().max_by(cmp).unwrap().clone();
        for i in 0..left_spectrum.len() {
            left_spectrum[i] /= max_left;
            right_spectrum[i] /= max_right;
        }

        let len = left_spectrum.len() / 2;
        self.l_raw_spectrum.copy_from_slice(&left_spectrum[..len]);
        self.r_raw_spectrum.copy_from_slice(&right_spectrum[..len]);

        self.update_nice_fft();
    }

    fn update_nice_fft(&mut self) {
        self.l_spectrum.fill(0f32);
        self.r_spectrum.fill(0f32);

        let n = self.l_raw_spectrum.len() * 2;
        let bins = self.l_spectrum.len();

        let fs_over_n = self.sample_freq as f32 / n as f32;

        let half_n = self.l_raw_spectrum.len() as f32;
        let inv_half_n = 1f32 / half_n;

        let mut max_left = 0f32;
        let mut max_right = 0f32;
        for (i, (l, r)) in self
            .l_raw_spectrum
            .iter()
            .zip(self.r_raw_spectrum.iter())
            .enumerate()
        {
            let freq = i as f64 * fs_over_n as f64;

            // https://www.wikiwand.com/en/Piano_key_frequencies
            let bin = (12f64 * (freq / 440f64).log2()) as i32 + 49;
            let bi = if bin >= bins as _ {
                bins - 1
            } else if bin < 0 {
                0
            } else {
                bin as usize
            };

            //https://github.com/jberg/butterchurn/blob/master/src/audio/fft.js#L20
            let eq = -0.02 * ((half_n - i as f32) * inv_half_n).log10();
            let l_int = l * eq;
            let r_int = r * eq;
            max_left = max_left.max(l_int);
            max_right = max_right.max(r_int);

            self.l_spectrum[bi] = self.l_spectrum[bi].max(l_int);
            self.r_spectrum[bi] = self.r_spectrum[bi].max(r_int);
        }

        for i in 1..(bins - 1) {
            if self.l_spectrum[i] == 0f32 {
                self.l_spectrum[i] = (self.l_spectrum[i - 1] + self.l_spectrum[i + 1]) / 2f32;
            }
            if self.r_spectrum[i] == 0f32 {
                self.r_spectrum[i] = (self.r_spectrum[i - 1] + self.r_spectrum[i + 1]) / 2f32;
            }
        }

        for i in 0..bins {
            self.l_spectrum[i] /= max_left;
            self.r_spectrum[i] /= max_right;
        }
    }

    #[allow(dead_code)]
    pub fn get_samples(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.update_samples();
        left.copy_from_slice(&self.l_signal);
        right.copy_from_slice(&self.r_signal);
    }
}
