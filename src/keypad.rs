
/// Stores the state of the virtual keypad used for input.
///
/// The Chip8 virtual keypad has the following layout :
///
/// Virtual Keypad       Keyboard (QWERTY)
/// +-+-+-+-+                +-+-+-+-+
/// |1|2|3|C|                |1|2|3|4|
/// +-+-+-+-+                +-+-+-+-+
/// |4|5|6|D|                |Q|W|E|R|
/// +-+-+-+-+       =>       +-+-+-+-+
/// |7|8|9|E|                |A|S|D|F|
/// +-+-+-+-+                +-+-+-+-+
/// |A|0|B|F|                |Z|X|C|V|
/// +-+-+-+-+                +-+-+-+-+
///
/// source :
/// http://www.multigesture.net/articles/how-to-write-an-emulator-chip-8-interpreter/
///
pub struct Keypad {
    /// The state of the 16 keys (true = currently pressed).
    keys: [bool; 16],
}

impl Keypad {
    /// Create and return a new Keypad instance.
    pub fn new() -> Keypad {
        Keypad {
            keys: [false; 16]
        }
    }

    /// Return the state of the key at the given index, or None
    /// if the index is invalid.
    pub fn is_pressed(&self, index: usize) -> Option<bool> {
        //println!("is_pressed({})", index); // DEBUG
        // TODO : security check necessary ?
        if index < self.keys.len() {
            // DEBUG
            /*if self.keys[index] {
                println!("acquiring pressed : {:X}", index);
            }*/
            Some(self.keys[index])
        } else {
            None
        }
    }

    /// Set the key at the given index as currently pressed.
    pub fn pressed(&mut self, index: usize) {
        // DEBUG
        debug_assert!(index < 16);
        //println!("pressed {:X}", index);

        self.keys[index] = true;
    }

    /// Set the key at the given index as currently not pressed.
    pub fn released(&mut self, index: usize) {
        // DEBUG
        debug_assert!(index < 16);
        //println!("released {:X}", index);

        self.keys[index] = false;
    }
}
