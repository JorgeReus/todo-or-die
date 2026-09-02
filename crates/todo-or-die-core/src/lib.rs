use chrono::{NaiveDate, Utc};
use serde::Serialize;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Serialize)]
pub struct SourceSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone)]
pub struct SourceComment {
    pub raw_text: String,
    pub content: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    After { date: NaiveDate },
    Cel { source: String },
}

#[derive(Debug, Clone)]
pub struct Todo {
    pub file: PathBuf,
    pub span: SourceSpan,
    pub condition: Condition,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoState {
    Active,
    Triggered,
}

#[derive(Debug, Error)]
pub enum DirectiveError {
    #[error("invalid todo-or-die directive: {0}")]
    Invalid(String),
    #[error("invalid date: {0}")]
    Date(#[from] chrono::ParseError),
}

pub fn parse_directive(
    comment: &SourceComment,
) -> Result<Option<(Condition, Option<String>)>, DirectiveError> {
    let Some(rest) = comment.content.strip_prefix("TODO-OR-DIE:") else {
        return Ok(None);
    };
    let rest = rest.trim();
    let (condition, message) = if let Some(value) = rest.strip_prefix("after ") {
        (
            Condition::After {
                date: NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")?,
            },
            None,
        )
    } else if rest.starts_with("cel(") && rest.ends_with(')') {
        (
            Condition::Cel {
                source: rest[4..rest.len() - 1].to_owned(),
            },
            None,
        )
    } else {
        return Err(DirectiveError::Invalid(rest.to_owned()));
    };
    Ok(Some((condition, message)))
}

pub fn state(condition: &Condition, today: NaiveDate) -> TodoState {
    match condition {
        Condition::After { date } if today >= *date => TodoState::Triggered,
        _ => TodoState::Active,
    }
}

pub fn today() -> NaiveDate {
    Utc::now().date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_and_evaluates_after() {
        let c = SourceComment {
            raw_text: "// TODO-OR-DIE: after 2020-01-01".into(),
            content: "TODO-OR-DIE: after 2020-01-01".into(),
            span: SourceSpan {
                start_byte: 0,
                end_byte: 35,
                start_line: 0,
                start_column: 0,
                end_line: 0,
                end_column: 35,
            },
        };
        let (condition, _) = parse_directive(&c).unwrap().unwrap();
        assert_eq!(
            state(&condition, NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
            TodoState::Triggered
        );
    }
}
