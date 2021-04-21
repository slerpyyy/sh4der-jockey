//#![feature(external_doc)]
//#![doc(include = "../README.md")]

extern crate gl;
extern crate imgui;
extern crate imgui_opengl_renderer;
extern crate imgui_sdl2;
extern crate sdl2;

mod jockey;
mod pipeline;
mod util;

use getopts::Options;
use imgui::im_str;
use jockey::Jockey;
use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use std::fs::File;
use std::time::Instant;
use util::RunningAverage;

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

    let title = Jockey::title();
    let mut jockey = Jockey::init();

    let mut total_perf = RunningAverage::<f32, 128>::new();
    let mut do_update_pipeline = true;
    let mut last_frame = Instant::now();

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

        jockey.imgui_sdl2.prepare_frame(
            jockey.imgui.io_mut(),
            &jockey.window,
            &jockey.event_pump.mouse_state(),
        );

        // tell imgui what time it is
        let now = Instant::now();
        let delta_time = (now - last_frame).as_secs_f32();
        jockey.imgui.io_mut().delta_time = delta_time;
        last_frame = now;

        // record frame time
        total_perf.push(1000.0 * delta_time);
        let total_ms = total_perf.get();

        // run all shader stages
        jockey.draw();

        // ui magic
        let ui = jockey.imgui.frame();
        ui.text(&title);
        ui.separator();
        ui.text("...");
        ui.separator();
        ui.text(format!(
            "FPS: {:.2} ({:.2} ms)",
            1000.0 / total_ms,
            total_ms
        ));
        ui.plot_lines(im_str!("dt [ms]"), &total_perf.buffer)
            .build();
        let mut stage_sum_ms = 0.0;
        for (k, stage) in jockey.pipeline.stages.iter().enumerate() {
            let stage_ms = stage.perf.get();
            stage_sum_ms += stage_ms;
            if let Some(tex_name) = stage.target.as_ref() {
                ui.text(format!(
                    "Stage {}: {:.4} ms (-> {:?})",
                    k, stage_ms, tex_name
                ));
            } else {
                ui.text(format!("Stage {}: {:.4} ms", k, stage_ms));
            }
        }
        ui.text(format!(
            "Total: {:.4} ms ({:.2}% stress)",
            stage_sum_ms,
            100.0 * stage_sum_ms / total_ms
        ));

        // update ui
        jockey.imgui_sdl2.prepare_render(&ui, &jockey.window);
        jockey.renderer.render(ui);
        jockey.window.gl_swap_window();
    }
}
