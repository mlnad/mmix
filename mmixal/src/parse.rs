use std::collections::HashMap;
use mmix_core::SpecialRegister;

pub(crate) fn parse_reg(s: &str) -> Result<u8, String> {
    let s = s.trim();
    if !s.starts_with('$') {
        return Err(format!("expected register (e.g. $0), got '{}'", s));
    }
    s[1..].parse::<u8>().map_err(|_| format!("invalid register '{}'", s))
}

pub(crate) fn parse_number(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(&s[2..], 16).map_err(|e| format!("invalid hex '{}': {}", s, e))
    } else if s.starts_with('#') {
        u64::from_str_radix(&s[1..], 16).map_err(|e| format!("invalid hex '{}': {}", s, e))
    } else {
        if s.starts_with('-') {
            let v: i64 = s.parse().map_err(|e| format!("invalid number '{}': {}", s, e))?;
            Ok(v as u64)
        } else {
            s.parse::<u64>().map_err(|e| format!("invalid number '{}': {}", s, e))
        }
    }
}

pub(crate) fn parse_reg_or_imm(s: &str) -> Result<(u8, bool), String> {
    let s = s.trim();
    if s.starts_with('$') {
        Ok((parse_reg(s)?, false))
    } else {
        let v = parse_number(s)?;
        if v > 255 {
            return Err(format!("immediate {} out of range 0..255", v));
        }
        Ok((v as u8, true))
    }
}

pub(crate) fn parse_reg_or_imm_with_labels(s: &str, labels: &HashMap<String, u64>, cur_offset: u64) -> Result<(u8, bool), String> {
    let s = s.trim();
    if s.starts_with('$') {
        Ok((parse_reg(s)?, false))
    } else {
        let v = resolve_label_or_number(s, labels, cur_offset)?;
        if v > 255 {
            return Err(format!("immediate {} out of range 0..255", v));
        }
        Ok((v as u8, true))
    }
}

pub(crate) fn parse_special_reg(s: &str) -> Result<u8, String> {
    let s = s.trim().to_lowercase();
    for &sr in &SpecialRegister::ALL {
        if s == sr.name() {
            return Ok(sr.encoding());
        }
    }
    s.parse::<u8>().map_err(|_| format!("unknown special register '{}'", s))
}

pub(crate) fn parse_string_literal(s: &str) -> Result<String, String> {
    if !s.starts_with('"') {
        return Err("expected string literal".into());
    }
    let end = s[1..].find('"').ok_or("unterminated string literal")?;
    let inner = &s[1..1 + end];
    let mut result = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('0') => result.push('\0'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => return Err("trailing backslash".into()),
            }
        } else {
            result.push(c);
        }
    }
    Ok(result)
}

pub(crate) fn resolve_label_or_number(s: &str, labels: &HashMap<String, u64>, cur_offset: u64) -> Result<u64, String> {
    let s = s.trim();
    if s == "@" {
        return Ok(cur_offset);
    }
    if s.starts_with('$') || s.starts_with('#') || s.starts_with("0x") || s.starts_with("0X")
        || s.starts_with('-') || s.chars().next().is_some_and(|c| c.is_ascii_digit())
    {
        parse_number(s)
    } else {
        labels.get(s).copied().ok_or_else(|| format!("undefined label '{}'", s))
    }
}
