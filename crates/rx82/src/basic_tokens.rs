//! Packed BASIC program image and shared ROM token codec.
#![expect(
    clippy::indexing_slicing,
    reason = "codec cursors are checked against payload lengths; Program owns a validated chain"
)]
use super::{Token, check_string};
use anyhow::{Context as _, Result, bail, ensure};

pub(super) fn keywords() -> impl Iterator<Item = (u8, &'static str)> {
    static TABLE: std::sync::OnceLock<Vec<(u8, &'static str)>> = std::sync::OnceLock::new();
    TABLE
        .get_or_init(|| {
            include_str!("../sys/basic_rom.asm")
                .split_once("TOKEN_TABLE:\n")
                .map_or("", |(_, table)| table)
                .split("TOKEN_TABLE_END:")
                .next()
                .unwrap_or("")
                .lines()
                .filter_map(|line| {
                    let line = line.trim().strip_prefix("data 0x")?;
                    let (code, rest) = line.split_once(", \"")?;
                    Some((u8::from_str_radix(code, 16).ok()?, rest.split('"').next()?))
                })
                .collect()
        })
        .iter()
        .copied()
}

pub(super) fn encode(source: &str) -> Result<Vec<u8>> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut pos = 0_usize;
    let mut reference = false;
    let mut unary = true;
    let mut address = false;
    while let Some(&byte) = bytes.get(pos) {
        let negative_digit = if byte == b'-' && unary {
            bytes[pos.strict_add(1)..]
                .iter()
                .position(|candidate| !candidate.is_ascii_whitespace())
                .map(|offset| pos.strict_add(1).strict_add(offset))
        } else {
            None
        };
        if byte.is_ascii_whitespace() {
            pos = pos.strict_add(1);
            continue;
        }
        if byte == b'"' {
            pos = pos.strict_add(1);
            let start = pos;
            while bytes.get(pos).is_some_and(|&candidate| candidate != b'"') {
                pos = pos.strict_add(1);
            }
            ensure!(pos < bytes.len(), "unterminated string");
            let text = source.get(start..pos).context("invalid source")?;
            check_string(text)?;
            out.extend([0xB1, u8::try_from(text.len())?]);
            out.extend(text.bytes());
            pos = pos.strict_add(1);
            reference = false;
            unary = false;
            address = false;
        } else if byte.is_ascii_alphabetic() {
            let start = pos;
            while bytes.get(pos).is_some_and(u8::is_ascii_alphanumeric) {
                pos = pos.strict_add(1);
            }
            if bytes.get(pos) == Some(&b'$') {
                pos = pos.strict_add(1);
            }
            let word = source
                .get(start..pos)
                .context("invalid source")?
                .to_ascii_uppercase();
            if let Some((code, _)) = keywords().find(|&(_, name)| name == word) {
                out.push(code);
                if code == 0x83 || code == 0x8e {
                    let text = source.get(pos..).context("invalid source")?;
                    ensure!(
                        text.bytes()
                            .all(|candidate| candidate == 9 || (32..127).contains(&candidate)),
                        "invalid string character"
                    );
                    out.extend(text.bytes());
                    break;
                }
                reference = matches!(code, 0x88 | 0x8b | 0x8c | 0x97);
                unary = true;
                address = matches!(code, 0x9b | 0x9c);
            } else {
                out.extend(word.bytes());
                reference = false;
                unary = false;
                address = false;
            }
        } else if byte.is_ascii_digit()
            || negative_digit.is_some_and(|index| bytes[index].is_ascii_digit())
        {
            let negative = byte == b'-';
            if negative {
                pos = negative_digit.context("expected number")?;
            }
            let start = pos;
            while bytes.get(pos).is_some_and(u8::is_ascii_digit) {
                pos = pos.strict_add(1);
            }
            let value: u16 = source
                .get(start..pos)
                .context("invalid source")?
                .parse()
                .context("integer overflow")?;
            ensure!(
                if negative {
                    value <= 0x8000
                } else {
                    reference || address || value <= 0x7FFF
                },
                "integer out of range"
            );
            let value = if negative {
                0_u16.wrapping_sub(value)
            } else {
                value
            };
            out.push(if reference && !negative { 0xb2 } else { 0xb0 });
            out.extend(value.to_le_bytes());
            reference = false;
            unary = false;
            address = false;
        } else {
            let pair = bytes.get(pos..pos.strict_add(2));
            if let Some(code) = match pair {
                Some(b"<>") => Some(0xb3),
                Some(b"<=") => Some(0xb4),
                Some(b">=") => Some(0xb5),
                _ => None,
            } {
                out.push(code);
                pos = pos.strict_add(2);
            } else {
                ensure!(
                    b"+-*/()=<>;,?".contains(&byte),
                    "unexpected character: {}",
                    char::from(byte)
                );
                out.push(if byte == b'?' { 0x90 } else { byte });
                pos = pos.strict_add(1);
            }
            reference = false;
            unary = byte != b')';
        }
    }
    out.push(0);
    Ok(out)
}

pub(super) fn decode(bytes: &[u8]) -> Result<Vec<Token>> {
    let mut out = Vec::new();
    let mut pos = 0_usize;
    while let Some(&byte) = bytes.get(pos) {
        pos = pos.strict_add(1);
        match byte {
            0 => break,
            0xb0 | 0xb2 => {
                let word: [u8; 2] = bytes
                    .get(pos..pos.strict_add(2))
                    .context("truncated number")?
                    .try_into()?;
                out.push(Token::Number(if byte == 0xb2 {
                    i32::from(u16::from_le_bytes(word))
                } else {
                    i32::from(i16::from_le_bytes(word))
                }));
                pos = pos.strict_add(2);
            }
            0xb1 => {
                let length = usize::from(*bytes.get(pos).context("truncated string")?);
                pos = pos.strict_add(1);
                out.push(Token::Text(
                    core::str::from_utf8(
                        bytes
                            .get(pos..pos.strict_add(length))
                            .context("truncated string")?,
                    )?
                    .to_owned(),
                ));
                pos = pos.strict_add(length);
            }
            0xb3..=0xb5 => {
                out.push(Token::Comparison(byte));
            }
            0x80..=0xa6 => {
                let (_, word) = keywords()
                    .find(|&(code, _)| code == byte)
                    .context("invalid token")?;
                out.push(Token::Word(word.to_owned()));
                if byte == 0x8e {
                    break;
                }
                if byte == 0x83 {
                    let end = bytes[pos..]
                        .iter()
                        .position(|&candidate| candidate == 0)
                        .context("unterminated DATA")?;
                    out.push(Token::Text(
                        core::str::from_utf8(&bytes[pos..pos.strict_add(end)])?.to_owned(),
                    ));
                    break;
                }
            }
            b'A'..=b'Z' => {
                let start = pos.strict_sub(1);
                while bytes.get(pos).is_some_and(u8::is_ascii_alphanumeric) {
                    pos = pos.strict_add(1);
                }
                if bytes.get(pos) == Some(&b'$') {
                    pos = pos.strict_add(1);
                }
                out.push(Token::Word(
                    core::str::from_utf8(&bytes[start..pos])?.to_owned(),
                ));
            }
            b' ' | b'\t' => {}
            1..=127 => out.push(Token::Symbol(char::from(byte))),
            _ => bail!("invalid token"),
        }
    }
    Ok(out)
}

pub(super) fn text(bytes: &[u8]) -> Result<String> {
    let mut out = String::new();
    let mut pos = 0_usize;
    while let Some(&byte) = bytes.get(pos) {
        pos = pos.strict_add(1);
        match byte {
            0 => break,
            0xb0 | 0xb2 => {
                let word: [u8; 2] = bytes
                    .get(pos..pos.strict_add(2))
                    .context("truncated number")?
                    .try_into()?;
                out.push_str(&if byte == 0xb2 {
                    u16::from_le_bytes(word).to_string()
                } else {
                    i16::from_le_bytes(word).to_string()
                });
                pos = pos.strict_add(2);
            }
            0xb1 => {
                let len = usize::from(*bytes.get(pos).context("truncated string")?);
                pos = pos.strict_add(1);
                out.push('"');
                out.push_str(core::str::from_utf8(
                    bytes
                        .get(pos..pos.strict_add(len))
                        .context("truncated string")?,
                )?);
                out.push('"');
                pos = pos.strict_add(len);
            }
            0xb3..=0xb5 => out.push_str(match byte {
                0xb3 => "<>",
                0xb4 => "<=",
                _ => ">=",
            }),
            0x80..=0xa6 => {
                let (_, word) = keywords()
                    .find(|&(code, _)| code == byte)
                    .context("invalid token")?;
                if !out.is_empty() && !out.ends_with(' ') && matches!(byte, 0x97..=0x99) {
                    out.push(' ');
                }
                out.push_str(word);
                if byte == 0x83 || byte == 0x8e {
                    let end = bytes[pos..]
                        .iter()
                        .position(|&candidate| candidate == 0)
                        .context("unterminated payload")?;
                    out.push_str(core::str::from_utf8(&bytes[pos..pos.strict_add(end)])?);
                    break;
                }
                if bytes.get(pos) != Some(&0) && (byte < 0x9a || matches!(byte, 0x9c | 0x9e)) {
                    out.push(' ');
                }
            }
            1..=127 => out.push(char::from(byte)),
            _ => bail!("invalid token"),
        }
    }
    Ok(out)
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Program {
    bytes: Vec<u8>,
}
impl Default for Program {
    fn default() -> Self {
        Self { bytes: vec![0, 0] }
    }
}
impl Program {
    pub(super) fn image(&self) -> &[u8] {
        &self.bytes
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "edits cap the image at 32768 bytes"
    )]
    pub(super) fn end(&self) -> u16 {
        0x1000_u16.strict_add(self.bytes.len() as u16).strict_sub(2)
    }
    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.bytes.len() == 2
    }
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }
    pub(super) fn record(&self, address: u16) -> Option<(u16, u16, &[u8])> {
        let offset = usize::from(address.checked_sub(0x1000)?);
        let header: &[u8; 4] = self
            .bytes
            .get(offset..offset.strict_add(4))?
            .try_into()
            .ok()?;
        let next = u16::from_le_bytes([header[0], header[1]]);
        let line = u16::from_le_bytes([header[2], header[3]]);
        Some((
            next,
            line,
            self.bytes
                .get(offset.strict_add(4)..usize::from(next.checked_sub(0x1000)?))?,
        ))
    }
    pub(super) fn find(&self, line: u16) -> Option<u16> {
        let mut address = 0x1000;
        while let Some((next, number, _)) = self.record(address) {
            if number == line {
                return Some(address);
            }
            if number > line {
                break;
            }
            address = next;
        }
        None
    }
    pub(super) fn iter(&self) -> impl Iterator<Item = (u16, &[u8])> {
        let mut offset = 0_usize;
        core::iter::from_fn(move || {
            let next = u16::from_le_bytes([self.bytes[offset], self.bytes[offset.strict_add(1)]]);
            if next == 0 {
                return None;
            }
            let number = u16::from_le_bytes([
                self.bytes[offset.strict_add(2)],
                self.bytes[offset.strict_add(3)],
            ]);
            let start = offset.strict_add(4);
            offset = usize::from(next.strict_sub(0x1000));
            Some((number, &self.bytes[start..offset]))
        })
    }
    pub(super) fn get(&self, line: u16) -> Option<&[u8]> {
        self.iter()
            .find(|&(number, _)| number == line)
            .map(|(_, bytes)| bytes)
    }
    pub(super) fn contains_key(&self, line: u16) -> bool {
        self.get(line).is_some()
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "links are bounded by the checked program window"
    )]
    pub(super) fn edit(&mut self, line: u16, source: &str) -> Result<()> {
        let tokens = if source.is_empty() {
            Vec::new()
        } else {
            encode(source)?
        };
        let mut start = 0_usize;
        let mut old_end = 0;
        for (number, body) in self.iter() {
            if number >= line {
                if number == line {
                    old_end = start.strict_add(4).strict_add(body.len());
                }
                break;
            }
            start = start.strict_add(4).strict_add(body.len());
        }
        old_end = old_end.max(start);
        let size = if tokens.is_empty() {
            0
        } else {
            4_usize.strict_add(tokens.len())
        };
        ensure!(size <= 0x7ffe, "LINE TOO LONG");
        // Long native entries stage after the old chain; retain identical
        // admission rules in the reference interpreter.
        ensure!(
            size <= 0x0504 || size <= 0x8000_usize.strict_sub(self.bytes.len()),
            "LINE TOO LONG"
        );
        let length = self
            .bytes
            .len()
            .strict_sub(old_end.strict_sub(start))
            .strict_add(size);
        ensure!(length <= 0x8000, "PROGRAM FULL");
        let mut record = Vec::new();
        if size != 0 {
            record.extend([0, 0]);
            record.extend(line.to_le_bytes());
            record.extend(tokens);
        }
        self.bytes.splice(start..old_end, record);
        // Links after the shifted tail retain their old lengths; rebuild using
        // the previous link plus the edit delta, without scanning token payloads.
        let delta = size
            .cast_signed()
            .strict_sub(old_end.strict_sub(start).cast_signed());
        let mut offset = 0_usize;
        while offset < self.bytes.len().strict_sub(2) {
            let next = if offset == start && size != 0 {
                offset.strict_add(size)
            } else {
                let old = usize::from(
                    u16::from_le_bytes([self.bytes[offset], self.bytes[offset.strict_add(1)]])
                        .strict_sub(0x1000),
                );
                if offset >= start {
                    old.checked_add_signed(delta).context("invalid chain")?
                } else {
                    old
                }
            };
            self.bytes[offset..offset.strict_add(2)]
                .copy_from_slice(&(0x1000_u16.strict_add(next as u16)).to_le_bytes());
            offset = next;
        }
        Ok(())
    }
}

#[cfg(test)]
#[expect(
    clippy::panic_in_result_fn,
    reason = "fixture assertions return codec errors directly"
)]
mod tests {
    use super::*;
    #[test]
    fn canonical_text_retains_keyword_boundaries() -> Result<()> {
        for source in [
            "POKE 256,7",
            "RANDOMIZE -7",
            "FOR I=- 1 TO 2 STEP 1",
            "PRINT INSTR(\"abc\",MID$(\"abcd\",2,2))",
            "NOTE=1",
            "IF 1<>2 THEN 65535",
        ] {
            let bytes = encode(source)?;
            assert_eq!(encode(&text(&bytes)?)?, bytes, "{source}");
        }
        Ok(())
    }

    #[test]
    fn packed_fixture_and_tail_edits() -> Result<()> {
        let mut program = Program::default();
        program.edit(10, "PRINT \"HI\"")?;
        assert_eq!(
            program.image(),
            &[0x0a, 0x10, 10, 0, 0x90, 0xb1, 2, b'H', b'I', 0, 0, 0]
        );
        assert_eq!(program.end(), 0x100a);
        program.edit(30, "END")?;
        let before = program.clone();
        program.edit(20, "PRINT 20")?;
        assert_eq!(
            program.iter().map(|(n, _)| n).collect::<Vec<_>>(),
            vec![10, 20, 30]
        );
        program.edit(20, "")?;
        assert_eq!(program, before);
        program.edit(10, "REM longer replacement")?;
        assert_eq!(
            program.iter().map(|(n, _)| n).collect::<Vec<_>>(),
            vec![10, 30]
        );
        program.edit(10, "END")?;
        program.edit(30, "")?;
        program.edit(10, "")?;
        assert_eq!(program, Program::default());
        Ok(())
    }
    #[test]
    fn full_window_is_atomic() -> Result<()> {
        let mut program = Program::default();
        program.edit(10, &format!("REM {}", "x".repeat(0x7FF7)))?;
        assert_eq!(program.end(), 0x8ffe);
        let before = program.clone();
        assert!(program.edit(20, "END").is_err());
        assert_eq!(program, before);
        assert!(
            program
                .edit(10, &format!("REM {}", "x".repeat(0x7FF8)))
                .is_err()
        );
        assert_eq!(program, before);
        Ok(())
    }
}
