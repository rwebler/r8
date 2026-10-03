//! A small, host-side integer BASIC interpreter.
#![allow(
    clippy::arbitrary_source_item_ordering,
    reason = "keep parsing and execution helpers in reading order"
)]
extern crate alloc;
use alloc::collections::BTreeMap;
use anyhow::{Context as _, Result, bail, ensure};
use std::io::{BufRead, Write};

/// BASIC program and variable storage. Variables contain signed 32-bit integers.
#[derive(Default)]
pub struct Basic {
    lines: BTreeMap<u16, String>,
    vars: BTreeMap<String, i32>,
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
            let word = value.to_ascii_uppercase();
            let remark = word == "REM";
            tokens.push(Token::Word(word));
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
    tokens: &'source [Token],
    pos: usize,
    vars: &'source BTreeMap<String, i32>,
}
impl Parser<'_> {
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
    fn expression(&mut self) -> Result<i32> {
        self.binary(0)
    }
    fn binary(&mut self, min: u8) -> Result<i32> {
        let mut left = match self.next().context("expected expression")? {
            Token::Number(n) => n,
            Token::Word(name) => self.vars.get(&name).copied().unwrap_or_default(),
            Token::Symbol('-') => self.binary(3)?.checked_neg().context("integer overflow")?,
            Token::Symbol('+') => self.binary(3)?,
            Token::Symbol('(') => {
                let value = self.expression()?;
                ensure!(self.symbol(')'), "expected )");
                value
            }
            _ => bail!("expected expression"),
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
                    ensure!(right != 0_i32, "division by zero");
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
}

impl Basic {
    /// Loads numbered source lines, replacing the program only on success.
    /// # Errors
    /// Returns an error for missing or invalid line numbers.
    pub fn load(&mut self, source: &str) -> Result<()> {
        let mut program = Self::default();
        for line in source.lines().filter(|line| !line.trim().is_empty()) {
            ensure!(program.edit(line)?, "file requires numbered lines");
        }
        self.lines = program.lines;
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
        Ok(true)
    }
    fn statement(
        &mut self,
        tokens: &[Token],
        input: &mut impl BufRead,
        output: &mut impl Write,
    ) -> Result<Flow> {
        let mut parser = Parser {
            tokens,
            pos: 0,
            vars: &self.vars,
        };
        if parser.word("REM") {
            return Ok(Flow::Next);
        }
        if parser.word("IF") {
            let left = parser.expression()?;
            let Some(Token::Symbol(op @ ('=' | '<' | '>'))) = parser.next() else {
                bail!("expected comparison");
            };
            let equal = parser.symbol('=');
            let unequal = op == '<' && !equal && parser.symbol('>');
            ensure!(op != '=' || !equal, "use = for equality");
            let right = parser.expression()?;
            ensure!(parser.word("THEN"), "expected THEN");
            let condition = match op {
                '=' => left == right,
                '<' if unequal => left != right,
                '<' if equal => left <= right,
                '<' => left < right,
                '>' if equal => left >= right,
                _ => left > right,
            };
            let rest = tokens
                .get(parser.pos..)
                .context("expected THEN statement")?;
            ensure!(!rest.is_empty(), "expected THEN statement");
            if !condition {
                return Ok(Flow::Next);
            }
            if matches!(rest.first(), Some(Token::Number(_))) {
                let target = u16::try_from(parser.expression()?).context("invalid line number")?;
                parser.end()?;
                return Ok(Flow::Jump(target));
            }
            return self.statement(rest, input, output);
        }
        if parser.word("PRINT") || parser.symbol('?') {
            let mut newline = true;
            while parser.pos < tokens.len() {
                #[expect(
                    clippy::pattern_type_mismatch,
                    reason = "borrow string without cloning"
                )]
                if let Some(Token::Text(text)) = tokens.get(parser.pos) {
                    write!(output, "{text}")?;
                    parser.pos = parser.pos.saturating_add(1);
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
            let target = u16::try_from(parser.expression()?).context("invalid line number")?;
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
        self.vars.insert(name, value);
        Ok(Flow::Next)
    }
    /// Runs the stored program, clearing variables first.
    /// # Errors
    /// Reports syntax, arithmetic, input/output, and control-flow errors with line numbers.
    pub fn run(&mut self, input: &mut impl BufRead, output: &mut impl Write) -> Result<()> {
        self.vars.clear();
        let mut pc = self.lines.keys().next().copied();
        let mut stack = Vec::new();
        while let Some(line) = pc {
            let source = self.lines.get(&line).context("missing line")?.clone();
            let next = self
                .lines
                .range((
                    core::ops::Bound::Excluded(line),
                    core::ops::Bound::Unbounded,
                ))
                .next()
                .map(|(&n, _)| n);
            let flow = lex(&source)
                .and_then(|tokens| self.statement(&tokens, input, output))
                .with_context(|| format!("in line {line}"))?;
            pc = match flow {
                Flow::Next => next,
                Flow::Jump(target) | Flow::Call(target) => {
                    ensure!(
                        self.lines.contains_key(&target),
                        "undefined line {target} in line {line}"
                    );
                    if matches!(flow, Flow::Call(_)) {
                        ensure!(stack.len() < 256, "GOSUB stack full in line {line}");
                        stack.push(next);
                    }
                    Some(target)
                }
                Flow::Return => stack
                    .pop()
                    .with_context(|| format!("RETURN without GOSUB in line {line}"))?,
                Flow::End => None,
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
                    "Numbered lines edit the program; a bare number deletes a line.\nRUN, LIST, NEW, QUIT\nSAVE \"file.bas\", LOAD \"file.bas\" (prompt only)\nPRINT, LET, INPUT, IF ... THEN, GOTO, GOSUB, RETURN, REM, END\nIntegers, variables, + - * / and parentheses; comparisons = <> < <= > >="
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
                self.vars.clear();
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
            ("PRINT 2147483647+1", "integer overflow"),
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
}
