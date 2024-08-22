use std::path::PathBuf;

use anyhow::Context;
use clap::{Arg, ArgAction, Command};

mod init;

mod connector;
mod renderer;
mod screencopy;
mod configuration;

fn main() -> Result<(), anyhow::Error> {
    // parse command line arguments
    let cmd = Command::new("ambient-led")
        .about("ws2812b monitor backlight controller")
        .version("0.1.0")
        .author("PancakeTAS")
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("override the log level to trace")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("frames")
                .short('t')
                .long("frames")
                .help("capture only the specified amount of frames, then exit")
                .action(ArgAction::Set)
                .num_args(1)
                .value_parser(clap::value_parser!(u32))
        )
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .help("load the specified configuration file")
                .action(ArgAction::Set)
                .num_args(1)
                .value_parser(clap::value_parser!(PathBuf))
        );

    // launch the application
    let matches = cmd.get_matches();
    Ok(init::init(
        matches.get_flag("verbose"),
        matches.get_one("frames"),
        matches.get_one("config"),
    ).context("application main failed")?)
}
