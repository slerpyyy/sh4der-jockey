use midir::{Ignore, MidiInput, MidiInputConnection, MidiInputPort};
use std::{
    collections::HashMap,
    sync::mpsc::{channel, Receiver},
    time::Instant,
};

pub const MIDI_N: usize = 32;

pub struct Midi {
    pub conns: Vec<MidiInputConnection<()>>,
    pub queues: Vec<Receiver<[u8; 3]>>,
    pub last_button: [u8; 2],
    pub last_slider: [u8; 2],
    pub sliders: [f32; MIDI_N],
    pub buttons: [(f32, Instant, Instant, u32); MIDI_N],
    pub button_bindings: HashMap<[u8; 2], usize>,
    pub slider_bindings: HashMap<[u8; 2], usize>,
    config_file: std::path::PathBuf,
}

#[derive(Debug, Clone, Copy)]
pub enum MessageKind {
    NoteOn { channel: u8, key: u8, velocity: u8 },
    NoteOff { channel: u8, key: u8, velocity: u8 },
    KeyPressure { channel: u8, key: u8, pressure: u8 },
    ControlChange { channel: u8, key: u8, value: u8 },
}

impl Midi {
    pub fn new() -> Self {
        let conns = Vec::new();
        let queues = Vec::new();
        let last_button = [0, 0];
        let last_slider = [0, 0];
        let sliders = [0.0; MIDI_N];
        let buttons = [(0f32, Instant::now(), Instant::now(), 0); MIDI_N];
        let mut button_bindings = HashMap::new();
        let mut slider_bindings = HashMap::new();

        let mut config_file = std::env::current_exe().unwrap();
        config_file.set_file_name("midi-config.yaml");

        if let Ok(file) = std::fs::File::open(&config_file) {
            let tuple: (_, _) = serde_yaml::from_reader(file).unwrap();
            button_bindings = tuple.0;
            slider_bindings = tuple.1;
        }

        let mut this = Self {
            conns,
            queues,
            last_button,
            last_slider,
            sliders,
            buttons,
            button_bindings,
            slider_bindings,
            config_file,
        };

        this.connect();
        this
    }

    pub fn check_connections(&mut self) {
        let midi_in = MidiInput::new("Sh4derJockey").unwrap();
        if midi_in.port_count() == self.conns.len() {
            return;
        }
        self.conns = Vec::new();
        self.queues = Vec::new();
        self.connect();
    }

    pub fn connect(&mut self) {
        let mut midi_in = MidiInput::new("Sh4derJockey").unwrap();
        midi_in.ignore(Ignore::None);
        // Get an input port (read from console if multiple are available)
        let in_ports = midi_in.ports();
        if midi_in.port_count() == 0 {
            println!("Failed to find midi input port.");
            return;
        }

        let mut conns = Vec::new();
        let mut queues = Vec::new();
        for in_port in in_ports.iter() {
            let (conn, rx) = self.new_connection(in_port);
            conns.push(conn);
            queues.push(rx);
        }

        self.conns = conns;
        self.queues = queues;
    }

    fn new_connection(
        &mut self,
        in_port: &MidiInputPort,
    ) -> (MidiInputConnection<()>, Receiver<[u8; 3]>) {
        let mut midi_input = MidiInput::new("Sh4derJockey").unwrap();
        midi_input.ignore(Ignore::None);
        let port_name = midi_input.port_name(&in_port).unwrap();
        println!("Connecting to input port: {}", port_name);

        let (tx, rx) = channel();
        let conn = midi_input
            .connect(
                in_port,
                format!("sh4der-jockey-read-input-{}", port_name).as_str(),
                move |_, message, _| {
                    if message.len() != 3 {
                        return;
                    }
                    let mut out = [0; 3];
                    out.copy_from_slice(message);
                    tx.send(out).unwrap();
                },
                (),
            )
            .expect("Failed to create MIDI connection");
        (conn, rx)
    }

    pub fn parse_msg(message: [u8; 3]) -> Option<MessageKind> {
        let status = message[0];
        let data0 = message[1];
        let data1 = message[2];
        let kind_bits = 0xF0_u8 & status;
        let channel = status & 0x0F_u8;
        match kind_bits {
            0x80 => Some(MessageKind::NoteOff {
                channel,
                key: data0,
                velocity: data1,
            }),
            0x90 => Some(MessageKind::NoteOn {
                channel,
                key: data0,
                velocity: data1,
            }),
            0xA0 => Some(MessageKind::KeyPressure {
                channel,
                key: data0,
                pressure: data1,
            }),
            0xB0 => Some(MessageKind::ControlChange {
                channel,
                key: data0,
                value: data1,
            }),
            _ => None,
        }
    }

    pub fn handle_input(&mut self) {
        for queue in &self.queues {
            for message in queue.try_iter() {
                let kind = Self::parse_msg(message);
                // println!("{:#02x} {} {}", message[0], message[1], message[2]);
                // println!("{:?}", kind);
                match kind {
                    None => {
                        continue;
                    }
                    Some(k) => match k {
                        MessageKind::NoteOn {
                            channel,
                            key,
                            velocity,
                        } => {
                            self.last_button = [channel, key];
                            if let Some(&id) = self.button_bindings.get(&self.last_button) {
                                self.buttons[id].0 = velocity as f32 / 127_f32;
                                self.buttons[id].1 = Instant::now();
                                self.buttons[id].3 += 1;
                            }
                        }
                        MessageKind::NoteOff { channel, key, .. } => {
                            self.last_button = [channel, key];
                            if let Some(&id) = self.button_bindings.get(&self.last_button) {
                                self.buttons[id].0 = 0_f32;
                                self.buttons[id].2 = Instant::now();
                            }
                        }
                        MessageKind::KeyPressure {
                            channel,
                            key,
                            pressure,
                        } => {
                            self.last_button = [channel, key];
                            if let Some(&id) = self.button_bindings.get(&self.last_button) {
                                self.buttons[id].0 = pressure as f32 / 127_f32;
                            }
                        }
                        MessageKind::ControlChange {
                            channel,
                            key,
                            value,
                        } => {
                            self.last_slider = [channel, key];
                            if let Some(&id) = self.slider_bindings.get(&self.last_slider) {
                                self.sliders[id] = value as f32 / 127_f32;
                            }
                        }
                    },
                }
            }
        }
    }

    pub fn auto_bind_slider(&mut self, id: usize) {
        if id < MIDI_N {
            self.slider_bindings.insert(self.last_slider, id);
        }
    }

    pub fn auto_bind_button(&mut self, id: usize) {
        if id < MIDI_N {
            self.button_bindings.insert(self.last_button, id);
        }
    }
}

impl Drop for Midi {
    fn drop(&mut self) {
        let file = std::fs::File::create(&self.config_file).unwrap();
        let tuple = (&self.button_bindings, &self.slider_bindings);
        serde_yaml::to_writer(file, &tuple).unwrap();
    }
}
