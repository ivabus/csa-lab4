#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AluOp {
    Add = 0x0,
    Sub = 0x1,
    Mul = 0x2,
    Div = 0x3,
    Mod = 0x4,
    Or = 0x5,
    And = 0x6,
    Xor = 0x7,
    Not = 0x8,
    Inc = 0x9,
    Dec = 0xA,
    Ls = 0xB,
    Rs = 0xC,
    Ars = 0xD,
    Lcs = 0xE,
    Rcs = 0xF,
}

impl AluOp {
    pub fn from_u8(val: u8) -> Self {
        match val & 0x0F {
            0x0 => AluOp::Add,
            0x1 => AluOp::Sub,
            0x2 => AluOp::Mul,
            0x3 => AluOp::Div,
            0x4 => AluOp::Mod,
            0x5 => AluOp::Or,
            0x6 => AluOp::And,
            0x7 => AluOp::Xor,
            0x8 => AluOp::Not,
            0x9 => AluOp::Inc,
            0xA => AluOp::Dec,
            0xB => AluOp::Ls,
            0xC => AluOp::Rs,
            0xD => AluOp::Ars,
            0xE => AluOp::Lcs,
            0xF => AluOp::Rcs,
            _ => unreachable!(),
        }
    }
}

pub struct AluResult {
    pub val: u32,
    pub n: bool,
    pub z: bool,
    pub v: bool,
    pub c: bool,
}

pub fn execute(op: AluOp, a: u32, b: u32, n_param: bool) -> AluResult {
    let mut carry = false;
    let mut overflow = false;

    let res = match op {
        AluOp::Add => {
            let (r, c) = a.overflowing_add(b);
            carry = c;
            overflow = ((a ^ r) & (b ^ r) & 0x8000_0000) != 0;
            r
        }
        AluOp::Sub => {
            let (r, c) = a.overflowing_sub(b);
            carry = c;
            overflow = ((a ^ b) & (a ^ r) & 0x8000_0000) != 0;
            r
        }
        AluOp::Mul => a.wrapping_mul(b),
        AluOp::Div => {
            if b != 0 {
                a.wrapping_div(b)
            } else {
                0
            }
        }
        AluOp::Mod => {
            if b != 0 {
                a.wrapping_rem(b)
            } else {
                0
            }
        }
        AluOp::Or => a | b,
        AluOp::And => a & b,
        AluOp::Xor => a ^ b,
        AluOp::Not => !a,
        AluOp::Inc => {
            let val = if n_param { 4 } else { 1 };
            a.wrapping_add(val)
        }
        AluOp::Dec => {
            let val = if n_param { 4 } else { 1 };
            a.wrapping_sub(val)
        }
        AluOp::Ls => a.wrapping_shl(b & 0x1F),
        AluOp::Rs => a.wrapping_shr(b & 0x1F),
        AluOp::Ars => (a as i32).wrapping_shr(b & 0x1F) as u32,
        AluOp::Lcs => a.rotate_left(b & 0x1F),
        AluOp::Rcs => a.rotate_right(b & 0x1F),
    };

    AluResult {
        val: res,
        n: (res & 0x8000_0000) != 0,
        z: res == 0,
        v: overflow,
        c: carry,
    }
}
