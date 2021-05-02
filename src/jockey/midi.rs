use midir::{Ignore, MidiInput, MidiInputConnection, MidiInputPort};
use std::{
    collections::HashMap,
    sync::mpsc::{channel, Receiver},
    time::Instant,
};

pub struct Midi<const N: usize> {
    pub conns: Vec<MidiInputConnection<()>>,
    pub queues: Vec<Receiver<[u8; 3]>>,
    pub last: [u8; 2],
    pub sliders: [f32; N],
    pub buttons: [Instant; N],
    pub bindings: HashMap<[u8; 2], usize>,
}

impl<const N: usize> Midi<N> {
    pub fn new() -> Self {
        let conns = Vec::new();
        let queues = Vec::new();
        let last = [0, 0];
        let sliders = [0.0; N];
        let buttons = [Instant::now(); N];
        let bindings = HashMap::new();

        let mut this = Self {
            conns,
            queues,
            last,
            sliders,
            buttons,
            bindings,
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
                    let mut out = [0; 3];
                    out.copy_from_slice(message);
                    tx.send(out).unwrap();
                },
                (),
            )
            .expect("Failed to create MIDI connection");
        (conn, rx)
    }

    pub fn handle_input(&mut self) {
        for queue in &self.queues {
            for message in queue.try_iter() {
                // println!("incoming messge: {:x?}", &message);
                let key = &message[..2];
                self.last.copy_from_slice(key);
                match self.bindings.get(key) {
                    Some(&id) if id < N => {
                        self.sliders[id as usize] = (message[2] as f32) / 127.0;
                    }

                    Some(&id) if id < 2 * N => {
                        self.buttons[(id - N) as usize] = Instant::now();
                    }

                    _ => (),
                }
            }
        }
    }

    pub fn bind(&mut self, key: [u8; 2], id: usize) {
        self.bindings.insert(key, id);
    }

    pub fn auto_bind_slider(&mut self, id: usize) {
        if id < N {
            self.bind(self.last, id);
        }
    }

    pub fn auto_bind_button(&mut self, id: usize) {
        if id < N {
            self.bind(self.last, id + N);
        }
    }
}
