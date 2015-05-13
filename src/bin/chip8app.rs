use std::cmp;
use std::path::Path;

extern crate chip8vm;
use self::chip8vm::vm::CPU_CLOCK;
use self::chip8vm::display::{DISPLAY_WIDTH, DISPLAY_HEIGHT};
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

/// Trait that any CHIP 8 emulator backend must implement.
/// The backend is free to implement its simulation however it wants to
/// but has to respect as completely as it can the 'Chip8Config' it is given.
pub trait Chip8Emulator {
    /// Try and load the given ROM filepath.
    /// Return true if succeeded, false otherwise.
    fn load_rom(&mut self, filepath: &Path) -> bool;

    /// Start the emulation.
    fn run(&mut self);
}

/// Return the best (pixel_scale, width, height) combination with the given
pub fn get_display_size(w_width: u16, w_height: u16) -> (u16, u16, u16) {
    let scale_w = w_width / DISPLAY_WIDTH;
    let scale_h = w_height / DISPLAY_HEIGHT;
    let scale = cmp::min(scale_w, scale_h);

    // adjust to the smallest scale and recompute the window dimensions
    (scale, scale * DISPLAY_WIDTH, scale * DISPLAY_HEIGHT)
}
