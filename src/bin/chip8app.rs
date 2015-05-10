use std::collections::HashMap;
use std::path::Path;

extern crate sdl2;
use self::sdl2::video::{Window, OPENGL, WindowPos};
use self::sdl2::render::{RenderDriverIndex, ACCELERATED, Renderer};
use self::sdl2::event::Event;
use self::sdl2::pixels::Color;
use self::sdl2::keycode::KeyCode;

extern crate chip8vm;
use self::chip8vm::vm::Chip8;

/// Enumerates the supported keyboard bindings for the virtual keypad.
/// TODO : add a Custom(...key bindings...) type, loaded from a file ?
pub enum KeyboardBinding {
    QWERTY,
    AZERTY,
}

/// Structure facilitating the configuration of a 'Chip8Application'.
/// The configuration functions (e.g. 'w_title') work with moved 'self' values
/// to allow chaining them inside the Chip8Application::new function call.
pub struct Chip8Config {
    window_title   : &'static str,
    window_width   : u32,
    window_height  : u32,
    keypad_binding : KeyboardBinding,
}

impl Chip8Config {
    /// Create and return the default set of options.
    pub fn new() -> Chip8Config {
        Chip8Config {
            window_title  : "",
            window_width  : 64,
            window_height : 32,
            keypad_binding: KeyboardBinding::QWERTY
        }
    }

    /// Set the window title.
    pub fn w_title(mut self, title: &'static str) -> Chip8Config {
        self.window_title = title; self
    }

    /// Set the window width.
    pub fn w_width(mut self, width: u32) -> Chip8Config {
        self.window_width = width; self
    }

    /// Set the window height.
    pub fn w_height(mut self, height: u32) -> Chip8Config {
        self.window_height = height; self
    }

    /// Set the keyboard configuration.
    pub fn key_binds(mut self, keyboard: KeyboardBinding) -> Chip8Config {
        self.keypad_binding = keyboard; self
    }
}

/// Return the HashMap<KeyCode, usize> corresponding to the
/// given keyboard configuration which allows to simulate the virtual keypad.
/// See 'chip8vm::keypad::Keypad' for the QWERTY binding.
/// NB : the value in the HashMap is not the key index but its actual name.
fn get_key_bindings(keyboard: &KeyboardBinding) -> HashMap<KeyCode, usize> {
    let mut hm = HashMap::new();

    // since we only support AZERTY and QWERTY for now, insert the common keys
    hm.insert(KeyCode::Num1, 0x1);
    hm.insert(KeyCode::Num2, 0x2);
    hm.insert(KeyCode::Num3, 0x3);
    hm.insert(KeyCode::Num4, 0xC);
    hm.insert(KeyCode::E, 0x6);
    hm.insert(KeyCode::R, 0xD);
    hm.insert(KeyCode::S, 0x8);
    hm.insert(KeyCode::D, 0x9);
    hm.insert(KeyCode::F, 0xE);
    hm.insert(KeyCode::C, 0xB);
    hm.insert(KeyCode::V, 0xF);

    match *keyboard {
        KeyboardBinding::QWERTY => {
            hm.insert(KeyCode::Q, 0x4);
            hm.insert(KeyCode::W, 0x5);
            hm.insert(KeyCode::A, 0x7);
            hm.insert(KeyCode::Z, 0xA);
            hm.insert(KeyCode::X, 0x0);
        },
        KeyboardBinding::AZERTY => {
            hm.insert(KeyCode::Q, 0x7);
            hm.insert(KeyCode::W, 0xA);
            hm.insert(KeyCode::A, 0x4);
            hm.insert(KeyCode::Z, 0x5);
            hm.insert(KeyCode::X, 0x0);
        },
    }

    assert_eq!(hm.len(), 16);

    hm
}

/// Chip8Application uses SDL2 and the chip8vm (internal) library to run CHIP 8
/// ROMs.
pub struct Chip8Application {
    /// The instance of the virtual machine simulating the Chip 8's components
    /// (CPU, display, input and sound).
    vm             : Chip8,
    /// The 'Chip8Config' instance holding the application's configuration.
    config         : Chip8Config,
}

impl Chip8Application {
    /// Create and return a new Chip8Application, with the given 'Chip8Config'.
    pub fn new(config: Chip8Config) -> Chip8Application {
        Chip8Application {
            vm: Chip8::new(),
            config: config
        }
    }

    /// Try and load the given ROM filepath.
    /// Return true if succeeded, false otherwise.
    pub fn load_rom(&mut self, filepath: &Path) -> bool {
        info!("loading the ROM file \"{}\"...", filepath.display());
        let oerror = self.vm.load(filepath);
        if oerror.is_none() {
            info!("successfully loaded the ROM file.");
            true
        } else {
            error!("loading error : {}", oerror.unwrap());
            false
        }
    }

    /// Start the emulation.
    /// Will panic if SDL2 fails to create the application window.
    /// On exit, return false if something went unexpectedly and true otherwise.
    pub fn run(&mut self) -> bool {
        info!("creating the application window...");
        let sdl_context = sdl2::init(sdl2::INIT_VIDEO).unwrap();
        let window = match Window::new(&sdl_context,
                                      self.config.window_title,
                                      WindowPos::PosCentered,
                                      WindowPos::PosCentered,
                                      self.config.window_width as i32,
                                      self.config.window_height as i32,
                                      OPENGL) {
            Ok(window) => window,
            Err(err) => panic!("failed to create window: {}", err)
        };
        let mut renderer = match Renderer::from_window(window,
                                                       RenderDriverIndex::Auto,
                                                       ACCELERATED) {
            Ok(renderer) => renderer,
            Err(err) => panic!("failed to create renderer: {}", err)
        };
        let mut drawer = renderer.drawer();
        drawer.set_draw_color(Color::RGB(0, 0, 0));
        drawer.clear();
        drawer.present();

        let mut event_pump = sdl_context.event_pump();
        let key_binds = get_key_bindings(&self.config.keypad_binding);
        // avoid multiple redundant 'pressed' events
        // does not work with multiple keys pressed at the exact same time
        let mut last_pressed = 0xFF_usize; // invalid value by default

        info!("starting the main emulation loop.");

        'main : loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit {..}          => break 'main,
                    Event::KeyDown {keycode, ..} => {
                        // quit on Escape
                        if keycode == KeyCode::Escape {
                            break 'main;
                        }
                        // keypad emulation
                        match key_binds.get(&keycode) {
                            Some(index) => {
                                if *index != last_pressed {
                                    self.vm.keypad.pressed(*index);
                                }
                                last_pressed = *index;
                            },
                            _           => {},
                        }
                    },
                    Event::KeyUp {keycode, ..} => {
                        // keypad emulation
                        match key_binds.get(&keycode) {
                            Some(index) => self.vm.keypad.released(*index),
                            _           => {},
                        }
                    },
                    _                          => {},
                }
            }
            //self.vm.emulate_cycle();
        }

        true
    }
}
