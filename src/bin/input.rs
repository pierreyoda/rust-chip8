use std::collections::HashMap;
use sdl2;
use self::sdl2::keyboard::Keycode;

/// Enumerates the supported keyboard bindings for the virtual keypad.
/// TODO : add a Custom(...key bindings...) type, loaded from a file ?
pub enum KeyboardBinding {
    QWERTY,
    AZERTY,
}

/// Return the HashMap<Keycode, usize> corresponding to the
/// given keyboard configuration which allows to simulate the virtual keypad.
/// See 'chip8vm::keypad::Keypad' for the QWERTY binding.
pub fn get_sdl_key_bindings(keyboard: &KeyboardBinding) -> HashMap<Keycode, usize> {
    let mut hm = HashMap::new();

    // since we only support AZERTY and QWERTY for now, insert the common keys
    hm.insert(Keycode::Num1, 0x1);
    hm.insert(Keycode::Num2, 0x2);
    hm.insert(Keycode::Num3, 0x3);
    hm.insert(Keycode::Num4, 0xC);
    hm.insert(Keycode::E, 0x6);
    hm.insert(Keycode::R, 0xD);
    hm.insert(Keycode::S, 0x8);
    hm.insert(Keycode::D, 0x9);
    hm.insert(Keycode::F, 0xE);
    hm.insert(Keycode::C, 0xB);
    hm.insert(Keycode::V, 0xF);

    match *keyboard {
        KeyboardBinding::QWERTY => {
            hm.insert(Keycode::Q, 0x4);
            hm.insert(Keycode::W, 0x5);
            hm.insert(Keycode::A, 0x7);
            hm.insert(Keycode::Z, 0xA);
            hm.insert(Keycode::X, 0x0);
        }
        KeyboardBinding::AZERTY => {
            hm.insert(Keycode::Q, 0x7);
            hm.insert(Keycode::W, 0xA);
            hm.insert(Keycode::A, 0x4);
            hm.insert(Keycode::Z, 0x5);
            hm.insert(Keycode::X, 0x0);
        }
    }

    assert_eq!(hm.len(), 16);

    hm
}
