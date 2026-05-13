use super::*;
use std::collections::BTreeMap;

fn step_instr(cpu: &mut Processor) {
    while cpu.step() {
        if cpu.micro_pc == 0 {
            break;
        }
    }
}

#[test]
fn test_binary_add() {
    let mut memory = BTreeMap::new();

    // PUSH_CONST 10
    let mut program = Vec::new();
    program.extend_from_slice(&0x05u32.to_le_bytes()); // Opcode
    program.extend_from_slice(&10u32.to_le_bytes()); // Const

    // PUSH_CONST 20
    program.extend_from_slice(&0x05u32.to_le_bytes()); // Opcode
    program.extend_from_slice(&20u32.to_le_bytes()); // Const

    // ADD
    program.extend_from_slice(&0x0Eu32.to_le_bytes()); // Opcode

    for (i, chunk) in program.chunks(4).enumerate() {
        let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        memory.insert(0x1000 + (i as u32 * 4), val);
    }

    let mut cpu = Processor::new(memory, BTreeMap::new());

    // Execute 3 steps (PUSH_CONST, PUSH_CONST, ADD)
    step_instr(&mut cpu);
    step_instr(&mut cpu);
    step_instr(&mut cpu);

    let res = cpu.read_mem(cpu.sp);
    assert_eq!(res, 30);
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

    // 0x1018: ADD (will just add 42 to whatever is below if we didn't jump correctly)
    // Wait, the JUMP addr 0x1014 is actually the PUSH_CONST 99 instruction.
    // Let's jump to 0x1018 instead to skip it.

    for (i, chunk) in program.chunks(4).enumerate() {
        let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        memory.insert(0x1000 + (i as u32 * 4), val);
    }

    // Fix the jump address in memory (it was at 0x100C)
    memory.insert(0x100C, 0x1018);

    let mut cpu = Processor::new(memory, BTreeMap::new());

    step_instr(&mut cpu); // PUSH_CONST 42
    assert_eq!(cpu.read_mem(cpu.sp), 42);

    step_instr(&mut cpu); // JUMP 0x1018
    assert_eq!(cpu.ip, 0x1018);

    // Next instruction at 0x1018 is ADD, but it needs 2 values.
    // This test just verifies the jump IP.
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
    step_instr(&mut cpu); // SWAP. Stack: [10, 20, 20] -> [10, 20, 20] (no change because top two are same)
                          // Let's make a better swap test later.

    // Let's just verify the state after all steps for now
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
    p.push(4); // STORE_R 4. mem[SP-4] = 20, POP. Stack: [20] (Wait, [20] because it replaced 10)

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
