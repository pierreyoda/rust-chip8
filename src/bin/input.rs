use std::collections::HashMap;
extern crate sdl2;
use self::sdl2::keycode::KeyCode;

/// Enumerates the supported keyboard bindings for the virtual keypad.
/// TODO : add a Custom(...key bindings...) type, loaded from a file ?
pub enum KeyboardBinding {
    QWERTY,
    AZERTY,
}

/// Return the HashMap<KeyCode, usize> corresponding to the
/// given keyboard configuration which allows to simulate the virtual keypad.
/// See 'chip8vm::keypad::Keypad' for the QWERTY binding.
pub fn get_sdl_key_bindings(keyboard: &KeyboardBinding)
    -> HashMap<KeyCode, usize> {
    let mut hm = HashMap::new();

    // since we only support AZERTY and QWERTY for now, insert the common keys
    hm.insert(KeyCode::Num1, 0x1);
    hm.insert(KeyCode::Num2, 0x2);
    hm.insert(KeyCode::Num3, 0x3);
    hm.insert(KeyCode::Num4, 0xC);
    hm.insert(KeyCode::E, 0x6);
    hm.insert(KeyCode::R, 0xD);
    hm.insert(KeyCode::S, 0x8);
    hm.insert(KeyCode::D, 0x9);
    hm.insert(KeyCode::F, 0xE);
    hm.insert(KeyCode::C, 0xB);
    hm.insert(KeyCode::V, 0xF);

    match *keyboard {
        KeyboardBinding::QWERTY => {
            hm.insert(KeyCode::Q, 0x4);
            hm.insert(KeyCode::W, 0x5);
            hm.insert(KeyCode::A, 0x7);
            hm.insert(KeyCode::Z, 0xA);
            hm.insert(KeyCode::X, 0x0);
        },
        KeyboardBinding::AZERTY => {
            hm.insert(KeyCode::Q, 0x7);
            hm.insert(KeyCode::W, 0xA);
            hm.insert(KeyCode::A, 0x4);
            hm.insert(KeyCode::Z, 0x5);
            hm.insert(KeyCode::X, 0x0);
        },
    }

    assert_eq!(hm.len(), 16);

    hm
}
