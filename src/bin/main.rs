use std::env;
use std::error::Error;
use std::path::Path;
extern crate getopts;
use getopts::Options;
#[macro_use]
extern crate log;

mod chip8app;
mod chip8app_sdl2;
mod input;
mod logger;
use chip8app::{Chip8Emulator, Chip8Config};
use chip8app_sdl2::Chip8ApplicationSDL2;

fn print_usage(opts: Options) {
    let brief =
        "rust-chip8 emulator.\n\nUsage:\n   rust-chip8 [OPTIONS] ROM_FILE\n";
    println!("{}", opts.usage(&brief));
}

fn main() {
    // Logger initialization
    match logger::init_console_logger() {
        Err(error) => panic!(format!("Logging setup error : {}",
                                     error.description())),
        _ => (),
    }

    // Program options
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help menu.");
    opts.optopt("k", "keyboard",
                "The keyboard configuration to use. QWERTY by default.",
                "QWERTY/AZERTY");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(why) => panic!(why.to_string()),
    };
    if matches.opt_present("h") {
        print_usage(opts);
        return;
    }
    let keyboard_config = match matches.opt_str("k") {
        Some(ref string) => match &string[..] {
            "QWERTY" => input::KeyboardBinding::QWERTY,
            "AZERTY" => input::KeyboardBinding::AZERTY,
            _        => {
                warn!("unrecognized keyboard configuration argument \"{}\".",
                      string);
                input::KeyboardBinding::QWERTY
            },
        },
        _ => input::KeyboardBinding::QWERTY,
    };
    let rom_file = if !matches.free.is_empty() { matches.free[0].clone() }
        else { /*print_usage(opts); return; */ "pong.ch8".to_string() /* TEST */};

    // Chip 8 virtual machine creation
    let mut emulator = Chip8ApplicationSDL2::new(Chip8Config::new()
        .w_title("rust-chip8 emulator")
        .w_width(800)
        .w_height(600)
        .key_binds(keyboard_config));

    // Load the ROM and start the emulation
    let rom_filepath = Path::new(&rom_file);
    if !emulator.load_rom(&rom_filepath) {
        panic!("error while loading the ROM.");
    }
    emulator.run();
}
