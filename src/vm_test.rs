use super::keypad::Keystate::*;
use super::vm::{Chip8, FLAG};

#[test]
fn jump_addr() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1793);
    assert_eq!(vm.pc(), 0x0793);
}

#[test]
fn subroutines_and_reset() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x2BBB);
    assert_eq!(vm.pc(), 0x0BBB);
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
    assert_eq!(vm.pc(), (vm.stack[1] + 2) as usize);
}

#[test]
fn regs_and_timers_load() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1200);

    vm.execute_opcode(0x6ABC); // ld_vx_nn
    assert_eq!(vm.register(0xA), 0xBC);
    vm.execute_opcode(0x8BA0); // ld_vx_vy
    vm.execute_opcode(0xA789); // ld_i_addr
    assert_eq!(vm.index(), 0x789);

    vm.execute_opcode(0xFB15); // ld_dt_vx
    vm.execute_opcode(0xF007); // ld_vx_dt
    assert_eq!(vm.register(0x0), 0xBC);
    vm.execute_opcode(0xFA18); // ld_st_vx
    assert_eq!(vm.sound_timer, 0xBC);

    assert_eq!(vm.pc(), 0x200 + 2 * 6);
}

#[test]
fn mem_regs_load() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1321);

    vm.execute_opcode(0x6011);
    vm.execute_opcode(0x6122);
    vm.execute_opcode(0x6233);
    vm.execute_opcode(0x6321);
    vm.execute_opcode(0xA500);
    vm.execute_opcode(0xF355); // ld_mem_i_regs
    assert_eq!(vm.memory[0x500 + 0], 0x11);
    assert_eq!(vm.memory[0x500 + 1], 0x22);
    assert_eq!(vm.memory[0x500 + 2], 0x33);
    assert_eq!(vm.memory[0x500 + 3], 0x21);
    assert_eq!(vm.index(), 0x500 + 4);

    vm.memory[0x500 + 0] = 0x12;
    vm.memory[0x500 + 1] = 0x24;
    vm.memory[0x500 + 2] = 0x56;
    vm.execute_opcode(0xA500);
    vm.execute_opcode(0xF365); // ld_regs_mem_i
    assert_eq!(vm.register(0x0), 0x12);
    assert_eq!(vm.register(0x1), 0x24);
    assert_eq!(vm.register(0x2), 0x56);
    assert_eq!(vm.register(0x3), 0x21);
    assert_eq!(vm.index(), 0x500 + 4);

    assert_eq!(vm.pc(), 0x0321 + 2 * 8);
}

#[test]
fn branches() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1250); // pc = 0x250
    vm.execute_opcode(0x6A18); // VA = 0x18
    assert_eq!(vm.register(10), 0x18);
    vm.execute_opcode(0x3A18); // se_vx_nn
    vm.execute_opcode(0x3A19); // se_vx_nn
    vm.execute_opcode(0x4A18); // sne_vx_nn
    vm.execute_opcode(0x4A19); // sne_vx_nn
    assert_eq!(vm.pc(), 0x0250 + 2 + 4 + 2 + 2 + 4);
    vm.execute_opcode(0x1300); // pc = 0x300
    vm.execute_opcode(0x6B18); // VB = 0x18
    vm.execute_opcode(0x5AB0); // se_vx_vy
    vm.execute_opcode(0x5AC0); // se_vx_vy
    vm.execute_opcode(0x9AF0); // sne_vx_vy
    assert_eq!(vm.pc(), 0x0300 + 2 + 4 + 2 + 4);
}

#[test]
fn add() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1FAF);
    vm.execute_opcode(0x6803);
    vm.execute_opcode(0x78FF); // add_vx_nn
    assert_eq!(vm.register(8), (0x03 + 0xFF) as u8);
    vm.execute_opcode(0x6EAF);
    vm.execute_opcode(0x6DFF);
    vm.execute_opcode(0x8ED4); // add_vx_vy
    assert_eq!(vm.register(FLAG), 0x1);
    assert_eq!(vm.register(14), (0xAF + 0xFF) as u8);
    vm.execute_opcode(0x6013);
    vm.execute_opcode(0x6114);
    vm.execute_opcode(0x8014);
    assert_eq!(vm.register(FLAG), 0x0);
    assert_eq!(vm.register(0), 0x13 + 0x14);
    vm.execute_opcode(0xA999); // I = 0x999
    vm.execute_opcode(0xFD1E); // add_i_vx
    assert_eq!(vm.index(), 0xFF + 0x999);
    assert_eq!(vm.pc(), 0xFAF + 2 * 10);
}

#[test]
fn or() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1234);
    vm.execute_opcode(0x6429);
    vm.execute_opcode(0x6530);
    vm.execute_opcode(0x8451); // or_vx_vy
    assert_eq!(vm.pc(), 0x234 + 2 * 3);
    assert_eq!(vm.register(4), 0x39);
}

#[test]
fn and() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1456);
    vm.execute_opcode(0x6ACF);
    vm.execute_opcode(0x606A);
    vm.execute_opcode(0x80A2); // and_vx_vy
    assert_eq!(vm.pc(), 0x456 + 2 * 3);
    assert_eq!(vm.register(0), 0x4A);
}

#[test]
fn xor() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1789);
    vm.execute_opcode(0x6142);
    vm.execute_opcode(0x627D);
    vm.execute_opcode(0x8123); // xor_vx_vy
    assert_eq!(vm.pc(), 0x789 + 2 * 3);
    assert_eq!(vm.register(1), 0x3F);
}

#[test]
fn sub() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1444);
    vm.execute_opcode(0x6009);
    vm.execute_opcode(0x610F);
    vm.execute_opcode(0x8015); // sub_vx_vy
    assert_eq!(vm.register(FLAG), 0x1);
    assert_eq!(vm.register(0), 0xFA);
    vm.execute_opcode(0x6009);
    vm.execute_opcode(0x8017); // subn_vx_vy
    assert_eq!(vm.register(FLAG), 0x0);
    assert_eq!(vm.register(0), 0x6);
    assert_eq!(vm.pc(), 0x444 + 2 * 5);
}

#[test]
fn shift() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1900);
    vm.execute_opcode(0x6006);
    vm.execute_opcode(0x610F);

    vm.should_shift_op_use_vy(false); // do not shift on VY
    vm.execute_opcode(0x8016); // shr_vy_vy
    assert_eq!(vm.register(0), 0x06 >> 1);
    assert_eq!(vm.register(FLAG), 0x06 & 0x01); // LSB
    vm.execute_opcode(0x8016);
    vm.execute_opcode(0x801E); // shl_vx_vy
    assert_eq!(vm.register(0), (0x06 >> 2) << 1);
    assert_eq!(vm.register(FLAG), (0x06 >> 2) & 0x80); // MSB

    vm.execute_opcode(0x6006);
    vm.should_shift_op_use_vy(true); // shift on VY
    vm.execute_opcode(0x8016);
    assert_eq!(vm.register(0), 0x0F >> 1);
    assert_eq!(vm.register(FLAG), 0x0F & 0x01);
    vm.execute_opcode(0x8116);
    vm.execute_opcode(0x801E);
    assert_eq!(vm.register(0), (0x0F >> 1) << 1);
    assert_eq!(vm.register(FLAG), (0x0F >> 1) & 0x80);
}

#[test]
fn bcd() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1515);
    vm.execute_opcode(0x6095); // 149
    vm.execute_opcode(0xA400);
    vm.execute_opcode(0xF033); // ld_mem_i_bcd_vx
    assert_eq!(vm.memory[0x400 + 0], 0b0001);
    assert_eq!(vm.memory[0x400 + 1], 0b0100);
    assert_eq!(vm.memory[0x400 + 2], 0b1001);
    assert_eq!(vm.pc(), 0x515 + 2 * 3);
}

#[test]
fn input() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1999);
    vm.execute_opcode(0x6D0F);
    vm.execute_opcode(0x610E);
    vm.keypad.set_key_state(0xF, Pressed);
    vm.keypad.set_key_state(0xE, Released);
    vm.execute_opcode(0xED9E); // skp_vx
    assert_eq!(vm.pc(), 0x999 + 4 + 4);
    vm.execute_opcode(0xE1A1); // sknp_vx
    vm.execute_opcode(0xE19E);
    assert_eq!(vm.pc(), 0x999 + 4 + 4 + 6);
}

#[test]
fn drawing() {
    let mut vm = Chip8::new();
    vm.execute_opcode(0x1200);
    vm.execute_opcode(0xA250);
    vm.memory[0x250 + 0] = 0b1110_0111;
    vm.memory[0x250 + 1] = 0b0110_0110;
    vm.memory[0x250 + 2] = 0b0011_1100;

    vm.execute_opcode(0x6A19); // x = 25
    vm.execute_opcode(0x6B07); // y =  7
    vm.execute_opcode(0xDAB3);
    assert_eq!(vm.display.dirty, true);
    assert_eq!(vm.register(FLAG), 0x0);
    let a0 = [1, 1, 1, 0, 0, 1, 1, 1];
    let a1 = [0, 1, 1, 0, 0, 1, 1, 0];
    let a2 = [0, 0, 1, 1, 1, 1, 0, 0];
    for i in 0..8 {
        assert_eq!(vm.display.gfx[7 + 0][25 + i], a0[i]);
        assert_eq!(vm.display.gfx[7 + 1][25 + i], a1[i]);
        assert_eq!(vm.display.gfx[7 + 2][25 + i], a2[i]);
    }

    vm.display.dirty = false;
    vm.execute_opcode(0xA251);
    vm.execute_opcode(0xDAB1);
    assert_eq!(vm.display.dirty, true);
    assert_eq!(vm.register(FLAG), 0x1);
    let a0_bis = [1, 0, 0, 0, 0, 0, 0, 1];
    for i in 0..8 {
        assert_eq!(vm.display.gfx[7 + 0][25 + i], a0_bis[i]);
        assert_eq!(vm.display.gfx[7 + 1][25 + i], a1[i]);
        assert_eq!(vm.display.gfx[7 + 2][25 + i], a2[i]);
    }

    assert_eq!(vm.pc(), 0x200 + 2 * 6);
}
