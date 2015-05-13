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
use super::chip8app::{Chip8Emulator, Chip8Config, get_display_size};
use super::input;

/// The SDL2 backend for the Chip8 emulator.
/// Uses SDL2 and the chip8vm (internal) library to run CHIP 8 ROMs.
pub struct Chip8ApplicationSDL2 {
    /// The instance of the virtual machine simulating the Chip 8's components
    /// (CPU, display, input and sound).
    vm     : Chip8,
    /// The 'Chip8Config' instance holding the application's configuration.
    config : Chip8Config,
}

impl Chip8ApplicationSDL2 {
    /// Create and return a new Chip8ApplicationSDL2, with the given
    /// 'Chip8Config'.
    pub fn new(config: Chip8Config) -> Chip8ApplicationSDL2 {
        Chip8ApplicationSDL2 {
            vm: Chip8::new(),
            config: config
        }
    }
}

impl Chip8Emulator for Chip8ApplicationSDL2 {

    fn load_rom(&mut self, filepath: &Path) -> bool {
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

    /// Initialize and run the emulation.
    /// Will panic if SDL2 fails to create the application window.
    fn run(&mut self) {
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

        let max_cycles_per_sec = self.config.vm_cpu_clock;
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
                                if !self.vm.is_waiting_for_key() {
                                    if *index != last_pressed {
                                        self.vm.keypad.pressed(*index);
                                    }
                                    last_pressed = *index;
                                } else {
                                    self.vm.end_wait_for_key(*index);
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
            if !self.vm.is_waiting_for_key() && cycles < max_cycles_per_sec {
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
    }
}
