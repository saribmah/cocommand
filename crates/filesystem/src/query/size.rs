//! Size predicate parsing and matching.

use crate::error::{FilesystemError, Result};

/// A parsed size predicate for filtering by file size.
#[derive(Debug, Clone)]
pub struct SizePredicate {
    kind: SizePredicateKind,
}

#[derive(Debug, Clone)]
pub enum SizePredicateKind {
    Comparison { op: SizeComparisonOp, value: u64 },
    Range { min: Option<u64>, max: Option<u64> },
}

#[derive(Debug, Clone, Copy)]
pub enum SizeComparisonOp {
    Lt,
    Lte,
    Gt,
    Gte,
    Eq,
    Ne,
}

impl SizePredicate {
    pub fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(FilesystemError::QueryParse(
                "size: requires a value".to_string(),
            ));
        }

        if let Some((op, value_raw)) = parse_size_comparison(trimmed) {
            if size_keyword(value_raw).is_some() {
                return Err(FilesystemError::QueryParse(
                    "size keywords cannot be used with comparison operators".to_string(),
                ));
            }
            let value = parse_size_literal(value_raw)?;
            return Ok(Self {
                kind: SizePredicateKind::Comparison { op, value },
            });
        }

        if let Some((start_raw, end_raw)) = parse_size_range(trimmed) {
            let min = if start_raw.is_empty() {
                None
            } else {
                Some(parse_size_literal(start_raw)?)
            };
            let max = if end_raw.is_empty() {
                None
            } else {
                Some(parse_size_literal(end_raw)?)
            };
            if let (Some(start), Some(end)) = (min, max) {
                if start > end {
                    return Err(FilesystemError::QueryParse(
                        "size range start must be less than or equal to end".to_string(),
                    ));
                }
            }
            return Ok(Self {
                kind: SizePredicateKind::Range { min, max },
            });
        }

        if let Some((min, max)) = size_keyword(trimmed) {
            return Ok(Self {
                kind: SizePredicateKind::Range { min, max },
            });
        }

        Ok(Self {
            kind: SizePredicateKind::Comparison {
                op: SizeComparisonOp::Eq,
                value: parse_size_literal(trimmed)?,
            },
        })
    }

    pub fn matches(&self, value: u64) -> bool {
        match &self.kind {
            SizePredicateKind::Comparison {
                op: SizeComparisonOp::Lt,
                value: right,
            } => value < *right,
            SizePredicateKind::Comparison {
                op: SizeComparisonOp::Lte,
                value: right,
            } => value <= *right,
            SizePredicateKind::Comparison {
                op: SizeComparisonOp::Gt,
                value: right,
            } => value > *right,
            SizePredicateKind::Comparison {
                op: SizeComparisonOp::Gte,
                value: right,
            } => value >= *right,
            SizePredicateKind::Comparison {
                op: SizeComparisonOp::Eq,
                value: right,
            } => value == *right,
            SizePredicateKind::Comparison {
                op: SizeComparisonOp::Ne,
                value: right,
            } => value != *right,
            SizePredicateKind::Range { min, max } => {
                if let Some(minimum) = min {
                    if value < *minimum {
                        return false;
                    }
                }
                if let Some(maximum) = max {
                    if value > *maximum {
                        return false;
                    }
                }
                true
            }
        }
    }
}

fn parse_size_comparison(raw: &str) -> Option<(SizeComparisonOp, &str)> {
    for (operator, kind) in [
        ("<=", SizeComparisonOp::Lte),
        (">=", SizeComparisonOp::Gte),
        ("!=", SizeComparisonOp::Ne),
        ("<", SizeComparisonOp::Lt),
        (">", SizeComparisonOp::Gt),
        ("=", SizeComparisonOp::Eq),
    ] {
        if let Some(value) = raw.strip_prefix(operator) {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return None;
            }
            return Some((kind, trimmed));
        }
    }
    None
}

fn parse_size_range(raw: &str) -> Option<(&str, &str)> {
    let split = raw.find("..")?;
    let start = raw[..split].trim();
    let end = raw[split + 2..].trim();
    if start.is_empty() && end.is_empty() {
        return None;
    }
    Some((start, end))
}

const KB: u64 = 1024;
const MB: u64 = 1024 * 1024;

fn size_keyword(raw: &str) -> Option<(Option<u64>, Option<u64>)> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "empty" => Some((Some(0), Some(0))),
        "tiny" => Some((Some(0), Some(10 * KB))),
        "small" => Some((Some(10 * KB + 1), Some(100 * KB))),
        "medium" => Some((Some(100 * KB + 1), Some(MB))),
        "large" => Some((Some(MB + 1), Some(16 * MB))),
        "huge" => Some((Some(16 * MB + 1), Some(128 * MB))),
        "gigantic" | "giant" => Some((Some(128 * MB + 1), None)),
        _ => None,
    }
}

fn parse_size_literal(raw: &str) -> Result<u64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(FilesystemError::QueryParse(
            "size: expected a number".to_string(),
        ));
    }

    let mut split = trimmed.len();
    for (index, ch) in trimmed.char_indices() {
        if ch.is_ascii_digit() || ch == '.' {
            continue;
        }
        split = index;
        break;
    }
    let (number_part, unit_part) = trimmed.split_at(split);
    if number_part.is_empty() {
        return Err(FilesystemError::QueryParse(format!(
            "size: expected a numeric value in {raw:?}"
        )));
    }

    let value: f64 = number_part.parse().map_err(|_| {
        FilesystemError::QueryParse(format!("size: failed to parse number in {raw:?}"))
    })?;
    let multiplier = size_unit_multiplier(unit_part)?;
    let bytes = (value * multiplier as f64).round();
    if !bytes.is_finite() || bytes < 0.0 {
        return Err(FilesystemError::QueryParse(format!(
            "size: value {raw:?} is out of range"
        )));
    }

    if bytes > u64::MAX as f64 {
        Ok(u64::MAX)
    } else {
        Ok(bytes as u64)
    }
}

fn size_unit_multiplier(unit: &str) -> Result<u64> {
    match unit.trim().to_ascii_lowercase().as_str() {
        "" | "b" | "byte" | "bytes" => Ok(1),
        "k" | "kb" | "kib" | "kilobyte" | "kilobytes" => Ok(1024),
        "m" | "mb" | "mib" | "megabyte" | "megabytes" => Ok(1024 * 1024),
        "g" | "gb" | "gib" | "gigabyte" | "gigabytes" => Ok(1024 * 1024 * 1024),
        "t" | "tb" | "tib" | "terabyte" | "terabytes" => Ok(1024_u64.pow(4)),
        "p" | "pb" | "pib" | "petabyte" | "petabytes" => Ok(1024_u64.pow(5)),
        _ => Err(FilesystemError::QueryParse(format!(
            "unknown size unit: {unit}"
        ))),
    }
}
