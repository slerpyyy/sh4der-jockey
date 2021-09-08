#![warn(unsafe_op_in_unsafe_fn)]
#![warn(missing_debug_implementations)]

#[macro_use]
mod util;
mod jockey;

use jockey::Jockey;
use clap::{AppSettings, Clap};
use lazy_static::lazy_static;
use simplelog::*;

lazy_static! {
    static ref VERSION: String = format!(
        "{} (commit {})",
        env!("VERGEN_BUILD_SEMVER"),
        &env!("VERGEN_GIT_SHA")[..14]
    );
}

#[derive(Clap)]
#[clap(name = "Sh4derJockey", about)]
#[clap(version = VERSION.as_str())]
#[clap(setting = AppSettings::ColoredHelp)]
struct Args {
    #[clap(subcommand)]
    subcmd: Option<SubCommand>,

    #[clap(short, long, parse(from_occurrences))]
    verbose: u32,
}

#[derive(Clap)]
enum SubCommand {
    #[clap(about = "Create a new project in an existing directory")]
    Init,
    #[clap(about = "Start the tool in the current working directory (default)")]
    Run,
}

fn main() {
    let args: Args = Args::parse();

    let log_level = match args.verbose {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    };

    TermLogger::init(log_level, Default::default(), TerminalMode::Mixed, ColorChoice::Always).unwrap();

    if let Some(SubCommand::Init) = args.subcmd {
        let plf = std::path::Path::new("./pipeline.yaml");
        let shf = std::path::Path::new("./scene.frag");

        if plf.exists() || shf.exists() {
            log::error!(
                "File with same name already exists.\n\n\
                Please make sure there are no files named \"pipeline.yaml\" or \"scene.frag\"\n\
                in your current working directory already. Try renaming or deleting these\n\
                files and running the command again.\n"
            );
            return;
        }

        std::fs::write(plf, include_str!("defaults/pipeline.yaml")).unwrap();
        std::fs::write(shf, include_str!("defaults/scene.frag")).unwrap();

        return;
    }

    // create the jockey
    let mut jockey = Jockey::init();

    // close console
    #[cfg(all(windows, not(debug_assertions)))]
    close_console();

    loop {
        // do event stuff
        jockey.handle_events();

        // exit loop
        if jockey.done {
            break;
        }

        // run all shader stages
        jockey.draw();

        // update ui
        jockey.update_ui();
    }
}

#[cfg(all(windows, not(debug_assertions)))]
fn close_console() {
    let console = unsafe { winapi::um::wincon::GetConsoleWindow() };
    if console.is_null() {
        return;
    }

    let mut console_pid = 0;
    let status =
        unsafe { winapi::um::winuser::GetWindowThreadProcessId(console, &mut console_pid) };
    if status == 0 {
        return;
    }

    let self_pid = unsafe { winapi::um::processthreadsapi::GetCurrentProcessId() };
    if console_pid != self_pid {
        return;
    }

    unsafe { winapi::um::wincon::FreeConsole() };
}
