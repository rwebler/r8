//! A small, host-side BASIC interpreter with integers and strings.
#![allow(
    clippy::arbitrary_source_item_ordering,
    reason = "keep parsing and execution helpers in reading order"
)]
extern crate alloc;
use alloc::collections::BTreeMap;
use anyhow::{Context as _, Result, bail, ensure};
use std::io::{BufRead, Write};

/// BASIC program storage, signed 16-bit integers, arrays, and ASCII strings.
#[derive(Default)]
pub struct Basic {
    // Reference-only byte memory; native BASIC uses the RX-82 bus instead.
    memory: BTreeMap<u16, u8>,
    data_line: u16,
    data_offset: Option<usize>,
    arrays: BTreeMap<String, Vec<i16>>,
    strings: BTreeMap<String, String>,
    lines: BTreeMap<u16, String>,
    vars: BTreeMap<String, i16>,
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(i32),
    Word(String),
    Text(String),
    Symbol(char),
}

fn lex(source: &str) -> Result<Vec<Token>> {
    let mut chars = source.chars().peekable();
    let mut tokens = Vec::new();
    while let Some(character) = chars.next() {
        if character.is_whitespace() {
            continue;
        }
        if character == '"' {
            let mut value = String::new();
            loop {
                let next = chars.next().context("unterminated string")?;
                if next == '"' {
                    break;
                }
                value.push(next);
            }
            tokens.push(Token::Text(value));
        } else if character.is_ascii_digit() {
            let mut value = character.to_string();
            while chars.peek().is_some_and(char::is_ascii_digit) {
                if let Some(next) = chars.next() {
                    value.push(next);
                }
            }
            tokens.push(Token::Number(
                value.parse().context("integer out of range")?,
            ));
        } else if character.is_ascii_alphabetic() {
            let mut value = character.to_string();
            while chars.peek().is_some_and(char::is_ascii_alphanumeric) {
                if let Some(next) = chars.next() {
                    value.push(next);
                }
            }
            if chars.peek() == Some(&'$') {
                chars.next();
                value.push('$');
            }
            let word = value.to_ascii_uppercase();
            let remark = word == "REM";
            let data = word == "DATA";
            tokens.push(Token::Word(word));
            if data {
                tokens.push(Token::Text(chars.collect()));
                break;
            }
            if remark {
                break;
            }
        } else {
            ensure!(
                "+-*/()=<>;,?".contains(character),
                "unexpected character: {character}"
            );
            tokens.push(Token::Symbol(character));
        }
    }
    Ok(tokens)
}

struct Parser<'source> {
    memory: &'source BTreeMap<u16, u8>,
    tokens: &'source [Token],
    pos: usize,
    vars: &'source BTreeMap<String, i16>,
    arrays: &'source BTreeMap<String, Vec<i16>>,
    strings: &'source BTreeMap<String, String>,
}

fn check_string(value: &str) -> Result<()> {
    ensure!(value.len() <= 63, "string too long");
    ensure!(
        value
            .bytes()
            .all(|byte| byte == b'\t' || (b' '..=b'~').contains(&byte)),
        "invalid string character"
    );
    Ok(())
}

fn data_body(source: &str) -> Option<&str> {
    let source = source.trim_start();
    if !source.get(..4)?.eq_ignore_ascii_case("DATA") {
        return None;
    }
    let body = source.get(4..)?;
    if body.starts_with(|character: char| character.is_ascii_alphanumeric() || character == '$') {
        return None;
    }
    Some(body)
}

#[expect(
    clippy::single_call_fn,
    reason = "keep DATA field parsing separate from cursor traversal"
)]
fn data_constant(source: &str) -> Result<(String, bool, Option<usize>)> {
    let trimmed = source.trim_start_matches([' ', '\t']);
    let (value, quoted, rest) = if let Some(text) = trimmed.strip_prefix('"') {
        let (text, rest) = text.split_once('"').context("unterminated string")?;
        (text, true, rest.trim_start_matches([' ', '\t']))
    } else {
        let end = trimmed.find(',').unwrap_or(trimmed.len());
        let text = trimmed
            .get(..end)
            .context("invalid DATA")?
            .trim_end_matches([' ', '\t']);
        ensure!(
            !text.is_empty() && !text.contains(['"', ':']),
            "invalid DATA"
        );
        (text, false, trimmed.get(end..).context("invalid DATA")?)
    };
    let next = if rest.is_empty() {
        None
    } else {
        ensure!(rest.starts_with(','), "invalid DATA");
        Some(source.len().saturating_sub(rest.len()).saturating_add(1))
    };
    Ok((value.to_owned(), quoted, next))
}

impl Parser<'_> {
    fn is_string(&self) -> bool {
        let first = self
            .tokens
            .iter()
            .skip(self.pos)
            .find(|token| **token != Token::Symbol('('));
        matches!(first, Some(Token::Text(_)))
            || matches!(first, Some(Token::Word(name)) if name.ends_with('$'))
    }
    fn string_expression(&mut self) -> Result<String> {
        let mut value = String::new();
        loop {
            let part = match self.next() {
                Some(Token::Text(text)) => text,
                Some(Token::Word(name)) if name.ends_with('$') => {
                    self.strings.get(&name).cloned().unwrap_or_default()
                }
                Some(Token::Symbol('(')) => {
                    let text = self.string_expression()?;
                    ensure!(self.symbol(')'), "expected )");
                    text
                }
                _ => bail!("type mismatch"),
            };
            value.push_str(&part);
            check_string(&value)?;
            if !self.symbol('+') {
                return Ok(value);
            }
        }
    }
    fn length(&mut self) -> Result<i16> {
        ensure!(self.symbol('('), "expected (");
        let size = if self.is_string() {
            self.string_expression()?.len()
        } else {
            let Some(Token::Word(name)) = self.next() else {
                bail!("type mismatch");
            };
            ensure!(
                self.tokens.get(self.pos) == Some(&Token::Symbol(')')),
                "type mismatch"
            );
            self.arrays
                .get(&name)
                .context("array not dimensioned")?
                .len()
        };
        ensure!(self.symbol(')'), "expected )");
        i16::try_from(size).context("integer overflow")
    }
    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        self.pos = self.pos.saturating_add(1);
        token
    }
    fn symbol(&mut self, symbol: char) -> bool {
        if self.tokens.get(self.pos) == Some(&Token::Symbol(symbol)) {
            self.pos = self.pos.saturating_add(1);
            true
        } else {
            false
        }
    }
    fn word(&mut self, word: &str) -> bool {
        if self.tokens.get(self.pos) == Some(&Token::Word(word.to_owned())) {
            self.pos = self.pos.saturating_add(1);
            true
        } else {
            false
        }
    }
    fn end(&self) -> Result<()> {
        ensure!(self.pos >= self.tokens.len(), "unexpected trailing input");
        Ok(())
    }
    fn target(&mut self) -> Result<u16> {
        // Line labels keep their unsigned 16-bit range, independently of values.
        if let Some(Token::Number(number)) = self.tokens.get(self.pos).cloned()
            && self.pos.saturating_add(1) == self.tokens.len()
        {
            self.pos = self.pos.saturating_add(1);
            return u16::try_from(number).context("invalid line number");
        }
        u16::try_from(self.expression()?).context("invalid line number")
    }
    fn subscript(&mut self, name: &str) -> Result<Option<usize>> {
        if !self.symbol('(') {
            return Ok(None);
        }
        let index = self.expression()?;
        ensure!(self.symbol(')'), "expected )");
        let array = self.arrays.get(name).context("array not dimensioned")?;
        let index = usize::try_from(index).context("subscript out of range")?;
        ensure!(index < array.len(), "subscript out of range");
        Ok(Some(index))
    }
    fn address(&mut self) -> Result<u16> {
        // A bare unsigned literal reaches the whole address space. Computed
        // addresses retain ordinary signed arithmetic and use the resulting bits.
        if let Some(Token::Number(number)) = self.tokens.get(self.pos).cloned()
            && matches!(
                self.tokens.get(self.pos.saturating_add(1)),
                Some(Token::Symbol(',' | ')'))
            )
        {
            self.pos = self.pos.saturating_add(1);
            return u16::try_from(number).context("integer overflow");
        }
        Ok(u16::from_le_bytes(self.expression()?.to_le_bytes()))
    }
    fn expression(&mut self) -> Result<i16> {
        self.binary(0)
    }
    fn binary(&mut self, min: u8) -> Result<i16> {
        let mut left = match self.next().context("expected expression")? {
            Token::Number(n) => i16::try_from(n).context("integer out of range")?,
            Token::Word(name) if name == "LEN" => self.length()?,
            Token::Word(name) if name == "PEEK" => {
                ensure!(self.symbol('('), "expected (");
                let address = self.address()?;
                ensure!(self.symbol(')'), "expected )");
                i16::from(self.memory.get(&address).copied().unwrap_or_default())
            }
            Token::Word(name) => {
                ensure!(!name.ends_with('$'), "type mismatch");
                if let Some(index) = self.subscript(&name)? {
                    *self
                        .arrays
                        .get(&name)
                        .and_then(|array| array.get(index))
                        .context("subscript out of range")?
                } else {
                    self.vars.get(&name).copied().unwrap_or_default()
                }
            }
            Token::Symbol('-') => {
                if self.tokens.get(self.pos) == Some(&Token::Number(0x8000)) {
                    self.pos = self.pos.saturating_add(1);
                    i16::MIN
                } else {
                    self.binary(3)?.checked_neg().context("integer overflow")?
                }
            }
            Token::Symbol('+') => self.binary(3)?,
            Token::Symbol('(') => {
                let value = self.expression()?;
                ensure!(self.symbol(')'), "expected )");
                value
            }
            Token::Text(_) => bail!("type mismatch"),
            Token::Symbol(_) => bail!("expected expression"),
        };
        while let Some(Token::Symbol(op)) = self.tokens.get(self.pos).cloned() {
            let precedence = match op {
                '+' | '-' => 1,
                '*' | '/' => 2,
                _ => break,
            };
            if precedence < min {
                break;
            }
            self.pos = self.pos.saturating_add(1);
            let right = self.binary(precedence.saturating_add(1))?;
            left = match op {
                '+' => left.checked_add(right),
                '-' => left.checked_sub(right),
                '*' => left.checked_mul(right),
                '/' => {
                    ensure!(right != 0_i16, "division by zero");
                    left.checked_div(right)
                }
                _ => unreachable!(),
            }
            .context("integer overflow")?;
        }
        Ok(left)
    }
}

enum Flow {
    Next,
    Jump(u16),
    Call(u16),
    Return,
    End,
    For {
        name: String,
        start: i16,
        limit: i16,
        step: i16,
    },
    NextLoop,
}

struct LoopFrame {
    name: String,
    limit: i16,
    step: i16,
    start_line: u16,
    end_line: u16,
    body: Option<u16>,
}

/// Match lexical loop boundaries without evaluating their expressions.
#[expect(
    clippy::single_call_fn,
    reason = "separate structural loop validation from execution"
)]
fn loop_pairs(program: &BTreeMap<u16, Vec<Token>>) -> Result<BTreeMap<u16, u16>> {
    let mut pending: Vec<(u16, String)> = Vec::new();
    let mut pairs = BTreeMap::new();
    for (&line, tokens) in program {
        match tokens.first().cloned() {
            Some(Token::Word(word)) if word == "FOR" => {
                let Some(Token::Word(name)) = tokens.get(1).cloned() else {
                    bail!("expected FOR variable in line {line}");
                };
                pending.push((line, name));
            }
            Some(Token::Word(word)) if word == "NEXT" => {
                let (start, name) = pending
                    .pop()
                    .with_context(|| format!("NEXT without FOR in line {line}"))?;
                ensure!(
                    tokens.len() == 1
                        || (tokens.len() == 2 && tokens.get(1) == Some(&Token::Word(name))),
                    "NEXT must match the innermost FOR variable in line {line}"
                );
                pairs.insert(start, line);
            }
            _ => {}
        }
    }
    if let Some(&(line, _)) = pending.last() {
        bail!("FOR without NEXT in line {line}");
    }
    Ok(pairs)
}

impl Basic {
    fn reset_data(&mut self) {
        self.data_line = 0;
        self.data_offset = None;
    }
    fn data_item(&mut self) -> Result<(String, bool, Option<usize>)> {
        if self.data_offset.is_none() {
            let (&line, _) = self
                .lines
                .range((
                    core::ops::Bound::Excluded(self.data_line),
                    core::ops::Bound::Unbounded,
                ))
                .find(|&(_, body)| data_body(body).is_some())
                .context("out of DATA")?;
            self.data_line = line;
            self.data_offset = Some(0);
        }
        let offset = self.data_offset.context("out of DATA")?;
        let body = self
            .lines
            .get(&self.data_line)
            .and_then(|body| data_body(body))
            .context("out of DATA")?;
        let (value, quoted, next) = data_constant(body.get(offset..).context("invalid DATA")?)?;
        check_string(&value)?;
        Ok((value, quoted, next.map(|next| offset.saturating_add(next))))
    }
    fn read_data(&mut self, tokens: &[Token]) -> Result<Flow> {
        let mut pos = 0;
        loop {
            let mut parser = Parser {
                memory: &self.memory,
                tokens,
                pos,
                vars: &self.vars,
                arrays: &self.arrays,
                strings: &self.strings,
            };
            let Some(Token::Word(name)) = parser.next() else {
                bail!("expected variable");
            };
            let string = name.ends_with('$');
            let index = if string {
                None
            } else {
                parser.subscript(&name)?
            };
            let more = parser.symbol(',');
            if !more {
                parser.end()?;
            }
            pos = parser.pos;
            let (text, quoted, next) = self.data_item()?;
            if string {
                check_string(&text)?;
                self.strings.insert(name, text);
            } else {
                ensure!(!quoted, "type mismatch");
                let digits = text.strip_prefix(['+', '-']).unwrap_or(&text);
                ensure!(
                    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()),
                    "type mismatch"
                );
                let value = text.parse::<i16>().context("integer overflow")?;
                if let Some(index) = index {
                    *self
                        .arrays
                        .get_mut(&name)
                        .and_then(|array| array.get_mut(index))
                        .context("subscript out of range")? = value;
                } else {
                    self.vars.insert(name, value);
                }
            }
            self.data_offset = next;
            if !more {
                return Ok(Flow::Next);
            }
        }
    }
    /// Loads numbered source lines, replacing the program and clearing variables
    /// and arrays only on success, including string variables.
    /// # Errors
    /// Returns an error for missing or invalid line numbers.
    pub fn load(&mut self, source: &str) -> Result<()> {
        let mut program = Self::default();
        for line in source.lines().filter(|line| !line.trim().is_empty()) {
            ensure!(program.edit(line)?, "file requires numbered lines");
        }
        self.lines = program.lines;
        self.vars.clear();
        self.arrays.clear();
        self.strings.clear();
        self.reset_data();
        Ok(())
    }
    fn edit(&mut self, source: &str) -> Result<bool> {
        let source = source.trim();
        let count = source.bytes().take_while(u8::is_ascii_digit).count();
        if count == 0 {
            return Ok(false);
        }
        let number: u16 = source.get(..count).context("invalid line")?.parse()?;
        ensure!(number > 0, "line number must be 1..65535");
        let body = source.get(count..).context("invalid line")?.trim();
        if body.is_empty() {
            self.lines.remove(&number);
        } else {
            self.lines.insert(number, body.to_owned());
        }
        self.reset_data();
        Ok(true)
    }
    fn statement(
        &mut self,
        tokens: &[Token],
        input: &mut impl BufRead,
        output: &mut impl Write,
    ) -> Result<Flow> {
        let mut parser = Parser {
            memory: &self.memory,
            tokens,
            pos: 0,
            vars: &self.vars,
            arrays: &self.arrays,
            strings: &self.strings,
        };
        if parser.word("POKE") {
            let address = parser.address()?;
            ensure!(parser.symbol(','), "expected comma");
            let value = u8::try_from(parser.expression()?).context("byte out of range")?;
            parser.end()?;
            self.memory.insert(address, value);
            return Ok(Flow::Next);
        }
        if parser.word("DATA") {
            return Ok(Flow::Next);
        }
        if parser.word("READ") {
            return self.read_data(tokens.get(parser.pos..).context("expected variable")?);
        }
        if parser.word("RESTORE") {
            let line = if parser.pos == tokens.len() {
                0
            } else {
                let Some(Token::Number(number)) = parser.next() else {
                    bail!("invalid line number");
                };
                let line = u16::try_from(number).context("invalid line number")?;
                ensure!(line != 0, "invalid line number");
                parser.end()?;
                ensure!(self.lines.contains_key(&line), "undefined line {line}");
                line.saturating_sub(1)
            };
            self.data_line = line;
            self.data_offset = None;
            return Ok(Flow::Next);
        }
        if parser.word("DIM") {
            let Some(Token::Word(name)) = parser.next() else {
                bail!("expected array name");
            };
            ensure!(!name.ends_with('$'), "string arrays not supported");
            ensure!(parser.symbol('('), "expected (");
            let upper = parser.expression()?;
            ensure!(parser.symbol(')'), "expected )");
            parser.end()?;
            let size = usize::try_from(upper)
                .context("subscript out of range")?
                .saturating_add(1);
            ensure!(
                !self.arrays.contains_key(&name),
                "array already dimensioned"
            );
            let used = self
                .arrays
                .values()
                .fold(0_usize, |total, array| total.saturating_add(array.len()));
            ensure!(used.saturating_add(size) <= 2048, "array memory full");
            self.arrays.insert(name, vec![0; size]);
            return Ok(Flow::Next);
        }
        if parser.word("REM") {
            return Ok(Flow::Next);
        }
        if parser.word("FOR") {
            let Some(Token::Word(name)) = parser.next() else {
                bail!("expected FOR variable");
            };
            ensure!(!name.ends_with('$'), "type mismatch");
            ensure!(parser.symbol('='), "expected =");
            let start = parser.expression()?;
            ensure!(parser.word("TO"), "expected TO");
            let limit = parser.expression()?;
            let step = if parser.word("STEP") {
                parser.expression()?
            } else {
                1
            };
            parser.end()?;
            ensure!(step != 0_i16, "STEP must not be zero");
            return Ok(Flow::For {
                name,
                start,
                limit,
                step,
            });
        }
        if parser.word("NEXT") {
            if parser.pos < tokens.len() {
                ensure!(
                    matches!(parser.next(), Some(Token::Word(_))),
                    "expected NEXT variable"
                );
            }
            parser.end()?;
            return Ok(Flow::NextLoop);
        }
        if parser.word("IF") {
            let text = if parser.is_string() {
                Some(parser.string_expression()?)
            } else {
                None
            };
            let left = if text.is_none() {
                parser.expression()?
            } else {
                0
            };
            let Some(Token::Symbol(op @ ('=' | '<' | '>'))) = parser.next() else {
                bail!("expected comparison");
            };
            let equal = parser.symbol('=');
            let unequal = op == '<' && !equal && parser.symbol('>');
            ensure!(op != '=' || !equal, "use = for equality");
            let ordering = if let Some(text) = text {
                text.cmp(&parser.string_expression()?)
            } else {
                left.cmp(&parser.expression()?)
            };
            ensure!(parser.word("THEN"), "expected THEN");
            let condition = match op {
                '=' => ordering.is_eq(),
                '<' if unequal => !ordering.is_eq(),
                '<' if equal => !ordering.is_gt(),
                '<' => ordering.is_lt(),
                '>' if equal => !ordering.is_lt(),
                _ => ordering.is_gt(),
            };
            let rest = tokens
                .get(parser.pos..)
                .context("expected THEN statement")?;
            ensure!(!rest.is_empty(), "expected THEN statement");
            if !condition {
                return Ok(Flow::Next);
            }
            if matches!(rest.first(), Some(Token::Number(_))) {
                let target = parser.target()?;
                parser.end()?;
                return Ok(Flow::Jump(target));
            }
            ensure!(
                !matches!(rest.first(), Some(Token::Word(word)) if word == "FOR" || word == "NEXT"),
                "FOR and NEXT must be standalone numbered statements"
            );
            return self.statement(rest, input, output);
        }
        if parser.word("PRINT") || parser.symbol('?') {
            let mut newline = true;
            while parser.pos < tokens.len() {
                if parser.is_string() {
                    write!(output, "{}", parser.string_expression()?)?;
                } else {
                    write!(output, "{}", parser.expression()?)?;
                }
                if parser.symbol(';') {
                    newline = false;
                } else if parser.symbol(',') {
                    write!(output, "\t")?;
                    newline = false;
                } else {
                    newline = true;
                    break;
                }
                if parser.pos < tokens.len() {
                    newline = true;
                }
            }
            parser.end()?;
            if newline {
                writeln!(output)?;
            }
            return Ok(Flow::Next);
        }
        let call = parser.word("GOSUB");
        if call || parser.word("GOTO") {
            let target = parser.target()?;
            parser.end()?;
            return Ok(if call {
                Flow::Call(target)
            } else {
                Flow::Jump(target)
            });
        }
        if parser.word("RETURN") {
            parser.end()?;
            return Ok(Flow::Return);
        }
        if parser.word("END") || parser.word("STOP") {
            parser.end()?;
            return Ok(Flow::End);
        }
        let read = parser.word("INPUT");
        if !read {
            parser.word("LET");
        }
        let Some(Token::Word(name)) = parser.next() else {
            bail!("expected statement or variable");
        };
        if name.ends_with('$') {
            let value = if read {
                parser.end()?;
                write!(output, "? ")?;
                output.flush()?;
                let mut line = String::new();
                ensure!(input.read_line(&mut line)? != 0, "end of input");
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                check_string(&line)?;
                line
            } else {
                ensure!(parser.symbol('='), "expected =");
                let text = parser.string_expression()?;
                parser.end()?;
                text
            };
            self.strings.insert(name, value);
            return Ok(Flow::Next);
        }
        let index = parser.subscript(&name)?;
        let value = if read {
            parser.end()?;
            write!(output, "? ")?;
            output.flush()?;
            let mut line = String::new();
            ensure!(input.read_line(&mut line)? != 0, "end of input");
            line.trim().parse().context("expected integer")?
        } else {
            ensure!(parser.symbol('='), "expected =");
            let value = parser.expression()?;
            parser.end()?;
            value
        };
        if let Some(index) = index {
            *self
                .arrays
                .get_mut(&name)
                .and_then(|array| array.get_mut(index))
                .context("subscript out of range")? = value;
        } else {
            self.vars.insert(name, value);
        }
        Ok(Flow::Next)
    }
    /// Runs the stored program, clearing integers, strings, and arrays first.
    /// # Errors
    /// Reports syntax, arithmetic, input/output, and control-flow errors with line numbers.
    pub fn run(&mut self, input: &mut impl BufRead, output: &mut impl Write) -> Result<()> {
        self.vars.clear();
        self.arrays.clear();
        self.strings.clear();
        self.reset_data();
        let program: BTreeMap<u16, Vec<Token>> = self
            .lines
            .iter()
            .map(|(&line, source)| {
                lex(source)
                    .map(|tokens| (line, tokens))
                    .with_context(|| format!("in line {line}"))
            })
            .collect::<Result<_>>()?;
        let pairs = loop_pairs(&program)?;
        let after = |line| {
            program
                .range((
                    core::ops::Bound::Excluded(line),
                    core::ops::Bound::Unbounded,
                ))
                .next()
                .map(|(&number, _)| number)
        };
        let mut pc = program.keys().next().copied();
        let mut stack: Vec<(Option<u16>, usize)> = Vec::new();
        let mut loops: Vec<LoopFrame> = Vec::new();
        while let Some(line) = pc {
            let tokens = program.get(&line).context("missing line")?;
            let next = after(line);
            let flow = self
                .statement(tokens, input, output)
                .with_context(|| format!("in line {line}"))?;
            let loop_base = stack.last().map_or(0, |&(_, depth)| depth);
            pc = match flow {
                Flow::Next => next,
                Flow::Jump(target) | Flow::Call(target) => {
                    ensure!(
                        program.contains_key(&target),
                        "undefined line {target} in line {line}"
                    );
                    if matches!(flow, Flow::Call(_)) {
                        ensure!(stack.len() < 256, "GOSUB stack full in line {line}");
                        stack.push((next, loops.len()));
                    } else {
                        // A jump out of a loop discards it, preserving caller loops.
                        while loops.len() > loop_base
                            && loops.last().is_some_and(|frame| {
                                target <= frame.start_line || target > frame.end_line
                            })
                        {
                            loops.pop();
                        }
                    }
                    Some(target)
                }
                Flow::Return => {
                    let (address, depth) = stack
                        .pop()
                        .with_context(|| format!("RETURN without GOSUB in line {line}"))?;
                    loops.truncate(depth);
                    address
                }
                Flow::End => None,
                Flow::For {
                    name,
                    start,
                    limit,
                    step,
                } => {
                    let end_line = *pairs
                        .get(&line)
                        .context("FOR must be a standalone numbered statement")?;
                    ensure!(
                        !loops.iter().any(|frame| frame.name == name),
                        "FOR variable {name} is already active in line {line}"
                    );
                    self.vars.insert(name.clone(), start);
                    if (step > 0 && start > limit) || (step < 0 && start < limit) {
                        after(end_line)
                    } else {
                        ensure!(loops.len() < 256, "FOR stack full in line {line}");
                        loops.push(LoopFrame {
                            name,
                            limit,
                            step,
                            start_line: line,
                            end_line,
                            body: next,
                        });
                        next
                    }
                }
                Flow::NextLoop => {
                    ensure!(
                        loops.len() > loop_base,
                        "NEXT without active FOR in line {line}"
                    );
                    let frame = loops.last().context("missing loop frame")?;
                    ensure!(
                        frame.end_line == line,
                        "NEXT does not match active FOR in line {line}"
                    );
                    let value = self
                        .vars
                        .get(&frame.name)
                        .copied()
                        .context("missing loop variable")?
                        .checked_add(frame.step)
                        .with_context(|| format!("integer overflow in line {line}"))?;
                    self.vars.insert(frame.name.clone(), value);
                    if (frame.step > 0 && value <= frame.limit)
                        || (frame.step < 0 && value >= frame.limit)
                    {
                        frame.body
                    } else {
                        loops.pop();
                        next
                    }
                }
            };
        }
        Ok(())
    }
    /// Starts the line editor and immediate-mode prompt. EOF and QUIT exit.
    /// # Errors
    /// Returns terminal input/output errors; BASIC errors are printed at the prompt.
    pub fn interact(&mut self, input: &mut impl BufRead, output: &mut impl Write) -> Result<()> {
        writeln!(output, "RX-82 BASIC\nType HELP for commands.\nReady.")?;
        loop {
            write!(output, "> ")?;
            output.flush()?;
            let mut line = String::new();
            if input.read_line(&mut line)? == 0 {
                break;
            }
            let source = line.trim();
            if source.eq_ignore_ascii_case("QUIT") {
                break;
            }
            let result = self.command(source, input, output);
            if let Err(error) = result {
                writeln!(output, "? {error:#}")?;
            }
        }
        Ok(())
    }
    fn command(
        &mut self,
        source: &str,
        input: &mut impl BufRead,
        output: &mut impl Write,
    ) -> Result<()> {
        if source.is_empty() || self.edit(source)? {
            return Ok(());
        }
        match source.to_ascii_uppercase().as_str() {
            "RUN" => return self.run(input, output),
            "NEW" => {
                self.lines.clear();
                self.vars.clear();
                self.arrays.clear();
                self.strings.clear();
                self.reset_data();
                return Ok(());
            }
            "LIST" => {
                for (number, body) in &self.lines {
                    writeln!(output, "{number} {body}")?;
                }
                return Ok(());
            }
            "HELP" => {
                writeln!(
                    output,
                    "Numbered lines edit the program; a bare number deletes a line.\nRUN, LIST, NEW, QUIT\nSAVE \"file.bas\", LOAD \"file.bas\" (prompt only)\nPRINT, LET, INPUT, IF ... THEN, GOTO, GOSUB, RETURN, REM, END\nDIM name(upper): zero-based arrays, 2048 total integer elements\nFOR name = start TO limit [STEP step], NEXT [name] (program only)\nSigned 16-bit integers, variables, + - * / and parentheses; comparisons = <> < <= > >="
                )?;
                writeln!(
                    output,
                    "Strings: name$, 63 ASCII characters, + joins strings; LEN(name$) or LEN(array)"
                )?;
                writeln!(
                    output,
                    "DATA constants; READ variable[,variable...]; RESTORE [line]"
                )?;
                writeln!(
                    output,
                    "PEEK(address), POKE address,byte: separate reference memory; use --native for RX-82 RAM and devices"
                )?;
                return Ok(());
            }
            _ => {}
        }
        let tokens = lex(source)?;
        if let Some(Token::Word(command)) = tokens.first().cloned()
            && (command == "SAVE" || command == "LOAD")
        {
            #[expect(
                clippy::pattern_type_mismatch,
                reason = "borrow the filename from the token slice"
            )]
            let [Token::Word(_), Token::Text(path)] = tokens.as_slice() else {
                bail!("expected {command} \"file.bas\"");
            };
            ensure!(!path.is_empty(), "filename must not be empty");
            if command == "SAVE" {
                let mut contents = Vec::new();
                for (number, body) in &self.lines {
                    writeln!(contents, "{number} {body}")?;
                }
                std::fs::write(path, contents).with_context(|| format!("saving {path}"))?;
            } else {
                let contents =
                    std::fs::read_to_string(path).with_context(|| format!("loading {path}"))?;
                self.load(&contents)
                    .with_context(|| format!("loading {path}"))?;
            }
            return Ok(());
        }
        match self.statement(&tokens, input, output)? {
            Flow::Next | Flow::End => Ok(()),
            _ => bail!("control flow requires RUN"),
        }
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "tests")]
mod tests {
    use super::*;

    fn execute(source: &str, input: &str) -> Result<String> {
        let mut basic = Basic::default();
        basic.load(source)?;
        let mut output = Vec::new();
        basic.run(&mut input.as_bytes(), &mut output)?;
        Ok(String::from_utf8(output)?)
    }

    #[test]
    fn reference_memory_has_unsigned_bytes_and_signed_address_aliases() {
        let mut basic = Basic::default();
        let mut output = Vec::new();
        basic.interact(&mut "PRINT PEEK(0),PEEK(65535)\nPOKE 65535,255\nPOKE -32768,128\nPRINT PEEK(-1),PEEK(32768)\nPOKE 256,42\nNEW\n10 PRINT PEEK(256)\nRUN\nQUIT\n".as_bytes(), &mut output).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("0\t0\n"), "{output}");
        assert!(output.contains("255\t128\n"), "{output}");
        assert!(output.contains("> 42\n"), "{output}");
        basic.load("10 PRINT PEEK(256)").unwrap();
        let mut after_load = Vec::new();
        basic.run(&mut "".as_bytes(), &mut after_load).unwrap();
        assert_eq!(after_load, b"42\n");
    }

    #[test]
    fn invalid_memory_operations_preserve_reference_bytes() {
        for (statement, cause) in [
            ("POKE 256,-1", "byte out of range"),
            ("POKE 256,256", "byte out of range"),
            ("POKE 256,1 2", "unexpected trailing input"),
            ("POKE 256 1", "expected comma"),
            ("POKE 65536,1", "integer overflow"),
            ("PRINT PEEK(-32769)", "integer out of range"),
            ("PRINT PEEK(65536)", "integer overflow"),
            ("PRINT PEEK(256", "expected )"),
            ("PRINT PEEK 256", "expected ("),
        ] {
            let mut basic = Basic::default();
            let mut output = Vec::new();
            basic
                .command("POKE 256,42", &mut "".as_bytes(), &mut output)
                .unwrap();
            let error = basic
                .command(statement, &mut "".as_bytes(), &mut output)
                .unwrap_err();
            assert!(
                format!("{error:#}").contains(cause),
                "{statement}: {error:#}"
            );
            basic
                .command("PRINT PEEK(256)", &mut "".as_bytes(), &mut output)
                .unwrap();
            assert_eq!(output, b"42\n");
        }
    }

    #[test]
    fn arrays_support_expressions_input_and_separate_scalars() {
        assert_eq!(execute("10 DIM A(100)\n20 A=7\n30 A(0)=100\n40 INPUT A(A(0))\n50 LET A(1)=A(100)+2\n60 PRINT A,A(2),A(100),A(1)", "-32768\n").unwrap(), "? 7\t0\t-32768\t-32766\n");
    }

    #[test]
    fn array_errors_and_capacity() {
        for (source, cause) in [
            ("10 PRINT A(0)", "array not dimensioned"),
            ("10 A(0)=1", "array not dimensioned"),
            ("10 DIM A(-1)", "subscript out of range"),
            ("10 DIM A(0)\n20 A(1)=3", "subscript out of range"),
            ("10 DIM A(0)\n20 PRINT A(-1)", "subscript out of range"),
            ("10 DIM A(0)\n20 DIM A(2)", "array already dimensioned"),
            ("10 DIM A(2048)", "array memory full"),
            ("10 DIM A(2047)\n20 DIM B(0)", "array memory full"),
            ("10 DIM A", "expected ("),
            ("10 DIM A(2", "expected )"),
        ] {
            assert!(
                format!("{:#}", execute(source, "").unwrap_err()).contains(cause),
                "{source}"
            );
        }
        assert_eq!(
            execute(
                "10 DIM A(2046)\n20 DIM B(0)\n30 B(0)=42\n40 PRINT A(2046),B(0)",
                ""
            )
            .unwrap(),
            "0\t42\n"
        );
    }

    #[test]
    fn arrays_reset_on_run_new_and_successful_load() {
        let mut basic = Basic::default();
        basic.load("10 DIM A(0)\n20 A(0)=42").unwrap();
        for _ in 0..2_u8 {
            basic.run(&mut "".as_bytes(), &mut Vec::new()).unwrap();
            assert_eq!(basic.arrays.get("A"), Some(&vec![42]));
        }
        assert!(basic.load("invalid source").is_err());
        assert_eq!(basic.arrays.get("A"), Some(&vec![42]));
        basic.load("10 END").unwrap();
        assert!(basic.arrays.is_empty());
        basic
            .interact(&mut "DIM A(0)\nNEW\nQUIT\n".as_bytes(), &mut Vec::new())
            .unwrap();
        assert!(basic.arrays.is_empty());
    }

    #[test]
    fn strings_reset_only_after_a_successful_load() {
        let mut basic = Basic::default();
        basic
            .command("A$=\"kept\"", &mut "".as_bytes(), &mut Vec::new())
            .unwrap();
        assert!(basic.load("invalid source").is_err());
        assert_eq!(basic.strings.get("A$").map(String::as_str), Some("kept"));
        basic.load("10 END").unwrap();
        assert!(basic.strings.is_empty());
    }

    #[test]
    fn data_cursor_resets_only_after_a_successful_load() {
        let mut basic = Basic::default();
        let mut output = Vec::new();
        basic.load("100 DATA 5,6").unwrap();
        basic
            .command("READ A", &mut "".as_bytes(), &mut output)
            .unwrap();
        assert!(basic.load("invalid source").is_err());
        basic
            .command("READ B", &mut "".as_bytes(), &mut output)
            .unwrap();
        basic
            .command("PRINT A,B", &mut "".as_bytes(), &mut output)
            .unwrap();
        basic.load("100 DATA 5,6").unwrap();
        basic
            .command("READ C", &mut "".as_bytes(), &mut output)
            .unwrap();
        basic
            .command("PRINT C", &mut "".as_bytes(), &mut output)
            .unwrap();
        assert_eq!(output, b"5\t6\n5\n");
    }

    #[test]
    fn expressions_and_print_separators() {
        assert_eq!(
            execute(
                "10 let total=2+3*4\n20 PRINT total;\" hello\";(-8+2)/3,missing\n30 ? +5",
                ""
            )
            .unwrap(),
            "14 hello-2\t0\n5\n"
        );
    }

    #[test]
    fn loop_and_nested_subroutines() {
        let source = "10 INPUT N\n20 A=A+1\n30 GOSUB 100\n40 IF A<N THEN 20\n50 END\n100 GOSUB 200\n110 RETURN\n200 PRINT A*A\n210 RETURN";
        assert_eq!(execute(source, "3\n").unwrap(), "? 1\n4\n9\n");
    }

    #[test]
    fn comparisons_and_inline_statements() {
        for operator in ["=", "<=", ">="] {
            assert_eq!(
                execute(&format!("10 IF 2{operator}2 THEN PRINT \"yes\""), "").unwrap(),
                "yes\n"
            );
        }
        for operator in ["<>", "<", ">"] {
            assert_eq!(
                execute(
                    &format!("10 IF 2{operator}2 THEN PRINT \"no\"\n20 REM arbitrary ! comment"),
                    ""
                )
                .unwrap(),
                ""
            );
        }
    }

    #[test]
    fn runtime_errors_include_line_and_cause() {
        for (statement, expected) in [
            ("PRINT 1/0", "division by zero"),
            ("PRINT 32767+1", "integer overflow"),
            ("GOTO 999", "undefined line 999"),
            ("RETURN", "RETURN without GOSUB"),
            ("PRINT (1+2", "expected )"),
            ("PRINT \"oops", "unterminated string"),
            ("INPUT A", "end of input"),
            ("END junk", "unexpected trailing input"),
        ] {
            let error = format!("{:#}", execute(&format!("10 {statement}"), "").unwrap_err());
            assert!(error.contains("line 10"), "{error}");
            assert!(error.contains(expected), "{error}");
        }
    }

    #[test]
    fn editor_orders_replaces_deletes_and_recovers() {
        let mut basic = Basic::default();
        let mut output = Vec::new();
        basic.interact(&mut b"20 PRINT 2\n10 PRINT 1\n20 PRINT 3\n30 END\n30\nLIST\nPRINT 1/0\nRUN\nNEW\nRUN\nQUIT\n".as_slice(), &mut output).unwrap();
        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("10 PRINT 1\n20 PRINT 3\n"));
        assert!(text.contains("? division by zero"));
        assert!(text.contains("1\n3\n"));
        assert!(basic.lines.is_empty());
    }

    #[test]
    fn load_is_atomic_and_run_resets_variables() {
        let mut basic = Basic::default();
        basic.load("10 A=A+1\n20 PRINT A").unwrap();
        assert!(basic.load("10 END\nPRINT 4").is_err());
        let mut output = Vec::new();
        for _ in 0_u8..2 {
            basic.run(&mut b"".as_slice(), &mut output).unwrap();
        }
        assert_eq!(output, b"1\n1\n");
        assert!(basic.load("0 END").is_err());
        assert!(basic.load("65536 END").is_err());
    }
    #[test]
    fn files_round_trip_and_failures_preserve_session() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("rx82-basic-{}-{unique}", std::process::id()));
        #[expect(
            clippy::create_dir,
            reason = "fail on collision rather than reuse a test directory"
        )]
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("My Program.bas");
        let missing = directory.join("missing.bas");
        let mut basic = Basic::default();
        let mut output = Vec::new();
        let mut input = b"".as_slice();
        basic.load("20 PRINT A\n10 A=7").unwrap();
        basic
            .command(
                &format!("save \"{}\"", path.display()),
                &mut input,
                &mut output,
            )
            .unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "10 A=7\n20 PRINT A\n"
        );
        basic.command("NEW", &mut input, &mut output).unwrap();
        basic.command("A=99", &mut input, &mut output).unwrap();
        basic
            .command(
                &format!("load \"{}\"", path.display()),
                &mut input,
                &mut output,
            )
            .unwrap();
        assert!(basic.vars.is_empty());
        basic.run(&mut input, &mut output).unwrap();
        assert_eq!(output, b"7\n");
        let lines = basic.lines.clone();
        let vars = basic.vars.clone();
        std::fs::write(&path, "10 END\nnot numbered").unwrap();
        for command in [
            format!("LOAD \"{}\"", path.display()),
            format!("LOAD \"{}\"", missing.display()),
            format!("SAVE \"{}\"", directory.display()),
            "SAVE".to_owned(),
            "LOAD unquoted.bas".to_owned(),
            "SAVE \"\"".to_owned(),
            "LOAD \"x\" extra".to_owned(),
        ] {
            assert!(
                basic.command(&command, &mut input, &mut output).is_err(),
                "{command}"
            );
            assert_eq!(basic.lines, lines);
            assert_eq!(basic.vars, vars);
        }
        // An empty program overwrites the old file and loads as an empty program.
        basic.command("NEW", &mut input, &mut output).unwrap();
        basic
            .command(
                &format!("SAVE \"{}\"", path.display()),
                &mut input,
                &mut output,
            )
            .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
        basic.load("10 END").unwrap();
        basic
            .command(
                &format!("LOAD \"{}\"", path.display()),
                &mut input,
                &mut output,
            )
            .unwrap();
        assert!(basic.lines.is_empty());
        std::fs::remove_dir_all(directory).unwrap();
    }
    #[test]
    fn for_loops_count_in_both_directions() {
        for (header, expected) in [
            ("FOR I=1 TO 3", "1\n2\n3\n4\n"),
            ("for i=1 to 6 step 2", "1\n3\n5\n7\n"),
            ("FOR I=3 TO -1 STEP -2", "3\n1\n-1\n-3\n"),
            ("FOR I=2 TO 2 STEP -1", "2\n1\n"),
        ] {
            assert_eq!(
                execute(
                    &format!("10 {header}\n20 PRINT I\n30 NEXT i\n40 PRINT I"),
                    ""
                )
                .unwrap(),
                expected
            );
        }
    }

    #[test]
    fn for_loops_nest_and_capture_bounds_once() {
        let source = "10 N=2\n20 S=1\n30 FOR I=1 TO N STEP S\n40 N=0\n50 S=0\n60 FOR J=2 TO 1 STEP -1\n70 PRINT I;J\n80 NEXT\n90 NEXT I";
        assert_eq!(execute(source, "").unwrap(), "12\n11\n22\n21\n");
        assert_eq!(
            execute("10 FOR I=1+1 TO (3*2) STEP 4/2\n20 PRINT I\n30 NEXT", "").unwrap(),
            "2\n4\n6\n"
        );
    }

    #[test]
    fn empty_loops_skip_nested_bodies_without_evaluation() {
        for header in ["FOR I=3 TO 1", "FOR I=1 TO 3 STEP -1"] {
            let source = format!(
                "10 {header}\n20 FOR J=1/0 TO 2\n30 INPUT N\n40 NEXT J\n50 NEXT I\n60 PRINT \"done\""
            );
            assert_eq!(execute(&source, "").unwrap(), "done\n");
        }
    }

    #[test]
    fn loop_errors_identify_the_line() {
        for (source, expected, line) in [
            (
                "10 FOR I=1 TO 2 STEP 0\n20 NEXT",
                "STEP must not be zero",
                "10",
            ),
            ("10 FOR I=1 TO 2", "FOR without NEXT", "10"),
            ("10 NEXT", "NEXT without FOR", "10"),
            ("10 FOR I=1 TO 2\n20 NEXT J", "innermost FOR variable", "20"),
            (
                "10 GOTO 30\n20 FOR I=1 TO 2\n30 NEXT",
                "NEXT without active FOR",
                "30",
            ),
            (
                "10 FOR I=1 TO 2\n20 FOR I=1 TO 2\n30 NEXT\n40 NEXT",
                "already active",
                "20",
            ),
            ("10 FOR I=32767 TO 32767\n20 NEXT", "integer overflow", "20"),
            (
                "10 FOR I=-32768 TO -32768 STEP -1\n20 NEXT",
                "integer overflow",
                "20",
            ),
            ("10 FOR I=1 2\n20 NEXT", "expected TO", "10"),
            (
                "10 FOR I=1 TO 2 STEP 1 extra\n20 NEXT",
                "unexpected trailing input",
                "10",
            ),
            ("10 IF 1=1 THEN FOR I=1 TO 2", "standalone numbered", "10"),
        ] {
            let error = format!("{:#}", execute(source, "").unwrap_err());
            assert!(
                error.contains(expected) && error.contains(&format!("line {line}")),
                "{error}"
            );
        }
    }

    #[test]
    fn subroutine_loops_preserve_caller_and_return_discards_local_loops() {
        let source = "10 FOR I=1 TO 2\n20 GOSUB 100\n30 NEXT I\n40 END\n100 FOR J=1 TO 3\n110 PRINT I;J\n120 RETURN\n130 NEXT J";
        assert_eq!(execute(source, "").unwrap(), "11\n21\n");
    }

    #[test]
    fn jumps_out_of_loops_discard_frames_and_restart_is_allowed() {
        let source = "10 FOR I=1 TO 2\n20 FOR J=1 TO 2\n30 PRINT I\n40 GOTO 60\n50 NEXT J\n60 NEXT I\n70 FOR J=1 TO 1\n80 PRINT J\n90 NEXT";
        assert_eq!(execute(source, "").unwrap(), "1\n2\n1\n");
        let restart = "10 N=N+1\n20 FOR I=1 TO 1\n30 IF N<2 THEN 10\n40 PRINT N\n50 NEXT";
        assert_eq!(execute(restart, "").unwrap(), "2\n");
    }

    #[test]
    fn signed_sixteen_bit_boundaries_and_unsigned_line_labels() {
        assert_eq!(execute("10 PRINT -32768,32767\n20 INPUT A\n30 PRINT A\n40 GOTO 65535\n65535 PRINT -32768+1", "-32768\n").unwrap(), "-32768\t32767\n? -32768\n-32767\n");
        assert_eq!(execute("10 IF 1=1 THEN 65535\n65535 END", "").unwrap(), "");
        assert_eq!(
            execute("10 GOSUB 65535\n20 END\n65535 RETURN", "").unwrap(),
            ""
        );
        for expression in [
            "32768",
            "-32769",
            "32767+1",
            "-32768-1",
            "200*200",
            "-32768/-1",
            "--32768",
        ] {
            assert!(
                execute(&format!("10 PRINT {expression}"), "").is_err(),
                "{expression}"
            );
        }
        execute("10 INPUT A", "32768\n").unwrap_err();
    }

    #[test]
    fn immediate_loops_do_not_modify_variables_and_runs_reset_loop_state() {
        let mut basic = Basic::default();
        let mut output = Vec::new();
        basic
            .command("I=7", &mut b"".as_slice(), &mut output)
            .unwrap();
        for command in ["FOR I=1 TO 2", "NEXT I"] {
            assert!(
                basic
                    .command(command, &mut b"".as_slice(), &mut output)
                    .is_err()
            );
        }
        basic
            .command("PRINT I", &mut b"".as_slice(), &mut output)
            .unwrap();
        basic
            .load("10 FOR I=1 TO 2\n20 PRINT I\n30 END\n40 NEXT")
            .unwrap();
        for _ in 0_u8..2 {
            basic.run(&mut b"".as_slice(), &mut output).unwrap();
        }
        assert_eq!(output, b"7\n1\n1\n");
    }
}
