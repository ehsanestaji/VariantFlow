use std::fmt;

use thiserror::Error;

use crate::vcf;

#[derive(Debug, Clone, PartialEq)]
pub struct Expression {
    root: ExprNode,
}

#[derive(Debug, Clone, PartialEq)]
enum ExprNode {
    Comparison(Comparison),
    And(Box<ExprNode>, Box<ExprNode>),
    Or(Box<ExprNode>, Box<ExprNode>),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RequiredFields {
    pub chrom: bool,
    pub pos: bool,
    pub qual: bool,
    pub filter: bool,
    pub info: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct Comparison {
    field: Field,
    op: Operator,
    literal: Literal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Field {
    Chrom,
    Pos,
    Qual,
    Filter,
    Dp,
    Af,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operator {
    Gt,
    Gte,
    Lt,
    Lte,
    Eq,
    Ne,
}

#[derive(Debug, Clone, PartialEq)]
enum Literal {
    Number(f64),
    String(String),
}

#[derive(Debug, Clone, Copy)]
pub struct EvalRecord<'a> {
    pub chrom: &'a str,
    pub pos: u64,
    pub qual: Option<f64>,
    pub filter: &'a str,
    pub info: &'a str,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{message}")]
pub struct ParseError {
    message: String,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Ident(String),
    Number(f64),
    String(String),
    Op(Operator),
    And,
    Or,
    LeftParen,
    RightParen,
}

pub fn parse_expression(input: &str) -> Result<Expression, ParseError> {
    let tokens = tokenize(input)?;
    let mut parser = Parser { tokens, cursor: 0 };
    let root = parser.parse_or()?;

    if !parser.is_done() {
        return Err(parser.error("unexpected token"));
    }

    Ok(Expression { root })
}

impl Expression {
    pub fn evaluate(&self, record: &EvalRecord<'_>) -> bool {
        self.root.evaluate(record)
    }

    pub(crate) fn required_fields(&self) -> RequiredFields {
        let mut required = RequiredFields::default();
        self.root.collect_required_fields(&mut required);
        required
    }
}

impl ExprNode {
    fn evaluate(&self, record: &EvalRecord<'_>) -> bool {
        match self {
            ExprNode::Comparison(comparison) => comparison.evaluate(record),
            ExprNode::And(left, right) => left.evaluate(record) && right.evaluate(record),
            ExprNode::Or(left, right) => left.evaluate(record) || right.evaluate(record),
        }
    }

    fn collect_required_fields(&self, required: &mut RequiredFields) {
        match self {
            ExprNode::Comparison(comparison) => comparison.collect_required_fields(required),
            ExprNode::And(left, right) | ExprNode::Or(left, right) => {
                left.collect_required_fields(required);
                right.collect_required_fields(required);
            }
        }
    }
}

impl Comparison {
    fn evaluate(&self, record: &EvalRecord<'_>) -> bool {
        match (self.field, &self.literal) {
            (Field::Chrom, Literal::String(expected)) => {
                compare_strings(record.chrom, expected, self.op)
            }
            (Field::Filter, Literal::String(expected)) => {
                compare_strings(record.filter, expected, self.op)
            }
            (Field::Pos, Literal::Number(expected)) => {
                compare_numbers(record.pos as f64, *expected, self.op)
            }
            (Field::Qual, Literal::Number(expected)) => record
                .qual
                .is_some_and(|actual| compare_numbers(actual, *expected, self.op)),
            (Field::Dp, Literal::Number(expected)) => {
                info_number_any(record.info, "DP", *expected, self.op)
            }
            (Field::Af, Literal::Number(expected)) => {
                info_number_any(record.info, "AF", *expected, self.op)
            }
            _ => false,
        }
    }

    fn collect_required_fields(&self, required: &mut RequiredFields) {
        match self.field {
            Field::Chrom => required.chrom = true,
            Field::Pos => required.pos = true,
            Field::Qual => required.qual = true,
            Field::Filter => required.filter = true,
            Field::Dp | Field::Af => required.info = true,
        }
    }
}

fn compare_numbers(actual: f64, expected: f64, op: Operator) -> bool {
    match op {
        Operator::Gt => actual > expected,
        Operator::Gte => actual >= expected,
        Operator::Lt => actual < expected,
        Operator::Lte => actual <= expected,
        Operator::Eq => actual == expected,
        Operator::Ne => actual != expected,
    }
}

fn compare_strings(actual: &str, expected: &str, op: Operator) -> bool {
    match op {
        Operator::Eq => actual == expected,
        Operator::Ne => actual != expected,
        _ => false,
    }
}

fn info_number_any(info: &str, key: &str, expected: f64, op: Operator) -> bool {
    vcf::info_number_any(info, key, |actual| compare_numbers(actual, expected, op))
}

struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

impl Parser {
    fn parse_or(&mut self) -> Result<ExprNode, ParseError> {
        let mut node = self.parse_and()?;

        while self.match_token(|token| matches!(token, Token::Or)) {
            let right = self.parse_and()?;
            node = ExprNode::Or(Box::new(node), Box::new(right));
        }

        Ok(node)
    }

    fn parse_and(&mut self) -> Result<ExprNode, ParseError> {
        let mut node = self.parse_primary()?;

        while self.match_token(|token| matches!(token, Token::And)) {
            let right = self.parse_primary()?;
            node = ExprNode::And(Box::new(node), Box::new(right));
        }

        Ok(node)
    }

    fn parse_primary(&mut self) -> Result<ExprNode, ParseError> {
        if self.match_token(|token| matches!(token, Token::LeftParen)) {
            let expression = self.parse_or()?;
            self.expect_right_paren()?;
            return Ok(expression);
        }

        Ok(ExprNode::Comparison(self.parse_comparison()?))
    }

    fn parse_comparison(&mut self) -> Result<Comparison, ParseError> {
        let field = self.parse_field()?;
        let op = self.parse_operator()?;
        let literal = self.parse_literal()?;

        Ok(Comparison { field, op, literal })
    }

    fn parse_field(&mut self) -> Result<Field, ParseError> {
        match self.advance() {
            Some(Token::Ident(value)) => match value.as_str() {
                "CHROM" => Ok(Field::Chrom),
                "POS" => Ok(Field::Pos),
                "QUAL" => Ok(Field::Qual),
                "FILTER" => Ok(Field::Filter),
                "DP" | "INFO/DP" => Ok(Field::Dp),
                "AF" | "INFO/AF" => Ok(Field::Af),
                _ => Err(ParseError {
                    message: format!("unsupported field '{value}'"),
                }),
            },
            _ => Err(self.error("expected field")),
        }
    }

    fn parse_operator(&mut self) -> Result<Operator, ParseError> {
        match self.advance() {
            Some(Token::Op(op)) => Ok(op),
            _ => Err(self.error("expected operator")),
        }
    }

    fn parse_literal(&mut self) -> Result<Literal, ParseError> {
        match self.advance() {
            Some(Token::Number(value)) => Ok(Literal::Number(value)),
            Some(Token::String(value)) => Ok(Literal::String(value)),
            _ => Err(self.error("expected literal")),
        }
    }

    fn expect_right_paren(&mut self) -> Result<(), ParseError> {
        if self.match_token(|token| matches!(token, Token::RightParen)) {
            Ok(())
        } else {
            Err(self.error("expected ')'"))
        }
    }

    fn match_token(&mut self, predicate: impl FnOnce(&Token) -> bool) -> bool {
        if self.peek().is_some_and(predicate) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.cursor).cloned();
        if token.is_some() {
            self.cursor += 1;
        }
        token
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    fn is_done(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn error(&self, message: &str) -> ParseError {
        ParseError {
            message: message.to_string(),
        }
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut cursor = 0;

    while cursor < chars.len() {
        match chars[cursor] {
            char if char.is_whitespace() => cursor += 1,
            '&' if chars.get(cursor + 1) == Some(&'&') => {
                tokens.push(Token::And);
                cursor += 2;
            }
            '|' if chars.get(cursor + 1) == Some(&'|') => {
                tokens.push(Token::Or);
                cursor += 2;
            }
            '(' => {
                tokens.push(Token::LeftParen);
                cursor += 1;
            }
            ')' => {
                tokens.push(Token::RightParen);
                cursor += 1;
            }
            '>' | '<' | '=' | '!' => {
                let (op, consumed) = read_operator(&chars, cursor)?;
                tokens.push(Token::Op(op));
                cursor += consumed;
            }
            '"' => {
                let (value, consumed) = read_string(&chars, cursor)?;
                tokens.push(Token::String(value));
                cursor += consumed;
            }
            '-' | '0'..='9' => {
                let (value, consumed) = read_number(&chars, cursor)?;
                tokens.push(Token::Number(value));
                cursor += consumed;
            }
            char if is_ident_start(char) => {
                let (value, consumed) = read_ident(&chars, cursor);
                tokens.push(Token::Ident(value));
                cursor += consumed;
            }
            other => {
                return Err(ParseError {
                    message: format!("unexpected character '{other}'"),
                });
            }
        }
    }

    Ok(tokens)
}

fn read_operator(chars: &[char], cursor: usize) -> Result<(Operator, usize), ParseError> {
    let current = chars[cursor];
    let next = chars.get(cursor + 1);

    match (current, next) {
        ('>', Some('=')) => Ok((Operator::Gte, 2)),
        ('<', Some('=')) => Ok((Operator::Lte, 2)),
        ('=', Some('=')) => Ok((Operator::Eq, 2)),
        ('!', Some('=')) => Ok((Operator::Ne, 2)),
        ('>', _) => Ok((Operator::Gt, 1)),
        ('<', _) => Ok((Operator::Lt, 1)),
        _ => Err(ParseError {
            message: "expected operator".to_string(),
        }),
    }
}

fn read_string(chars: &[char], cursor: usize) -> Result<(String, usize), ParseError> {
    let mut value = String::new();
    let mut consumed = 1;

    while cursor + consumed < chars.len() {
        let char = chars[cursor + consumed];
        consumed += 1;

        if char == '"' {
            return Ok((value, consumed));
        }

        value.push(char);
    }

    Err(ParseError {
        message: "unterminated string literal".to_string(),
    })
}

fn read_number(chars: &[char], cursor: usize) -> Result<(f64, usize), ParseError> {
    let mut consumed = 0;

    while cursor + consumed < chars.len() {
        let char = chars[cursor + consumed];
        if char.is_ascii_digit() || char == '.' || (char == '-' && consumed == 0) {
            consumed += 1;
        } else {
            break;
        }
    }

    let raw: String = chars[cursor..cursor + consumed].iter().collect();
    let value = raw.parse::<f64>().map_err(|_| ParseError {
        message: format!("invalid numeric literal '{raw}'"),
    })?;

    Ok((value, consumed))
}

fn read_ident(chars: &[char], cursor: usize) -> (String, usize) {
    let mut consumed = 0;

    while cursor + consumed < chars.len() && is_ident_continue(chars[cursor + consumed]) {
        consumed += 1;
    }

    (chars[cursor..cursor + consumed].iter().collect(), consumed)
}

fn is_ident_start(char: char) -> bool {
    char.is_ascii_alphabetic() || char == '_'
}

fn is_ident_continue(char: char) -> bool {
    char.is_ascii_alphanumeric() || char == '_' || char == '/'
}

impl fmt::Display for Expression {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
