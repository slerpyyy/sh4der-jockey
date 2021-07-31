use rodio::{Decoder, OutputStream, Sink, Source};
use std::{fs::File, io::BufReader, time::Duration};

pub struct Playback {
    _sink: Sink,
    // music stops when this thing drops
    _stream: OutputStream,
}

impl Playback {
    pub fn new(start: f32, speed: f32) -> Option<Self> {
        if speed < 0.05 {
            return None;
        }

        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        let file = BufReader::new(File::open("test.wav").unwrap());
        let source = Decoder::new(file).unwrap()
            .repeat_infinite()
            .skip_duration(Duration::from_secs_f32(start))
            .speed(speed);

        sink.append(source);

        Some(Self {
            _stream: stream,
            _sink: sink,
        })
    }
}
