/// Core CPU implementation.

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use rand::random;

use display::{Display, FONT_SET};
use keypad::Keypad;

/// CHIP 8 virtual machine.
/// The references used to implement this particular interpreter include :
/// http://en.wikipedia.org/wiki/CHIP-8
/// http://mattmik.com/chip8.html
/// http://devernay.free.fr/hacks/chip8/C8TECH10.HTM
pub struct Chip8 {
    /// The CPU clock frequency, i.e. the number of instructions it can execute
    /// per second.
    pub clock_hz           : u32,
    /// The current opcode.
    opcode                 : u16,
    /// The chip's 4096 bytes of memory.
    pub memory             : [u8; 4096], // TEMPORARY pub for debug purposes
    /// The chip's 16 registers, from V0 to VF.
    /// VF is used for the 'carry flag'.
    v                      : [u8; 16],
    /// Index register.
    i                      : usize,
    /// Program counter.
    pc                     : usize,
    /// The 16-levels stack.
    stack                  : [u16; 16],
    /// Stack pointer.
    sp                     : usize,
    // Timer registers, must be updated at 60 Hz by the emulator.
    pub delay_timer        : u8,
    pub sound_timer        : u8,
    /// Screen component.
    pub display            : Display,
    /// Input component.
    pub keypad             : Keypad,
    /// Is the virtual machine waiting for a keypress ?
    pub is_waiting_for_key : bool,
}

/// Macro for handling invalid/unimplemented opcodes.
/// As of now only prints a error message, could maybe panic in the future.
macro_rules! op_not_implemented {
    ($op: expr, $pc: expr) => (
        println!("Not implemented opcode {:X} at {:X}", $op as usize, $pc);
    )
}


impl Chip8 {
    /// Create and return a new, initialized Chip8 virtual machine.
    pub fn new() -> Chip8 {
        let mut chip8 = Chip8 {
            clock_hz           : 600,
            opcode             : 0u16,
            memory             : [0u8; 4096],
            v                  : [0u8; 16],
            i                  : 0usize,
            pc                 : 0usize,
            stack              : [0u16; 16],
            sp                 : 0usize,
            delay_timer        : 0u8,
            sound_timer        : 0u8,
            display            : Display::new(),
            keypad             : Keypad::new(),
            is_waiting_for_key : false,
        };
        // load the font set in memory in the space [0x0, 0x200[ = [0, 80[
        for i in 0..80 {
            chip8.memory[i] = FONT_SET[i];
        }
        // the program space starts at 0x200
        chip8.pc = 0x200;

        chip8
    }

    /// Called by the emulator application to inform the virtual machine
    /// waiting for a key pressed that a key has been pressed.
    pub fn end_wait_for_key_press(&mut self, key_pressed: usize) {
        if !self.is_waiting_for_key {
            warn!(concat!("Chip8::end_wait_for_key_press called but the VM ",
                          "wasn't waiting for a key press - ignoring"));
            return;
        }
        self.v[self.get_op_x()] = key_pressed as u8;
        self.is_waiting_for_key = false;
        self.pc += 2;
    }

    /// Load a Chip8 rom from the given filepath.
    /// If the operation fails, return a String explaining why.
    pub fn load(&mut self, filepath: &Path) -> Option<String> {
        let file = match File::open(filepath) {
            Ok(f) => f,
            Err(ref why) => {
                return Some(format!("couldn't open rom file \"{}\" : {}",
                                    filepath.display(),
                                    Error::description(why)));
            },
        };
        for (i, b) in file.bytes().enumerate() {
            //if b.is_none() /* EOF */ { break; }
            match b {
                Ok(byte) => self.memory[self.pc + i] = byte,
                Err(e) => {
                    return Some(format!("error while reading ROM : {}",
                                        e.to_string()));
                },
            }
        }
        None
    }

    /// Emulate a Chip8 CPU cycle.
    /// Return true if the loaded program is done.
    pub fn emulate_cycle(&mut self) -> bool {
        // Is the program finished ?
        if self.pc >= 4094 {
            return true;
        }
        // Fetch and decode the opcode to execute ;
        // an opcode being 2 bytes long, we need to read 2 bytes from memory
        self.opcode = (self.memory[self.pc] as u16) << 8
                      | (self.memory[self.pc + 1] as u16);

        //println!("{:0>4X} {:0>4X}", self.opcode, self.pc);

        // Execute the opcode
        let op = self.opcode.clone();
        match self.opcode & 0xF000 {
            0x0000 => {
                match self.opcode & 0x000F {
                    // 00E0 : clear the screen
                    0x0000 => self.display.clear(),
                    // 00EE : return from a subroutine
                    0x000E => {
                        self.sp -= 1;
                        self.pc = self.stack[self.sp] as usize;
                    },
                    _ => op_not_implemented!(self.opcode, self.pc),
                };
                self.pc += 2;
            },
            // 0NNN : jump the program counter to the NNN address
            0x1000 => self.jump(op & 0x0FFF),
            // 2NNN : call subroutine at NNN
            0x2000 => self.call(op & 0x0FFF),
            // 3XNN : skip the next instruction if VX equals NN
            0x3000 => {
                if self.v[self.get_op_x()] == self.get_op_nn() {
                    self.pc += 2;
                }
                self.pc += 2;
            },
            // 4XNN : skip the next instruction if VX doesn't equal NN
            0x4000 => {
                if self.v[self.get_op_x()] != self.get_op_nn() {
                    self.pc += 2;
                }
                self.pc += 2;
            },
            // 5XY0 : skip the next instruction if VX equals VY
            0x5000 => {
                self.pc += 2;
                if self.v[self.get_op_x()] == self.v[self.get_op_y()] {
                    self.pc += 2;
                }
            },
            // 6XNN : set VX to NN
            0x6000 => {
                self.v[self.get_op_x()] = self.get_op_nn();
                self.pc += 2;
            },
            // 7XNN : add NN to VX
            // wrap the value around 256 if needed
            0x7000 => {
                let vx: u16 = self.v[self.get_op_x()] as u16 +
                    self.get_op_nn() as u16; // avoid u8 overflow with u16
                self.v[self.get_op_x()] = vx as u8;
                self.pc += 2;
            },
            // 8XYZ : arithmetic operations on VX and VY
            0x8000 => self.op_8xyz(),
            // 9XY0 : skip the next instruction if VX doesn't equal VY
            0x9000 => {
                if self.v[self.get_op_x()] != self.v[self.get_op_y()] {
                    self.pc += 2;
                }
                self.pc += 2;
            }
            // ANNN : set I to the address NNN
            0xA000 => {
                self.i = (op & 0x0FFF) as usize;
                self.pc += 2;
            },
            // BNNN : jump to the adress (NNN+V0)
            0xB000 => {
                let v0 = self.v[0] as u16;
                self.jump(op & 0x0FF + v0);
            },
            // CXNN : sets VX to a random number, masked by NN
            0xC000 => {
                self.v[self.get_op_x()] = random::<u8>() & self.get_op_nn();
                self.pc += 2;
            },
            // DXYN : draw sprite
            0xD000 => { self.draw_sprite(); self.pc += 2; },
            // EX9E / EXA1 : input
            0xE000 => {
                let keypad_index = self.v[self.get_op_x()] as usize;
                match self.opcode & 0x00FF {
                    // skip the next instruction if the key at index VX is pressed
                    0x009E => {
                        if self.keypad.is_pressed(keypad_index).unwrap() {
                            self.pc += 2;
                        }
                    },
                    // skip the next instruction if the key at index VX isn't pressed
                    0x00A1 => {
                        if !self.keypad.is_pressed(keypad_index).unwrap() {
                            self.pc += 2;
                        }
                    }
                    _ => op_not_implemented!(self.opcode, self.pc),
                }
                self.pc += 2;
            },
            // FXYZ
            0xF000 => self.op_fxyz(),
            _ => op_not_implemented!(self.opcode, self.pc),
        };

        false
    }

    /// Jump to the 0x0NNN adress contained in the current opcode.
    fn jump(&mut self, address: u16) {
        self.pc = address as usize;
    }

    /// Opcode 2NNN : call the subroutine at the provided address by storing
    /// the current program counter in the current stack level and jumping to
    /// 0x0NNN.
    // TODO : handle stack overflow ?
    fn call(&mut self, address: u16) {
        self.stack[self.sp] = self.pc as u16;
        self.sp += 1;
        self.jump(address);
    }

    /// Opcode 8XYZ.
    fn op_8xyz(&mut self) {
        let x = self.get_op_x();
        let y = self.get_op_y();
        // match the YZ value
        match self.opcode & 0x000F {
            // set VX to the value of VY
            0 => self.v[x] = self.v[y],
            // sets VX to (VX or VY)
            1 => self.v[x] |= self.v[y],
            // set VX to (VX and VY)
            2 => self.v[x] &= self.v[y],
            // set VX to (VX xor VY)
            3 => self.v[x] ^= self.v[y],
            // add VY to VX
            4 => {
                let vx: u16 = self.v[x] as u16 + self.v[y] as u16;
                self.v[x] = vx as u8; // avoid overflow
                // if there is a carry set VF to 1, otherwise set it to 0
                self.v[15] = if vx > 255 { 1 } else { 0 };
            }
            // substract VY from VX
            5 => {
                let vx: i8 = self.v[x] as i8 - self.v[y] as i8;
                self.v[x] = vx as u8; // avoid underflow
                // set VF to 1 if there is a borrow, set it to 0 otherwise
                self.v[15] = if vx < 0 { 1 } else { 0 };
            }
            // Shift VX right by one. VF is set to the value of the least
            // significant bit of VX before the shift.
            6 => {
                self.v[15] = self.v[x] & 0b0001;
                self.v[x] >>= 1;
            }
            // set VX to (VY minus VX)
            7 => {
                let vx: i8 = self.v[y] as i8 - self.v[x] as i8;
                self.v[x] = vx as u8; // avoid underflow
                // VF is set to 0 when there's a borrow, and 1 otherwise
                self.v[15] = if vx < 0 { 1 } else { 0 };
            }
            // Shift VX left by one. VF is set to the value of the most
            // significant bit of VX before the shift.
            0xE => {
                self.v[15] = self.v[x] & 0b1000;
                self.v[x] <<= 1;
            }
            // unsupported Z
            _ => op_not_implemented!(self.opcode, self.pc),
        }
        self.pc += 2;
    }

    /// Opcode DXYN : draw a sprite at position VX, VY with N bytes of sprite
    /// data starting at the address stored in I.
    /// Set VF to 01 if any set pixel was cleared (collision flag), and
    /// to 00 otherwise.
    fn draw_sprite(&mut self) {
        let posx  = self.v[self.get_op_x()] as usize;
        let posy  = self.v[self.get_op_y()] as usize;
        //println!("{:?}, {:?}", posx, posy);
        let start = self.i;
        let end   = self.i + (self.opcode & 0x000F) as usize;
        if self.display.draw(posx, posy, &self.memory[start..end]) {
            self.v[15] = 0x01;
        } else {
            self.v[15] = 0x00;
        }
    }

    /// Opcode FXYZ.
    fn op_fxyz(&mut self) {
        let x = self.get_op_x();
        // match the YZ value
        match self.opcode & 0x00FF {
            // FX07 set VX to the value of the delay timer
            0x07 => self.v[x] = self.delay_timer,
            // FX0A : wait for a key press and store its index in VX
            0x0A => {
                // implementation : the emulator app must call the
                // 'end_wait_for_key_press' function
                // this is needed to achieve better independance from the
                // framerate
                self.is_waiting_for_key = true;
                self.pc -= 2;
            },
            // FX15 : set the delay timer to VX
            0x15 => self.delay_timer = self.v[x],
            // FX18 : set the sound timer to VX
            0x18 => self.sound_timer = self.v[x],
            // FX29 : set I to the location of the sprite for the character
            // in VX. The characters are thus 0-F.
            0x29 => {
                // the font set is in the memory range [0..80]
                // and each character is represented by 5 bytes
                self.i = (self.v[x] * 5) as usize;
            },
            // FX33 : store the binary-coded decimal equivalent of the value
            // stored in VX at the addresses I, I+1, and I+2 since VX is a
            // byte and hence can be a decimal up to 255.
            0x33 => {
                let vx = self.v[x];
                self.memory[self.i]   = vx / 100;
                self.memory[self.i+1] = (vx / 10)  % 10;
                self.memory[self.i+2] = (vx % 100) % 10;
            }
            // FX55 : store V0 to VX in memory starting at the address I
            0x55 => {
                for j in 0..x {
                    self.memory[self.i + j] = self.v[j];
                }
            },
            // FX65 : fill V0 to VX with values from memory starting at the
            // address I
            0x65 => {
                for j in 0..x {
                    self.v[j] = self.memory[self.i + j];
                }
            }
            _ => op_not_implemented!(self.opcode, self.pc),
        }
        self.pc += 2;
    }

    /// Get the X value in the current opcode of the form 0x-X--.
    fn get_op_x(&self) -> usize {
        ((self.opcode & 0x0F00) >> 8) as usize
    }

    /// Get the Y value in the current opcode of the form 0x--Y-.
    fn get_op_y(&self) -> usize {
        ((self.opcode & 0x00F0) >> 4) as usize
    }

    /// Get the NN value in the current opcode of the form 0x--NN.
    fn get_op_nn(&self) -> u8 {
        (self.opcode & 0x00FF) as u8
    }
}
