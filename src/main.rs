use arm7tdmi_rs::{Cpu, Memory, ARM_INIT};

mod ram;

use crate::ram::Ram;

fn main() {
    env_logger::init();

    // mov r0, #1
    // mov r1, #2
    // mov r2, #3
    // adds r3, r2, r1
    // mov r4, #0
    // moveq r4, r3
    // mov r6, #0x100
    // str r3, [r6, #0]
    // str r4, [r6, #4]
    let prog: &[u8] = &[
        0x01, 0x00, 0xa0, 0xe3, 0x02, 0x10, 0xa0, 0xe3, 0x03, 0x20, 0xa0, 0xe3, 0x01, 0x30, 0x92,
        0xe0, 0x00, 0x40, 0xa0, 0xe3, 0x03, 0x40, 0xa0, 0x01, 0x01, 0x6c, 0xa0, 0xe3, 0x00, 0x30,
        0x86, 0xe5, 0x04, 0x40, 0x86, 0xe5, 0x10, 0xad, 0xde, 0xe7,
    ];
    let mmu = Ram::new_with_data(0x1000, prog);
    let mut cpu = Cpu::new(mmu, ARM_INIT);

    while cpu.cycle() {}

    let mem = cpu.borrow_mut_mmu();
    for &(addr, val) in [(0x100, 5), (0x104, 0)].iter() {
        let emuval = mem.r32(addr);
        assert_eq!(val, emuval, "addr: {:#010x}", addr);
        println!("{:?}", emuval);
    }
}
