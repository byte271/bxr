use crate::{Gpr, Width};

pub const MAX_INSTRUCTION_LEN: usize = 15;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Prefixes {
    pub operand_size_override: bool,
    pub rep: Option<RepPrefix>,
    pub segment: Option<SegmentPrefix>,
    pub rex: Option<Rex>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepPrefix {
    Rep,
    Repne,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SegmentPrefix {
    Cs,
    Ss,
    Ds,
    Es,
    Fs,
    Gs,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rex {
    pub w: bool,
    pub r: bool,
    pub x: bool,
    pub b: bool,
}

impl Rex {
    const fn from_byte(byte: u8) -> Self {
        Self {
            w: (byte & 0b1000) != 0,
            r: (byte & 0b0100) != 0,
            x: (byte & 0b0010) != 0,
            b: (byte & 0b0001) != 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instruction {
    pub len: u8,
    pub bytes: [u8; MAX_INSTRUCTION_LEN],
    pub prefixes: Prefixes,
    pub opcode: u8,
    pub operation: Operation,
}

impl Instruction {
    pub fn operation_code(&self) -> u32 {
        self.operation.code()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Operation {
    Nop,
    Ret,
    Int3,
    Hlt,
    Syscall,
    MovImm { dst: Gpr, width: Width, imm: u64 },
    Push { src: Gpr },
    Pop { dst: Gpr },
    AddAccumulatorImm { width: Width, imm32: u32 },
    JmpRel32 { rel: i32 },
    OutImm8Al { port: u8 },
}

impl Operation {
    pub const fn code(&self) -> u32 {
        match self {
            Self::Nop => 1,
            Self::Ret => 2,
            Self::Int3 => 3,
            Self::Hlt => 4,
            Self::Syscall => 5,
            Self::MovImm { .. } => 6,
            Self::Push { .. } => 7,
            Self::Pop { .. } => 8,
            Self::AddAccumulatorImm { .. } => 9,
            Self::JmpRel32 { .. } => 10,
            Self::OutImm8Al { .. } => 11,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodeError {
    EmptyInput,
    Incomplete,
    TooLong,
    UnsupportedOpcode { opcode: u8, offset: u8 },
}

pub fn decode_one(input: &[u8]) -> Result<Instruction, DecodeError> {
    if input.is_empty() {
        return Err(DecodeError::EmptyInput);
    }

    let mut offset = 0_usize;
    let mut prefixes = Prefixes::default();

    loop {
        if offset >= input.len() {
            return Err(DecodeError::Incomplete);
        }
        if offset >= MAX_INSTRUCTION_LEN {
            return Err(DecodeError::TooLong);
        }

        match input[offset] {
            0x66 => prefixes.operand_size_override = true,
            0xf3 => prefixes.rep = Some(RepPrefix::Rep),
            0xf2 => prefixes.rep = Some(RepPrefix::Repne),
            0x2e => prefixes.segment = Some(SegmentPrefix::Cs),
            0x36 => prefixes.segment = Some(SegmentPrefix::Ss),
            0x3e => prefixes.segment = Some(SegmentPrefix::Ds),
            0x26 => prefixes.segment = Some(SegmentPrefix::Es),
            0x64 => prefixes.segment = Some(SegmentPrefix::Fs),
            0x65 => prefixes.segment = Some(SegmentPrefix::Gs),
            0x40..=0x4f => prefixes.rex = Some(Rex::from_byte(input[offset])),
            _ => break,
        }

        offset += 1;
    }

    let opcode_offset = offset;
    let opcode = *input.get(offset).ok_or(DecodeError::Incomplete)?;
    offset += 1;

    let operation = match opcode {
        0x90 => Operation::Nop,
        0xc3 => Operation::Ret,
        0xcc => Operation::Int3,
        0xf4 => Operation::Hlt,
        0x0f => {
            let subopcode = *input.get(offset).ok_or(DecodeError::Incomplete)?;
            offset += 1;
            match subopcode {
                0x05 => Operation::Syscall,
                _ => {
                    return Err(DecodeError::UnsupportedOpcode {
                        opcode: subopcode,
                        offset: (offset - 1) as u8,
                    });
                }
            }
        }
        0x05 => {
            let imm32 = read_u32(input, offset)?;
            offset += 4;
            Operation::AddAccumulatorImm {
                width: accumulator_width(prefixes),
                imm32,
            }
        }
        0x50..=0x57 => Operation::Push {
            src: Gpr::from_low3(opcode & 0b111, rex_b(prefixes)),
        },
        0x58..=0x5f => Operation::Pop {
            dst: Gpr::from_low3(opcode & 0b111, rex_b(prefixes)),
        },
        0xb8..=0xbf => {
            let width = accumulator_width(prefixes);
            let imm = match width {
                Width::U64 => {
                    let value = read_u64(input, offset)?;
                    offset += 8;
                    value
                }
                Width::U32 => {
                    let value = u64::from(read_u32(input, offset)?);
                    offset += 4;
                    value
                }
                Width::U16 | Width::U8 => unreachable!("accumulator_width only returns U32/U64"),
            };
            Operation::MovImm {
                dst: Gpr::from_low3(opcode & 0b111, rex_b(prefixes)),
                width,
                imm,
            }
        }
        0xe9 => {
            let rel = read_u32(input, offset)? as i32;
            offset += 4;
            Operation::JmpRel32 { rel }
        }
        0xe6 => {
            let port = *input.get(offset).ok_or(DecodeError::Incomplete)?;
            offset += 1;
            Operation::OutImm8Al { port }
        }
        _ => {
            return Err(DecodeError::UnsupportedOpcode {
                opcode,
                offset: opcode_offset as u8,
            });
        }
    };

    if offset > MAX_INSTRUCTION_LEN {
        return Err(DecodeError::TooLong);
    }

    let mut bytes = [0; MAX_INSTRUCTION_LEN];
    bytes[..offset].copy_from_slice(&input[..offset]);

    Ok(Instruction {
        len: offset as u8,
        bytes,
        prefixes,
        opcode,
        operation,
    })
}

fn accumulator_width(prefixes: Prefixes) -> Width {
    if prefixes.rex.map(|rex| rex.w).unwrap_or(false) {
        Width::U64
    } else {
        Width::U32
    }
}

fn rex_b(prefixes: Prefixes) -> bool {
    prefixes.rex.map(|rex| rex.b).unwrap_or(false)
}

fn read_u32(input: &[u8], offset: usize) -> Result<u32, DecodeError> {
    let bytes = input
        .get(offset..offset + 4)
        .ok_or(DecodeError::Incomplete)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u64(input: &[u8], offset: usize) -> Result<u64, DecodeError> {
    let bytes = input
        .get(offset..offset + 8)
        .ok_or(DecodeError::Incomplete)?;
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_nop() {
        let inst = decode_one(&[0x90]).unwrap();
        assert_eq!(inst.len, 1);
        assert_eq!(inst.operation, Operation::Nop);
    }

    #[test]
    fn decodes_mov_rax_imm64() {
        let inst =
            decode_one(&[0x48, 0xb8, 0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11]).unwrap();
        assert_eq!(inst.len, 10);
        assert_eq!(
            inst.operation,
            Operation::MovImm {
                dst: Gpr::Rax,
                width: Width::U64,
                imm: 0x1122_3344_5566_7788
            }
        );
    }

    #[test]
    fn decodes_mov_r8_imm64_with_rex_b() {
        let inst = decode_one(&[0x49, 0xb8, 1, 0, 0, 0, 0, 0, 0, 0]).unwrap();
        assert_eq!(
            inst.operation,
            Operation::MovImm {
                dst: Gpr::R8,
                width: Width::U64,
                imm: 1
            }
        );
    }

    #[test]
    fn decodes_push_r15() {
        let inst = decode_one(&[0x41, 0x57]).unwrap();
        assert_eq!(inst.operation, Operation::Push { src: Gpr::R15 });
    }

    #[test]
    fn incomplete_immediate_is_error() {
        assert_eq!(decode_one(&[0x48, 0xb8, 1]), Err(DecodeError::Incomplete));
    }

    #[test]
    fn unsupported_opcode_is_error() {
        assert_eq!(
            decode_one(&[0x0f, 0xff]),
            Err(DecodeError::UnsupportedOpcode {
                opcode: 0xff,
                offset: 1
            })
        );
    }

    #[test]
    fn decodes_out_imm8_al() {
        let inst = decode_one(&[0xe6, 0xe9]).unwrap();
        assert_eq!(inst.len, 2);
        assert_eq!(inst.operation, Operation::OutImm8Al { port: 0xe9 });
    }
}
