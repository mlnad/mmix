use std::collections::HashMap;
use mmix_core::{op, NAME_TABLE, FORMAT_TABLE, OperandFormat};
use crate::parse::*;

#[derive(Debug, Clone, Copy)]
pub(crate) enum AliasLowering {
    Direct { target_op: u8, format: OperandFormat },
    BySecondOperand {
        reg_op: u8,
        reg_fixed_z: u8,
        imm_op: u8,
        imm_format: OperandFormat,
    },
}

pub(crate) struct AliasEntry {
    pub name: &'static str,
    pub lowering: AliasLowering,
}

pub(crate) static ALIASES: &[AliasEntry] = &[
    AliasEntry {
        name: "LDA",
        lowering: AliasLowering::Direct {
            target_op: op::ADDU,
            format: OperandFormat::ThreeReg,
        },
    },
    AliasEntry {
        name: "SET",
        lowering: AliasLowering::BySecondOperand {
            reg_op: op::ORI,
            reg_fixed_z: 0,
            imm_op: op::SETL,
            imm_format: OperandFormat::RegImm16,
        },
    },
];

#[derive(Debug, Clone, Copy)]
pub(crate) enum InstrEntry {
    Real { base_op: u8, format: OperandFormat },
    Alias(&'static AliasLowering),
}

pub(crate) fn build_opcode_table() -> HashMap<&'static str, InstrEntry> {
    let mut m = HashMap::new();
    for i in 0u16..256 {
        let fmt = FORMAT_TABLE[i as usize];
        let is_odd = i & 1 != 0;
        let is_auto_suffix_format = matches!(
            fmt,
            OperandFormat::ThreeReg
                | OperandFormat::NegStyle
                | OperandFormat::Branch
                | OperandFormat::Jump
                | OperandFormat::Put
                | OperandFormat::PushJ
        );
        if is_odd && is_auto_suffix_format {
            continue;
        }
        let name = NAME_TABLE[i as usize];
        m.insert(name, InstrEntry::Real { base_op: i as u8, format: fmt });
    }
    for alias in ALIASES {
        m.insert(alias.name, InstrEntry::Alias(&alias.lowering));
    }
    m
}

pub(crate) fn build_mnemonic_set(optable: &HashMap<&str, InstrEntry>) -> std::collections::HashSet<String> {
    let mut s: std::collections::HashSet<String> = optable.keys().map(|k| k.to_string()).collect();
    for &name in &NAME_TABLE {
        s.insert(name.to_string());
    }
    for &pseudo in &["BYTE", "WYDE", "TETRA", "OCTA", "IS"] {
        s.insert(pseudo.to_string());
    }
    s
}

pub(crate) fn encode_instruction(
    base_op: u8,
    format: OperandFormat,
    args: &[&str],
    cur_offset: u64,
    labels: &HashMap<String, u64>,
    line: usize,
) -> Result<u32, crate::AssembleError> {
    let err = |msg: String| crate::AssembleError { line, message: msg };

    match format {
        OperandFormat::ThreeReg => {
            if args.len() != 3 {
                return Err(err(format!("expected 3 operands, got {}", args.len())));
            }
            let x = parse_reg(args[0]).map_err(|e| err(e))?;
            let y = parse_reg(args[1]).map_err(|e| err(e))?;
            let (z, is_imm) = parse_reg_or_imm_with_labels(args[2], labels, cur_offset).map_err(|e| err(e))?;
            let op = if is_imm { base_op + 1 } else { base_op };
            Ok(((op as u32) << 24) | ((x as u32) << 16) | ((y as u32) << 8) | (z as u32))
        }
        OperandFormat::NegStyle => {
            if args.len() != 3 {
                return Err(err(format!("expected 3 operands, got {}", args.len())));
            }
            let x = parse_reg(args[0]).map_err(|e| err(e))?;
            let y_val = resolve_label_or_number(args[1].trim(), labels, cur_offset).map_err(|e| err(e))? as u8;
            let (z, is_imm) = parse_reg_or_imm_with_labels(args[2], labels, cur_offset).map_err(|e| err(e))?;
            let op = if is_imm { base_op + 1 } else { base_op };
            Ok(((op as u32) << 24) | ((x as u32) << 16) | ((y_val as u32) << 8) | (z as u32))
        }
        OperandFormat::RegImm16 => {
            if args.len() != 2 {
                return Err(err(format!("expected 2 operands, got {}", args.len())));
            }
            let x = parse_reg(args[0]).map_err(|e| err(e))?;
            let yz = resolve_label_or_number(args[1].trim(), labels, cur_offset).map_err(|e| err(e))? as u16;
            Ok(((base_op as u32) << 24) | ((x as u32) << 16) | (yz as u32))
        }
        OperandFormat::Branch => {
            if args.len() != 2 {
                return Err(err(format!("expected 2 operands, got {}", args.len())));
            }
            let x = parse_reg(args[0]).map_err(|e| err(e))?;
            let target = resolve_label_or_number(args[1], labels, cur_offset).map_err(|e| err(e))?;
            let diff = target as i64 - cur_offset as i64;
            if diff < 0 {
                let offset = ((-diff) / 4) as u64;
                let yz = (0x10000 - offset) as u16;
                Ok((((base_op + 1) as u32) << 24) | ((x as u32) << 16) | (yz as u32))
            } else {
                let yz = (diff / 4) as u16;
                Ok(((base_op as u32) << 24) | ((x as u32) << 16) | (yz as u32))
            }
        }
        OperandFormat::Jump => {
            if args.len() != 1 {
                return Err(err(format!("expected 1 operand, got {}", args.len())));
            }
            let target = resolve_label_or_number(args[0], labels, cur_offset).map_err(|e| err(e))?;
            let diff = target as i64 - cur_offset as i64;
            if diff < 0 {
                let offset = ((-diff) / 4) as u64;
                let xyz = (0x1000000u64 - offset) as u32 & 0xFFFFFF;
                Ok((((base_op + 1) as u32) << 24) | xyz)
            } else {
                let xyz = (diff / 4) as u32 & 0xFFFFFF;
                Ok(((base_op as u32) << 24) | xyz)
            }
        }
        OperandFormat::Trap => {
            if args.len() != 3 {
                return Err(err(format!("expected 3 operands, got {}", args.len())));
            }
            let x = parse_number(args[0].trim()).map_err(|e| err(e))? as u8;
            let y = parse_number(args[1].trim()).map_err(|e| err(e))? as u8;
            let z = parse_number(args[2].trim()).map_err(|e| err(e))? as u8;
            Ok(((base_op as u32) << 24) | ((x as u32) << 16) | ((y as u32) << 8) | (z as u32))
        }
        OperandFormat::Get => {
            if args.len() != 2 {
                return Err(err(format!("expected 2 operands, got {}", args.len())));
            }
            let x = parse_reg(args[0]).map_err(|e| err(e))?;
            let sr = parse_special_reg(args[1]).map_err(|e| err(e))?;
            Ok(((base_op as u32) << 24) | ((x as u32) << 16) | (sr as u32))
        }
        OperandFormat::Put => {
            if args.len() != 2 {
                return Err(err(format!("expected 2 operands, got {}", args.len())));
            }
            let sr = parse_special_reg(args[0]).map_err(|e| err(e))?;
            let (z, is_imm) = parse_reg_or_imm(args[1]).map_err(|e| err(e))?;
            let op = if is_imm { base_op + 1 } else { base_op };
            Ok(((op as u32) << 24) | ((sr as u32) << 16) | (z as u32))
        }
        OperandFormat::Pop => {
            if args.len() != 2 {
                return Err(err(format!("expected 2 operands, got {}", args.len())));
            }
            let x = parse_number(args[0].trim()).map_err(|e| err(e))? as u8;
            let yz = parse_number(args[1].trim()).map_err(|e| err(e))? as u16;
            Ok(((base_op as u32) << 24) | ((x as u32) << 16) | (yz as u32))
        }
        OperandFormat::PushJ => {
            // PUSHJ $X,Label  or  PUSHGO $X,$Y,$Z
            // PUSHJ is branch-like (reg + 24-bit relative), PUSHGO is ThreeReg-like
            // Distinguish by opcode: PUSHJ/PUSHJB use Branch encoding, PUSHGO uses ThreeReg
            if base_op == op::PUSHGO {
                // PUSHGO $X,$Y,$Z — ThreeReg format
                if args.len() != 3 {
                    return Err(err(format!("expected 3 operands, got {}", args.len())));
                }
                let x = parse_reg(args[0]).map_err(|e| err(e))?;
                let y = parse_reg(args[1]).map_err(|e| err(e))?;
                let (z, is_imm) = parse_reg_or_imm_with_labels(args[2], labels, cur_offset).map_err(|e| err(e))?;
                let op = if is_imm { base_op + 1 } else { base_op };
                Ok(((op as u32) << 24) | ((x as u32) << 16) | ((y as u32) << 8) | (z as u32))
            } else {
                // PUSHJ $X,Label — Branch-like
                if args.len() != 2 {
                    return Err(err(format!("expected 2 operands, got {}", args.len())));
                }
                let x = parse_reg(args[0]).map_err(|e| err(e))?;
                let target = resolve_label_or_number(args[1], labels, cur_offset).map_err(|e| err(e))?;
                let diff = target as i64 - cur_offset as i64;
                if diff < 0 {
                    let offset = ((-diff) / 4) as u64;
                    let yz = (0x10000 - offset) as u16;
                    Ok((((base_op + 1) as u32) << 24) | ((x as u32) << 16) | (yz as u32))
                } else {
                    let yz = (diff / 4) as u16;
                    Ok(((base_op as u32) << 24) | ((x as u32) << 16) | (yz as u32))
                }
            }
        }
        OperandFormat::Special => {
            // RESUME, SAVE, UNSAVE, SYNC, SWYM — encode with raw XYZ from args
            // Most take 0 or minimal operands
            match args.len() {
                0 => Ok((base_op as u32) << 24),
                1 => {
                    // e.g. RESUME 0, UNSAVE $Z, SAVE $X
                    let val = if args[0].trim().starts_with('$') {
                        parse_reg(args[0]).map_err(|e| err(e))? as u32
                    } else {
                        parse_number(args[0].trim()).map_err(|e| err(e))? as u32
                    };
                    // For SAVE: $X goes in X field; for UNSAVE: $Z goes in Z field; for RESUME/SYNC: goes in Z
                    if base_op == op::SAVE {
                        Ok(((base_op as u32) << 24) | (val << 16))
                    } else if base_op == op::UNSAVE {
                        Ok(((base_op as u32) << 24) | val)
                    } else {
                        Ok(((base_op as u32) << 24) | val)
                    }
                }
                _ => Err(err(format!("too many operands for special instruction")))
            }
        }
    }
}

pub(crate) fn encode_alias(
    lowering: &AliasLowering,
    args: &[&str],
    cur_offset: u64,
    labels: &HashMap<String, u64>,
    line: usize,
) -> Result<u32, crate::AssembleError> {
    use AliasLowering::*;
    let err = |msg: String| crate::AssembleError { line, message: msg };
    match lowering {
        Direct { target_op, format } => {
            encode_instruction(*target_op, *format, args, cur_offset, labels, line)
        }
        BySecondOperand { reg_op, reg_fixed_z, imm_op, imm_format } => {
            if args.len() != 2 {
                return Err(err(format!("expected 2 operands, got {}", args.len())));
            }
            let x = parse_reg(args[0]).map_err(|e| err(e))?;
            let second = args[1].trim();
            if second.starts_with('$') {
                let y = parse_reg(second).map_err(|e| err(e))?;
                Ok(((*reg_op as u32) << 24) | ((x as u32) << 16) | ((y as u32) << 8) | (*reg_fixed_z as u32))
            } else {
                encode_instruction(*imm_op, *imm_format, args, cur_offset, labels, line)
            }
        }
    }
}
