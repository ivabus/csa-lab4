#![allow(clippy::vec_init_then_push)]

use super::*;
use std::collections::BTreeMap;
fn step_instr(cpu: &mut Processor) {
    while cpu.step() {
        if cpu.micro_pc == 0 {
            break;
        }
    }
}

fn load_program(words: &[u32]) -> BTreeMap<u32, u32> {
    let mut mem = BTreeMap::new();
    for (i, &w) in words.iter().enumerate() {
        mem.insert(0x1000 + (i as u32) * 4, w);
    }
    mem
}

fn prog(insts: &[u32]) -> BTreeMap<u32, u32> {
    let mut words = Vec::new();
    for &w in insts {
        words.push(w);
    }
    load_program(&words)
}

#[test]
fn test_binary_add() {
    let memory = prog(&[0x05, 10, 0x05, 20, 0x0E]);
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 30);
}

#[test]
fn test_binary_sub() {
    let memory = prog(&[0x05, 100, 0x05, 30, 0x0F]);
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 70);
}

#[test]
fn test_binary_mul() {
    let memory = prog(&[0x05, 7, 0x05, 6, 0x10]);
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 42);
}

#[test]
fn test_binary_div() {
    let memory = prog(&[0x05, 42, 0x05, 5, 0x11]);
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 8);
}

#[test]
fn test_binary_mod() {
    let memory = prog(&[0x05, 42, 0x05, 5, 0x12]);
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 2);
}

#[test]
fn test_binary_or() {
    let memory = prog(&[0x05, 0xF0, 0x05, 0x0F, 0x13]);
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 0xFF);
}

#[test]
fn test_binary_and() {
    let memory = prog(&[0x05, 0xFF, 0x05, 0x0F, 0x14]);
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 0x0F);
}

#[test]
fn test_binary_xor() {
    let memory = prog(&[0x05, 0xFF, 0x05, 0x0F, 0x15]);
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 0xF0);
}

#[test]
fn test_binary_jump() {
    let mut memory = BTreeMap::new();

    let mut program = Vec::new();
    // 0x1000: PUSH_CONST 42
    program.extend_from_slice(&0x05u32.to_le_bytes());
    program.extend_from_slice(&42u32.to_le_bytes());

    // 0x1008: JUMP 0x1014
    program.extend_from_slice(&0x20u32.to_le_bytes());
    program.extend_from_slice(&0x1014u32.to_le_bytes());

    // 0x1010: PUSH_CONST 99 (skipped)
    program.extend_from_slice(&0x05u32.to_le_bytes());
    program.extend_from_slice(&99u32.to_le_bytes());

    for (i, chunk) in program.chunks(4).enumerate() {
        let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        memory.insert(0x1000 + (i as u32 * 4), val);
    }

    memory.insert(0x100C, 0x1018);

    let mut cpu = Processor::new(memory, BTreeMap::new());

    step_instr(&mut cpu); // PUSH_CONST 42
    assert_eq!(cpu.read_mem(cpu.sp), 42);

    step_instr(&mut cpu); // JUMP 0x1018
    assert_eq!(cpu.ip, 0x1018);
}

#[test]
fn test_stack_ops() {
    let mut memory = BTreeMap::new();
    let mut program = Vec::new();

    // PUSH_CONST 10
    program.push(0x05);
    program.push(10);
    // PUSH_CONST 20
    program.push(0x05);
    program.push(20);
    // DUP
    program.push(0x0C);
    // SWAP
    program.push(0x0B);
    // OVER
    program.push(0x0D);
    // POP
    program.push(0x08);
    // EXT
    program.push(0x07);

    for (i, &val) in program.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }

    let mut cpu = Processor::new(memory, BTreeMap::new());

    step_instr(&mut cpu); // PUSH 10. Stack: [10]
    assert_eq!(cpu.read_mem(cpu.sp), 10);
    step_instr(&mut cpu); // PUSH 20. Stack: [10, 20]
    assert_eq!(cpu.read_mem(cpu.sp), 20);
    step_instr(&mut cpu); // DUP. Stack: [10, 20, 20]
    assert_eq!(cpu.read_mem(cpu.sp), 20);
    assert_eq!(cpu.read_mem(cpu.sp - 4), 20);
    step_instr(&mut cpu); // SWAP. Stack: [10, 20, 20] -> [10, 20, 20]
}

#[test]
fn test_more_stack_ops() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(1); // [1]
    p.push(0x05);
    p.push(2); // [1, 2]
    p.push(0x0B); // SWAP -> [2, 1]
    p.push(0x0D); // OVER -> [2, 1, 2]
    p.push(0x0C); // DUP -> [2, 1, 2, 2]
    p.push(0x08); // POP -> [2, 1, 2]
    p.push(0x06);
    p.push(8); // PUSH_R 8 (relative to top). SP points to 2. SP-4 is 1. SP-8 is 2.
               // PUSH_R 8 (after inc) uses SP_new - 4 - 8 = SP_old - 8.
               // Stack: [2, 1, 2, 2]

    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 1);
    assert_eq!(cpu.read_mem(cpu.sp - 4), 2);
    step_instr(&mut cpu); // OVER
    assert_eq!(cpu.read_mem(cpu.sp), 2);
    step_instr(&mut cpu); // DUP
    assert_eq!(cpu.read_mem(cpu.sp), 2);
    step_instr(&mut cpu); // POP
    assert_eq!(cpu.read_mem(cpu.sp), 2);
    step_instr(&mut cpu); // PUSH_R 8
    assert_eq!(cpu.read_mem(cpu.sp), 2);
}

#[test]
fn test_arithmetic_unary() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(10); // [10]
    p.push(0x17); // INC -> [11]
    p.push(0x19); // INC4 -> [15]
    p.push(0x18); // DEC -> [14]
    p.push(0x1A); // DEC4 -> [10]
    p.push(0x16); // NOT -> [!10]

    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 11);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 15);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 14);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 10);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), !10);
}

#[test]
fn test_shifts() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(0x1234); // [0x1234]
    p.push(0x05);
    p.push(4); // [0x1234, 4]
    p.push(0x1B); // LS -> [0x12340]
    p.push(0x05);
    p.push(4);
    p.push(0x1C); // RS -> [0x1234]

    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 0x12340);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 0x1234);
}

#[test]
fn test_call_ret() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    // 0x1000: CALL 0x1008
    p.push(0x27);
    p.push(0x1008);
    // 0x1008: RET
    p.push(0x28);

    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu); // CALL
    assert_eq!(cpu.ip, 0x1008);
    assert_eq!(cpu.read_mem(cpu.sp), 0x1008); // Return address should be IP+4 = 0x1008
    step_instr(&mut cpu); // RET
    assert_eq!(cpu.ip, 0x1008); // Returns to 0x1008
}

#[test]
fn test_write_store() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(42); // [42]
    p.push(0x0A);
    p.push(0x2000); // STORE 0x2000. Stack: []
    p.push(0x01);
    p.push(0x2000); // PUSH 0x2000. Stack: [42]

    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(0x2000), 42);
    assert_eq!(cpu.sp, 0x8000_0000);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 42);
}

#[test]
fn test_byte_ops() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    // 0x1000: PUSH_BYTE 0x12345678 (byte 0 is 0x78)
    p.push(0x02);
    p.push(0x12345678);
    // 0x1008: PUSH_SIGNED_BYTE 0x000000FF (byte 0 is 0xFF -> sign extended to 0xFFFFFFFF)
    p.push(0x03);
    p.push(0x000000FF);

    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 0x78);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 0xFFFFFFFF);
}

#[test]
fn test_storer() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(10); // [10]
    p.push(0x05);
    p.push(20); // [10, 20]
    p.push(0x2A);
    p.push(4); // STORE_R 4. mem[SP-4] = 20, POP. Stack: [20]
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 20);
    assert_eq!(cpu.sp, 0x8000_0004);
}

#[test]
fn test_push_signed_byte_w() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    // 0x1000: PUSH_SIGNED_BYTE_W 0x80 -> MSB of full word is 1 -> 0xFFFFFF00 | 0x80 = 0xFFFFFF80
    p.push(0x04);
    p.push((-128i32) as u32); // full word = 0xFFFFFF80, MSB=1 -> sign-extends byte
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 0xFFFFFF80u32);
}

#[test]
fn test_write_addr() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(99); // [99]
    p.push(0x09);
    p.push(0x3000); // WRITE 0x3000. mem[0x3000] = 99. Stack unchanged.
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(0x3000), 99);
    assert_eq!(cpu.sp, 0x8000_0004); // SP unchanged (not popped)
}

#[test]
fn test_conditional_jumps_jeq_taken() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(10); // [10]
    p.push(0x05);
    p.push(10); // [10, 10]
    p.push(0x21);
    p.push(0x1020); // JEQ 0x1020 (10==10 -> jump)
    p.push(0x05);
    p.push(42); // SKIPPED
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu); // JEQ - should jump to 0x1020
    assert_eq!(cpu.ip, 0x1020);
}

#[test]
fn test_conditional_jumps_jeq_not_taken() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(10); // [10]
    p.push(0x05);
    p.push(99); // [10, 99]
    p.push(0x21);
    p.push(0x1020); // JEQ 0x1020 (10!=99 -> no jump)
    p.push(0x05);
    p.push(42); // Should execute
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu); // JEQ - not taken
    assert_eq!(cpu.ip, 0x1018);
    step_instr(&mut cpu); // PUSH_CONST 42
    assert_eq!(cpu.read_mem(cpu.sp), 42);
}

#[test]
fn test_conditional_jumps_jne_taken() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(10); // [10]
    p.push(0x05);
    p.push(99); // [10, 99]
    p.push(0x22);
    p.push(0x1020); // JNE 0x1020 (10!=99 -> jump)
    p.push(0x05);
    p.push(42); // SKIPPED
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.ip, 0x1020);
}

#[test]
fn test_conditional_jumps_jgt_taken() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(100); // [100]
    p.push(0x05);
    p.push(50); // [100, 50]
    p.push(0x23);
    p.push(0x1020); // JGT 0x1020 (100>50 -> jump)
    p.push(0x05);
    p.push(99); // SKIPPED
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.ip, 0x1020);
}

#[test]
fn test_conditional_jumps_jlt_taken() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(10); // [10]
    p.push(0x05);
    p.push(50); // [10, 50]
    p.push(0x24);
    p.push(0x1020); // JLT 0x1020 (10<50 -> jump)
    p.push(0x05);
    p.push(99); // SKIPPED
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.ip, 0x1020);
}

#[test]
fn test_conditional_jumps_jge_taken() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(100); // [100]
    p.push(0x05);
    p.push(50); // [100, 50]
    p.push(0x25);
    p.push(0x1020); // JGE 0x1020 (100>=50 -> jump)
    p.push(0x05);
    p.push(99); // SKIPPED
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.ip, 0x1020);
}

#[test]
fn test_conditional_jumps_jle_taken() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(10); // [10]
    p.push(0x05);
    p.push(50); // [10, 50]
    p.push(0x26);
    p.push(0x1020); // JLE 0x1020 (10<=50 -> jump)
    p.push(0x05);
    p.push(99); // SKIPPED
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.ip, 0x1020);
}

#[test]
fn test_conditional_jumps_jge_equal() {
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(50); // [50]
    p.push(0x05);
    p.push(50); // [50, 50]
    p.push(0x25);
    p.push(0x1020); // JGE 0x1020 (50>=50 -> jump, equal case)
    p.push(0x05);
    p.push(99); // SKIPPED
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.ip, 0x1020);
}

#[test]
fn test_shift_ars() {
    let memory = prog(&[0x05, 0xFFFFFFF0u32, 0x05, 2, 0x1D]);
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 0xFFFFFFFCu32);
}

#[test]
fn test_shift_lcs() {
    let memory = prog(&[0x05, 0x80000000u32, 0x05, 1, 0x1E]);
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 0x00000001);
}

#[test]
fn test_shift_rcs() {
    let memory = prog(&[0x05, 0x00000001, 0x05, 1, 0x1F]);
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 0x80000000u32);
}

#[test]
fn test_wret() {
    // CALL a function that returns via WRET.
    // CALL instruction at 0x1000 with target 0x1010.
    // CALL pushes IP+4 (0x1008) as return address, then jumps to 0x1010.
    // WRET at 0x1010 pops return address and does extra arg cleanup.
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    // 0x1000: CALL 0x1010 (opcode + target addr)
    p.push(0x27);
    p.push(0x1010);
    // 0x1008: PUSH_CONST 42 (instruction after CALL, should be jumped to by ret)
    p.push(0x05);
    p.push(42);
    // 0x1010: WRET (pop return addr, SP -= 4 for arg cleanup)
    p.push(0x29);
    for (i, &val) in p.iter().enumerate() {
        // Offset the base when inserting into memory: CALL is at 0x1000,
        // the words in p[0..] map to 0x1000, 0x1004, 0x1008, 0x100C, 0x1010
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu); // CALL: SP +=4 (push ret addr = 0x1008), jump to 0x1010
    assert_eq!(cpu.ip, 0x1010);
    let sp_after_call = cpu.sp;
    step_instr(&mut cpu); // WRET: pop ret addr, set IP = 0x1008, SP -=4 (arg cleanup)
    assert_eq!(cpu.ip, 0x1008);
    // WRET pops 8 bytes total from sp_after_call: ret addr (4) + arg slot (4)
    assert_eq!(cpu.sp, sp_after_call.wrapping_sub(8));
}

#[test]
fn test_halt() {
    // Just HALT opcode
    let memory = prog(&[0x00]);
    let mut cpu = Processor::new(memory, BTreeMap::new());
    let result = cpu.step(); // Halt returns false
    assert!(!result);
}

#[test]
fn test_alu_flags_add_overflow() {
    let memory = prog(&[0x05, 0x7FFFFFFFu32, 0x05, 1, 0x0E]);
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 0x80000000u32);
    assert!(cpu.n);
    assert!(cpu.v);
}

#[test]
fn test_alu_flags_sub_zero() {
    let memory = prog(&[0x05, 42, 0x05, 42, 0x0F]);
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    assert_eq!(cpu.read_mem(cpu.sp), 0);
    assert!(cpu.z);
}

#[test]
fn test_superscalar_deferred_store() {
    // Write to a memory address and verify it stays in shadow (not in main memory)
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(1234); // [1234]
    p.push(0x0A);
    p.push(0x5000); // STORE to 0x5000 (via shadow)
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    // Value should be in shadow, not in main memory
    assert!(cpu.shadow_store.is_some());
    assert_eq!(cpu.shadow_store, Some((0x5000, 1234)));
    assert_ne!(cpu.memory.get(&0x5000), Some(&1234));
}

#[test]
fn test_superscalar_forwarding() {
    // Write then read the same address -> should forward from shadow
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(777); // [777]
    p.push(0x0A);
    p.push(0x5000); // STORE to 0x5000 (deferred to shadow)
    p.push(0x01);
    p.push(0x5000); // PUSH_ADDR 0x5000 -> should forward from shadow
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    // shadow has (0x5000, 777)
    let read_val = cpu.read_mem(0x5000); // Should read from shadow (forwarding)
    assert_eq!(read_val, 777);
}

#[test]
fn test_superscalar_dead_store_elimination() {
    // Two stores to the same address -> second one replaces first in shadow
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(111); // [111]
    p.push(0x0A);
    p.push(0x5000); // STORE 111 to 0x5000 (shadow)
    p.push(0x05);
    p.push(222); // [222]
    p.push(0x0A);
    p.push(0x5000); // STORE 222 to the same 0x5000 (overwrites shadow)
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    // Shadow should have the latest value
    assert_eq!(cpu.shadow_store, Some((0x5000, 222)));
}

#[test]
fn test_superscalar_parallel_flush() {
    // Store to two different addresses -> second should trigger parallel flush of first
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(1001); // [1001]
    p.push(0x0A);
    p.push(0x5000); // STORE to 0x5000 -> deferred
    p.push(0x05);
    p.push(2002); // [2002]
    p.push(0x0A);
    p.push(0x6000); // STORE to 0x6000 -> parallel flush of 0x5000
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    // First address should be flushed to memory
    assert_eq!(cpu.memory.get(&0x5000), Some(&1001));
    // Second address should be in shadow
    assert_eq!(cpu.shadow_store, Some((0x6000, 2002)));
}

#[test]
fn test_superscalar_flush_on_halt() {
    // Store to address, then halt -> should flush shadow
    let mut memory = BTreeMap::new();
    let mut p = Vec::new();
    p.push(0x05);
    p.push(555); // [555]
    p.push(0x0A);
    p.push(0x5000); // STORE to 0x5000 (deferred)
    p.push(0x00); // HALT
    for (i, &val) in p.iter().enumerate() {
        memory.insert(0x1000 + (i as u32 * 4), val);
    }
    let mut cpu = Processor::new(memory, BTreeMap::new());
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    // Halt should flush shadow
    assert!(!cpu.step());
    assert_eq!(cpu.memory.get(&0x5000), Some(&555));
    assert!(cpu.shadow_store.is_none());
}
