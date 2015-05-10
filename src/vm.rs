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
    /// The current opcode.
    opcode      : u16,
    /// The chip's 4096 bytes of memory.
    memory      : [u8; 4096],
    /// The chip's 16 registers, from V0 to VF.
    /// VF is used for the 'carry flag'.
    v           : [u8; 16],
    /// Index register.
    i           : usize,
    /// Program counter.
    pc          : usize,
    /// The 16-levels stack.
    stack       : [u16; 16],
    /// Stack pointer.
    sp          : usize,
    // Timer registers, must be updated at 60 Hz by the emulator.
    pub delay_timer : u8,
    pub sound_timer : u8,
    /// Screen component.
    pub display : Display,
    /// Input component.
    pub keypad  : Keypad,
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
            opcode: 0u16,
            memory: [0u8; 4096],
            v: [0u8; 16],
            i: 0usize,
            pc: 0usize,
            stack: [0u16; 16],
            sp: 0usize,
            delay_timer: 0u8,
            sound_timer: 0u8,
            display: Display::new(),
            keypad: Keypad::new(),
        };
        // load the font set in memory in the space [0x0, 0x200[ = [0, 80[
        for i in 0..80 {
            chip8.memory[i] = FONT_SET[i];
        }
        // the program space starts at 0x200
        chip8.pc = 0x200;

        chip8
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
    pub fn emulate_cycle(&mut self) {
        // Fetch and decode the opcode to execute ;
        // an opcode being 2 bytes long, we need to read 2 bytes from memory
        self.opcode = (self.memory[self.pc] as u16) << 8
                      | (self.memory[self.pc + 1] as u16);

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
            0x7000 => {
                self.v[self.get_op_x()] += self.get_op_nn();
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
            // EX9E : input
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
                self.v[x] += self.v[y];
                // if there is a carry set VF to 1, otherwise set it to 0
                self.v[15] = if self.v[x] < self.v[y] { 1 } else { 0 };
            }
            // substract VY from VX
            5 => {
                // set VF to 1 if there is a borrow, set it to 0 otherwise
                self.v[15] = if self.v[x] < self.v[y] { 1 } else { 0 };
                self.v[x] -= self.v[y];
            }
            // Shift VX right by one. VF is set to the value of the least
            // significant bit of VX before the shift.
            6 => {
                self.v[15] = self.v[x] & 0b0001;
                self.v[x] >>= 1;
            }
            // set VX to (VY minus VX)
            7 => {
                // VF is set to 0 when there's a borrow, and 1 otherwise
                self.v[15] = if self.v[y] < self.v[x] { 1 } else { 0 };
                self.v[x] = self.v[y] - self.v[x];
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

    /// Opcode FXYZ.
    fn op_fxyz(&mut self) {
        let x = self.get_op_x();
        // match the YZ value
        match self.opcode & 0x00FF {
            // set VX to the value of the delay timer
            0x07 => self.v[x] = self.delay_timer,
            // wait for a key press and store its index in VX
            0x0A => {
                // implementation : block the CPU on this opcode
                // until any key is pressed
                for key in 0..16 {
                    if self.keypad.is_pressed(key).unwrap() {
                        self.pc += 2;
                        self.v[x] = key as u8;
                        break;
                    }
                }
                self.pc -= 2;
            },
            // set the delay timer to VX
            0x15 => self.delay_timer = self.v[x],
            // set the sound timer to VX
            0x18 => self.sound_timer = self.v[x],
            // FX29
            _ => op_not_implemented!(self.opcode, self.pc),
        }
        self.pc += 2;
    }

    /// Get the X value in the current opcode of the form 0x-X--.
    fn get_op_x(&self) -> usize {
        (self.opcode & 0x0F00 >> 8) as usize
    }

    /// Get the Y value in the current opcode of the form 0x--Y-.
    fn get_op_y(&self) -> usize {
        (self.opcode & 0x00F0 >> 4) as usize
    }

    /// Get the NN value in the current opcode of the form 0x--NN.
    fn get_op_nn(&self) -> u8 {
        (self.opcode & 0x00FF) as u8
    }
}
