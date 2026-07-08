use std::collections::BTreeMap;

pub struct ConditionEvaluator;

impl ConditionEvaluator {
    pub fn evaluate(expression: &str, capabilities: &BTreeMap<String, ConditionValue>) -> bool {
        let trimmed = expression.trim();
        if trimmed.is_empty() {
            return true;
        }
        let tokens = match ConditionTokenizer::new(trimmed).tokenize() {
            Ok(tokens) => tokens,
            Err(_) => return false,
        };
        let mut parser = ConditionParser::new(tokens, capabilities);
        match parser.parseExpression() {
            Ok(value) => value.isTruthy(),
            Err(_) => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ConditionValue {
    Bool(bool),
    Num(f64),
    Str(String),
    Null,
    Array(Vec<ConditionValue>),
}

impl ConditionValue {
    fn isTruthy(&self) -> bool {
        match self {
            Self::Bool(value) => *value,
            Self::Num(value) => *value != 0.0 && !value.is_nan(),
            Self::Str(value) => !value.is_empty(),
            Self::Null => false,
            Self::Array(items) => !items.is_empty(),
        }
    }

    fn toNumberOrNull(&self) -> Option<f64> {
        match self {
            Self::Num(value) => Some(*value),
            Self::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    fn compareTo(&self, other: &ConditionValue) -> Result<std::cmp::Ordering, String> {
        match (self, other) {
            (Self::Str(left), Self::Str(right)) => Ok(left.cmp(right)),
            _ => {
                let left = self
                    .toNumberOrNull()
                    .ok_or_else(|| "Cannot compare non-number".to_string())?;
                let right = other
                    .toNumberOrNull()
                    .ok_or_else(|| "Cannot compare non-number".to_string())?;
                left.partial_cmp(&right)
                    .ok_or_else(|| "Cannot compare NaN".to_string())
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ConditionToken {
    Identifier(String),
    StringLiteral(String),
    NumberLiteral(f64),
    BooleanLiteral(bool),
    NullLiteral,
    Operator(String),
    Punct(char),
    Eof,
}

struct ConditionTokenizer<'a> {
    input: &'a str,
    chars: Vec<char>,
    i: usize,
}

impl<'a> ConditionTokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().collect(),
            i: 0,
        }
    }

    fn tokenize(&mut self) -> Result<Vec<ConditionToken>, String> {
        let mut out = Vec::new();
        loop {
            self.skipWs();
            if self.i >= self.chars.len() {
                out.push(ConditionToken::Eof);
                return Ok(out);
            }
            let c = self.chars[self.i];
            if matches!(c, '(' | ')' | '[' | ']' | ',') {
                out.push(ConditionToken::Punct(c));
                self.i += 1;
            } else if c == '"' || c == '\'' {
                out.push(ConditionToken::StringLiteral(self.readString(c)?));
            } else if c.is_ascii_digit()
                || (c == '.'
                    && self.i + 1 < self.chars.len()
                    && self.chars[self.i + 1].is_ascii_digit())
            {
                out.push(ConditionToken::NumberLiteral(self.readNumber()?));
            } else if isConditionIdentStart(c) {
                let ident = self.readIdentifier();
                match ident.as_str() {
                    "true" => out.push(ConditionToken::BooleanLiteral(true)),
                    "false" => out.push(ConditionToken::BooleanLiteral(false)),
                    "null" => out.push(ConditionToken::NullLiteral),
                    "in" => out.push(ConditionToken::Operator("in".to_string())),
                    _ => out.push(ConditionToken::Identifier(ident)),
                }
            } else if let Some(op) = self.readOperator() {
                out.push(ConditionToken::Operator(op));
            } else {
                return Err(format!("Unexpected character '{c}'"));
            }
        }
    }

    fn skipWs(&mut self) {
        while self.i < self.chars.len() && self.chars[self.i].is_whitespace() {
            self.i += 1;
        }
    }

    fn readIdentifier(&mut self) -> String {
        let start = self.i;
        self.i += 1;
        while self.i < self.chars.len() && isConditionIdentPart(self.chars[self.i]) {
            self.i += 1;
        }
        self.chars[start..self.i].iter().collect()
    }

    fn readString(&mut self, quote: char) -> Result<String, String> {
        self.i += 1;
        let mut out = String::new();
        while self.i < self.chars.len() {
            let c = self.chars[self.i];
            if c == quote {
                self.i += 1;
                return Ok(out);
            }
            if c == '\\' {
                if self.i + 1 >= self.chars.len() {
                    return Err("Unterminated escape".to_string());
                }
                let n = self.chars[self.i + 1];
                out.push(match n {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\\' => '\\',
                    '\'' => '\'',
                    '"' => '"',
                    _ => n,
                });
                self.i += 2;
            } else {
                out.push(c);
                self.i += 1;
            }
        }
        Err("Unterminated string".to_string())
    }

    fn readNumber(&mut self) -> Result<f64, String> {
        let start = self.i;
        let mut hasDot = false;
        while self.i < self.chars.len() {
            let c = self.chars[self.i];
            if c.is_ascii_digit() {
                self.i += 1;
            } else if c == '.' && !hasDot {
                hasDot = true;
                self.i += 1;
            } else {
                break;
            }
        }
        self.input
            .chars()
            .skip(start)
            .take(self.i - start)
            .collect::<String>()
            .parse::<f64>()
            .map_err(|error| error.to_string())
    }

    fn readOperator(&mut self) -> Option<String> {
        for op in ["&&", "||", "==", "!=", ">=", "<=", ">", "<", "!"] {
            if self.input[self.byteIndex(self.i)..].starts_with(op) {
                self.i += op.chars().count();
                return Some(op.to_string());
            }
        }
        None
    }

    fn byteIndex(&self, charIndex: usize) -> usize {
        self.input
            .char_indices()
            .nth(charIndex)
            .map(|(index, _)| index)
            .unwrap_or(self.input.len())
    }
}

fn isConditionIdentStart(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn isConditionIdentPart(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '.'
}

struct ConditionParser<'a> {
    tokens: Vec<ConditionToken>,
    pos: usize,
    capabilities: &'a BTreeMap<String, ConditionValue>,
}

impl<'a> ConditionParser<'a> {
    fn new(
        tokens: Vec<ConditionToken>,
        capabilities: &'a BTreeMap<String, ConditionValue>,
    ) -> Self {
        Self {
            tokens,
            pos: 0,
            capabilities,
        }
    }

    fn parseExpression(&mut self) -> Result<ConditionValue, String> {
        self.parseOr()
    }

    fn parseOr(&mut self) -> Result<ConditionValue, String> {
        let mut left = self.parseAnd()?;
        while self.matchOp("||") {
            if left.isTruthy() {
                let _ = self.parseAnd()?;
                left = ConditionValue::Bool(true);
            } else {
                left = ConditionValue::Bool(self.parseAnd()?.isTruthy());
            }
        }
        Ok(left)
    }

    fn parseAnd(&mut self) -> Result<ConditionValue, String> {
        let mut left = self.parseEquality()?;
        while self.matchOp("&&") {
            if !left.isTruthy() {
                let _ = self.parseEquality()?;
                left = ConditionValue::Bool(false);
            } else {
                left = ConditionValue::Bool(self.parseEquality()?.isTruthy());
            }
        }
        Ok(left)
    }

    fn parseEquality(&mut self) -> Result<ConditionValue, String> {
        let mut left = self.parseRelational()?;
        loop {
            if self.matchOp("==") {
                left = ConditionValue::Bool(left == self.parseRelational()?);
            } else if self.matchOp("!=") {
                left = ConditionValue::Bool(left != self.parseRelational()?);
            } else {
                return Ok(left);
            }
        }
    }

    fn parseRelational(&mut self) -> Result<ConditionValue, String> {
        let mut left = self.parseUnary()?;
        loop {
            if self.matchOp(">=") {
                left = ConditionValue::Bool(
                    left.compareTo(&self.parseUnary()?)? != std::cmp::Ordering::Less,
                );
            } else if self.matchOp("<=") {
                left = ConditionValue::Bool(
                    left.compareTo(&self.parseUnary()?)? != std::cmp::Ordering::Greater,
                );
            } else if self.matchOp(">") {
                left = ConditionValue::Bool(
                    left.compareTo(&self.parseUnary()?)? == std::cmp::Ordering::Greater,
                );
            } else if self.matchOp("<") {
                left = ConditionValue::Bool(
                    left.compareTo(&self.parseUnary()?)? == std::cmp::Ordering::Less,
                );
            } else if self.matchOp("in") {
                let right = self.parseUnary()?;
                let ok = matches!(right, ConditionValue::Array(items) if items.iter().any(|item| item == &left));
                left = ConditionValue::Bool(ok);
            } else {
                return Ok(left);
            }
        }
    }

    fn parseUnary(&mut self) -> Result<ConditionValue, String> {
        if self.matchOp("!") {
            return Ok(ConditionValue::Bool(!self.parseUnary()?.isTruthy()));
        }
        self.parsePrimary()
    }

    fn parsePrimary(&mut self) -> Result<ConditionValue, String> {
        match self.peek().clone() {
            ConditionToken::BooleanLiteral(value) => {
                self.pos += 1;
                Ok(ConditionValue::Bool(value))
            }
            ConditionToken::NullLiteral => {
                self.pos += 1;
                Ok(ConditionValue::Null)
            }
            ConditionToken::NumberLiteral(value) => {
                self.pos += 1;
                Ok(ConditionValue::Num(value))
            }
            ConditionToken::StringLiteral(value) => {
                self.pos += 1;
                Ok(ConditionValue::Str(value))
            }
            ConditionToken::Identifier(name) => {
                self.pos += 1;
                Ok(self
                    .capabilities
                    .get(&name)
                    .cloned()
                    .unwrap_or(ConditionValue::Null))
            }
            ConditionToken::Punct('(') => {
                self.pos += 1;
                let inner = self.parseExpression()?;
                self.expectPunct(')')?;
                Ok(inner)
            }
            ConditionToken::Punct('[') => {
                self.pos += 1;
                let mut elements = Vec::new();
                if !self.checkPunct(']') {
                    elements.push(self.parseExpression()?);
                    while self.matchPunct(',') {
                        elements.push(self.parseExpression()?);
                    }
                }
                self.expectPunct(']')?;
                Ok(ConditionValue::Array(elements))
            }
            token => Err(format!("Unexpected token: {token:?}")),
        }
    }

    fn peek(&self) -> &ConditionToken {
        self.tokens.get(self.pos).unwrap_or(&ConditionToken::Eof)
    }

    fn matchOp(&mut self, op: &str) -> bool {
        if matches!(self.peek(), ConditionToken::Operator(value) if value == op) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn matchPunct(&mut self, ch: char) -> bool {
        if matches!(self.peek(), ConditionToken::Punct(value) if *value == ch) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn checkPunct(&self, ch: char) -> bool {
        matches!(self.peek(), ConditionToken::Punct(value) if *value == ch)
    }

    fn expectPunct(&mut self, ch: char) -> Result<(), String> {
        if self.matchPunct(ch) {
            Ok(())
        } else {
            Err(format!("Expected '{ch}'"))
        }
    }
}
