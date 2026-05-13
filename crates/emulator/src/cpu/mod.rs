use crate::alu::{self, AluOp};
use crate::memory::IO;
use crate::microcode::init::init_micro_memory;
use std::collections::BTreeMap;

pub fn decode_opcode(val: u8) -> String {
    if let Some(op) = shared::Opcode::from_u8(val) {
        match op {
            shared::Opcode::Halt => "HALT",
            shared::Opcode::PushAddr => "PUSH_ADDR",
            shared::Opcode::PushByte => "PUSH_BYTE",
            shared::Opcode::PushSignedByte => "PUSH_SIGNED_BYTE",
            shared::Opcode::PushSignedByteW => "PUSH_SIGNED_BYTE_W",
            shared::Opcode::PushConst => "PUSH_CONST",
            shared::Opcode::PushR => "PUSH_R",
            shared::Opcode::Ext => "EXT",
            shared::Opcode::Pop => "POP",
            shared::Opcode::WriteAddr => "WRITE_ADDR",
            shared::Opcode::StoreAddr => "STORE_ADDR",
            shared::Opcode::Swap => "SWAP",
            shared::Opcode::Dup => "DUP",
            shared::Opcode::Over => "OVER",
            shared::Opcode::Add => "ADD",
            shared::Opcode::Sub => "SUB",
            shared::Opcode::Mul => "MUL",
            shared::Opcode::Div => "DIV",
            shared::Opcode::Mod => "MOD",
            shared::Opcode::Or => "OR",
            shared::Opcode::And => "AND",
            shared::Opcode::Xor => "XOR",
            shared::Opcode::Not => "NOT",
            shared::Opcode::Inc => "INC",
            shared::Opcode::Dec => "DEC",
            shared::Opcode::Inc4 => "INC4",
            shared::Opcode::Dec4 => "DEC4",
            shared::Opcode::Ls => "LS",
            shared::Opcode::Rs => "RS",
            shared::Opcode::Ars => "ARS",
            shared::Opcode::Lcs => "LCS",
            shared::Opcode::Rcs => "RCS",
            shared::Opcode::JumpAddr => "JUMP_ADDR",
            shared::Opcode::JeqAddr => "JEQ_ADDR",
            shared::Opcode::JneAddr => "JNE_ADDR",
            shared::Opcode::JgtAddr => "JGT_ADDR",
            shared::Opcode::JltAddr => "JLT_ADDR",
            shared::Opcode::JgeAddr => "JGE_ADDR",
            shared::Opcode::JleAddr => "JLE_ADDR",
            shared::Opcode::CallAddr => "CALL_ADDR",
            shared::Opcode::Ret => "RET",
            shared::Opcode::Wret => "WRET",
            shared::Opcode::StoreR => "STORE_R",
        }
        .to_string()
    } else {
        format!("UNKNOWN(0x{:02X})", val)
    }
}

pub fn decode_micro(instr: u8) -> String {
    let op_type = instr >> 5;
    let args = instr & 0x1F;
    match op_type {
        0 => "NOOP".to_string(),
        1 => {
            let r = (args >> 4) & 1 != 0;
            let s = (args >> 3) & 1 != 0;
            let y = (args >> 2) & 1 != 0;
            let xx = args & 3;
            format!("LOAD r:{} s:{} y:{} xx:{}", r, s, y, xx)
        }
        2 => {
            let y = (args >> 2) & 1 != 0;
            let xx = args & 3;
            format!("STORE y:{} xx:{}", y, xx)
        }
        3 => {
            let src = match (args >> 3) & 3 {
                0 => "Or",
                1 => "Sp",
                2 => "Ip",
                3 => "Ar",
                _ => "?",
            };
            let dst = match args & 7 {
                0 => "Or",
                1 => "Sp",
                2 => "Ir",
                3 => "Ar",
                4 => "Ip",
                5 => "Alu0",
                6 => "Alu1",
                _ => "?",
            };
            format!("MOV {} -> {}", src, dst)
        }
        4 => {
            let dec = (args & 1) != 0;
            format!("STACK {}", if dec { "DEC" } else { "INC" })
        }
        5 => {
            let n = (args >> 4) & 1 != 0;
            let op = match args & 0x0F {
                0x0 => "Add",
                0x1 => "Sub",
                0x2 => "Mul",
                0x3 => "Div",
                0x4 => "Mod",
                0x5 => "Or",
                0x6 => "And",
                0x7 => "Xor",
                0x8 => "Not",
                0x9 => "Inc",
                0xA => "Dec",
                0xB => "Ls",
                0xC => "Rs",
                0xD => "Ars",
                0xE => "Lcs",
                0xF => "Rcs",
                _ => "?",
            };
            format!("ALU {} n:{}", op, n)
        }
        6 => {
            let cond = match args & 0x0F {
                0x0 => "Always",
                0x1 => "Eq",
                0x2 => "Ne",
                0x3 => "GeU",
                0x4 => "LtU",
                0x5 => "V",
                0x6 => "N",
                0x7 => "Pl",
                0x8 => "Lt",
                0x9 => "Ge",
                0xA => "Gt",
                0xB => "Le",
                _ => "?",
            };
            format!("JUMP {}", cond)
        }
        _ => "UNKNOWN".to_string(),
    }
}

#[cfg(test)]
mod tests;

pub struct Processor<'a> {
    pub ip: u32,
    pub ir: u32,
    pub ar: u32,
    pub sp: u32,
    pub or: u32,
    pub micro_pc: u8,
    pub alu0: u32,
    pub alu1: u32,
    pub n: bool,
    pub z: bool,
    pub v: bool,
    pub c: bool,
    pub trace: u8,
    pub memory: BTreeMap<u32, u32>,
    pub io: BTreeMap<u32, IO<'a>>,
    pub micro_memory: [u8; 256 * 32],
    pub shadow_store: Option<(u32, u32)>,
}

impl<'a> Processor<'a> {
    pub fn new(memory: BTreeMap<u32, u32>, io: BTreeMap<u32, IO<'a>>) -> Self {
        let mut p = Self {
            ip: 0x1000,
            ir: 0,
            ar: 0,
            sp: 0x8000_0000,
            or: 0,
            micro_pc: 0,
            alu0: 0,
            alu1: 0,
            n: false,
            z: false,
            v: false,
            c: false,
            trace: 0,
            memory,
            io,
            micro_memory: [0; 256 * 32],
            shadow_store: None,
        };
        init_micro_memory(&mut p.micro_memory);
        p
    }

    pub fn flush_shadow(&mut self) {
        if let Some((s_addr, s_val)) = self.shadow_store.take() {
            if self.trace > 1 {
                println!(
                    "SUPER | FLUSH | ADDR: 0x{:08X} | VAL: 0x{:08X} (shadow to ram)",
                    s_addr, s_val
                );
            }
            self.memory.insert(s_addr, s_val);
        }
    }

    pub fn read_mem(&mut self, addr: u32) -> u32 {
        let aligned_addr = addr & !3;
        if let Some((s_addr, s_val)) = self.shadow_store {
            if s_addr == aligned_addr {
                if self.trace > 1 {
                    println!(
                        "SUPER | FWD   | ADDR: 0x{:08X} | VAL: 0x{:08X} (from shadow)",
                        aligned_addr, s_val
                    );
                }
                return s_val;
            }
        }
        if aligned_addr == 0x0000_0000 {
            self.flush_shadow();
            if let Some(IO::I(input)) = self.io.get_mut(&0x0000_0000) {
                let mut buf = [0u8; 1];
                if let Ok(1) = input.read(&mut buf) {
                    let val = buf[0] as u32;
                    if self.trace > 1 {
                        println!("READ  | ADDR: 0x{:08X} | VAL: 0x{:08X} (STDIN)", addr, val);
                    }
                    return val;
                }
            }
            return 0;
        }
        let val = *self.memory.get(&aligned_addr).unwrap_or(&0);
        if self.trace > 1 {
            println!("READ  | ADDR: 0x{:08X} | VAL: 0x{:08X}", aligned_addr, val);
        }
        val
    }

    pub fn write_mem(&mut self, addr: u32, val: u32) {
        let aligned_addr = addr & !3;
        if aligned_addr == 0x0000_0004 {
            self.flush_shadow();
            if self.trace > 1 {
                println!(
                    "WRITE | ADDR: 0x{:08X} | VAL: 0x{:08X} (STDOUT)",
                    aligned_addr, val
                );
            }
            if let Some(IO::O(output)) = self.io.get_mut(&0x0000_0004) {
                let _ = output.write(&[val as u8]);
                let _ = output.flush();
            }
            return;
        }
        if let Some((s_addr, s_val)) = self.shadow_store {
            if s_addr == aligned_addr {
                if self.trace > 1 {
                    println!(
                        "SUPER | ELIM  | ADDR: 0x{:08X} | VAL: 0x{:08X} (dead store elim)",
                        aligned_addr, val
                    );
                }
                self.shadow_store = Some((aligned_addr, val));
            } else {
                // Parallel Flush: пишем старое значение в память, а новое кладем в теневой регистр (параллельно)
                if self.trace > 1 {
                    println!(
                        "SUPER | P-FLSH| ADDR1: 0x{:08X} | ADDR2: 0x{:08X} (parallel flush)",
                        s_addr, aligned_addr
                    );
                }
                self.memory.insert(s_addr, s_val);
                self.shadow_store = Some((aligned_addr, val));
            }
        } else {
            if self.trace > 1 {
                println!(
                    "SUPER | DEFER | ADDR: 0x{:08X} | VAL: 0x{:08X} (to shadow)",
                    aligned_addr, val
                );
            }
            self.shadow_store = Some((aligned_addr, val));
        }
    }

    pub fn step(&mut self) -> bool {
        if self.micro_pc == 0 {
            let opcode_addr = self.ip;
            self.ar = self.ip;
            self.ir = self.read_mem(self.ar);

            let opcode = (self.ir & 0xFF) as u8;
            if opcode == 0x00 {
                self.flush_shadow();
                if self.trace > 0 {
                    println!(
                        "INSTR | IP: 0x{:08X} | SP: 0x{:08X} | OP: HALT (0x00)",
                        opcode_addr, self.sp
                    );
                }
                return false;
            }

            if self.trace > 0 {
                let op_name = decode_opcode(opcode);
                println!(
                    "INSTR | IP: 0x{:08X} | SP: 0x{:08X} | OP: {} (0x{:02X})",
                    opcode_addr, self.sp, op_name, opcode
                );
            }

            self.alu0 = self.ip;
            self.alu1 = 4;
            self.execute_alu(AluOp::Add, false);
            self.ip = self.or;
        }

        let opcode = (self.ir & 0xFF) as u8;
        let micro_instr = self.micro_memory[(opcode as usize) << 5 | (self.micro_pc as usize)];

        if micro_instr == 0 {
            self.micro_pc = 0;
            // Returning true to allow stepping to continue to the next instruction
            return true;
        }

        if self.trace > 1 {
            let micro_name = decode_micro(micro_instr);
            println!("  MICRO | uPC: {:02} | {}", self.micro_pc, micro_name);
        }

        let (keep_going, jump_taken) = self.execute_micro_ext(micro_instr);
        if !keep_going || jump_taken {
            self.micro_pc = 0;
        } else {
            self.micro_pc += 1;
            if self.micro_pc >= 32 {
                self.micro_pc = 0;
            }
        }
        true
    }

    fn execute_micro_ext(&mut self, instr: u8) -> (bool, bool) {
        let op_type = instr >> 5;
        let args = instr & 0x1F;
        let mut jump_taken = false;

        match op_type {
            0 => return (false, false),
            1 => {
                // LOAD
                let r = (args >> 4) & 1 != 0;
                let s = (args >> 3) & 1 != 0;
                let y = (args >> 2) & 1 != 0;
                let xx = args & 3;
                let full_word = self.read_mem(self.ar);
                let mut val = full_word;
                if y {
                    let byte = (full_word >> (xx * 8)) & 0xFF;
                    if r {
                        if s {
                            if (full_word & 0x8000_0000) != 0 {
                                val = 0xFFFF_FF00 | byte;
                            } else {
                                val = byte;
                            }
                        } else {
                            val = (byte as i8 as i32) as u32;
                        }
                    } else {
                        val = byte;
                    }
                }
                self.or = val;
            }
            2 => {
                // STORE
                let y = (args >> 2) & 1 != 0;
                let xx = args & 3;
                if y {
                    let mut word = self.read_mem(self.ar);
                    let mask = 0xFF << (xx * 8);
                    word = (word & !mask) | ((self.or & 0xFF) << (xx * 8));
                    self.write_mem(self.ar, word);
                } else {
                    self.write_mem(self.ar, self.or);
                }
            }
            3 => {
                // MOV
                let src_val = match (args >> 3) & 0x03 {
                    0 => self.or,
                    1 => self.sp,
                    2 => self.ip,
                    3 => self.ar,
                    _ => unreachable!(),
                };
                match args & 0x07 {
                    0 => self.or = src_val,
                    1 => self.sp = src_val & !3,
                    2 => self.ir = src_val,
                    3 => self.ar = src_val,
                    4 => self.ip = src_val,
                    5 => self.alu0 = src_val,
                    6 => self.alu1 = src_val,
                    _ => {}
                }
            }
            4 => {
                // STACK
                let dec = (args & 1) != 0;
                if dec {
                    self.sp = self.sp.wrapping_sub(4);
                } else {
                    self.sp = self.sp.wrapping_add(4);
                }
            }
            5 => {
                // ALU
                let n_param = (args >> 4) & 1 != 0;
                let op = AluOp::from_u8(args & 0x0F);
                self.execute_alu(op, n_param);
            }
            6 => {
                // JUMP
                let cond = args & 0x0F;
                if self.check_cond(cond) {
                    self.ip = self.ar;
                    jump_taken = true;
                }
            }
            _ => unreachable!(),
        }
        (true, jump_taken)
    }

    fn execute_alu(&mut self, op: AluOp, n_param: bool) {
        let res = alu::execute(op, self.alu0, self.alu1, n_param);
        self.or = res.val;
        self.n = res.n;
        self.z = res.z;
        self.v = res.v;
        self.c = res.c;
    }

    fn check_cond(&self, cond: u8) -> bool {
        match cond {
            0x0 => true,
            0x1 => self.z,
            0x2 => !self.z,
            0x3 => self.c,
            0x4 => !self.c,
            0x5 => self.v,
            0x6 => self.n,
            0x7 => !self.n,
            0x8 => self.n != self.v,
            0x9 => self.n == self.v,
            0xA => (self.n == self.v) && !self.z,
            0xB => (self.n != self.v) || self.z,
            _ => false,
        }
    }
}
