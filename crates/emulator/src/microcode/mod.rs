pub mod init;

use crate::alu::AluOp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicroOp {
    Noop,
    Load {
        r: bool,
        s: bool,
        y: bool,
        xx: u8,
    },
    Store {
        y: bool,
        xx: u8,
    },
    Mov {
        src: RegisterSource,
        dst: RegisterDestination,
    },
    Stack {
        dec: bool,
    },
    Alu {
        n: bool,
        op: AluOp,
    },
    Jump {
        cond: JumpCondition,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterSource {
    Or = 0,
    Sp = 1,
    Ip = 2,
    Ar = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterDestination {
    Or = 0,
    Sp = 1,
    Ir = 2,
    Ar = 3,
    Ip = 4,
    Alu0 = 5,
    Alu1 = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpCondition {
    Always = 0x0,
    Eq = 0x1,
    Ne = 0x2,
    GeU = 0x3,
    LtU = 0x4,
    V = 0x5,
    N = 0x6,
    Pl = 0x7,
    Lt = 0x8,
    Ge = 0x9,
    Gt = 0xA,
    Le = 0xB,
}

impl MicroOp {
    pub fn encode(&self) -> u8 {
        match self {
            MicroOp::Noop => 0,
            MicroOp::Load { r, s, y, xx } => {
                0x20 | ((*r as u8) << 4) | ((*s as u8) << 3) | ((*y as u8) << 2) | (xx & 0x03)
            }
            MicroOp::Store { y, xx } => 0x40 | ((*y as u8) << 2) | (xx & 0x03),
            MicroOp::Mov { src, dst } => 0x60 | ((*src as u8) << 3) | (*dst as u8),
            MicroOp::Stack { dec } => 0x80 | (*dec as u8),
            MicroOp::Alu { n, op } => 0xA0 | ((*n as u8) << 4) | (*op as u8),
            MicroOp::Jump { cond } => 0xC0 | (*cond as u8),
        }
    }
}

pub fn noop() -> u8 {
    MicroOp::Noop.encode()
}
pub fn load(r: bool, s: bool, y: bool, xx: u8) -> u8 {
    MicroOp::Load { r, s, y, xx }.encode()
}
pub fn store(y: bool, xx: u8) -> u8 {
    MicroOp::Store { y, xx }.encode()
}
pub fn mov(src: RegisterSource, dst: RegisterDestination) -> u8 {
    MicroOp::Mov { src, dst }.encode()
}
pub fn stack(dec: bool) -> u8 {
    MicroOp::Stack { dec }.encode()
}
pub fn alu(n: bool, op: AluOp) -> u8 {
    MicroOp::Alu { n, op }.encode()
}
pub fn jump(cond: JumpCondition) -> u8 {
    MicroOp::Jump { cond }.encode()
}
