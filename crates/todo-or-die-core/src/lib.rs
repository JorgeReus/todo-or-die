use chrono::{NaiveDate, Utc};
use serde::Serialize;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Condition {
    After {
        date: NaiveDate,
    },
    Cel {
        source: String,
    },
    Issue {
        provider: IssueProvider,
        repository: String,
        number: u64,
        state: IssueState,
    },
    Package {
        ecosystem: PackageEcosystem,
        package: String,
        requirement: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueProvider {
    Github,
    Gitlab,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    Open,
    Closed,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageEcosystem {
    Npm,
    Crates,
}

#[derive(Debug, Clone)]
pub struct Todo {
    pub file: PathBuf,
    pub span: SourceSpan,
    pub source: String,
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
        let (date, message) = value.split_once(char::is_whitespace).unwrap_or((value, ""));
        (
            Condition::After {
                date: NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d")?,
            },
            (!message.trim().is_empty())
                .then(|| message.trim().trim_start_matches('*').trim().to_owned()),
        )
    } else if let Some(value) = rest.strip_prefix("github ") {
        issue(value, IssueProvider::Github)?
    } else if let Some(value) = rest.strip_prefix("gitlab ") {
        issue(value, IssueProvider::Gitlab)?
    } else if let Some(value) = rest.strip_prefix("package ") {
        package(value)?
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

fn package(value: &str) -> Result<(Condition, Option<String>), DirectiveError> {
    let (target, requirement) = value
        .rsplit_once(' ')
        .ok_or_else(|| DirectiveError::Invalid(value.into()))?;
    let (ecosystem, package) = target
        .split_once('/')
        .ok_or_else(|| DirectiveError::Invalid(value.into()))?;
    let ecosystem = match ecosystem {
        "npm" => PackageEcosystem::Npm,
        "crates" => PackageEcosystem::Crates,
        _ => return Err(DirectiveError::Invalid(value.into())),
    };
    Ok((
        Condition::Package {
            ecosystem,
            package: package.into(),
            requirement: requirement.into(),
        },
        None,
    ))
}

fn issue(
    value: &str,
    provider: IssueProvider,
) -> Result<(Condition, Option<String>), DirectiveError> {
    let (target, expected) = value
        .rsplit_once(' ')
        .ok_or_else(|| DirectiveError::Invalid(value.into()))?;
    let (repository, number) = target
        .rsplit_once('#')
        .ok_or_else(|| DirectiveError::Invalid(value.into()))?;
    let number = number
        .parse()
        .map_err(|_| DirectiveError::Invalid(value.into()))?;
    let state = match expected {
        "open" => IssueState::Open,
        "closed" => IssueState::Closed,
        _ => return Err(DirectiveError::Invalid(value.into())),
    };
    Ok((
        Condition::Issue {
            provider,
            repository: repository.into(),
            number,
            state,
        },
        None,
    ))
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

    #[test]
    fn extracts_inline_message() {
        let comment = SourceComment {
            raw_text: String::new(),
            content: "TODO-OR-DIE: after 2020-01-01 remove this".into(),
            span: SourceSpan {
                start_byte: 0,
                end_byte: 0,
                start_line: 0,
                start_column: 0,
                end_line: 0,
                end_column: 0,
            },
        };
        let (_, message) = parse_directive(&comment).unwrap().unwrap();
        assert_eq!(message.as_deref(), Some("remove this"));
    }

    #[test]
    fn parses_package_condition() {
        let comment = SourceComment {
            raw_text: String::new(),
            content: "TODO-OR-DIE: package npm/react >= 20".into(),
            span: SourceSpan {
                start_byte: 0,
                end_byte: 0,
                start_line: 0,
                start_column: 0,
                end_line: 0,
                end_column: 0,
            },
        };
        assert!(matches!(
            parse_directive(&comment).unwrap().unwrap().0,
            Condition::Package {
                ecosystem: PackageEcosystem::Npm,
                ..
            }
        ));
    }
}
