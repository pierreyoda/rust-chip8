use std::sync::mpsc::{Receiver, Sender};

extern crate sdl2;
use self::sdl2::event::Event;
use self::sdl2::keyboard::Keycode;
use self::sdl2::pixels::{Color, PixelFormatEnum};
use self::sdl2::rect::Rect;
use self::sdl2::render::{Texture, TextureCreator, WindowCanvas};
use self::sdl2::video::WindowContext;

extern crate chip8vm;
use self::chip8vm::display::{Display, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use self::chip8vm::keypad::Keystate::{Pressed, Released};
use super::chip8app::Chip8UICommand::*;
use super::chip8app::Chip8VMCommand::*;
use super::chip8app::{
    get_display_size, Chip8Config, Chip8EmulatorBackend, Chip8UICommand, Chip8VMCommand,
};
use super::input;

// todo : make this a backend-agnostic option
const COLOR_PIXEL_OFF: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 0xFF,
};
const COLOR_PIXEL_ON: Color = Color {
    r: 0xFF,
    g: 0xFF,
    b: 0xFF,
    a: 0xFF,
};

/// The SDL2 backend for the Chip8 emulator.
pub struct Chip8BackendSDL2;

impl Chip8BackendSDL2 {
    fn render_display<'c>(
        t: &'c TextureCreator<WindowContext>,
        c: &mut WindowCanvas,
        display: Display,
        scale: u32,
    ) -> Texture<'c> {
        let display_width = DISPLAY_WIDTH as u32;
        let display_height = DISPLAY_HEIGHT as u32;
        let pixel_size = scale as i32;

        let mut texture = t
            .create_texture_target(
                PixelFormatEnum::RGB24,
                display_width * scale,
                display_height * scale,
            )
            .unwrap();
        c.with_texture_canvas(&mut texture, |texture_canvas| {
            texture_canvas.set_draw_color(COLOR_PIXEL_OFF);
            texture_canvas.clear();
            texture_canvas.set_draw_color(COLOR_PIXEL_ON);
            for y in 0i32..(display_height as i32) {
                for x in 0i32..(display_width as i32) {
                    // TODO : precompute the used Rect ?
                    // since they only change at window resize...
                    if display.gfx[y as usize][x as usize] == 1u8 {
                        let _ = texture_canvas.fill_rect(Rect::new(
                            x * pixel_size,
                            y * pixel_size,
                            scale,
                            scale,
                        ));
                    }
                }
            }
        });
        texture
    }
}

impl Chip8EmulatorBackend for Chip8BackendSDL2 {
    /// Initialize and run the emulation.
    /// Will panic if SDL2 fails to create the application window.
    fn exec(
        &mut self,
        config: &Chip8Config,
        tx: Sender<Chip8VMCommand>,
        rx: Receiver<Chip8UICommand>,
    ) {
        info!("starting the main application / rendering thread");

        // window dimensions
        let (scale, width, height) = get_display_size(config.window_width, config.window_height);
        info!("chosen scale : {} pixels per CHIP 8 pixel", scale);

        // window creation and rendering setup
        info!("creating the application window...");
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();
        let mut timer_subsystem = sdl_context.timer().unwrap();
        let window = video_subsystem
            .window(config.window_title, width as u32, height as u32)
            .position_centered()
            .opengl()
            .build()
            .unwrap();
        let mut canvas = window.into_canvas().accelerated().build().unwrap();
        let texture_creator = canvas.texture_creator();

        let pixel_size = scale as u32;
        let display_width = DISPLAY_WIDTH as u32;
        let display_height = DISPLAY_HEIGHT as u32;
        canvas.set_draw_color(COLOR_PIXEL_OFF);
        canvas.clear();
        canvas.present();

        let mut event_pump = sdl_context.event_pump().unwrap();
        let key_binds = input::get_sdl_key_bindings(&config.keypad_binding);
        // avoid spamming the channel with redundant 'pressed' events
        let mut keys_pressed = Vec::new();

        // Framerate handling
        // inspired from the excellent article :
        // http://gafferongames.com/game-physics/fix-your-timestep/
        let fps = 60.0; // target emulator updates per second
        let mut t = timer_subsystem.ticks(); // internal clock, in ms
        let mut t_prev; // time at the previous frame
        let mut dt; // frametime, in ms
        let mut update_timer = 0.0;
        let max_dt = 1000.0 / fps; // target max frametime, in fps

        // Emulation state
        let mut paused = false;

        'main: loop {
            // Frame time
            t_prev = t;
            t = timer_subsystem.ticks();
            dt = t - t_prev;
            // SDL event handling
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => {
                        paused = true;
                        tx.send(Quit).unwrap();
                    }
                    Event::KeyDown { keycode, .. } => {
                        if keys_pressed.contains(&keycode) {
                            continue;
                        }
                        match keycode.unwrap() {
                            // quit on Escape
                            Keycode::Escape => {
                                paused = true;
                                tx.send(Quit).unwrap();
                            }
                            // toggle pause on Return
                            Keycode::Return => {
                                tx.send(UpdateRunStatus(paused)).unwrap();
                                paused = !paused;
                            }
                            // reset on backspace
                            Keycode::Backspace => {
                                info!("Reinitializing the virtual machine.");
                                tx.send(Reset).unwrap();
                            }
                            _ => {
                                if !paused {
                                    if let Some(index) = key_binds.get(&keycode.unwrap()) {
                                        tx.send(UpdateKeyStatus(*index, Pressed)).unwrap();
                                    }
                                }
                            }
                        }
                        keys_pressed.push(keycode);
                    }
                    Event::KeyUp { keycode, .. } => {
                        for i in 0..keys_pressed.len() {
                            if keys_pressed[i] == keycode {
                                keys_pressed.remove(i);
                                break;
                            }
                        }
                        if let Some(index) = key_binds.get(&keycode.unwrap()) {
                            tx.send(UpdateKeyStatus(*index, Released)).unwrap();
                        }
                    }
                    _ => continue,
                }
            }

            // Command from the VM
            match rx.try_recv() {
                // non-blocking receiving function
                Ok(ui_command) => {
                    match ui_command {
                        UpdateBeepingStatus(beeping) => {
                            // TODO
                            if beeping {
                                println!("BEEP !");
                            }
                        }
                        UpdateDisplay(display) => {
                            let texture = Chip8BackendSDL2::render_display(
                                &texture_creator,
                                &mut canvas,
                                display,
                                scale as u32,
                            );
                            canvas
                                .copy(
                                    &texture,
                                    None,
                                    Some(Rect::new(
                                        0,
                                        0,
                                        display_width * pixel_size,
                                        display_height * pixel_size,
                                    )),
                                )
                                .unwrap();
                        }
                        Finished => break 'main,
                    }
                }
                _ => {}
            }

            // Always render at 60 FPS (allows framerate displayers to work)
            while update_timer >= max_dt {
                update_timer -= max_dt;
                canvas.present(); // switch the buffers
            }
            update_timer += dt as f32;
        }

        info!("terminating the main application thread")
    }
}
