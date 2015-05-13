
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

/// The possible status for a key of the virtual keypad.
#[derive(Copy, Clone, Debug)]
pub enum Keystate {
    Pressed,
    Released,
}

pub struct Keypad {
    /// The state of the 16 keys (true = currently pressed).
    keys: [Keystate; 16],
}

impl Keypad {
    /// Create and return a new Keypad instance.
    pub fn new() -> Keypad {
        Keypad {
            keys: [Keystate::Released; 16]
        }
    }

    /// Return the state of the key at the given index.
    pub fn get_key_state(&self, index: usize) -> Keystate {
        //println!("get_key_state({:X})", index); // DEBUG
        debug_assert!(index < 16);
        self.keys[index]
    }

    /// Set the current key state for the key at the given index.
    pub fn set_key_state(&mut self, index: usize, state: Keystate) {
        //println!("set_key_state({:X}, {:?})", index, state); // DEBUG
        debug_assert!(index < 16);
        self.keys[index] = state;
    }
}
