#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(dead_code, unused_imports)]

#[macro_use]
extern crate glium;

use glium::{
    glutin,
    index::{NoIndices, PrimitiveType},
    texture::Texture2d,
    uniforms::{MagnifySamplerFilter, UniformValue},
    Display, Frame, Surface, VertexBuffer,
};

use glutin::{
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    ContextBuilder,
};

use getopts::Options;
use std::{
    collections::HashMap,
    fs::File,
    time::{Duration, Instant},
};

mod pipeline;
use pipeline::Pipeline;

mod helper;
use helper::*;

#[derive(Clone, Copy)]
struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    fn new(x: f32, y: f32) -> Self {
        Vertex { position: [x, y] }
    }
}

implement_vertex!(Vertex, position);

fn print_usage(name: &str, opts: &Options) {
    println!("Usage: {} [option]\n", name);
    print!("A custom VJ tool written by sp4ghet and slerpy.");
    println!("{}", opts.usage(""));
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bin_name = args.get(0).unwrap();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help message");
    opts.optflag("i", "init", "set up a simple example project");

    let matches = opts.parse(&args[1..]).expect("failed to parse args");
    if matches.opt_present("h") {
        print_usage(bin_name, &opts);
        return;
    }

    if matches.opt_present("i") {
        std::fs::write("./pipeline.json", include_str!("defaults/pipeline.json")).unwrap();
        std::fs::write("./fs.glsl", include_str!("defaults/fs.glsl")).unwrap();
        std::fs::write("./vs.glsl", include_str!("defaults/vs.glsl")).unwrap();
        return;
    }

    let dir_name = std::env::current_dir()
        .expect("could not resolve cwd")
        .to_str()
        .expect("could not convert path to string")
        .to_string();

    println!("Using folder: \"{}\"", dir_name);

    let pipeline_file =
        File::open(format!("{}/pipeline.json", dir_name)).expect("could not open pipeline file");
    let pipeline_json: serde_json::Value =
        serde_json::from_reader(pipeline_file).expect("could not parse pipeline");

    println!("Json parsed: \"{:#?}\"", pipeline_json);

    let event_loop = EventLoop::new();
    let wb = WindowBuilder::new().with_title("VJ tool stuff idk");
    let cb = ContextBuilder::new();
    let display = Display::new(wb, cb, &event_loop).unwrap();

    let pipeline = Pipeline::from_json(pipeline_json, &display);
    println!("{:#?}", pipeline);

    // create fullscreen rect
    let v1 = Vertex::new(-1.0, -1.0);
    let v2 = Vertex::new(-1.0, 1.0);
    let v3 = Vertex::new(1.0, 1.0);
    let v4 = Vertex::new(1.0, -1.0);

    let shape = vec![v1, v2, v3, v3, v4, v1];
    let vertex_buffer = VertexBuffer::new(&display, &shape).unwrap();
    let indices = NoIndices(PrimitiveType::TrianglesList);

    // get screen size
    let (mut width, mut height) = display.get_framebuffer_dimensions();

    // init buffers
    let mut textures = HashMap::new();
    for stage in pipeline.stages.iter() {
        if let Some(tex_name) = stage.target.clone() {
            textures.insert(
                tex_name,
                Texture2d::empty(&display, width, height).unwrap(),
            );
        }
    }

    // make dummy texture
    let dummy = Texture2d::empty(&display, 1, 1).unwrap();

    // start event loop
    let start_time = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        // react to event
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                WindowEvent::Resized(size) => {
                    width = size.width;
                    height = size.height;
                }
                _ => return,
            },
            Event::NewEvents(cause) => match cause {
                StartCause::ResumeTimeReached { .. } => (),
                StartCause::Init => (),
                _ => return,
            },
            _ => return,
        }

        // compute uniforms
        let time = start_time.elapsed().as_secs_f32();

        // draw stuff
        for stage in pipeline.stages.iter() {
            let c0 = stage.channels[0].clone().map(|s| textures.get(&s)).flatten().unwrap_or(&dummy);
            let c1 = stage.channels[1].clone().map(|s| textures.get(&s)).flatten().unwrap_or(&dummy);
            let c2 = stage.channels[2].clone().map(|s| textures.get(&s)).flatten().unwrap_or(&dummy);
            let c3 = stage.channels[3].clone().map(|s| textures.get(&s)).flatten().unwrap_or(&dummy);

            let uniforms = uniform! {
                R: [width as f32, height as f32, time],
                C0: c0, C1: c1, C2: c2, C3: c3
            };

            if let Some(tex) = stage.target.clone().map(|s| textures.get(&s)).flatten() {
                let mut target = tex.as_surface();
                target
                    .draw(
                        &vertex_buffer,
                        &indices,
                        &stage.prog,
                        &uniforms,
                        &Default::default(),
                    )
                    .unwrap();
            } else {
                let mut target = display.draw();
                target
                    .draw(
                        &vertex_buffer,
                        &indices,
                        &stage.prog,
                        &uniforms,
                        &Default::default(),
                    )
                    .unwrap();
                target.finish().unwrap();
            }
        }

        // wait for next frame
        let next_frame_time = Instant::now() + Duration::from_millis(5);
        *control_flow = ControlFlow::WaitUntil(next_frame_time);
    });
}
