/// Core CPU implementation.

use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use rand::random;

use display::{Display, FONT_SET};
use keypad::{Keypad, Keystate};


/// The default CPU clock, in Hz.
pub const CPU_CLOCK       : u32   = 600;
/// The timers clock, in Hz.
pub const TIMERS_CLOCK    : u32   = 60;

/// The index of the register used for the 'carry flag'.
/// VF is used according to the CHIP 8 specifications.
pub const FLAG            : usize = 15;
/// The size of the stack.
const STACK_SIZE          : usize = 16;

/// CHIP 8 virtual machine.
/// The references used to implement this particular interpreter include :
/// http://en.wikipedia.org/wiki/CHIP-8
/// http://mattmik.com/chip8.html
/// http://devernay.free.fr/hacks/chip8/C8TECH10.HTM
pub struct Chip8 {
    /// The current opcode.
    opcode              : u16,
    /// The chip's 4096 bytes of memory.
    pub memory          : [u8; 4096], // TEMPORARY pub for debug purposes
    /// The chip's 16 registers, from V0 to VF.
    /// VF is used for the 'carry flag'.
    pub v               : [u8; 16],
    /// Index register.
    pub i               : usize,
    /// Program counter.
    pub pc              : usize,
    /// The stack, used for subroutine operations.
    /// By default has 16 levels of nesting.
    pub stack           : [u16; STACK_SIZE],
    /// Stack pointer.
    pub sp              : usize,
    // Timer registers, must be updated at 60 Hz by the emulator.
    pub delay_timer     : u8,
    pub sound_timer     : u8,
    /// Screen component.
    pub display         : Display,
    /// Input component.
    pub keypad          : Keypad,
    /// Is the virtual machine waiting for a keypress ?
    /// If so, when any key is pressed store its index in VX where X is
    /// the value stored in this tuple.
    pub wait_for_key    : (bool, u8),
    /// Implementation option.
    /// Should the shifting opcodes 8XY6 and 8XYE use the original implementation,
    /// i.e. set VX to VY shifted respectively right and left by one bit ?
    /// If false, the VM will instead consider as many ROMs seem to do that Y=X.
    /// See http://mattmik.com/chip8.html for more detail.
    shift_op_use_vy : bool,
}

/// Macro for handling invalid/unimplemented opcodes.
/// As of now only prints a error message, could maybe panic in the future.
macro_rules! op_not_implemented {
    ($op: expr, $pc: expr) => (
        println!("Not implemented opcode {:0>4X} at {:0>5X}",
                 $op as usize,
                 $pc);
    )
}


impl Chip8 {
    /// Create and return a new, initialized Chip8 virtual machine.
    pub fn new() -> Chip8 {
        let mut chip8 = Chip8 {
            opcode          : 0u16,
            memory          : [0u8; 4096],
            v               : [0u8; 16],
            i               : 0usize,
            pc              : 0usize,
            stack           : [0u16; STACK_SIZE],
            sp              : 0usize,
            delay_timer     : 0u8,
            sound_timer     : 0u8,
            display         : Display::new(),
            keypad          : Keypad::new(),
            wait_for_key    : (false, 0x0),
            shift_op_use_vy : false,
        };
        // load the font set in memory in the space [0x0, 0x200[ = [0, 80[
        for i in 0..80 {
            chip8.memory[i] = FONT_SET[i];
        }
        // the program space starts at 0x200
        chip8.pc = 0x200;

        chip8
    }

    /// Reinitialize the virtual machine's state but keep the loaded program
    /// in memory.
    pub fn reset(&mut self) {
        self.opcode       = 0u16;
        self.v            = [0u8; 16];
        self.i            = 0usize;
        self.pc           = 0x200;
        self.stack        = [0u16; STACK_SIZE];
        self.sp           = 0usize;
        self.delay_timer  = 0u8;
        self.sound_timer  = 0u8;
        self.display      = Display::new();
        self.keypad       = Keypad::new();
        self.wait_for_key = (false, 0x0);
    }

    /// Set the shift_op_use_vy flag.
    pub fn should_shift_op_use_vy(&mut self, b: bool) {
        self.shift_op_use_vy = b;
    }

    /// Is the CPU waiting for a key press ?
    pub fn is_waiting_for_key(&self) -> bool {
        self.wait_for_key.0
    }

    /// Called by the emulator application to inform the virtual machine
    /// waiting for a key pressed that a key has been pressed.
    pub fn end_wait_for_key(&mut self, key_index: usize) {
        if !self.is_waiting_for_key() {
            warn!(concat!("Chip8::end_wait_for_key_press called but the VM ",
                          "wasn't waiting for a key press - ignoring"));
            return;
        }
        self.v[self.wait_for_key.1 as usize] = key_index as u8;
        self.wait_for_key.0 = false;
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
        // Fetch and execute the opcode to execute ;
        // an opcode being 2 bytes long, we need to read 2 bytes from memory
        let op = (self.memory[self.pc] as u16) << 8
                 | (self.memory[self.pc + 1] as u16);

        // println!("{:0>4X} {:0>4X}", self.opcode, self.pc); // DEBUG
        self.opcode = op;
        self.execute_opcode(op);
        false
    }

    /// Execute a single opcode.
    pub fn execute_opcode(&mut self, op: u16) {
        // For easier matching, get the values (nibbles) A, B, C, D
        // if the opcode is 0xABCD.
        let opcode_tuple = (
            ((op & 0xF000) >> 12) as u8,
            ((op & 0x0F00) >> 8)  as u8,
            ((op & 0x00F0) >> 4)  as u8,
            (op & 0x000F)         as u8);

        //println!("{:0>4X}/{:X},{:X},{:X},{:X}", self.opcode, a, b, c, d);

        // Opcode decoding
        match opcode_tuple
        {
            (0x0, 0x0, 0xE, 0x0) => self.cls(),
            (0x0, 0x0, 0xE, 0xE) => self.ret(),
            // 0NNN = sys addr : ignore
            (0x1, _, _, _)       => self.jump_addr(op & 0x0FFF),
            (0x2, _, _, _)       => self.call_addr(op & 0x0FFF),
            (0x3, x, _, _)       => self.se_vx_nn(x, (op & 0x00FF) as u8),
            (0x4, x, _, _)       => self.sne_vx_nn(x, (op & 0x00FF) as u8),
            (0x5, x, y, 0x0)     => self.se_vx_vy(x, y),
            (0x6, x, _, _)       => self.ld_vx_nn(x, (op & 0x00FF) as u8),
            (0x7, x, _, _)       => self.add_vx_nn(x, (op & 0x00FF) as u8),
            (0x8, x, y, 0x0)     => self.ld_vx_vy(x, y),
            (0x8, x, y, 0x1)     => self.or_vx_vy(x, y),
            (0x8, x, y, 0x2)     => self.and_vx_vy(x, y),
            (0x8, x, y, 0x3)     => self.xor_vx_vy(x, y),
            (0x8, x, y, 0x4)     => self.add_vx_vy(x, y),
            (0x8, x, y, 0x5)     => self.sub_vx_vy(x, y),
            (0x8, x, y, 0x6)     => self.shr_vx_vy(x, y),
            (0x8, x, y, 0x7)     => self.subn_vx_vy(x, y),
            (0x8, x, y, 0xE)     => self.shl_vx_vy(x, y),
            (0x9, x, y, 0x0)     => self.sne_vx_vy(x, y),
            (0xA, _, _, _)       => self.ld_i_addr(op & 0x0FFF),
            (0xB, _, _, _)       => {
                let v0 = self.v[0] as u16; // sacrifice to the god of borrows
                self.jump_addr(op & 0x0FFF + v0);
            },
            (0xC, x, _, _)       => self.rnd_vx_nn(x, (op & 0x00FF) as u8),
            (0xD, x, y, n)       => self.drw_vx_vy_n(x, y, n),
            (0xE, x, 0x9, 0xE)   => self.skp_vx(x),
            (0xE, x, 0xA, 0x1)   => self.sknp_vx(x),
            (0xF, x, 0x0, 0x7)   => self.ld_vx_dt(x),
            (0xF, x, 0x0, 0xA)   => self.ld_vx_key(x),
            (0xF, x, 0x1, 0x5)   => self.ld_dt_vx(x),
            (0xF, x, 0x1, 0x8)   => self.ld_st_vx(x),
            (0xF, x, 0x1, 0xE)   => self.add_i_vx(x),
            (0xF, x, 0x2, 0x9)   => self.ld_i_font_vx(x),
            (0xF, x, 0x3, 0x3)   => self.ld_mem_i_bcd_vx(x),
            (0xF, x, 0x5, 0x5)   => self.ld_mem_i_regs(x),
            (0xF, x, 0x6, 0x5)   => self.ld_regs_mem_i(x),
            _ => op_not_implemented!(op, self.pc),
        }
    }

    /// Clear the screen.
    fn cls(&mut self) {
        self.display.clear();
        self.pc += 2;
    }

    /// Return from a subroutine, by setting the program counter to the address
    /// popped from the stack.
    fn ret(&mut self) {
        self.sp -= 1;
        let addr = self.stack[self.sp];
        self.jump_addr(addr);
        self.pc += 2;
    }

    /// Jump to the given address of the form 0x0NNN.
    fn jump_addr(&mut self, addr: u16) {
        self.pc = addr as usize;
    }

    /// Execute the subroutine at the provided address pushing the current
    /// program counter to the stack and jumping to the given address of the
    /// form 0x0NNN.
    /// TODO : handle stack overflow error ?
    fn call_addr(&mut self, addr: u16) {
        self.stack[self.sp] = self.pc as u16;
        self.sp += 1;
        self.jump_addr(addr);
    }

    /// Skip the next instruction if the value of register VX is equal to 0xNN.
    fn se_vx_nn(&mut self, x: u8, nn: u8) {
        self.pc += if self.v[x as usize] == nn { 4 } else { 2 };
    }

    /// Skip the next instruction if the value of register VX isn't equal to
    /// 0xNN.
    fn sne_vx_nn(&mut self, x: u8, nn: u8) {
        self.pc += if self.v[x as usize] != nn { 4 } else { 2 };
    }

    /// Skip the next instruction if the value of register VX is equal to the
    /// value of register VY.
    fn se_vx_vy(&mut self, x: u8, y: u8) {
        self.pc += if self.v[x as usize] == self.v[y as usize] { 4 } else { 2 };
    }

    /// Skip the next instruction if the value of register VX is not equal to
    /// the value of register VY.
    fn sne_vx_vy(&mut self, x: u8, y: u8) {
        self.pc += if self.v[x as usize] != self.v[y as usize] { 4 } else { 2 };
    }

    /// Skip the next instruction if the key of index VX is currently pressed.
    fn skp_vx(&mut self, x: u8) {
        self.pc += match self.keypad.get_key_state(self.v[x as usize] as usize) {
                Keystate::Pressed  => 4,
                Keystate::Released => 2,
            };
    }

    /// Skip the next instruction if the key of index VX is currently released.
    fn sknp_vx(&mut self, x: u8) {
        self.pc += match self.keypad.get_key_state(self.v[x as usize] as usize) {
                Keystate::Pressed  => 2,
                Keystate::Released => 4,
            };
    }

    /// Store the value 0xNN in the the register VX.
    fn ld_vx_nn(&mut self, x: u8, nn: u8) {
        self.v[x as usize] = nn;
        self.pc += 2;
    }

    /// Store the value of the register VY in the register VX.
    fn ld_vx_vy(&mut self, x: u8, y: u8) {
        self.v[x as usize] = self.v[y as usize];
        self.pc += 2;
    }

    /// Store the memory address 0x0NNN in the register I.
    fn ld_i_addr(&mut self, addr: u16) {
        self.i = addr as usize;
        self.pc += 2;
    }

    /// Add the value 0xNN to the register VX, wrapping around the result if
    /// needed (VX is an unsigned byte so its maximum value is 255).
    fn add_vx_nn(&mut self, x: u8, nn: u8) {
        let new_vx_u16 = self.v[x as usize] as u16 + nn as u16; // no overflow
        self.v[x as usize] = new_vx_u16 as u8; // wrap around the value
        self.pc += 2;
    }

    /// Add the value of register VX to the value of register I.
    fn add_i_vx(&mut self, x: u8) {
        self.i += self.v[x as usize] as usize;
        self.pc += 2;
    }

    /// Set VX to (VX OR VY).
    fn or_vx_vy(&mut self, x: u8, y: u8) {
        self.v[x as usize] |= self.v[y as usize];
        self.pc += 2;
    }

    /// Set VX to (VX AND VY).
    fn and_vx_vy(&mut self, x: u8, y: u8) {
        self.v[x as usize] &= self.v[y as usize];
        self.pc += 2;
    }

    /// Set VX to (VX XOR VY).
    fn xor_vx_vy(&mut self, x: u8, y: u8) {
        self.v[x as usize] ^= self.v[y as usize];
        self.pc += 2;
    }

    /// Add the value of register VY to the value of register VX.
    /// Set V_FLAG to 0x1 if a carry occurs, and to 0x0 otherwise.
    fn add_vx_vy(&mut self, x: u8, y: u8) {
        let new_vx_u16 = self.v[x as usize] as u16 + self.v[y as usize] as u16;
        self.v[x as usize] = new_vx_u16 as u8;
        self.v[FLAG] = if new_vx_u16 > 255 { 0x1 } else { 0x0 };
        self.pc += 2;
    }

    /// Substract the value of register VY from the value of register VX, and
    /// store the (wrapped) result in register VX.
    /// Set V_FLAG to 0x1 if a borrow occurs, and to 0x0 otherwise.
    fn sub_vx_vy(&mut self, x: u8, y: u8) {
        let new_vx_i8 = self.v[x as usize] as i8 - self.v[y as usize] as i8;
        self.v[x as usize] = new_vx_i8 as u8;
        self.v[FLAG] = if new_vx_i8 < 0 { 0x1 } else { 0x0 };
        self.pc += 2;
    }

    /// Substract the value of register VX from the value of register VY, and
    /// store the (wrapped) result in register VX.
    /// Set V_FLAG to 0x1 if a borrow occurs, and to 0x0 otherwise.
    fn subn_vx_vy(&mut self, x: u8, y: u8) {
        let new_vx_i8 = self.v[y as usize] as i8 - self.v[x as usize] as i8;
        self.v[x as usize] = new_vx_i8 as u8;
        self.v[FLAG] = if new_vx_i8 < 0 { 0x1 } else { 0x0 };
        self.pc += 2;
    }

    /// Store the value of the register VY shifted right one bit in register VX
    /// and set register VF to the least significant bit prior to the shift.
    /// NB : references disagree on this opcode, we use the one defined here :
    /// http://mattmik.com/chip8.html
    /// If shift_op_use_vy is false, will consider VX instead of VY.
    fn shr_vx_vy(&mut self, x: u8, y: u8) {
        let shift_on = if self.shift_op_use_vy { y } else { x };
        self.v[FLAG] = self.v[shift_on as usize] & 0x01;
        self.v[x as usize] = self.v[shift_on as usize] >> 1;
        self.pc += 2;
    }

    /// Same as 'shr_vx_vy' but with a left shift.
    /// Set register VF to the most significant bit prior to the shift.
    /// If shift_op_use_vy is false, will consider VX instead of VY.
    fn shl_vx_vy(&mut self, x: u8, y: u8) {
        let shift_on = if self.shift_op_use_vy { y } else { x };
        self.v[FLAG] = self.v[shift_on as usize] & 0x80;
        self.v[x as usize] = self.v[shift_on as usize] << 1;
        self.pc += 2;
    }

    /// Set VX to a random byte with a mask of 0xNN.
    fn rnd_vx_nn(&mut self, x: u8, nn: u8) {
        self.v[x as usize] = random::<u8>() & nn;
        self.pc += 2;
    }

    /// Draw a sprite at position VX, VY with 0xN bytes of sprite data starting
    /// at the address stored in I. N is thus the height of the sprite.
    /// The drawing is implemented by 'Display' as a XOR operation.
    /// VF will act here as a collision flag, i.e. if any set pixel is erased
    /// set it to 0x1, and to 0x0 otherwise.
    fn drw_vx_vy_n(&mut self, x: u8, y: u8, n: u8) {
        let pos_x     = self.v[x as usize] as usize;
        let pos_y     = self.v[y as usize] as usize;
        let mem_start = self.i;
        let mem_end   = self.i + n as usize;
        if self.display.draw(pos_x, pos_y, &self.memory[mem_start..mem_end]) {
            self.v[FLAG] = 0x1;
        } else {
            self.v[FLAG] = 0x0;
        }
        self.pc += 2;
    }

    /// Store the current value of the delay timer in register VX.
    fn ld_vx_dt(&mut self, x: u8) {
        self.v[x as usize] = self.delay_timer;
        self.pc += 2;
    }

    /// Set the delay timer to the value stored in register VX.
    fn ld_dt_vx(&mut self, x: u8) {
        self.delay_timer = self.v[x as usize];
        self.pc += 2;
    }

    /// Set the sound timer to the value stored in register VX.
    fn ld_st_vx(&mut self, x: u8) {
        self.sound_timer = self.v[x as usize];
        self.pc += 2;
    }

    /// Wait for a key press and store the result in the register VX.
    /// Implementation : the emulation application must trigger the
    /// 'end_wait_for_key_press' function ; this allows to achieve better
    /// decoupling from the framerate.
    fn ld_vx_key(&mut self, x: u8) {
        self.wait_for_key = (true, x);
        /*for i in 0..16 {
            match self.keypad.get_key_state(i) {
                Keystate::Pressed => {
                    self.v[x as usize] = i as u8;
                    self.pc += 2;
                    break;
                }
                Keystate::Released => {},
            }
        }*/
    }

    /// Set I to the memory address of the sprite data corresponding to the
    /// hexadecimal digit (0x0..0xF) stored in register VX.
    /// Will use the internal fontset stored in memory.
    fn ld_i_font_vx(&mut self, x: u8) {
        // the font set is in the memory range 0x0..0x80
        // and each character is represented by 5 bytes
        self.i = (self.v[x as usize] * 5) as usize;
        self.pc += 2;
    }

    /// Store the Binary-Coded Decimal equivalent of the value stored in
    /// register VX in memory at the addresses I, I+1, and I+2.
    fn ld_mem_i_bcd_vx(&mut self, x: u8) {
        // VX is a byte : its decimal value is in 0..256
        let vx = self.v[x as usize];
        self.memory[self.i]   = vx / 100;
        self.memory[self.i+1] = (vx / 10)  % 10;
        self.memory[self.i+2] = (vx % 100) % 10;
        self.pc += 2;
    }

    /// Store the values of registers V0 to VX inclusive in memory starting at
    /// the address I, and set I to I + X + 1 after operation.
    fn ld_mem_i_regs(&mut self, x: u8) {
        let x_usize = x as usize;
        for j in 0..x_usize {
            self.memory[self.i + j] = self.v[j];
        }
        self.i += x_usize + 1;
        self.pc += 2;
    }

    /// Fill registers V0 to VX inclusive with the values stored in memory
    /// starting at the address I.
    fn ld_regs_mem_i(&mut self, x: u8) {
        let x_usize = x as usize;
        for j in 0..x_usize {
            self.v[j] = self.memory[self.i + j];
        }
        self.i += x_usize + 1;
        self.pc += 2;
    }
}
