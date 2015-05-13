use std::env;
use std::error::Error;
use std::path::Path;

extern crate getopts;
use getopts::{Options, Matches};

// we need a backend-agnostic time handling for the VM
extern crate time;

#[macro_use]
extern crate log;

mod chip8app;
mod chip8app_sdl2;
mod input;
mod logger;
use chip8app::{Chip8Emulator, Chip8EmulatorBackend, Chip8Config};
use chip8app_sdl2::Chip8BackendSDL2;

fn print_usage(opts: Options) {
    let brief =
        "rust-chip8 emulator.\n\nUsage:\n   rust-chip8 [OPTIONS] ROM_FILE\n";
    println!("{}", opts.usage(&brief));
}

fn config_from_matches(matches: &Matches) -> Chip8Config {
    let mut config = Chip8Config::new();

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

    match matches.opt_str("c") {
        Some(ref string) => match string.parse::<u32>() {
            Ok(cpu_clock) => { config = config.vm_cpu_clock(cpu_clock); }
            Err(_)        => warn!("\"{}\" is not a valid CPU clock number",
                                     string),
        },
        _ => {},
    }

    config
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
    opts.optopt("c", "cpu-clock",
                "The CPU clock speed to target. 600 Hz by default.",
                "CPU_CLOCK_SPEED");
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
    let rom_file = if !matches.free.is_empty() { matches.free[0].clone() }
        else { print_usage(opts); return; };

    // Chip 8 virtual machine creation
    let config =  config_from_matches(&matches)
        .w_title("rust-chip8 emulator")
        .w_width(800)
        .w_height(600);
    let mut emulator = Chip8Emulator::new(config,
        Box::new(Chip8BackendSDL2) as Box<Chip8EmulatorBackend>);

    // Load the ROM and start the emulation
    let rom_filepath = Path::new(&rom_file);
    if !emulator.run_rom(&rom_filepath) {
        panic!("error while loading or running the ROM.");
    }
}
