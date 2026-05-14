pub enum Opcode {
    Halt = 0x00,
    PushAddr = 0x01,
    PushByte = 0x02,
    PushSignedByte = 0x03,
    PushSignedByteW = 0x04,
    PushConst = 0x05,
    PushR = 0x06,
    Ext = 0x07,
    Pop = 0x08,
    WriteAddr = 0x09,
    StoreAddr = 0x0A,
    Swap = 0x0B,
    Dup = 0x0C,
    Over = 0x0D,
    Add = 0x0E,
    Sub = 0x0F,
    Mul = 0x10,
    Div = 0x11,
    Mod = 0x12,
    Or = 0x13,
    And = 0x14,
    Xor = 0x15,
    Not = 0x16,
    Inc = 0x17,
    Dec = 0x18,
    Inc4 = 0x19,
    Dec4 = 0x1A,
    Ls = 0x1B,
    Rs = 0x1C,
    Ars = 0x1D,
    Lcs = 0x1E,
    Rcs = 0x1F,
    JumpAddr = 0x20,
    JeqAddr = 0x21,
    JneAddr = 0x22,
    JgtAddr = 0x23,
    JltAddr = 0x24,
    JgeAddr = 0x25,
    JleAddr = 0x26,
    CallAddr = 0x27,
    Ret = 0x28,
    Wret = 0x29,
    StoreR = 0x2A,
}

impl Opcode {
    pub fn name(&self) -> &'static str {
        match self {
            Opcode::Halt => "Halt",
            Opcode::PushAddr => "PushAddr",
            Opcode::PushByte => "PushByte",
            Opcode::PushSignedByte => "PushSignedByte",
            Opcode::PushSignedByteW => "PushSignedByteW",
            Opcode::PushConst => "PushConst",
            Opcode::PushR => "PushR",
            Opcode::Ext => "Ext",
            Opcode::Pop => "Pop",
            Opcode::WriteAddr => "WriteAddr",
            Opcode::StoreAddr => "StoreAddr",
            Opcode::Swap => "Swap",
            Opcode::Dup => "Dup",
            Opcode::Over => "Over",
            Opcode::Add => "Add",
            Opcode::Sub => "Sub",
            Opcode::Mul => "Mul",
            Opcode::Div => "Div",
            Opcode::Mod => "Mod",
            Opcode::Or => "Or",
            Opcode::And => "And",
            Opcode::Xor => "Xor",
            Opcode::Not => "Not",
            Opcode::Inc => "Inc",
            Opcode::Dec => "Dec",
            Opcode::Inc4 => "Inc4",
            Opcode::Dec4 => "Dec4",
            Opcode::Ls => "Ls",
            Opcode::Rs => "Rs",
            Opcode::Ars => "Ars",
            Opcode::Lcs => "Lcs",
            Opcode::Rcs => "Rcs",
            Opcode::JumpAddr => "JumpAddr",
            Opcode::JeqAddr => "JeqAddr",
            Opcode::JneAddr => "JneAddr",
            Opcode::JgtAddr => "JgtAddr",
            Opcode::JltAddr => "JltAddr",
            Opcode::JgeAddr => "JgeAddr",
            Opcode::JleAddr => "JleAddr",
            Opcode::CallAddr => "CallAddr",
            Opcode::Ret => "Ret",
            Opcode::Wret => "Wret",
            Opcode::StoreR => "StoreR",
        }
    }

    pub fn has_imm(&self) -> bool {
        matches!(
            self,
            Opcode::PushAddr
                | Opcode::PushByte
                | Opcode::PushSignedByte
                | Opcode::PushSignedByteW
                | Opcode::PushConst
                | Opcode::PushR
                | Opcode::WriteAddr
                | Opcode::StoreAddr
                | Opcode::JumpAddr
                | Opcode::JeqAddr
                | Opcode::JneAddr
                | Opcode::JgtAddr
                | Opcode::JltAddr
                | Opcode::JgeAddr
                | Opcode::JleAddr
                | Opcode::CallAddr
                | Opcode::StoreR
        )
    }

    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Opcode::Halt),
            0x01 => Some(Opcode::PushAddr),
            0x02 => Some(Opcode::PushByte),
            0x03 => Some(Opcode::PushSignedByte),
            0x04 => Some(Opcode::PushSignedByteW),
            0x05 => Some(Opcode::PushConst),
            0x06 => Some(Opcode::PushR),
            0x07 => Some(Opcode::Ext),
            0x08 => Some(Opcode::Pop),
            0x09 => Some(Opcode::WriteAddr),
            0x0A => Some(Opcode::StoreAddr),
            0x0B => Some(Opcode::Swap),
            0x0C => Some(Opcode::Dup),
            0x0D => Some(Opcode::Over),
            0x0E => Some(Opcode::Add),
            0x0F => Some(Opcode::Sub),
            0x10 => Some(Opcode::Mul),
            0x11 => Some(Opcode::Div),
            0x12 => Some(Opcode::Mod),
            0x13 => Some(Opcode::Or),
            0x14 => Some(Opcode::And),
            0x15 => Some(Opcode::Xor),
            0x16 => Some(Opcode::Not),
            0x17 => Some(Opcode::Inc),
            0x18 => Some(Opcode::Dec),
            0x19 => Some(Opcode::Inc4),
            0x1A => Some(Opcode::Dec4),
            0x1B => Some(Opcode::Ls),
            0x1C => Some(Opcode::Rs),
            0x1D => Some(Opcode::Ars),
            0x1E => Some(Opcode::Lcs),
            0x1F => Some(Opcode::Rcs),
            0x20 => Some(Opcode::JumpAddr),
            0x21 => Some(Opcode::JeqAddr),
            0x22 => Some(Opcode::JneAddr),
            0x23 => Some(Opcode::JgtAddr),
            0x24 => Some(Opcode::JltAddr),
            0x25 => Some(Opcode::JgeAddr),
            0x26 => Some(Opcode::JleAddr),
            0x27 => Some(Opcode::CallAddr),
            0x28 => Some(Opcode::Ret),
            0x29 => Some(Opcode::Wret),
            0x2A => Some(Opcode::StoreR),
            _ => None,
        }
    }
}
