extern crate gl;
extern crate imgui;
extern crate imgui_opengl_renderer;
extern crate imgui_sdl2;
extern crate sdl2;

mod jockey;
mod pipeline;
mod texture;
mod util;

use getopts::Options;
use jockey::*;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use std::fs::File;
use std::str;
use std::time::Instant;

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
        todo!()
        //std::fs::write("./pipeline.json", include_str!("defaults/pipeline.json")).unwrap();
        //std::fs::write("./fs.glsl", include_str!("defaults/fs.glsl")).unwrap();
        //std::fs::write("./vs.glsl", include_str!("defaults/vs.glsl")).unwrap();
        //return;
    }

    let mut jockey = Jockey::init();

    let mut do_update_pipeline = true;

    let mut last_frame = Instant::now();
    let start_time = Instant::now();

    'running: loop {
        for event in jockey.event_pump.poll_iter() {
            jockey.imgui_sdl2.handle_event(&mut jockey.imgui, &event);

            if jockey.imgui_sdl2.ignore_event(&event) {
                continue;
            }

            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,

                Event::KeyDown {
                    keycode: Some(Keycode::Return),
                    keymod: Mod::LCTRLMOD,
                    ..
                } => do_update_pipeline = true,

                _ => {}
            }
        }

        if do_update_pipeline {
            let pipeline_file = File::open("pipeline.json").expect("could not open pipeline file");
            jockey.update_pipeline(pipeline_file);
            println!("{:#?}", jockey);

            do_update_pipeline = false;
        }

        jockey.imgui_sdl2.prepare_frame(
            jockey.imgui.io_mut(),
            &jockey.window,
            &jockey.event_pump.mouse_state(),
        );

        let now = Instant::now();
        jockey.imgui.io_mut().delta_time = (now - last_frame).as_secs_f32();
        last_frame = now;

        // compute uniforms
        let (width, height) = jockey.window.size();
        let time = start_time.elapsed().as_secs_f32();

        // run all shader stages
        jockey.draw(width as _, height as _, time);

        let ui = jockey.imgui.frame();
        ui.show_demo_window(&mut true); // Zhe magic

        jockey.imgui_sdl2.prepare_render(&ui, &jockey.window);
        jockey.renderer.render(ui);

        jockey.window.gl_swap_window();

        std::thread::sleep(std::time::Duration::new(0, 1_000_000_000u32 / 60));
    }
}
