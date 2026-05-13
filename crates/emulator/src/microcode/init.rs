// ./crates/emulator/src/microcode/init.rs
use crate::alu::AluOp;
use crate::microcode::*;

pub fn init_micro_memory(micro_memory: &mut [u8; 256 * 32]) {
    // PUSH addr 0x01: SP += 4; mem[SP] = mem[mem[IP]]; IP += 4
    set_micro(
        micro_memory,
        0x01,
        &[
            mov(RegisterSource::Ip, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Ar),
            load(false, false, false, 0), // Or = value
            stack(false),
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            store(false, 0),
            mov(RegisterSource::Ip, RegisterDestination::Alu0),
            alu(true, AluOp::Inc),
            mov(RegisterSource::Or, RegisterDestination::Ip),
        ],
    );

    // PUSH_BYTE 0x02
    set_micro(
        micro_memory,
        0x02,
        &[
            mov(RegisterSource::Ip, RegisterDestination::Ar),
            load(false, false, true, 0),
            stack(false),
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            store(false, 0),
            mov(RegisterSource::Ip, RegisterDestination::Alu0),
            alu(true, AluOp::Inc),
            mov(RegisterSource::Or, RegisterDestination::Ip),
        ],
    );

    // PUSH_SIGNED_BYTE 0x03
    set_micro(
        micro_memory,
        0x03,
        &[
            mov(RegisterSource::Ip, RegisterDestination::Ar),
            load(true, false, true, 0),
            stack(false),
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            store(false, 0),
            mov(RegisterSource::Ip, RegisterDestination::Alu0),
            alu(true, AluOp::Inc),
            mov(RegisterSource::Or, RegisterDestination::Ip),
        ],
    );

    // PUSH_SIGNED_BYTE_W 0x04
    set_micro(
        micro_memory,
        0x04,
        &[
            mov(RegisterSource::Ip, RegisterDestination::Ar),
            load(true, true, true, 0),
            stack(false),
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            store(false, 0),
            mov(RegisterSource::Ip, RegisterDestination::Alu0),
            alu(true, AluOp::Inc),
            mov(RegisterSource::Or, RegisterDestination::Ip),
        ],
    );

    // PUSH_CONST 0x05
    set_micro(
        micro_memory,
        0x05,
        &[
            mov(RegisterSource::Ip, RegisterDestination::Ar),
            load(false, false, false, 0),
            stack(false),
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            store(false, 0),
            mov(RegisterSource::Ip, RegisterDestination::Alu0),
            alu(true, AluOp::Inc),
            mov(RegisterSource::Or, RegisterDestination::Ip),
        ],
    );

    // PUSH_R offset 0x06: target = SP - offset; SP += 4; mem[SP] = mem[target]; IP += 4
    set_micro(
        micro_memory,
        0x06,
        &[
            mov(RegisterSource::Ip, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Alu1),
            mov(RegisterSource::Sp, RegisterDestination::Alu0),
            alu(false, AluOp::Sub),
            mov(RegisterSource::Or, RegisterDestination::Ar),
            load(false, false, false, 0), // Or = value
            stack(false),
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            store(false, 0),
            mov(RegisterSource::Ip, RegisterDestination::Alu0),
            alu(true, AluOp::Inc),
            mov(RegisterSource::Or, RegisterDestination::Ip),
        ],
    );

    // EXT 0x07
    set_micro(micro_memory, 0x07, &[stack(false)]);

    // POP 0x08
    set_micro(micro_memory, 0x08, &[stack(true)]);

    // WRITE addr 0x09: mem[mem[IP]] = mem[SP]; IP += 4
    set_micro(
        micro_memory,
        0x09,
        &[
            mov(RegisterSource::Ip, RegisterDestination::Alu0),
            mov(RegisterSource::Ip, RegisterDestination::Alu1),
            alu(false, AluOp::Xor),
            mov(RegisterSource::Or, RegisterDestination::Alu0), // Alu0 = 0
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Alu1), // Alu1 = value
            mov(RegisterSource::Ip, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Ar), // Ar = addr
            alu(false, AluOp::Or),                            // Or = value (0 | value)
            store(false, 0),
            mov(RegisterSource::Ip, RegisterDestination::Alu0),
            alu(true, AluOp::Inc),
            mov(RegisterSource::Or, RegisterDestination::Ip),
        ],
    );

    // STORE addr 0x0A: mem[mem[IP]] = mem[SP]; SP -= 4; IP += 4
    set_micro(
        micro_memory,
        0x0A,
        &[
            mov(RegisterSource::Ip, RegisterDestination::Alu0),
            mov(RegisterSource::Ip, RegisterDestination::Alu1),
            alu(false, AluOp::Xor),
            mov(RegisterSource::Or, RegisterDestination::Alu0), // Alu0 = 0
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Alu1), // Alu1 = value
            mov(RegisterSource::Ip, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Ar), // Ar = addr
            alu(false, AluOp::Or),                            // Or = value
            store(false, 0),
            stack(true), // SP -= 4
            mov(RegisterSource::Ip, RegisterDestination::Alu0),
            alu(true, AluOp::Inc),
            mov(RegisterSource::Or, RegisterDestination::Ip),
        ],
    );

    // SWAP 0x0B (Алгоритм 3-х шагового In-place XOR Swap чтобы уложиться в регистры)
    set_micro(
        micro_memory,
        0x0B,
        &[
            // Шаг 1: mem[SP] = mem[SP] ^ mem[SP-4]
            mov(RegisterSource::Sp, RegisterDestination::Alu0),
            alu(true, AluOp::Dec),
            mov(RegisterSource::Or, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Alu1),
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Alu0),
            alu(false, AluOp::Xor),
            store(false, 0),
            // Шаг 2: mem[SP-4] = mem[SP-4] ^ mem[SP]
            mov(RegisterSource::Or, RegisterDestination::Alu1),
            mov(RegisterSource::Sp, RegisterDestination::Alu0),
            alu(true, AluOp::Dec),
            mov(RegisterSource::Or, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Alu0),
            alu(false, AluOp::Xor),
            store(false, 0),
            // Шаг 3: mem[SP] = mem[SP] ^ mem[SP-4]
            mov(RegisterSource::Or, RegisterDestination::Alu1),
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Alu0),
            alu(false, AluOp::Xor),
            store(false, 0),
        ],
    );

    // DUP 0x0C
    set_micro(
        micro_memory,
        0x0C,
        &[
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            load(false, false, false, 0),
            stack(false),
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            store(false, 0),
        ],
    );

    // OVER 0x0D
    set_micro(
        micro_memory,
        0x0D,
        &[
            mov(RegisterSource::Sp, RegisterDestination::Alu0),
            alu(true, AluOp::Dec),
            mov(RegisterSource::Or, RegisterDestination::Ar),
            load(false, false, false, 0),
            stack(false),
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            store(false, 0),
        ],
    );

    // Binary
    let arith = [
        (0x0E, AluOp::Add),
        (0x0F, AluOp::Sub),
        (0x10, AluOp::Mul),
        (0x11, AluOp::Div),
        (0x12, AluOp::Mod),
        (0x13, AluOp::Or),
        (0x14, AluOp::And),
        (0x15, AluOp::Xor),
        (0x1B, AluOp::Ls),
        (0x1C, AluOp::Rs),
        (0x1D, AluOp::Ars),
        (0x1E, AluOp::Lcs),
        (0x1F, AluOp::Rcs),
    ];
    for (op, alu_op) in arith {
        set_micro(
            micro_memory,
            op,
            &[
                mov(RegisterSource::Sp, RegisterDestination::Ar),
                load(false, false, false, 0),
                mov(RegisterSource::Or, RegisterDestination::Alu1),
                stack(true),
                mov(RegisterSource::Sp, RegisterDestination::Ar),
                load(false, false, false, 0),
                mov(RegisterSource::Or, RegisterDestination::Alu0),
                alu(false, alu_op),
                store(false, 0),
            ],
        );
    }

    // Unary
    let unary = [
        (0x16, AluOp::Not, false),
        (0x17, AluOp::Inc, false),
        (0x18, AluOp::Dec, false),
        (0x19, AluOp::Inc, true),
        (0x1A, AluOp::Dec, true),
    ];
    for (op, alu_op, n) in unary {
        set_micro(
            micro_memory,
            op,
            &[
                mov(RegisterSource::Sp, RegisterDestination::Ar),
                load(false, false, false, 0),
                mov(RegisterSource::Or, RegisterDestination::Alu0),
                alu(n, alu_op),
                store(false, 0),
            ],
        );
    }

    // Jumps
    let jumps = [
        (0x20, JumpCondition::Always),
        (0x21, JumpCondition::Eq),
        (0x22, JumpCondition::Ne),
        (0x23, JumpCondition::Gt),
        (0x24, JumpCondition::Lt),
        (0x25, JumpCondition::Ge),
        (0x26, JumpCondition::Le),
    ];
    for (op, cond) in jumps {
        if op == 0x20 {
            set_micro(
                micro_memory,
                op,
                &[
                    mov(RegisterSource::Ip, RegisterDestination::Ar),
                    load(false, false, false, 0),
                    mov(RegisterSource::Or, RegisterDestination::Ar),
                    jump(JumpCondition::Always),
                ],
            );
        } else {
            set_micro(
                micro_memory,
                op,
                &[
                    mov(RegisterSource::Sp, RegisterDestination::Ar),
                    load(false, false, false, 0),
                    mov(RegisterSource::Or, RegisterDestination::Alu1),
                    stack(true),
                    mov(RegisterSource::Sp, RegisterDestination::Ar),
                    load(false, false, false, 0),
                    mov(RegisterSource::Or, RegisterDestination::Alu0),
                    alu(false, AluOp::Sub),
                    stack(true),
                    mov(RegisterSource::Ip, RegisterDestination::Ar),
                    load(false, false, false, 0),
                    mov(RegisterSource::Or, RegisterDestination::Ar),
                    jump(cond),
                    mov(RegisterSource::Ip, RegisterDestination::Alu0),
                    alu(true, AluOp::Inc),
                    mov(RegisterSource::Or, RegisterDestination::Ip),
                ],
            );
        }
    }

    // CALL 0x27
    set_micro(
        micro_memory,
        0x27,
        &[
            mov(RegisterSource::Ip, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Alu1), // target in Alu1
            mov(RegisterSource::Ip, RegisterDestination::Alu0),
            alu(true, AluOp::Inc),
            mov(RegisterSource::Or, RegisterDestination::Alu0), // ret addr in Alu0
            stack(false),
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            // Or = Alu0 (Передача через инверсии, чтобы сохранить Alu1)
            alu(false, AluOp::Not),
            mov(RegisterSource::Or, RegisterDestination::Alu0),
            alu(false, AluOp::Not),
            store(false, 0),
            // Need target in Ar
            mov(RegisterSource::Ip, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Ar),
            jump(JumpCondition::Always),
        ],
    );

    // RET 0x28
    set_micro(
        micro_memory,
        0x28,
        &[
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Ar),
            stack(true),
            jump(JumpCondition::Always),
        ],
    );

    // WRET 0x29
    set_micro(
        micro_memory,
        0x29,
        &[
            mov(RegisterSource::Sp, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Ar),
            stack(true),
            stack(true),
            jump(JumpCondition::Always),
        ],
    );

    // STORE_R 0x2A: mem[SP - offset] = mem[SP]; IP += 4; SP -= 4
    set_micro(
        micro_memory,
        0x2A,
        &[
            mov(RegisterSource::Ip, RegisterDestination::Ar),
            load(false, false, false, 0),
            mov(RegisterSource::Or, RegisterDestination::Alu1), // offset
            mov(RegisterSource::Sp, RegisterDestination::Alu0),
            alu(false, AluOp::Sub),                           // SP - offset
            mov(RegisterSource::Or, RegisterDestination::Sp), // Временно заменяем SP адресом назначения
            mov(RegisterSource::Sp, RegisterDestination::Alu0),
            alu(false, AluOp::Add), // Sp + offset = Old Sp
            mov(RegisterSource::Or, RegisterDestination::Ar),
            load(false, false, false, 0), // Or = value (от старого SP)
            mov(RegisterSource::Sp, RegisterDestination::Ar), // Ar = addr (наш временный SP)
            store(false, 0),              // Записываем значение по адресу назначения
            mov(RegisterSource::Sp, RegisterDestination::Alu0),
            alu(false, AluOp::Add),                           // Or = Old Sp
            mov(RegisterSource::Or, RegisterDestination::Sp), // Восстанавливаем оригинальный SP
            stack(true),                                      // SP -= 4 согласно спецификации
            mov(RegisterSource::Ip, RegisterDestination::Alu0),
            alu(true, AluOp::Inc),
            mov(RegisterSource::Or, RegisterDestination::Ip),
        ],
    );
}

fn set_micro(micro_memory: &mut [u8; 256 * 32], opcode: u8, instrs: &[u8]) {
    let base = (opcode as usize) << 5;
    for (i, &instr) in instrs.iter().enumerate() {
        if i >= 32 {
            break;
        }
        micro_memory[base + i] = instr;
    }
}
