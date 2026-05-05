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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RequiredFields {
    pub chrom: bool,
    pub pos: bool,
    pub qual: bool,
    pub filter: bool,
    pub info_keys: Vec<Vec<u8>>,
    pub format_keys: Vec<Vec<u8>>,
    pub format_aggregates: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RequiredFormatFields {
    pub gt: bool,
    pub dp: bool,
    pub gq: bool,
}

impl RequiredFields {
    pub(crate) fn requires_info(&self) -> bool {
        !self.info_keys.is_empty()
    }

    pub(crate) fn requires_format(&self) -> bool {
        !self.format_keys.is_empty() || self.format_aggregates
    }

    pub(crate) fn legacy_format_fields(&self) -> RequiredFormatFields {
        RequiredFormatFields {
            gt: self.format_keys.iter().any(|key| key.as_slice() == b"GT"),
            dp: self.format_keys.iter().any(|key| key.as_slice() == b"DP"),
            gq: self.format_keys.iter().any(|key| key.as_slice() == b"GQ"),
        }
    }

    fn add_info_key(&mut self, key: &[u8]) {
        if !self.info_keys.iter().any(|existing| existing == key) {
            self.info_keys.push(key.to_vec());
        }
    }

    fn add_format_key(&mut self, key: &[u8]) {
        if !self.format_keys.iter().any(|existing| existing == key) {
            self.format_keys.push(key.to_vec());
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Comparison {
    field: Field,
    op: Operator,
    literal: Literal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Field {
    Chrom,
    Pos,
    Qual,
    Filter,
    Info(Vec<u8>),
    Format(Vec<u8>),
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
    pub format: FormatValues<'a>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FormatValues<'a> {
    pub gt: Option<&'a str>,
    pub dp: Option<&'a str>,
    pub gq: Option<&'a str>,
}

pub(crate) trait EvalContext {
    fn chrom(&self) -> Option<&[u8]>;
    fn pos(&self) -> Option<u64>;
    fn qual(&self) -> Option<f64>;
    fn filter(&self) -> Option<&[u8]>;
    fn info_value(&self, key: &[u8]) -> Option<&[u8]>;
    fn info_number_any(&self, key: &[u8], predicate: &mut dyn FnMut(f64) -> bool) -> bool;
    fn format_gt(&self) -> Option<&[u8]>;
    fn format_dp(&self) -> Option<&[u8]>;
    fn format_gq(&self) -> Option<&[u8]>;
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
        self.evaluate_context(record)
    }

    pub fn evaluate_record(&self, record: &EvalRecord<'_>) -> bool {
        self.evaluate(record)
    }

    pub(crate) fn evaluate_context(&self, record: &impl EvalContext) -> bool {
        self.root.evaluate(record)
    }

    pub(crate) fn required_fields(&self) -> RequiredFields {
        let mut required = RequiredFields::default();
        self.root.collect_required_fields(&mut required);
        required
    }
}

impl<'a> EvalRecord<'a> {
    pub fn new(chrom: &'a [u8], pos: Option<u64>, qual: Option<f64>, filter: &'a [u8]) -> Self {
        Self {
            chrom: std::str::from_utf8(chrom).unwrap_or(""),
            pos: pos.unwrap_or_default(),
            qual,
            filter: std::str::from_utf8(filter).unwrap_or(""),
            info: ".",
            format: FormatValues::default(),
        }
    }

    pub fn with_info(mut self, info: &'a [u8]) -> Self {
        self.info = std::str::from_utf8(info).unwrap_or(".");
        self
    }
}

impl ExprNode {
    fn evaluate(&self, record: &impl EvalContext) -> bool {
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
    fn evaluate(&self, record: &impl EvalContext) -> bool {
        match (&self.field, &self.literal) {
            (Field::Chrom, Literal::String(expected)) => record
                .chrom()
                .is_some_and(|actual| compare_bytes(actual, expected.as_bytes(), self.op)),
            (Field::Filter, Literal::String(expected)) => record
                .filter()
                .is_some_and(|actual| compare_bytes(actual, expected.as_bytes(), self.op)),
            (Field::Pos, Literal::Number(expected)) => record
                .pos()
                .is_some_and(|actual| compare_numbers(actual as f64, *expected, self.op)),
            (Field::Qual, Literal::Number(expected)) => record
                .qual()
                .is_some_and(|actual| compare_numbers(actual, *expected, self.op)),
            (Field::Info(key), Literal::Number(expected)) => {
                let mut predicate = |actual| compare_numbers(actual, *expected, self.op);
                record.info_number_any(key, &mut predicate)
            }
            (Field::Info(key), Literal::String(expected)) => record
                .info_value(key)
                .filter(|value| is_present_value(value))
                .is_some_and(|actual| compare_bytes(actual, expected.as_bytes(), self.op)),
            (Field::Format(key), Literal::String(expected)) if key == b"GT" => record
                .format_gt()
                .and_then(parse_format_string_bytes)
                .is_some_and(|actual| compare_bytes(actual, expected.as_bytes(), self.op)),
            (Field::Format(key), Literal::Number(expected)) if key == b"DP" => record
                .format_dp()
                .and_then(parse_format_number_bytes)
                .is_some_and(|actual| compare_numbers(actual, *expected, self.op)),
            (Field::Format(key), Literal::Number(expected)) if key == b"GQ" => record
                .format_gq()
                .and_then(parse_format_number_bytes)
                .is_some_and(|actual| compare_numbers(actual, *expected, self.op)),
            _ => false,
        }
    }

    fn collect_required_fields(&self, required: &mut RequiredFields) {
        mark_required_field(required, &self.field);
    }
}

fn mark_required_field(required: &mut RequiredFields, field: &Field) {
    match field {
        Field::Chrom => required.chrom = true,
        Field::Pos => required.pos = true,
        Field::Qual => required.qual = true,
        Field::Filter => required.filter = true,
        Field::Info(key) => required.add_info_key(key),
        Field::Format(key) => required.add_format_key(key),
    }
}

impl EvalContext for EvalRecord<'_> {
    fn chrom(&self) -> Option<&[u8]> {
        Some(self.chrom.as_bytes())
    }

    fn pos(&self) -> Option<u64> {
        Some(self.pos)
    }

    fn qual(&self) -> Option<f64> {
        self.qual
    }

    fn filter(&self) -> Option<&[u8]> {
        Some(self.filter.as_bytes())
    }

    fn info_value(&self, key: &[u8]) -> Option<&[u8]> {
        vcf::InfoView::scan(self.info.as_bytes()).value(key)
    }

    fn info_number_any(&self, key: &[u8], predicate: &mut dyn FnMut(f64) -> bool) -> bool {
        vcf::InfoView::scan(self.info.as_bytes()).number_any(key, predicate)
    }

    fn format_gt(&self) -> Option<&[u8]> {
        self.format.gt.map(str::as_bytes)
    }

    fn format_dp(&self) -> Option<&[u8]> {
        self.format.dp.map(str::as_bytes)
    }

    fn format_gq(&self) -> Option<&[u8]> {
        self.format.gq.map(str::as_bytes)
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

fn compare_bytes(actual: &[u8], expected: &[u8], op: Operator) -> bool {
    match op {
        Operator::Eq => actual == expected,
        Operator::Ne => actual != expected,
        _ => false,
    }
}

fn is_present_value(value: &[u8]) -> bool {
    !value.is_empty() && value != b"."
}

fn parse_format_number_bytes(value: &[u8]) -> Option<f64> {
    if value == b"." || value.is_empty() {
        None
    } else {
        std::str::from_utf8(value).ok()?.parse::<f64>().ok()
    }
}

fn parse_format_string_bytes(value: &[u8]) -> Option<&[u8]> {
    if value == b"." || value.is_empty() {
        None
    } else {
        Some(value)
    }
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
            Some(Token::Ident(value)) => parse_field_name(&value),
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

fn parse_field_name(name: &str) -> Result<Field, ParseError> {
    match name {
        "CHROM" => Ok(Field::Chrom),
        "POS" => Ok(Field::Pos),
        "QUAL" => Ok(Field::Qual),
        "FILTER" => Ok(Field::Filter),
        "DP" => Ok(Field::Info(b"DP".to_vec())),
        "AF" => Ok(Field::Info(b"AF".to_vec())),
        _ if name.starts_with("INFO/") && name.len() > "INFO/".len() => {
            Ok(Field::Info(name.as_bytes()["INFO/".len()..].to_vec()))
        }
        _ if name.starts_with("FORMAT/") && name.len() > "FORMAT/".len() => {
            Ok(Field::Format(name.as_bytes()["FORMAT/".len()..].to_vec()))
        }
        _ => Err(ParseError {
            message: format!("unsupported field '{name}'"),
        }),
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

#[cfg(test)]
mod tests {
    use super::{EvalContext, EvalRecord, FormatValues, parse_expression};
    use crate::vcf::{InfoView, RecordView};

    struct ByteContext<'a> {
        record: RecordView<'a>,
        info: InfoView<'a>,
        format: FormatValues<'a>,
    }

    impl<'a> ByteContext<'a> {
        fn parse(line: &'a [u8]) -> Self {
            let record = RecordView::parse(line).unwrap();
            let info = InfoView::scan(record.info());
            Self {
                record,
                info,
                format: FormatValues::default(),
            }
        }
    }

    impl EvalContext for ByteContext<'_> {
        fn chrom(&self) -> Option<&[u8]> {
            Some(self.record.chrom())
        }

        fn pos(&self) -> Option<u64> {
            self.record.pos_u64().ok()
        }

        fn qual(&self) -> Option<f64> {
            self.record.qual_float().ok().flatten()
        }

        fn filter(&self) -> Option<&[u8]> {
            Some(self.record.filter())
        }

        fn info_value(&self, key: &[u8]) -> Option<&[u8]> {
            self.info.value(key)
        }

        fn info_number_any(&self, key: &[u8], predicate: &mut dyn FnMut(f64) -> bool) -> bool {
            self.info.number_any(key, predicate)
        }

        fn format_gt(&self) -> Option<&[u8]> {
            self.format.gt.map(str::as_bytes)
        }

        fn format_dp(&self) -> Option<&[u8]> {
            self.format.dp.map(str::as_bytes)
        }

        fn format_gq(&self) -> Option<&[u8]> {
            self.format.gq.map(str::as_bytes)
        }
    }

    #[test]
    fn evaluates_string_and_numeric_predicates_against_byte_context() {
        let expr = parse_expression("CHROM == \"1\" && QUAL >= 30 && AF > 0.01").unwrap();
        let byte_record = ByteContext::parse(b"1\t20\t.\tA\tG\t42\tPASS\tDP=11;AF=0.005,0.02\n");
        let string_record = EvalRecord {
            chrom: "1",
            pos: 20,
            qual: Some(42.0),
            filter: "PASS",
            info: "DP=11;AF=0.005,0.02",
            format: FormatValues::default(),
        };

        assert_eq!(
            expr.evaluate_context(&byte_record),
            expr.evaluate(&string_record)
        );
        assert!(expr.evaluate_context(&byte_record));
    }

    #[test]
    fn required_fields_preserve_arbitrary_info_and_format_keys() {
        let expr = parse_expression("INFO/MQ > 50 && FORMAT/AD > 8 && FORMAT/DP > 10").unwrap();
        let required = expr.required_fields();

        assert_eq!(required.info_keys, vec![b"MQ".to_vec()]);
        assert_eq!(required.format_keys, vec![b"AD".to_vec(), b"DP".to_vec()]);
        assert!(required.requires_info());
        assert!(required.requires_format());
        assert!(required.legacy_format_fields().dp);
        assert!(!required.legacy_format_fields().gt);
    }
}
