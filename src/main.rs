//#![feature(external_doc)]
//#![doc(include = "../README.md")]

extern crate gl;
extern crate imgui;
extern crate imgui_opengl_renderer;
extern crate imgui_sdl2;
extern crate lazy_static;
extern crate sdl2;

mod jockey;
mod pipeline;
mod util;

use getopts::Options;
use jockey::Jockey;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use std::fs::File;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let bin_name = args.get(0).unwrap();

    let mut opts = Options::new();
    opts.optflag("h", "help", "print this help message");
    opts.optflag("i", "init", "set up a simple example project");

    let matches = opts.parse(&args[1..]).expect("failed to parse args");
    if matches.opt_present("h") {
        println!(
            "{}\n\n{}",
            opts.short_usage(bin_name),
            opts.usage("A custom VJ tool written by sp4ghet and slerpy.")
        );
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
                    keymod,
                    ..
                } if keymod & Mod::LCTRLMOD != Mod::NOMOD => do_update_pipeline = true,

                //Event::Window {
                //    win_event: WindowEvent::Resized(width, height),
                //    ..
                //} => {
                //    println!("resize detected {:?}", (width, height));
                //    jockey.window.set_size(width as _, height as _).unwrap();
                //}
                _ => {}
            }
        }

        // live shader reloading
        if do_update_pipeline {
            let pipeline_file = File::open("pipeline.json").expect("could not open pipeline file");
            jockey.update_pipeline(pipeline_file);
            println!("{:#?}", jockey);

            do_update_pipeline = false;
        }

        // run all shader stages
        jockey.draw();

        // do ui stuff
        jockey.build_ui();

        // update ui
        jockey.window.gl_swap_window();
    }
}
