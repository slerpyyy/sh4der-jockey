use std::{
    collections::HashMap,
    io::Write,
    sync::mpsc::{channel, Receiver},
    time::Instant,
};

use midir::{ConnectError, Ignore, MidiInput, MidiInputConnection, MidiInputPort};

use super::Config;

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
    preferred_devices: Vec<String>,
    config_file: std::path::PathBuf,
    port_count: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum MessageKind {
    NoteOn { channel: u8, key: u8, velocity: u8 },
    NoteOff { channel: u8, key: u8, velocity: u8 },
    KeyPressure { channel: u8, key: u8, pressure: u8 },
    ControlChange { channel: u8, key: u8, value: u8 },
}

impl Midi {
    pub fn new(config: &Config) -> Self {
        let now = Instant::now();
        let sliders = [0.0; MIDI_N];
        let buttons = [(0.0, now, now, 0); MIDI_N];
        let mut button_bindings = HashMap::new();
        let mut slider_bindings = HashMap::new();

        let mut config_file = std::env::current_exe().unwrap();
        config_file.set_file_name("midi-config.dat");

        let preferred_devices = config.midi_devices.clone();

        if let Ok(file) = std::fs::File::open(&config_file) {
            match serde_yaml::from_reader(file) {
                Ok((b, s)) => {
                    button_bindings = b;
                    slider_bindings = s;
                    log::info!("Loaded midi bindings successfully");
                }
                _ => log::error!(
                    "Failed to parse midi config file. Please do not edit the config file."
                ),
            };
        }

        let mut this = Self {
            conns: Vec::new(),
            queues: Vec::new(),
            last_button: [0, 0],
            last_slider: [0, 0],
            sliders,
            buttons,
            button_bindings,
            slider_bindings,
            preferred_devices,
            config_file,
            port_count: 0,
        };

        this.connect();
        this
    }

    pub fn check_connections(&mut self) {
        let midi_in = MidiInput::new("Sh4derJockey").unwrap();
        if midi_in.port_count() == self.port_count {
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
            log::warn!("No midi input port found.");
            return;
        }

        let in_ports = if self.preferred_devices.len() != 0 {
            in_ports
                .into_iter()
                .filter(|port| {
                    let mut matches = false;
                    for pref in self.preferred_devices.iter() {
                        matches =
                            matches || midi_in.port_name(port).unwrap_or_default().contains(pref);
                    }
                    matches
                })
                .collect()
        } else {
            in_ports
        };

        let mut conns = Vec::new();
        let mut queues = Vec::new();
        for in_port in in_ports.iter() {
            let (conn, rx) = match self.new_connection(in_port) {
                Ok(x) => x,
                Err(x) => {
                    log::warn!(
                        "Failed to connect to {}: {:?}",
                        midi_in.port_name(&in_port).unwrap(),
                        x
                    );
                    continue;
                }
            };
            conns.push(conn);
            queues.push(rx);
        }

        self.conns = conns;
        self.queues = queues;
        self.port_count = midi_in.port_count();
    }

    fn new_connection(
        &self,
        in_port: &MidiInputPort,
    ) -> Result<(MidiInputConnection<()>, Receiver<[u8; 3]>), ConnectError<MidiInput>> {
        let mut midi_input = MidiInput::new("Sh4derJockey").unwrap();
        midi_input.ignore(Ignore::None);
        let port_name = midi_input.port_name(&in_port).unwrap();
        log::info!("Connecting to input port: {}", port_name);

        let (tx, rx) = channel();
        let conn = midi_input.connect(
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
        )?;
        Ok((conn, rx))
    }

    pub fn handle_input(&mut self) {
        fn parse_msg(message: [u8; 3]) -> Option<MessageKind> {
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

        for queue in &self.queues {
            for message in queue.try_iter() {
                let kind = parse_msg(message);
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
                                self.buttons[id].0 = velocity as f32 / 127.0;
                                self.buttons[id].1 = Instant::now();
                                self.buttons[id].3 += 1;
                            }
                        }
                        MessageKind::NoteOff { channel, key, .. } => {
                            self.last_button = [channel, key];
                            if let Some(&id) = self.button_bindings.get(&self.last_button) {
                                self.buttons[id].0 = 0.0;
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
                                self.buttons[id].0 = pressure as f32 / 127.0;
                            }
                        }
                        MessageKind::ControlChange {
                            channel,
                            key,
                            value,
                        } => {
                            self.last_slider = [channel, key];
                            if let Some(&id) = self.slider_bindings.get(&self.last_slider) {
                                self.sliders[id] = value as f32 / 127.0;
                            }
                        }
                    },
                }
            }
        }
    }

    fn store_bindings(&self) {
        match std::fs::File::create(&self.config_file) {
            Ok(mut file) => {
                file.write_all(b"# This file was automatically generated by Sh4derJockey.\n# Please do not edit this file.\n").unwrap();
                let tuple = (&self.button_bindings, &self.slider_bindings);
                serde_yaml::to_writer(file, &tuple).unwrap();
                log::info!("Stored midi bindings successfully");
            }
            Err(err) => log::error!("Failed to save midi configs: {}", err),
        }
    }

    pub fn bind_slider(&mut self, id: usize) {
        if id < MIDI_N {
            self.slider_bindings.retain(|_, bid| *bid != id);
            self.slider_bindings.insert(self.last_slider, id);
            self.store_bindings();
        }
    }

    pub fn bind_button(&mut self, id: usize) {
        if id < MIDI_N {
            self.button_bindings.retain(|_, bid| *bid != id);
            self.button_bindings.insert(self.last_button, id);
            self.store_bindings();
        }
    }

    pub fn unbind_slider(&mut self, id: usize) {
        if id < MIDI_N {
            self.slider_bindings.retain(|_, bid| *bid != id);
            self.store_bindings();
        }
    }

    pub fn unbind_button(&mut self, id: usize) {
        if id < MIDI_N {
            self.button_bindings.retain(|_, bid| *bid != id);
            self.store_bindings();
        }
    }
}
