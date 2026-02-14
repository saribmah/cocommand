//! Query parser and tokenizer.

use crate::error::{FilesystemError, Result};

use super::date_filter::DatePredicate;
use super::expression::{query_expression_has_terms, QueryExpression, QueryFilter, QueryTerm};
use super::path::normalize_scope_filter_path;
use super::size::SizePredicate;
use super::type_filter::lookup_type_filter_target;

// ---------------------------------------------------------------------------
// Token types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct QueryToken {
    kind: QueryTokenKind,
    position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryTokenKind {
    Word(String),
    Phrase(String),
    LParen,
    RParen,
    LAngle,
    RAngle,
    Pipe,
    Bang,
    And,
    Or,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryGroupDelimiter {
    Parenthesis,
    Angle,
}

impl QueryGroupDelimiter {
    fn group_close_char(self) -> char {
        match self {
            Self::Parenthesis => ')',
            Self::Angle => '>',
        }
    }
}

// ---------------------------------------------------------------------------
// Query parser
// ---------------------------------------------------------------------------

pub struct QueryParser {
    tokens: Vec<QueryToken>,
    index: usize,
}

impl QueryParser {
    pub fn parse(input: &str) -> Result<QueryExpression> {
        let tokens = tokenize_query_input(input)?;
        if tokens.is_empty() {
            return Ok(QueryExpression::And(Vec::new()));
        }

        let mut parser = Self { tokens, index: 0 };
        let expression = parser.parse_and_expression(None)?;
        if parser.peek().is_some() {
            let token = parser.peek().expect("peek checked");
            return Err(FilesystemError::QueryParse(format!(
                "unexpected token near byte {}",
                token.position
            )));
        }
        if !query_expression_has_terms(&expression) {
            return Err(FilesystemError::QueryParse(
                "query must contain at least one term".to_string(),
            ));
        }

        Ok(expression)
    }

    fn parse_and_expression(
        &mut self,
        closing: Option<QueryGroupDelimiter>,
    ) -> Result<QueryExpression> {
        let mut parts = Vec::new();

        while !self.is_end() && !self.next_is_group_close(closing) {
            if self.consume_and_keyword() {
                continue;
            }

            parts.push(self.parse_or_expression(closing)?);

            if self.consume_and_keyword() {
                continue;
            }
            if self.next_starts_operand() {
                continue;
            }
            break;
        }

        Ok(match parts.len() {
            0 => QueryExpression::And(Vec::new()),
            1 => parts.remove(0),
            _ => QueryExpression::And(parts),
        })
    }

    fn parse_or_expression(
        &mut self,
        closing: Option<QueryGroupDelimiter>,
    ) -> Result<QueryExpression> {
        let mut parts = vec![self.parse_not_expression(closing)?];

        loop {
            if !self.consume_or_separator() {
                break;
            }
            if self.is_end() || self.next_is_group_close(closing) {
                break;
            }
            if self.peek_is_or_separator() {
                continue;
            }
            parts.push(self.parse_not_expression(closing)?);
        }

        Ok(match parts.len() {
            0 => QueryExpression::Or(Vec::new()),
            1 => parts.remove(0),
            _ => QueryExpression::Or(parts),
        })
    }

    fn parse_not_expression(
        &mut self,
        closing: Option<QueryGroupDelimiter>,
    ) -> Result<QueryExpression> {
        let mut negate = false;
        while self.consume_not_prefix() {
            negate = !negate;
        }

        let expression = self.parse_primary_expression(closing)?;
        if negate {
            Ok(QueryExpression::Not(Box::new(expression)))
        } else {
            Ok(expression)
        }
    }

    fn parse_primary_expression(
        &mut self,
        _closing: Option<QueryGroupDelimiter>,
    ) -> Result<QueryExpression> {
        if self.consume_group_open(QueryGroupDelimiter::Parenthesis) {
            return self.parse_group(QueryGroupDelimiter::Parenthesis);
        }
        if self.consume_group_open(QueryGroupDelimiter::Angle) {
            return self.parse_group(QueryGroupDelimiter::Angle);
        }

        let token = self.peek().ok_or_else(|| {
            FilesystemError::QueryParse("expected query term but reached end of query".to_string())
        })?;

        match &token.kind {
            QueryTokenKind::RParen | QueryTokenKind::RAngle => {
                let delimiter = if matches!(token.kind, QueryTokenKind::RParen) {
                    ")"
                } else {
                    ">"
                };
                Err(FilesystemError::QueryParse(format!(
                    "unexpected '{delimiter}' near byte {}",
                    token.position
                )))
            }
            QueryTokenKind::Word(_) | QueryTokenKind::Phrase(_) => {
                let token = self.next().expect("token exists");
                Ok(QueryExpression::Term(parse_query_term(&token)?))
            }
            _ => Err(FilesystemError::QueryParse(format!(
                "expected query term near byte {}",
                token.position
            ))),
        }
    }

    fn parse_group(&mut self, closing: QueryGroupDelimiter) -> Result<QueryExpression> {
        let expression = self.parse_and_expression(Some(closing))?;
        if self.consume_group_close(closing) {
            return Ok(expression);
        }

        let position = self
            .peek()
            .map(|token| token.position)
            .unwrap_or_else(|| self.last_position());
        Err(FilesystemError::QueryParse(format!(
            "missing closing '{}' near byte {position}",
            closing.group_close_char()
        )))
    }

    fn next_starts_operand(&self) -> bool {
        matches!(
            self.peek().map(|token| &token.kind),
            Some(
                QueryTokenKind::Word(_)
                    | QueryTokenKind::Phrase(_)
                    | QueryTokenKind::LParen
                    | QueryTokenKind::LAngle
                    | QueryTokenKind::Bang
                    | QueryTokenKind::Not
            )
        )
    }

    fn consume_and_keyword(&mut self) -> bool {
        matches!(
            self.peek().map(|token| &token.kind),
            Some(QueryTokenKind::And)
        ) && {
            self.index += 1;
            true
        }
    }

    fn consume_or_separator(&mut self) -> bool {
        match self.peek().map(|token| &token.kind) {
            Some(QueryTokenKind::Pipe | QueryTokenKind::Or) => {
                self.index += 1;
                true
            }
            _ => false,
        }
    }

    fn peek_is_or_separator(&self) -> bool {
        matches!(
            self.peek().map(|token| &token.kind),
            Some(QueryTokenKind::Pipe | QueryTokenKind::Or)
        )
    }

    fn consume_not_prefix(&mut self) -> bool {
        match self.peek().map(|token| &token.kind) {
            Some(QueryTokenKind::Bang | QueryTokenKind::Not) => {
                self.index += 1;
                true
            }
            _ => false,
        }
    }

    fn consume_group_open(&mut self, delimiter: QueryGroupDelimiter) -> bool {
        let expected = match delimiter {
            QueryGroupDelimiter::Parenthesis => QueryTokenKind::LParen,
            QueryGroupDelimiter::Angle => QueryTokenKind::LAngle,
        };
        matches!(self.peek().map(|token| &token.kind), Some(kind) if kind == &expected) && {
            self.index += 1;
            true
        }
    }

    fn consume_group_close(&mut self, delimiter: QueryGroupDelimiter) -> bool {
        let expected = match delimiter {
            QueryGroupDelimiter::Parenthesis => QueryTokenKind::RParen,
            QueryGroupDelimiter::Angle => QueryTokenKind::RAngle,
        };
        matches!(self.peek().map(|token| &token.kind), Some(kind) if kind == &expected) && {
            self.index += 1;
            true
        }
    }

    fn next_is_group_close(&self, delimiter: Option<QueryGroupDelimiter>) -> bool {
        match delimiter {
            None => false,
            Some(QueryGroupDelimiter::Parenthesis) => {
                matches!(
                    self.peek().map(|token| &token.kind),
                    Some(QueryTokenKind::RParen)
                )
            }
            Some(QueryGroupDelimiter::Angle) => {
                matches!(
                    self.peek().map(|token| &token.kind),
                    Some(QueryTokenKind::RAngle)
                )
            }
        }
    }

    fn is_end(&self) -> bool {
        self.index >= self.tokens.len()
    }

    fn peek(&self) -> Option<&QueryToken> {
        self.tokens.get(self.index)
    }

    fn next(&mut self) -> Option<QueryToken> {
        if self.is_end() {
            return None;
        }
        let token = self.tokens[self.index].clone();
        self.index += 1;
        Some(token)
    }

    fn last_position(&self) -> usize {
        self.tokens
            .last()
            .map(|token| token.position)
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// Term parsing
// ---------------------------------------------------------------------------

fn parse_query_term(token: &QueryToken) -> Result<QueryTerm> {
    match &token.kind {
        QueryTokenKind::Phrase(value) => Ok(QueryTerm::Text(value.clone())),
        QueryTokenKind::Word(raw) => {
            let Some(split) = raw.find(':') else {
                return Ok(QueryTerm::Text(raw.clone()));
            };
            if split == 0 {
                return Ok(QueryTerm::Text(raw.clone()));
            }
            let name = raw[..split].to_ascii_lowercase();
            let argument = raw[split + 1..].trim();
            match name.as_str() {
                "ext" => {
                    if argument.is_empty() {
                        return Err(FilesystemError::QueryParse(
                            "ext: requires at least one extension".to_string(),
                        ));
                    }
                    let values = argument
                        .split(';')
                        .filter_map(normalize_extension)
                        .collect::<Vec<_>>();
                    if values.is_empty() {
                        return Err(FilesystemError::QueryParse(
                            "ext: requires non-empty extensions".to_string(),
                        ));
                    }
                    Ok(QueryTerm::Filter(QueryFilter::Extension(values)))
                }
                "type" => {
                    if argument.is_empty() {
                        return Err(FilesystemError::QueryParse(
                            "type: requires a category".to_string(),
                        ));
                    }
                    let normalized = argument.to_ascii_lowercase();
                    let target =
                        lookup_type_filter_target(normalized.as_str()).ok_or_else(|| {
                            FilesystemError::QueryParse(format!(
                                "unknown type category: {argument}"
                            ))
                        })?;
                    Ok(QueryTerm::Filter(QueryFilter::Type(target)))
                }
                "size" => Ok(QueryTerm::Filter(QueryFilter::Size(SizePredicate::parse(
                    argument,
                )?))),
                "audio" | "video" | "doc" | "exe" => {
                    let target = lookup_type_filter_target(name.as_str()).ok_or_else(|| {
                        FilesystemError::QueryParse(format!(
                            "missing built-in type macro mapping: {name}"
                        ))
                    })?;
                    let macro_argument = if argument.is_empty() {
                        None
                    } else {
                        Some(argument.to_string())
                    };
                    Ok(QueryTerm::Filter(QueryFilter::TypeMacro {
                        target,
                        argument: macro_argument,
                    }))
                }
                "file" => {
                    let maybe_argument = if argument.is_empty() {
                        None
                    } else {
                        Some(argument.to_string())
                    };
                    Ok(QueryTerm::Filter(QueryFilter::File {
                        argument: maybe_argument,
                    }))
                }
                "folder" => {
                    let maybe_argument = if argument.is_empty() {
                        None
                    } else {
                        Some(argument.to_string())
                    };
                    Ok(QueryTerm::Filter(QueryFilter::Folder {
                        argument: maybe_argument,
                    }))
                }
                "parent" => {
                    let normalized = normalize_scope_filter_path(argument, "parent")?;
                    Ok(QueryTerm::Filter(QueryFilter::Parent { path: normalized }))
                }
                "in" | "infolder" => {
                    let normalized = normalize_scope_filter_path(argument, name.as_str())?;
                    Ok(QueryTerm::Filter(QueryFilter::InFolder {
                        path: normalized,
                    }))
                }
                "nosubfolders" => {
                    let normalized = normalize_scope_filter_path(argument, "nosubfolders")?;
                    Ok(QueryTerm::Filter(QueryFilter::NoSubfolders {
                        path: normalized,
                    }))
                }
                "content" => {
                    if argument.is_empty() {
                        return Err(FilesystemError::QueryParse(
                            "content: requires a search value".to_string(),
                        ));
                    }
                    Ok(QueryTerm::Filter(QueryFilter::Content {
                        needle: argument.to_string(),
                    }))
                }
                "tag" | "tags" => {
                    if argument.is_empty() {
                        return Err(FilesystemError::QueryParse(
                            "tag: requires at least one tag name".to_string(),
                        ));
                    }
                    // Support multiple tags separated by semicolon (like ext:)
                    let tags = argument
                        .split(';')
                        .map(|t| t.trim())
                        .filter(|t| !t.is_empty())
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>();
                    if tags.is_empty() {
                        return Err(FilesystemError::QueryParse(
                            "tag: requires non-empty tag names".to_string(),
                        ));
                    }
                    Ok(QueryTerm::Filter(QueryFilter::Tag { tags }))
                }
                "dm" | "datemodified" => {
                    let predicate = DatePredicate::parse(argument)?;
                    Ok(QueryTerm::Filter(QueryFilter::DateModified(predicate)))
                }
                "dc" | "datecreated" => {
                    let predicate = DatePredicate::parse(argument)?;
                    Ok(QueryTerm::Filter(QueryFilter::DateCreated(predicate)))
                }
                _ => Ok(QueryTerm::Text(raw.clone())),
            }
        }
        _ => Err(FilesystemError::QueryParse(
            "invalid query token while parsing term".to_string(),
        )),
    }
}

fn normalize_extension(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_start_matches('.');
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_ascii_lowercase())
    }
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

fn tokenize_query_input(input: &str) -> Result<Vec<QueryToken>> {
    let mut tokens = Vec::new();
    let mut cursor = 0usize;

    while cursor < input.len() {
        let rest = &input[cursor..];
        let ch = rest.chars().next().expect("cursor checked");
        if ch.is_whitespace() {
            cursor += ch.len_utf8();
            continue;
        }

        let position = cursor;
        match ch {
            '(' => {
                tokens.push(QueryToken {
                    kind: QueryTokenKind::LParen,
                    position,
                });
                cursor += 1;
            }
            ')' => {
                tokens.push(QueryToken {
                    kind: QueryTokenKind::RParen,
                    position,
                });
                cursor += 1;
            }
            '<' => {
                tokens.push(QueryToken {
                    kind: QueryTokenKind::LAngle,
                    position,
                });
                cursor += 1;
            }
            '>' => {
                tokens.push(QueryToken {
                    kind: QueryTokenKind::RAngle,
                    position,
                });
                cursor += 1;
            }
            '|' => {
                tokens.push(QueryToken {
                    kind: QueryTokenKind::Pipe,
                    position,
                });
                cursor += 1;
            }
            '!' => {
                tokens.push(QueryToken {
                    kind: QueryTokenKind::Bang,
                    position,
                });
                cursor += 1;
            }
            '"' => {
                let (phrase, next_cursor) = consume_quoted_phrase(input, cursor)?;
                tokens.push(QueryToken {
                    kind: QueryTokenKind::Phrase(phrase),
                    position,
                });
                cursor = next_cursor;
            }
            _ => {
                let mut end = cursor;
                let mut seen_colon = false;
                while end < input.len() {
                    let next = input[end..].chars().next().expect("end checked");
                    if next == ':' {
                        seen_colon = true;
                    }
                    if next.is_whitespace() || matches!(next, '(' | ')' | '|' | '!') {
                        break;
                    }
                    if !seen_colon && matches!(next, '<' | '>') {
                        break;
                    }
                    end += next.len_utf8();
                }

                let raw = &input[cursor..end];
                let kind = if raw.eq_ignore_ascii_case("and") {
                    QueryTokenKind::And
                } else if raw.eq_ignore_ascii_case("or") {
                    QueryTokenKind::Or
                } else if raw.eq_ignore_ascii_case("not") {
                    QueryTokenKind::Not
                } else {
                    QueryTokenKind::Word(raw.to_string())
                };
                tokens.push(QueryToken { kind, position });
                cursor = end;
            }
        }
    }

    Ok(tokens)
}

fn consume_quoted_phrase(input: &str, start: usize) -> Result<(String, usize)> {
    let mut cursor = start + 1;
    let mut phrase = String::new();
    let mut escaped = false;

    while cursor < input.len() {
        let ch = input[cursor..].chars().next().expect("cursor checked");
        cursor += ch.len_utf8();

        if escaped {
            phrase.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            return Ok((phrase, cursor));
        }

        phrase.push(ch);
    }

    Err(FilesystemError::QueryParse(format!(
        "missing closing quote near byte {start}"
    )))
}
