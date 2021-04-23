//#![feature(external_doc)]
//#![doc(include = "../README.md")]

extern crate gl;
extern crate imgui;
extern crate imgui_opengl_renderer;
extern crate imgui_sdl2;
extern crate lazy_static;
extern crate sdl2;

mod jockey;
mod util;

use getopts::Options;
use jockey::Jockey;

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
        let plf = std::path::Path::new("./pipeline.json");
        let shf = std::path::Path::new("./scene.frag");

        if plf.exists() || shf.exists() {
            eprintln!("Error: File with same name already exists.\n");
            eprintln!(
                "Please make sure there is no file named \"pipeline.json\" or \"scene.json\""
            );
            eprintln!("in your current working directory already. Try renaming or deleting these");
            eprintln!("files and running the command again.\n");
            return;
        }

        std::fs::write(plf, include_str!("defaults/pipeline.json")).unwrap();
        std::fs::write(shf, include_str!("defaults/scene.frag")).unwrap();

        return;
    }

    // create the jockey
    let mut jockey = Jockey::init();

    loop {
        // do event stuff
        jockey.handle_events();

        // exit loop
        if jockey.done {
            break;
        }

        // run all shader stages
        jockey.draw();

        // do ui stuff
        jockey.build_ui();

        // update ui
        jockey.window.gl_swap_window();
    }
}
