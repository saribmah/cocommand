//! Date predicate parsing and matching.
//!
//! This module provides date/time filtering capabilities
//! `dm:` (date modified) `dc:` (date created) filters.
//!
//! ## Supported Syntax
//!
//! ### Keywords
//! - `today`, `yesterday`
//! - `thisweek`, `lastweek`, `pastweek`
//! - `thismonth`, `lastmonth`, `pastmonth`
//! - `thisyear`, `lastyear`, `pastyear`
//!
//! ### Absolute Dates
//! - `YYYY-MM-DD`, `YYYY/MM/DD`, `YYYY.MM.DD`
//! - `DD-MM-YYYY`, `DD/MM/YYYY`, `DD.MM.YYYY`
//! - `MM-DD-YYYY`, `MM/DD/YYYY`, `MM.DD.YYYY`
//!
//! ### Comparisons
//! - `<2024-01-01`, `<=2024-01-01`
//! - `>2024-01-01`, `>=2024-01-01`
//! - `=2024-01-01`, `!=2024-01-01`
//!
//! ### Ranges
//! - `2024-01-01..2024-12-31`
//! - `..2024-12-31` (before date)
//! - `2024-01-01..` (after date)

use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};

use crate::error::{FilesystemError, Result};

/// A parsed date predicate for filtering by file modification or creation time.
#[derive(Debug, Clone)]
pub struct DatePredicate {
    kind: DatePredicateKind,
}

#[derive(Debug, Clone)]
pub enum DatePredicateKind {
    /// A range with optional start and end bounds (inclusive).
    /// Values are Unix timestamps in seconds.
    Range {
        start: Option<i64>,
        end: Option<i64>,
    },
    /// Not equal to a specific date range.
    NotEqual { start: i64, end: i64 },
}

/// Intermediate representation of a date value with day bounds.
#[derive(Debug, Clone, Copy)]
struct DateValue {
    /// Start of day (midnight) as Unix timestamp.
    start: i64,
    /// End of day (23:59:59) as Unix timestamp.
    end: i64,
}

/// Context for date parsing, capturing the current date and timezone.
#[derive(Debug, Clone)]
struct DateContext {
    today: NaiveDate,
}

impl DateContext {
    fn capture() -> Self {
        Self {
            today: Local::now().date_naive(),
        }
    }
}

impl DatePredicate {
    /// Parses a date predicate from the raw filter argument.
    pub fn parse(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(FilesystemError::QueryParse(
                "date filter requires a value".to_string(),
            ));
        }

        let context = DateContext::capture();

        // Try comparison operators first
        if let Some((op, value_raw)) = parse_date_comparison(trimmed) {
            let value = parse_date_value(value_raw, &context)?;
            return Ok(match op {
                DateComparisonOp::Lt => Self::range(None, Some(value.start.saturating_sub(1))),
                DateComparisonOp::Lte => Self::range(None, Some(value.end)),
                DateComparisonOp::Gt => Self::range(Some(value.end.saturating_add(1)), None),
                DateComparisonOp::Gte => Self::range(Some(value.start), None),
                DateComparisonOp::Eq => Self::range(Some(value.start), Some(value.end)),
                DateComparisonOp::Ne => Self {
                    kind: DatePredicateKind::NotEqual {
                        start: value.start,
                        end: value.end,
                    },
                },
            });
        }

        // Try range syntax (start..end)
        if let Some((start_raw, end_raw)) = parse_date_range(trimmed) {
            let start = if start_raw.is_empty() {
                None
            } else {
                Some(parse_date_value(start_raw, &context)?.start)
            };
            let end = if end_raw.is_empty() {
                None
            } else {
                Some(parse_date_value(end_raw, &context)?.end)
            };
            if let (Some(s), Some(e)) = (start, end) {
                if s > e {
                    return Err(FilesystemError::QueryParse(
                        "date range start must be before or equal to end".to_string(),
                    ));
                }
            }
            return Ok(Self::range(start, end));
        }

        // Try keyword or absolute date (exact match)
        let value = parse_date_value(trimmed, &context)?;
        Ok(Self::range(Some(value.start), Some(value.end)))
    }

    /// Creates a range predicate.
    fn range(start: Option<i64>, end: Option<i64>) -> Self {
        Self {
            kind: DatePredicateKind::Range { start, end },
        }
    }

    /// Checks if a timestamp (in seconds) matches this predicate.
    pub fn matches(&self, timestamp: i64) -> bool {
        match &self.kind {
            DatePredicateKind::Range { start, end } => {
                if let Some(bound) = start {
                    if timestamp < *bound {
                        return false;
                    }
                }
                if let Some(bound) = end {
                    if timestamp > *bound {
                        return false;
                    }
                }
                true
            }
            DatePredicateKind::NotEqual { start, end } => timestamp < *start || timestamp > *end,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DateComparisonOp {
    Lt,
    Lte,
    Gt,
    Gte,
    Eq,
    Ne,
}

fn parse_date_comparison(raw: &str) -> Option<(DateComparisonOp, &str)> {
    for (operator, kind) in [
        ("<=", DateComparisonOp::Lte),
        (">=", DateComparisonOp::Gte),
        ("!=", DateComparisonOp::Ne),
        ("<", DateComparisonOp::Lt),
        (">", DateComparisonOp::Gt),
        ("=", DateComparisonOp::Eq),
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

fn parse_date_range(raw: &str) -> Option<(&str, &str)> {
    let split = raw.find("..")?;
    let start = raw[..split].trim();
    let end = raw[split + 2..].trim();
    if start.is_empty() && end.is_empty() {
        return None;
    }
    Some((start, end))
}

/// Parses a date value (keyword or absolute date) into day bounds.
fn parse_date_value(raw: &str, context: &DateContext) -> Result<DateValue> {
    let trimmed = raw.trim();

    // Try keyword first
    if let Some(value) = keyword_range(trimmed, context) {
        return Ok(value);
    }

    // Try absolute date
    if let Some(date) = parse_absolute_date(trimmed) {
        return Ok(day_bounds(date));
    }

    Err(FilesystemError::QueryParse(format!(
        "unrecognized date value: {raw:?}"
    )))
}

/// Converts a date keyword to a date value range.
fn keyword_range(keyword: &str, context: &DateContext) -> Option<DateValue> {
    let lower = keyword.to_ascii_lowercase();
    let today = context.today;
    let year = today.year();
    let month = today.month();

    match lower.as_str() {
        "today" => Some(day_bounds(today)),
        "yesterday" => {
            let date = today.checked_sub_signed(Duration::days(1))?;
            Some(day_bounds(date))
        }
        "thisweek" => {
            // Monday-based week
            let weekday_offset = today.weekday().num_days_from_monday() as i64;
            let start = today.checked_sub_signed(Duration::days(weekday_offset))?;
            let end = start.checked_add_signed(Duration::days(6))?;
            Some(range_from_dates(start, end))
        }
        "lastweek" => {
            let weekday_offset = today.weekday().num_days_from_monday() as i64 + 7;
            let start = today.checked_sub_signed(Duration::days(weekday_offset))?;
            let end = start.checked_add_signed(Duration::days(6))?;
            Some(range_from_dates(start, end))
        }
        "thismonth" => month_range(year, month),
        "lastmonth" => {
            let (year, month) = if month == 1 {
                (year.checked_sub(1)?, 12)
            } else {
                (year, month - 1)
            };
            month_range(year, month)
        }
        "thisyear" => year_range(year),
        "lastyear" => year_range(year.checked_sub(1)?),
        // Trailing/rolling ranges
        "pastweek" => trailing_range(context, 7),
        "pastmonth" => trailing_range(context, 30),
        "pastyear" => trailing_range(context, 365),
        _ => None,
    }
}

/// Returns the Unix timestamp bounds for a single day.
fn day_bounds(date: NaiveDate) -> DateValue {
    let start_dt = date.and_hms_opt(0, 0, 0).expect("valid time");
    let end_dt = date.and_hms_opt(23, 59, 59).expect("valid time");

    let start = Local.from_local_datetime(&start_dt).single();
    let end = Local.from_local_datetime(&end_dt).single();

    DateValue {
        start: start.map(|dt| dt.timestamp()).unwrap_or(0),
        end: end.map(|dt| dt.timestamp()).unwrap_or(i64::MAX),
    }
}

/// Returns the Unix timestamp bounds for a date range.
fn range_from_dates(start: NaiveDate, end: NaiveDate) -> DateValue {
    let start_bounds = day_bounds(start);
    let end_bounds = day_bounds(end);
    DateValue {
        start: start_bounds.start,
        end: end_bounds.end,
    }
}

/// Returns the date range for a calendar month.
fn month_range(year: i32, month: u32) -> Option<DateValue> {
    let start = NaiveDate::from_ymd_opt(year, month, 1)?;
    let last_day = last_day_of_month(year, month)?;
    let end = NaiveDate::from_ymd_opt(year, month, last_day)?;
    Some(range_from_dates(start, end))
}

/// Returns the date range for a calendar year.
fn year_range(year: i32) -> Option<DateValue> {
    let start = NaiveDate::from_ymd_opt(year, 1, 1)?;
    let end = NaiveDate::from_ymd_opt(year, 12, 31)?;
    Some(range_from_dates(start, end))
}

/// Returns a trailing date range (last N days from today).
fn trailing_range(context: &DateContext, days: i64) -> Option<DateValue> {
    let end = context.today;
    let start = end.checked_sub_signed(Duration::days(days - 1))?;
    Some(range_from_dates(start, end))
}

/// Returns the last day of a month.
fn last_day_of_month(year: i32, month: u32) -> Option<u32> {
    // Get the first day of the next month and subtract one day
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let first_of_next = NaiveDate::from_ymd_opt(next_year, next_month, 1)?;
    let last = first_of_next.checked_sub_signed(Duration::days(1))?;
    Some(last.day())
}

/// Parses an absolute date from various formats.
fn parse_absolute_date(raw: &str) -> Option<NaiveDate> {
    let trimmed = raw.trim();
    let sep = trimmed.chars().find(|ch| matches!(ch, '-' | '/' | '.'))?;

    let formats = match sep {
        '-' => {
            if trimmed.len() >= 4 && trimmed[..4].chars().all(|c| c.is_ascii_digit()) {
                // Year-first
                vec!["%Y-%m-%d"]
            } else {
                vec!["%d-%m-%Y", "%m-%d-%Y", "%Y-%m-%d"]
            }
        }
        '/' => {
            if trimmed.len() >= 4 && trimmed[..4].chars().all(|c| c.is_ascii_digit()) {
                vec!["%Y/%m/%d"]
            } else {
                vec!["%m/%d/%Y", "%d/%m/%Y", "%Y/%m/%d"]
            }
        }
        '.' => {
            if trimmed.len() >= 4 && trimmed[..4].chars().all(|c| c.is_ascii_digit()) {
                vec!["%Y.%m.%d"]
            } else {
                vec!["%d.%m.%Y", "%m.%d.%Y", "%Y.%m.%d"]
            }
        }
        _ => vec![],
    };

    for format in formats {
        if let Ok(date) = NaiveDate::parse_from_str(trimmed, format) {
            return Some(date);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_today_keyword() {
        let predicate = DatePredicate::parse("today").expect("parse");
        let today = Local::now();
        let today_ts = today.timestamp();
        assert!(
            predicate.matches(today_ts),
            "should match today's timestamp"
        );
    }

    #[test]
    fn parse_yesterday_keyword() {
        let predicate = DatePredicate::parse("yesterday").expect("parse");
        let yesterday = Local::now() - Duration::days(1);
        let yesterday_ts = yesterday.timestamp();
        assert!(
            predicate.matches(yesterday_ts),
            "should match yesterday's timestamp"
        );
    }

    #[test]
    fn parse_absolute_date_ymd() {
        let predicate = DatePredicate::parse("2024-06-15").expect("parse");
        // Create a timestamp for 2024-06-15 12:00:00
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let dt = date.and_hms_opt(12, 0, 0).unwrap();
        let ts = Local.from_local_datetime(&dt).single().unwrap().timestamp();
        assert!(predicate.matches(ts), "should match 2024-06-15 noon");
    }

    #[test]
    fn parse_absolute_date_slash_format() {
        let predicate = DatePredicate::parse("2024/06/15").expect("parse");
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let dt = date.and_hms_opt(12, 0, 0).unwrap();
        let ts = Local.from_local_datetime(&dt).single().unwrap().timestamp();
        assert!(predicate.matches(ts), "should match 2024/06/15");
    }

    #[test]
    fn parse_comparison_greater_than() {
        let predicate = DatePredicate::parse(">2024-01-01").expect("parse");
        // 2024-06-15 should be after 2024-01-01
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let dt = date.and_hms_opt(12, 0, 0).unwrap();
        let ts = Local.from_local_datetime(&dt).single().unwrap().timestamp();
        assert!(
            predicate.matches(ts),
            "2024-06-15 should be after 2024-01-01"
        );

        // 2023-12-31 should NOT match
        let date_before = NaiveDate::from_ymd_opt(2023, 12, 31).unwrap();
        let dt_before = date_before.and_hms_opt(12, 0, 0).unwrap();
        let ts_before = Local
            .from_local_datetime(&dt_before)
            .single()
            .unwrap()
            .timestamp();
        assert!(
            !predicate.matches(ts_before),
            "2023-12-31 should not match >2024-01-01"
        );
    }

    #[test]
    fn parse_comparison_less_than() {
        let predicate = DatePredicate::parse("<2024-01-01").expect("parse");
        // 2023-12-15 should be before 2024-01-01
        let date = NaiveDate::from_ymd_opt(2023, 12, 15).unwrap();
        let dt = date.and_hms_opt(12, 0, 0).unwrap();
        let ts = Local.from_local_datetime(&dt).single().unwrap().timestamp();
        assert!(
            predicate.matches(ts),
            "2023-12-15 should be before 2024-01-01"
        );
    }

    #[test]
    fn parse_range() {
        let predicate = DatePredicate::parse("2024-01-01..2024-12-31").expect("parse");
        // 2024-06-15 should be in range
        let date = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let dt = date.and_hms_opt(12, 0, 0).unwrap();
        let ts = Local.from_local_datetime(&dt).single().unwrap().timestamp();
        assert!(predicate.matches(ts), "2024-06-15 should be in range");

        // 2023-06-15 should NOT be in range
        let date_before = NaiveDate::from_ymd_opt(2023, 6, 15).unwrap();
        let dt_before = date_before.and_hms_opt(12, 0, 0).unwrap();
        let ts_before = Local
            .from_local_datetime(&dt_before)
            .single()
            .unwrap()
            .timestamp();
        assert!(
            !predicate.matches(ts_before),
            "2023-06-15 should not be in range"
        );
    }

    #[test]
    fn parse_open_ended_range_before() {
        let predicate = DatePredicate::parse("..2024-06-30").expect("parse");
        // 2024-01-01 should match (before end)
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let dt = date.and_hms_opt(12, 0, 0).unwrap();
        let ts = Local.from_local_datetime(&dt).single().unwrap().timestamp();
        assert!(
            predicate.matches(ts),
            "2024-01-01 should be before 2024-06-30"
        );
    }

    #[test]
    fn parse_open_ended_range_after() {
        let predicate = DatePredicate::parse("2024-06-01..").expect("parse");
        // 2024-12-31 should match (after start)
        let date = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let dt = date.and_hms_opt(12, 0, 0).unwrap();
        let ts = Local.from_local_datetime(&dt).single().unwrap().timestamp();
        assert!(
            predicate.matches(ts),
            "2024-12-31 should be after 2024-06-01"
        );
    }

    #[test]
    fn parse_thisweek_keyword() {
        let predicate = DatePredicate::parse("thisweek").expect("parse");
        let today_ts = Local::now().timestamp();
        assert!(predicate.matches(today_ts), "today should be in thisweek");
    }

    #[test]
    fn parse_thismonth_keyword() {
        let predicate = DatePredicate::parse("thismonth").expect("parse");
        let today_ts = Local::now().timestamp();
        assert!(predicate.matches(today_ts), "today should be in thismonth");
    }

    #[test]
    fn parse_thisyear_keyword() {
        let predicate = DatePredicate::parse("thisyear").expect("parse");
        let today_ts = Local::now().timestamp();
        assert!(predicate.matches(today_ts), "today should be in thisyear");
    }

    #[test]
    fn parse_not_equal() {
        let predicate = DatePredicate::parse("!=2024-06-15").expect("parse");
        // 2024-06-14 should match (not 2024-06-15)
        let date = NaiveDate::from_ymd_opt(2024, 6, 14).unwrap();
        let dt = date.and_hms_opt(12, 0, 0).unwrap();
        let ts = Local.from_local_datetime(&dt).single().unwrap().timestamp();
        assert!(
            predicate.matches(ts),
            "2024-06-14 should not equal 2024-06-15"
        );

        // 2024-06-15 should NOT match
        let date_eq = NaiveDate::from_ymd_opt(2024, 6, 15).unwrap();
        let dt_eq = date_eq.and_hms_opt(12, 0, 0).unwrap();
        let ts_eq = Local
            .from_local_datetime(&dt_eq)
            .single()
            .unwrap()
            .timestamp();
        assert!(
            !predicate.matches(ts_eq),
            "2024-06-15 should equal 2024-06-15"
        );
    }

    #[test]
    fn empty_value_returns_error() {
        let result = DatePredicate::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_date_returns_error() {
        let result = DatePredicate::parse("notadate");
        assert!(result.is_err());
    }

    #[test]
    fn invalid_range_order_returns_error() {
        let result = DatePredicate::parse("2024-12-31..2024-01-01");
        assert!(result.is_err());
    }

    #[test]
    fn pastweek_includes_recent_days() {
        let predicate = DatePredicate::parse("pastweek").expect("parse");
        // Yesterday should be in pastweek
        let yesterday = Local::now() - Duration::days(1);
        assert!(
            predicate.matches(yesterday.timestamp()),
            "yesterday should be in pastweek"
        );
    }

    #[test]
    fn lastday_of_month_calculation() {
        assert_eq!(last_day_of_month(2024, 2), Some(29)); // Leap year
        assert_eq!(last_day_of_month(2023, 2), Some(28)); // Non-leap year
        assert_eq!(last_day_of_month(2024, 12), Some(31));
        assert_eq!(last_day_of_month(2024, 4), Some(30));
    }
}
