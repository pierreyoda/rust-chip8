
/// The graphics core.
/// The Chip8 uses a 64x32 monochrome display with the format :
/// +-----------------+
/// |(0,0)      (63,0)|
/// |                 |
/// |(0,31)    (63,31)|
/// +-----------------+
pub struct Display {
    /// 64x32 black and white screen.
    /// 'gfx[i]' contains the line number 'i'.
    /// For a single pixel, '1' means white and '0' black.
    gfx: [[u8; 64]; 32]
}

impl Display {
    /// Create and return a new Display instance.
    pub fn new() -> Display {
        Display {
            gfx: [[0u8; 64]; 32]
        }
    }

    // Clear the screen (set it to uniform black).
    pub fn clear(&mut self) {
        self.gfx = [[0u8; 64]; 32];
    }
}
