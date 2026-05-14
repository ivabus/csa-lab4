use shared::Opcode;

fn fmt_operand(opcode: Opcode, imm: u32) -> String {
    match opcode {
        Opcode::PushAddr | Opcode::WriteAddr | Opcode::StoreAddr => format!("0x{:04X}", imm),
        Opcode::PushByte => format!("{}", imm as u8),
        Opcode::PushSignedByte | Opcode::PushSignedByteW => format!("{}", imm as i8),
        Opcode::PushConst => format!("{}", imm as i32),
        Opcode::PushR | Opcode::StoreR => format!("{}", imm as i32),
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
    let op = shared::Opcode::from_u8(b[0]);
    op.is_none() && b[1..4].iter().any(|&x| (0x20..0x7F).contains(&x))
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

        let op = Opcode::from_u8(b0);

        let in_data_section =
            in_data || (offset > 0 && b0 == 0x00 && word0 != 0 && is_word_data(word0));

        if in_data_section || (op.is_none() && b0 != 0) {
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

        if let Some(opcode) = op {
            if opcode.has_imm() {
                if offset + 8 > binary.len() {
                    break;
                }
                let w0 = hex_word_val(binary, offset);
                let w1 = hex_word_val(binary, offset + 4);
                let hex = format!("{} {}", w0, w1);
                let word1 = read_u32_le(binary, offset + 4);
                let name = opcode.name();
                let operand = fmt_operand(opcode, word1);
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
                let name = opcode.name();
                out.push_str(&format!("{:08X} - {} - {}\n", addr, pad_hex(&hex), name));
                offset += 4;
            }
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
