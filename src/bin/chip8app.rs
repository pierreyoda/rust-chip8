use std::cmp;
use std::path::Path;

extern crate sdl2;
use self::sdl2::video::{Window, OPENGL, WindowPos};
use self::sdl2::render::{RenderDriverIndex, ACCELERATED, Renderer};
use self::sdl2::event::Event;
use self::sdl2::rect::Rect;
use self::sdl2::pixels::Color;
use self::sdl2::keycode::KeyCode;

extern crate chip8vm;
use self::chip8vm::vm::Chip8;
use self::chip8vm::display::{DISPLAY_WIDTH, DISPLAY_HEIGHT};
use super::input;


/// Structure facilitating the configuration of a 'Chip8Application'.
/// The configuration functions (e.g. 'w_title') work with moved 'self' values
/// to allow chaining them inside the Chip8Application::new function call.
pub struct Chip8Config {
    /// The title of the emulator window.
    window_title   : &'static str,
    /// The desired width for the emulator window.
    /// NB : this is just a hint, the application may resize to reach a proper
    /// aspect ratio.
    window_width   : u16,
    /// The desired height for the emulator window.
    /// NB : this is just a hint, the application may resize to reach a proper
    /// aspect ratio.
    window_height  : u16,
    /// The keyboard configuration. QWERTY by default.
    keypad_binding : input::KeyboardBinding,
}

impl Chip8Config {
    /// Create and return the default set of options.
    pub fn new() -> Chip8Config {
        Chip8Config {
            window_title  : "",
            window_width  : 64,
            window_height : 32,
            keypad_binding: input::KeyboardBinding::QWERTY
        }
    }

    /// Set the window title.
    pub fn w_title(mut self, title: &'static str) -> Chip8Config {
        self.window_title = title; self
    }

    /// Set the window width.
    pub fn w_width(mut self, width: u16) -> Chip8Config {
        self.window_width = width; self
    }

    /// Set the window height.
    pub fn w_height(mut self, height: u16) -> Chip8Config {
        self.window_height = height; self
    }

    /// Set the keyboard configuration.
    pub fn key_binds(mut self, keyboard: input::KeyboardBinding) -> Chip8Config {
        self.keypad_binding = keyboard; self
    }
}


/// Chip8Application uses SDL2 and the chip8vm (internal) library to run CHIP 8
/// ROMs.
/// TODO : explore multi-threading options (rendering and emulation in different
/// threads ?).
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
        // window dimensions
        let (scale, width, height) = get_display_size(
            self.config.window_width, self.config.window_height);
        info!("chosen scale : {} pixels per CHIP 8 pixel", scale);

        // window creation and rendering setup
        info!("creating the application window...");
        let sdl_context = sdl2::init(sdl2::INIT_VIDEO).unwrap();
        let window = match Window::new(&sdl_context,
                                      self.config.window_title,
                                      WindowPos::PosCentered,
                                      WindowPos::PosCentered,
                                      width as i32,
                                      height as i32,
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
        let mut drawer      = renderer.drawer();
        let pixel_size      = scale as i32;
        let display_width   = DISPLAY_WIDTH as i32;
        let display_height  = DISPLAY_HEIGHT as i32;
        let color_pixel_off = Color::RGB(0, 0, 0);
        let color_pixel_on  = Color::RGB(255, 255, 255);
        drawer.set_draw_color(color_pixel_off);
        drawer.clear();
        drawer.present();

        let mut event_pump = sdl_context.event_pump();
        let key_binds = input::get_sdl_key_bindings(&self.config.keypad_binding);
        // avoid multiple redundant 'pressed' events
        // does not work with multiple keys pressed at the exact same time
        let mut last_pressed = 0xFF_usize; // invalid value by default

        info!("starting the main emulation loop.");

        // Framerate handling
        // inspired from the excellent article :
        // http://gafferongames.com/game-physics/fix-your-timestep/
        let fps = 60.0; // target emulator updates per second
        let mut t = sdl2::get_ticks(); // internal clock, in ms
        let mut t_prev; // time at the previous frame
        let mut dt; // frametime, in ms
        let mut update_timer = 0.0;
        let max_dt = 1000.0 / fps; // target max frametime, in fps

        let max_cycles_per_sec = self.vm.clock_hz;
        let mut cycles = 0; // number of CPU cycles done in the current second
        let mut cycles_t = t;

        // TEST
        if false {
            let test_prog = [0xF00A, 0xF029, 0xD795, 0x1200];
            self.vm = Chip8::new();
            for (i, b) in test_prog.iter().enumerate() {
                self.vm.memory[0x200+i*2] = (*b >> 8) as u8;
                self.vm.memory[0x200+i*2+1] = (*b & 0x00FF) as u8;
            }
        }

        'main : loop {
            // Frame time
            t_prev = t;
            t = sdl2::get_ticks();
            dt = t - t_prev;
            // Event handling
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit {..}             => break 'main,
                    Event::KeyDown {keycode, ..} => {
                        // quit on Escape
                        if keycode == KeyCode::Escape {
                            break 'main;
                        }
                        // keypad emulation
                        match key_binds.get(&keycode) {
                            Some(index) => {
                                if !self.vm.is_waiting_for_key {
                                    if *index != last_pressed {
                                        self.vm.keypad.pressed(*index);
                                    }
                                    last_pressed = *index;
                                } else {
                                    self.vm.end_wait_for_key_press(*index);
                                }
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

            // Chip8 CPU cycles
            if t - cycles_t > 1000 {
                println!("ran {} cycles during the last second", cycles);
                cycles_t = t;
                cycles = 0;
            }
            if !self.vm.is_waiting_for_key && cycles < max_cycles_per_sec {
                cycles += 1;
                if self.vm.emulate_cycle() {
                    info!("The program ended properly.");
                    break 'main;
                }
            }

            // Emulator update : manage the Chip8 timers and render its display
            while update_timer >= max_dt {
                // timer updates
                if self.vm.delay_timer > 0 {
                    //println!("{:?}", self.vm.delay_timer); // debug
                    self.vm.delay_timer -= 1;
                }
                if self.vm.sound_timer > 0 {
                    self.vm.sound_timer -= 1;
                    // play a sound : TODO
                    if self.vm.sound_timer == 0 {
                        println!("BEEP !");
                    }
                }
                update_timer -= max_dt;
                // draw if needed
                if self.vm.display.dirty {
                    drawer.set_draw_color(color_pixel_off);
                    drawer.clear();
                    drawer.set_draw_color(color_pixel_on);
                    for y in 0i32..display_height {
                        for x in 0i32..display_width {
                            // TODO : precompute the used Rect ?
                            // since they only change at window resize...
                            if self.vm.display.gfx[y as usize][x as usize] == 1u8 {
                                let _ = drawer.fill_rect(Rect::new(
                                                             x * pixel_size,
                                                             y * pixel_size,
                                                             pixel_size,
                                                             pixel_size));
                            }
                        }
                    }
                    self.vm.display.dirty = false;
                }
                drawer.present();
            }
            update_timer += dt as f32;

            sdl2::timer::delay(5);
        }

        true
    }
}


/// Return the best (pixel_scale, width, height) combination with the given
fn get_display_size(w_width: u16, w_height: u16) -> (u16, u16, u16) {
    let scale_w = w_width / DISPLAY_WIDTH;
    let scale_h = w_height / DISPLAY_HEIGHT;
    let scale = cmp::min(scale_w, scale_h);

    // adjust to the smallest scale and recompute the window dimensions
    (scale, scale * DISPLAY_WIDTH, scale * DISPLAY_HEIGHT)
}
