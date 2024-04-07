#![warn(unsafe_op_in_unsafe_fn)]
#![warn(missing_debug_implementations)]

#[macro_use]
mod util;
mod jockey;

use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use clap::Parser;
use jockey::Jockey;
use lazy_static::lazy_static;
use simplelog::*;

lazy_static! {
    static ref VERSION: String = format!(
        "{} (commit {})",
        env!("CARGO_PKG_VERSION"),
        &env!("VERGEN_GIT_SHA")[..14]
    );
}

#[derive(clap::Parser)]
#[clap(name = "Sh4derJockey", about)]
#[clap(version = VERSION.as_str())]
struct Args {
    #[clap(subcommand)]
    subcmd: Option<SubCommand>,

    #[clap(short, long, action = clap::ArgAction::Count, global = true)]
    #[clap(help = "Use verbose output (can be applied multiple times)")]
    verbose: u8,
}

#[derive(Parser)]
enum SubCommand {
    #[clap(about = "Create a new project in an existing directory")]
    #[command(alias("i"))]
    Init,

    #[clap(about = "Start the tool in the current working directory (default)")]
    #[command(alias("r"))]
    Run,
}

fn main() {
    let args: Args = Args::parse();

    let log_level = match args.verbose {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        3 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    let mut config = ConfigBuilder::new();
    let log_utc = config.set_time_offset_to_local().is_err();

    TermLogger::init(
        log_level,
        config.build(),
        TerminalMode::Mixed,
        ColorChoice::Always,
    )
    .unwrap();

    log::info!("Version: {}", VERSION.as_str());
    log::info!("Log level: {}", log_level);

    if log_utc {
        log::warn!("Failed to resolve local time, logging in UTC");
    }

    if let Some(SubCommand::Init) = args.subcmd {
        let plf = Path::new("./pipeline.yaml");
        let shf = Path::new("./scene.frag");

        if plf.exists() || shf.exists() {
            log::error!(
                "File with same name already exists.\n\n\
                Please make sure there are no files named \"pipeline.yaml\" or \"scene.frag\"\n\
                in your current working directory already. Try renaming or deleting these\n\
                files and running the command again.\n"
            );
            return;
        }

        let plf_res = std::fs::write(plf, include_str!("defaults/pipeline.yaml"));
        let shf_res = std::fs::write(shf, include_str!("defaults/scene.frag"));

        if let Err(err) = plf_res.and(shf_res) {
            log::error!("{}", err);
        }

        return;
    }

    // set termination signal handler
    let kill_signal: &'static AtomicBool = Box::leak(Box::new(AtomicBool::new(false)));
    ctrlc::set_handler(move || {
        log::info!("Kill signal detected, attempt to shut down gracefully...");
        kill_signal.store(true, Ordering::Release);

        // give it a moment to exit peacefully
        std::thread::sleep(Duration::from_secs(3));

        log::info!("Alright, let's kill this thing");
        std::process::exit(0);
    })
    .unwrap();

    // create the jockey
    let mut jockey = Jockey::init();

    // close console window
    #[cfg(all(windows, not(debug_assertions)))]
    close_console();

    loop {
        // do event stuff
        jockey.handle_events();

        // exit loop
        if jockey.done || kill_signal.load(Ordering::Acquire) {
            break;
        }

        // run all shader stages
        jockey.draw();

        // update ui
        jockey.update_ui();
    }

    log::info!("Bye bye!");
}

// https://github.com/kirillkovalenko/nssm/blob/master/console.cpp
#[cfg(all(windows, not(debug_assertions)))]
fn close_console() {
    use winapi::um::{processthreadsapi, wincon, winuser};

    let console = unsafe { wincon::GetConsoleWindow() };
    if console.is_null() {
        return;
    }

    let mut console_pid = 0;
    let status = unsafe { winuser::GetWindowThreadProcessId(console, &mut console_pid) };
    if status == 0 {
        return;
    }

    let self_pid = unsafe { processthreadsapi::GetCurrentProcessId() };
    if console_pid != self_pid {
        return;
    }

    unsafe { wincon::FreeConsole() };
}
