use std::cmp;
use std::path::Path;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::Arc;
use std::thread;

use super::time::{Duration, SteadyTime};

extern crate chip8vm;
use self::chip8vm::vm::{Chip8, CPU_CLOCK, TIMERS_CLOCK};
use self::chip8vm::display::{Display, DISPLAY_WIDTH, DISPLAY_HEIGHT};
use self::chip8vm::keypad::{Keystate};
use super::input;


/// Structure facilitating the configuration of a 'Chip8Application'.
/// The configuration functions (e.g. 'w_title') work with moved 'self' values
/// to allow chaining them inside the Chip8Application::new function call.
pub struct Chip8Config {
    /// The title of the emulator window.
    pub window_title   : &'static str,
    /// The desired width for the emulator window.
    /// NB : this is just a hint, the application may resize to reach a proper
    /// aspect ratio.
    pub window_width   : u16,
    /// The desired height for the emulator window.
    /// NB : this is just a hint, the application may resize to reach a proper
    /// aspect ratio.
    pub window_height  : u16,
    /// The keyboard configuration. QWERTY by default.
    pub keypad_binding : input::KeyboardBinding,
    /// The virtual machine's desired CPU clock in Hz (cycles per second).
    pub vm_cpu_clock   : u32,
}

/// Macro to avoid boilerplate setter code.
macro_rules! config_set_param {
    ($setter_name: ident, $param_name: ident, $param_type: ty) => (
        pub fn $setter_name(mut self, $param_name: $param_type)
            -> Chip8Config {
            self.$param_name = $param_name; self
        }
    )
}

impl Chip8Config {
    /// Create and return the default set of options.
    pub fn new() -> Chip8Config {
        Chip8Config {
            window_title   : "",
            window_width   : 64,
            window_height  : 32,
            keypad_binding : input::KeyboardBinding::QWERTY,
            vm_cpu_clock   : CPU_CLOCK
        }
    }

    config_set_param!(w_title, window_title, &'static str);
    config_set_param!(w_width, window_width, u16);
    config_set_param!(w_height, window_height, u16);
    config_set_param!(key_binds, keypad_binding, input::KeyboardBinding);
    config_set_param!(vm_cpu_clock, vm_cpu_clock, u32);
}

/// A command for the Chip8 virtual machine.
/// Allows the UI (more specifically the Chip8Emulator's backend) to feed
/// orders and information to the virtual machine's thread.
pub enum Chip8VMCommand {
    /// Set the emulation state (running for true, paused for false).
    UpdateRunStatus(bool),
    /// Communicate an update in the status of the key at the given index.
    UpdateKeyStatus(usize, Keystate),
    /// Reset the virtual machine to its default state.
    Reset,
    /// Shutdown the virtual machine.
    Quit,
}

/// A command for the Chip8 emulator's UI.
/// Allows the virtual machine to communicate with the Chip8Emulator's thread.
pub enum Chip8UICommand {
    /// Signal whether the emulator should emit a sound or not (true whenever
    /// the VM's sound timer is not zero).
    UpdateBeepingStatus(bool),
    /// A drawing command for the UI, communicating the information needed to
    /// do so. As of now, the 'Display' structure is pretty much that so we can
    /// affort to pass a copy of it.
    /// Should be called only when needed (display flagged dirty).
    UpdateDisplay(Display),
    /// Signal that the emulation is finished, emitted either after a
    /// 'Chip8VMCommand::Quit' signal was received or when the virtual machine
    /// finished the execution of its loaded program.
    Finished,
}

/// Trait that any CHIP 8 emulator backend must implement.
/// The backend is free to implement its 'run' loop however it wants to
/// but has to respect as completely as it can the 'Chip8Config' it is given.
pub trait Chip8EmulatorBackend {
    /// Start the UI loop with the given configuration and the provided
    /// thread channels.
    fn exec(&mut self, config: &Chip8Config,
            tx: Sender<Chip8VMCommand>, rx: Receiver<Chip8UICommand>);
}

/// The backend-agnostic CHIP 8 emulator application.
/// Communication between the virtual machine's emulation loop and the
/// backend's UI loop is done with 2 channels using respectively
/// 'Chip8VMCommand' and 'Chip8UICommand'.
pub struct Chip8Emulator<'a> {
    /// The 'Chip8Config' instance holding the application's configuration.
    config  : Chip8Config,
    /// Pointer to the heap-allocated backend responsible for running the
    /// actual UI loop in the main thread.
    backend : Box<Chip8EmulatorBackend + 'a>,
}

impl<'a> Chip8Emulator<'a> {
    /// Create and return a new Chip8Emulator, with the given 'Chip8Config'.
    pub fn new(config: Chip8Config, backend: Box<Chip8EmulatorBackend + 'a>)
        -> Chip8Emulator<'a> {
        Chip8Emulator {
            config  : config,
            backend : backend,
        }
    }

    /// Run the emulator application after loading the given ROM.
    /// Return true if all went well, false otherwise.
    /// TODO : more flexible run function (maybe a LoadRomCommand ?)
    pub fn run_rom(&mut self, rom_filepath: &Path) -> bool {
        // VM creation and ROM loading
        let mut vm = Chip8::new();
        info!("loading the ROM file \"{}\"...", rom_filepath.display());
        let oerror = vm.load(rom_filepath);
        if oerror.is_none() {
            info!("successfully loaded the ROM file.");
        } else {
            error!("loading error : {}", oerror.unwrap());
            return false;
        }

        // Communication channels
        let (tx_ui, rx_ui) = channel::<Chip8UICommand>();
        let (tx_vm, rx_vm) = channel::<Chip8VMCommand>();

        // VM loop, in a secondary thread
        let cpu_clock = self.config.vm_cpu_clock;
        thread::spawn(move || {
            // VM thread moved to an external function for better clarity
            exec_vm(&mut vm, cpu_clock, tx_ui, rx_vm);
        });

        // UI loop, in the emulator's thread (should be the main thread)
        self.backend.exec(&self.config, tx_vm, rx_ui);

        true
    }
}

/// Emulation loop simulating the CHIP 8 virtual machine and communicating back
/// to the emulator's backend implementation by feeding Chip8UI
pub fn exec_vm(vm: &mut Chip8, cpu_clock: u32,
               tx: Sender<Chip8UICommand>, rx: Receiver<Chip8VMCommand>) {
    use self::Chip8VMCommand::*;
    use self::Chip8UICommand::*;

    info!("starting the virtual machine thread with a CPU clock of {} Hz",
        cpu_clock);

    // time handling is in nanoseconds
    let mut t             = SteadyTime::now();
    let mut last_t_cpu    = SteadyTime::now();
    let mut last_t_timers = t;
    let timers_step = Duration::nanoseconds(10i64.pow(9)
                                            / (TIMERS_CLOCK as i64));
    let cpu_step    = Duration::nanoseconds(10i64.pow(9) / (cpu_clock as i64));

    // VM state
    let mut running         = true;
    let mut beeping         = false;
    let mut waiting_for_key = false;
    // avoid triggering multiple 'wait for key' instructions at once
    // especially with a high CPU clock
    let mut wait_for_key_last_pressed = 0xFF;

    'vm: loop {
        // Command from the UI
        match rx.try_recv() { // non-blocking receiving function
            Ok(vm_command) => match vm_command {
                UpdateRunStatus(run)          => running = run,
                UpdateKeyStatus(index, state) => {
                    match state {
                        Keystate::Pressed  => {
                            if waiting_for_key &&
                                (index != wait_for_key_last_pressed) {
                                vm.end_wait_for_key(index);
                                wait_for_key_last_pressed = index;
                            } else {
                                vm.keypad.set_key_state(index, state);
                            }
                        },
                        Keystate::Released => {
                            wait_for_key_last_pressed = 0xFF;
                            if !waiting_for_key {
                                vm.keypad.set_key_state(index, state);
                            }
                        },
                    }
                },
                Reset                         => vm.reset(),
                Quit                          => {
                    running = false;
                    info!("terminating the virtual machine thread...");
                    tx.send(Finished).unwrap();
                    break 'vm;
                },
            },
            _              => {},
        }

        // CPU
        t = SteadyTime::now();
        if t - last_t_cpu >= cpu_step {
            last_t_cpu = t;
            if running && !waiting_for_key {
                vm.emulate_cycle();
                if vm.display.dirty {
                    let display = vm.display.clone();
                    tx.send(UpdateDisplay(display)).unwrap();
                }
            }
            waiting_for_key = vm.is_waiting_for_key();
        }

        // Timers
        t = SteadyTime::now(); // this may be overkill
        if t - last_t_timers >= timers_step {
            last_t_timers = t;
            if running {
                if vm.delay_timer > 0 {
                    vm.delay_timer -= 1;
                }
                if vm.sound_timer > 0 {
                    vm.sound_timer -= 1;
                    if beeping != (vm.sound_timer > 0) {
                        beeping = !beeping;
                        tx.send(UpdateBeepingStatus(beeping)).unwrap();
                    }
                }
            }
        }

        // avoid overloading the CPU
        // this will prevent reaching very high CPU clock these are
        // bug-prone and unpractible really
        thread::sleep_ms(1);
    }
}

/// Return the best (pixel_scale, width, height) combination with the given
pub fn get_display_size(w_width: u16, w_height: u16) -> (u16, u16, u16) {
    let scale_w = w_width / (DISPLAY_WIDTH as u16);
    let scale_h = w_height / (DISPLAY_HEIGHT as u16);
    let scale = cmp::min(scale_w, scale_h);

    // adjust to the smallest scale and recompute the window dimensions
    (scale, scale * (DISPLAY_WIDTH as u16), scale * (DISPLAY_HEIGHT as u16))
}
