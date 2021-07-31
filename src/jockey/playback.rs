use rodio::{decoder::DecoderError, Decoder, OutputStream, Sink, Source};
use std::{
    fs::File,
    io::BufReader,
    sync::{Arc, Mutex},
    time::Duration,
};

pub struct Playback {
    handle: Arc<Mutex<(f32, f32)>>,
    _sink: Sink,
    // music stops when this thing drops
    _stream: OutputStream,
}

impl Playback {
    pub fn new() -> Option<Self> {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        let (source, handle) = RemoteSource::from_file(File::open("test.wav").unwrap()).unwrap();

        sink.append(source);

        Some(Self {
            handle,
            _stream: stream,
            _sink: sink,
        })
    }

    pub fn resync(&self, time: f32, speed: f32) {
        *self.handle.lock().unwrap() = (time, speed);
    }
}

const RUBBER_BANDING: f32 = 0.0001;
const JUMP_THRESHOLD: f32 = 0.5;

struct RemoteSource {
    data: Vec<i16>,
    control: Arc<Mutex<(f32, f32)>>,
    time: f32,
    speed: f32,
    sample_rate: u32,
    channels: u16,
}

impl RemoteSource {
    pub fn from_file(file: File) -> Result<(Self, Arc<Mutex<(f32, f32)>>), DecoderError> {
        let decoder = Decoder::new(BufReader::new(file)).unwrap();
        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let data = decoder.collect();

        let control = Default::default();
        let control_handle = Arc::clone(&control);

        let this = Self {
            data,
            control,
            time: 0.0,
            speed: 1.0,
            sample_rate,
            channels,
        };

        Ok((this, control_handle))
    }
}

impl Iterator for RemoteSource {
    type Item = i16;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = (self.time * self.sample_rate as f32).floor() as usize;
        let index = (self.channels as usize * sample) % self.data.len();
        let value = self.data.get(index).cloned();

        const W0: f32 = 1.0 - RUBBER_BANDING;
        const W1: f32 = RUBBER_BANDING;

        let (target_time, target_speed) = *self.control.lock().unwrap();
        self.speed = W0 * self.speed + W1 * target_speed;
        let time_delta = target_time - self.time;
        if time_delta.abs() < JUMP_THRESHOLD {
            self.time += RUBBER_BANDING * time_delta;
        } else {
            self.time = target_time;
        }

        self.time += self.speed / self.sample_rate as f32;
        value
    }
}

impl Source for RemoteSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(1000)
    }

    fn channels(&self) -> u16 {
        self.channels
    }

    fn sample_rate(&self) -> u32 {
        //let rate = self.speed.abs() * self.sample_rate as f32;
        //4000.max(rate as _)
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}
