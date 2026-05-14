fn has_imm(opcode: u8) -> bool {
    matches!(
        opcode,
        0x01 | 0x02 | 0x03 | 0x04 | 0x05 | 0x06 | 0x09 | 0x0A | 0x20..=0x27 | 0x2A
    )
}

fn is_known_opcode(opcode: u8) -> bool {
    matches!(opcode, 0x00..=0x2A)
}

fn inst_name(opcode: u8) -> &'static str {
    match opcode {
        0x00 => "Halt",
        0x01 => "PushAddr",
        0x02 => "PushByte",
        0x03 => "PushSignedByte",
        0x04 => "PushSignedByteW",
        0x05 => "PushConst",
        0x06 => "PushR",
        0x07 => "Ext",
        0x08 => "Pop",
        0x09 => "WriteAddr",
        0x0A => "StoreAddr",
        0x0B => "Swap",
        0x0C => "Dup",
        0x0D => "Over",
        0x0E => "Add",
        0x0F => "Sub",
        0x10 => "Mul",
        0x11 => "Div",
        0x12 => "Mod",
        0x13 => "Or",
        0x14 => "And",
        0x15 => "Xor",
        0x16 => "Not",
        0x17 => "Inc",
        0x18 => "Dec",
        0x19 => "Inc4",
        0x1A => "Dec4",
        0x1B => "Ls",
        0x1C => "Rs",
        0x1D => "Ars",
        0x1E => "Lcs",
        0x1F => "Rcs",
        0x20 => "JumpAddr",
        0x21 => "JeqAddr",
        0x22 => "JneAddr",
        0x23 => "JgtAddr",
        0x24 => "JltAddr",
        0x25 => "JgeAddr",
        0x26 => "JleAddr",
        0x27 => "CallAddr",
        0x28 => "Ret",
        0x29 => "Wret",
        0x2A => "StoreR",
        _ => "db",
    }
}

fn fmt_operand(opcode: u8, imm: u32) -> String {
    match opcode {
        0x01 | 0x09 | 0x0A | 0x20..=0x27 => format!("0x{:04X}", imm),
        0x02 => format!("{}", imm as u8),
        0x03 | 0x04 => format!("{}", imm as i8),
        0x05 => format!("{}", imm as i32),
        0x06 | 0x2A => format!("{}", imm as i32),
        _ => String::new(),
    }
}

fn read_u32_le(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

fn hex_word_val(bytes: &[u8], offset: usize) -> String {
    format!("{:08X}", read_u32_le(bytes, offset))
}

fn is_word_data(word: u32) -> bool {
    let b = word.to_le_bytes();
    !is_known_opcode(b[0]) && b[1..4].iter().any(|&x| (0x20..0x7F).contains(&x))
}

const HEX_WIDTH: usize = 23;

fn pad_hex(hex: &str) -> String {
    let mut s = String::from(hex);
    while s.len() < HEX_WIDTH {
        s.push(' ');
    }
    s
}

pub fn disasm(binary: &[u8]) -> String {
    let mut out = String::new();
    let mut offset: usize = 0;
    let base_addr: u32 = 0x1000;
    let mut in_data = false;

    while offset + 4 <= binary.len() {
        let addr = base_addr + offset as u32;
        let word0 = read_u32_le(binary, offset);
        let b0 = (word0 & 0xFF) as u8;

        let in_data_section =
            in_data || (offset > 0 && b0 == 0x00 && word0 != 0 && is_word_data(word0));

        if in_data_section || (!is_known_opcode(b0) && b0 != 0) {
            in_data = true;
            let chunk = (binary.len() - offset).min(4);
            let mut hex = String::new();
            for i in 0..chunk {
                if i > 0 {
                    hex.push(' ');
                }
                hex.push_str(&format!("{:02X}", binary[offset + i]));
            }
            let printable: String = (offset..offset + chunk)
                .map(|i| {
                    let c = binary[i];
                    if (0x20..0x7F).contains(&c) {
                        c as char
                    } else {
                        '.'
                    }
                })
                .collect();
            out.push_str(&format!(
                "{:08X} - {} - db ; \"{}\"\n",
                addr,
                pad_hex(&hex),
                printable
            ));
            offset += chunk;
            continue;
        }

        if b0 == 0x00 {
            let hex = hex_word_val(binary, offset);
            out.push_str(&format!("{:08X} - {} - Halt\n", addr, pad_hex(&hex)));
            offset += 4;
            continue;
        }

        if has_imm(b0) {
            if offset + 8 > binary.len() {
                break;
            }
            let w0 = hex_word_val(binary, offset);
            let w1 = hex_word_val(binary, offset + 4);
            let hex = format!("{} {}", w0, w1);
            let word1 = read_u32_le(binary, offset + 4);
            let name = inst_name(b0);
            let operand = fmt_operand(b0, word1);
            out.push_str(&format!(
                "{:08X} - {} - {} {}\n",
                addr,
                pad_hex(&hex),
                name,
                operand
            ));
            offset += 8;
        } else {
            let hex = hex_word_val(binary, offset);
            let name = inst_name(b0);
            out.push_str(&format!("{:08X} - {} - {}\n", addr, pad_hex(&hex), name));
            offset += 4;
        }
    }

    if offset < binary.len() {
        out.push_str(&format!(
            "{:08X} - ... {} remaining bytes\n",
            base_addr + offset as u32,
            binary.len() - offset
        ));
    }

    out
}
