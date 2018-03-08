/// The display crate handles the display component of the CHIP 8 virtual
/// machine.

/// The CHIP 8 display width, in pixels.
pub const DISPLAY_WIDTH  : usize = 64;
/// The CHIP 8 display height, in pixels.
pub const DISPLAY_HEIGHT : usize = 32;


/// The graphics component of a Chip 8 virtual machine.
/// The Chip 8 uses a 64x32 monochrome display with the format :
/// O-----------------> X
/// |(0,0)      (63,0)|
/// |                 |
/// |(0,31)    (63,31)|
/// ∨-----------------.
/// Y
#[derive(Clone)]
pub struct Display {
    /// 64x32 black and white screen.
    /// 'gfx[i]' contains the pixel column number 'i'.
    /// For a single pixel, '1' means white and '0' black.
    /// Using bytes instead of booleans will make drawing instructions easier
    /// to implement for the same memory cost.
    pub gfx: [[u8; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
    /// Has the display been modified since the last time it was drawn ?
    /// Should be set to false by the emulator application after every draw.
    pub dirty: bool,
}

impl Display {
    /// Create and return a new Display instance.
    pub fn new() -> Display {
        Display {
            gfx: [[0u8; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
            dirty: true,
        }
    }

    /// Clear the screen (set it to uniform black).
    pub fn clear(&mut self) {
        self.gfx = [[0u8; DISPLAY_WIDTH]; DISPLAY_HEIGHT];
        self.dirty = true;
    }

    /// Draw the given sprite to the display at the given position.
    /// The sprite is a reference to the slice of an array of 8 * H pixels.
    /// Return true if there was a collision (i.e. if any of the written pixels
    /// changed from 1 to 0).
    pub fn draw(&mut self, xpos: usize, ypos: usize, sprite: &[u8]) -> bool {
        let mut collision = false;
        let h = sprite.len();

        for j in 0..h {
            for i in 0..8 {
                // screen wrap if necessary
                let y = (ypos + j) % DISPLAY_HEIGHT;
                let x = (xpos + i) % DISPLAY_WIDTH;

                // draw each sprite pixel with a XOR operation
                // i.e. toggle the pixel
                // 0x80 = 1000 0000 : allows to check each pixel in the sprite
                if (sprite[j] & (0x80 >> i)) != 0x00 {
                    if self.gfx[y][x] == 0x01 { collision = true; }
                    self.gfx[y][x] ^= 0x01;
                }
            }
        }
        self.dirty = true;

        collision
    }
}

/// Chip8 font set.
/// Each number or character is 4x5 pixels and is stored as 5 bytes.
/// In each byte, only the first nibble (the first 4 bytes) is used.
/// For instance, with the number 3 :
///  hex    bin     ==> drawn pixels
/// 0xF0  1111 0000        ****
/// 0X10  0001 0000           *
/// 0xF0  1111 0000        ****
/// 0x10  0001 0000           *
/// 0xF0  1111 0000        ****
///
pub static FONT_SET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];
