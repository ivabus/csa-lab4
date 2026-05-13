use crate::alu::{self, AluOp};
use crate::memory::IO;
use crate::microcode::init::init_micro_memory;
use std::collections::BTreeMap;

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
    pub trace: bool,
    pub memory: BTreeMap<u32, u32>,
    pub io: BTreeMap<u32, IO<'a>>,
    pub micro_memory: [u8; 256 * 32],
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
            trace: false,
            memory,
            io,
            micro_memory: [0; 256 * 32],
        };
        init_micro_memory(&mut p.micro_memory);
        p
    }

    pub fn read_mem(&mut self, addr: u32) -> u32 {
        let aligned_addr = addr & !3;
        if aligned_addr == 0x0000_0000 {
            if let Some(IO::I(input)) = self.io.get_mut(&0x0000_0000) {
                let mut buf = [0u8; 1];
                if let Ok(1) = input.read(&mut buf) {
                    let val = buf[0] as u32;
                    if self.trace {
                        println!("READ  | ADDR: 0x{:08X} | VAL: 0x{:08X} (STDIN)", addr, val);
                    }
                    return val;
                }
            }
            return 0;
        }
        let val = *self.memory.get(&aligned_addr).unwrap_or(&0);
        if self.trace {
            println!("READ  | ADDR: 0x{:08X} | VAL: 0x{:08X}", aligned_addr, val);
        }
        val
    }

    pub fn write_mem(&mut self, addr: u32, val: u32) {
        let aligned_addr = addr & !3;
        if self.trace {
            println!("WRITE | ADDR: 0x{:08X} | VAL: 0x{:08X}", aligned_addr, val);
        }
        if aligned_addr == 0x0000_0004 {
            if let Some(IO::O(output)) = self.io.get_mut(&0x0000_0004) {
                let _ = output.write(&[val as u8]);
                let _ = output.flush();
            }
            return;
        }
        self.memory.insert(aligned_addr, val);
    }

    pub fn step(&mut self) -> bool {
        let opcode_addr = self.ip;
        self.ar = self.ip;
        self.ir = self.read_mem(self.ar);

        if self.trace {
            println!(
                "TICK  | IP: 0x{:08X} | SP: 0x{:08X} | OP: 0x{:02X}",
                opcode_addr,
                self.sp,
                self.ir & 0xFF
            );
        }

        let opcode = (self.ir & 0xFF) as u8;
        if opcode == 0x00 {
            return false;
        }

        self.alu0 = self.ip;
        self.alu1 = 4;
        self.execute_alu(AluOp::Add, false);
        self.ip = self.or;

        self.micro_pc = 0;

        loop {
            let micro_instr = self.micro_memory[(opcode as usize) << 5 | (self.micro_pc as usize)];
            if micro_instr == 0 {
                break;
            }

            let (keep_going, jump_taken) = self.execute_micro_ext(micro_instr);
            if !keep_going || jump_taken {
                break;
            }
            self.micro_pc += 1;
            if self.micro_pc >= 32 {
                break;
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
