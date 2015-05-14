use std::path::Path;
use std::sync::mpsc::{Sender, Receiver};

extern crate sdl2;
use self::sdl2::video::{Window, OPENGL, WindowPos};
use self::sdl2::render::{RenderDriverIndex, ACCELERATED, Renderer, RenderDrawer};
use self::sdl2::event::Event;
use self::sdl2::rect::Rect;
use self::sdl2::pixels::Color;
use self::sdl2::keycode::KeyCode;

extern crate chip8vm;
use self::chip8vm::display::{Display, DISPLAY_WIDTH, DISPLAY_HEIGHT};
use self::chip8vm::keypad::Keystate::{Released, Pressed};
use super::chip8app::{Chip8EmulatorBackend, Chip8Config, Chip8VMCommand,
    Chip8UICommand, get_display_size};
use super::chip8app::Chip8VMCommand::*;
use super::chip8app::Chip8UICommand::*;
use super::input;

// todo : make this a backend-agnostic option
const COLOR_PIXEL_OFF: Color = Color::RGB(0, 0, 0);
const COLOR_PIXEL_ON: Color  = Color::RGB(255, 255, 255);

/// The SDL2 backend for the Chip8 emulator.
pub struct Chip8BackendSDL2;

impl Chip8BackendSDL2 {
    fn render_display(drawer: &mut RenderDrawer, display: Display,
                      pixel_size: i32) {
        let display_width = DISPLAY_WIDTH as i32;
        let display_height = DISPLAY_HEIGHT as i32;

        // TODO : render to a cache texture ?
        drawer.set_draw_color(COLOR_PIXEL_OFF);
        drawer.clear();
        drawer.set_draw_color(COLOR_PIXEL_ON);
        for y in 0i32..display_height {
            for x in 0i32..display_width {
                // TODO : precompute the used Rect ?
                // since they only change at window resize...
                if display.gfx[y as usize][x as usize] == 1u8 {
                    let _ = drawer.fill_rect(Rect::new(
                                                 x * pixel_size,
                                                 y * pixel_size,
                                                 pixel_size,
                                                 pixel_size));
                }
            }
        }
    }
}

impl Chip8EmulatorBackend for Chip8BackendSDL2 {
    /// Initialize and run the emulation.
    /// Will panic if SDL2 fails to create the application window.
    fn exec(&mut self, config: &Chip8Config,
            tx: Sender<Chip8VMCommand>, rx: Receiver<Chip8UICommand>) {
        info!("starting the main application / rendering thread");

        // window dimensions
        let (scale, width, height) = get_display_size(
            config.window_width, config.window_height);
        info!("chosen scale : {} pixels per CHIP 8 pixel", scale);

        // window creation and rendering setup
        info!("creating the application window...");
        let sdl_context = sdl2::init(sdl2::INIT_VIDEO).unwrap();
        let window = match Window::new(&sdl_context,
                                      config.window_title,
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
        drawer.set_draw_color(COLOR_PIXEL_OFF);
        drawer.clear();
        drawer.present();

        let mut event_pump = sdl_context.event_pump();
        let key_binds = input::get_sdl_key_bindings(&config.keypad_binding);
        // avoid spamming the channel with redundant 'pressed' events
        // does not work with multiple keys pressed at the exact same time
        let mut last_key_pressed = 0xFF_usize; // invalid value by default

        // Framerate handling
        // inspired from the excellent article :
        // http://gafferongames.com/game-physics/fix-your-timestep/
        let fps = 60.0; // target emulator updates per second
        let mut t = sdl2::get_ticks(); // internal clock, in ms
        let mut t_prev; // time at the previous frame
        let mut dt; // frametime, in ms
        let mut update_timer = 0.0;
        let max_dt = 1000.0 / fps; // target max frametime, in fps

        // Emulation state
        let mut paused = false;

        'main: loop {
            // Frame time
            t_prev = t;
            t = sdl2::get_ticks();
            dt = t - t_prev;
            // SDL event handling
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit {..}             => {
                        paused = true;
                        tx.send(Quit).unwrap();
                    },
                    Event::KeyDown {keycode, ..} => match keycode {
                        // quit on Escape
                        KeyCode::Escape => {
                            paused = true;
                            tx.send(Quit).unwrap();
                        },
                        // toggle pause on Return
                        KeyCode::Return => {
                            tx.send(UpdateRunStatus(paused)).unwrap();
                            paused = !paused;
                        },
                        // reset on backspace
                        KeyCode::Backspace => {
                            info!("Reinitializing the virtual machine.");
                            tx.send(Reset).unwrap();
                        },
                        _ => if !paused {
                            match key_binds.get(&keycode) {
                                Some(index) => {
                                    if *index != last_key_pressed {
                                        tx.send(UpdateKeyStatus(*index, Pressed))
                                            .unwrap();
                                        last_key_pressed = *index;
                                    }
                                },
                                _           => {},
                            }
                            last_key_pressed = 0xFF_usize;
                        },
                    },
                    Event::KeyUp {keycode, ..}   => {
                        match key_binds.get(&keycode) {
                            Some(index) => {
                                tx.send(UpdateKeyStatus(*index, Released)).unwrap();
                            },
                            _           => {},
                        }
                    },
                    _                          => {},
                }
            }

            // Command from the VM
            match rx.try_recv() { // non-blocking receiving function
                Ok(ui_command) => match ui_command {
                    UpdateBeepingStatus(beeping) => {
                        // TODO
                        if beeping { println!("BEEP !"); }
                    },
                    UpdateDisplay(display) => Chip8BackendSDL2::render_display(
                        &mut drawer, display, pixel_size),
                    Finished => break 'main,
                },
                _              => {},
            }

            // Always render at 60 FPS (allows framerate displayers to work)
            while update_timer >= max_dt {
                update_timer -= max_dt;
                drawer.present(); // switch the buffers
            }
            update_timer += dt as f32;
        }

        info!("terminating the main application thread")
    }
}
