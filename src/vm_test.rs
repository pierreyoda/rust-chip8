use super::vm::{Chip8, FLAG};

#[test]
fn jump_addr() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1793);
    assert_eq!(vm.pc, 0x0793);
}

#[test]
fn subroutines_and_reset() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x2BBB);
    assert_eq!(vm.pc, 0x0BBB);
    assert_eq!(vm.stack[0], 0x200);
    vm.execute_opcode(0x00EE);
    assert_eq!(vm.sp, 0);

    vm.reset();
    assert_eq!(vm.stack[0], 0);
    vm.stack[0] = 0x0AAA;
    vm.stack[1] = 0x0BBB;
    vm.sp = 0x2;
    vm.execute_opcode(0x00EE);
    assert_eq!(vm.sp, 1);
    assert_eq!(vm.pc, (vm.stack[1]+2) as usize);
}

#[test]
fn branches() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1250); // pc = 0x250
    vm.execute_opcode(0x6A18); // VA = 0x18
    assert_eq!(vm.v[10], 0x18);
    vm.execute_opcode(0x3A18); // se_vx_nn
    vm.execute_opcode(0x3A19); // se_vx_nn
    vm.execute_opcode(0x4A18); // sne_vx_nn
    vm.execute_opcode(0x4A19); // sne_vx_nn
    assert_eq!(vm.pc, 0x0250+2+4+2+2+4);
    vm.execute_opcode(0x1300); // pc = 0x300
    vm.execute_opcode(0x6B18); // VB = 0x18
    vm.execute_opcode(0x5AB0); // se_vx_vy
    vm.execute_opcode(0x5AC0); // se_vx_vy
    vm.execute_opcode(0x9AF0); // sne_vx_vy
    assert_eq!(vm.pc, 0x0300+2+4+2+4);
}

#[test]
fn add() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1FAF);
    vm.execute_opcode(0x6803);
    vm.execute_opcode(0x78FF); // add_vx_nn
    assert_eq!(vm.v[8], (0x03+0xFF) as u8);
    vm.execute_opcode(0x6EAF);
    vm.execute_opcode(0x6DFF);
    vm.execute_opcode(0x8ED4); // add_vx_vy
    assert_eq!(vm.v[FLAG], 0x1);
    assert_eq!(vm.v[14], (0xAF+0xFF) as u8);
    vm.execute_opcode(0xA999); // I = 0x999
    vm.execute_opcode(0xFD1E); // add_i_vx
    assert_eq!(vm.i, 0xFF+0x999);
    assert_eq!(vm.pc, 0xFAF+2*7);
}

#[test]
fn or() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1234);
    vm.execute_opcode(0x6429);
    vm.execute_opcode(0x6530);
    vm.execute_opcode(0x8451); // or_vx_vy
    assert_eq!(vm.pc, 0x234+2*3);
    assert_eq!(vm.v[4], 0x39);
}

#[test]
fn and() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1456);
    vm.execute_opcode(0x6ACF);
    vm.execute_opcode(0x606A);
    vm.execute_opcode(0x80A2); // and_vx_vy
    assert_eq!(vm.pc, 0x456+2*3);
    assert_eq!(vm.v[0], 0x4A);
}

#[test]
fn xor() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1789);
    vm.execute_opcode(0x6142);
    vm.execute_opcode(0x627D);
    vm.execute_opcode(0x8123); // xor_vx_vy
    assert_eq!(vm.pc, 0x789+2*3);
    assert_eq!(vm.v[1], 0x3F);
}
