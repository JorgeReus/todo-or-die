use std::{future::Future, pin::Pin};
use thiserror::Error;
use todo_or_die_core::{IssueProvider, IssueState, PackageEcosystem};

pub mod issues;
pub mod packages;
pub mod releases;

pub fn install_tls_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

pub type ProviderFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ProviderError>> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RepositoryId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IssueId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageId(pub String);

/// Capability interfaces. Implementations can be registered independently per provider.
pub trait IssueProviderBackend: Send + Sync {
    fn issue<'a>(&'a self, id: &'a IssueId) -> ProviderFuture<'a, IssueFact>;
}

pub trait ReleaseProviderBackend: Send + Sync {
    fn latest_release<'a>(&'a self, repo: &'a RepositoryId) -> ProviderFuture<'a, ReleaseFact>;
}

pub trait PackageProviderBackend: Send + Sync {
    fn latest_version<'a>(&'a self, package: &'a PackageId) -> ProviderFuture<'a, PackageFact>;
}

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("provider returned HTTP status {0}")]
    Status(reqwest::StatusCode),
    #[error("provider returned invalid semantic version: {0}")]
    InvalidVersion(String),
    #[error("provider API URL is not configured")]
    MissingApiUrl,
    #[error("provider configuration error: {0}")]
    Configuration(String),
}

#[derive(Debug, Clone)]
pub struct IssueFact {
    pub provider: IssueProvider,
    pub repository: String,
    pub number: u64,
    pub key: Option<String>,
    pub state: IssueState,
}

#[derive(Debug, Clone)]
pub struct ReleaseFact {
    pub provider: IssueProvider,
    pub repository: String,
    pub version: String,
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

#[derive(Debug, Clone)]
pub struct PackageFact {
    pub ecosystem: PackageEcosystem,
    pub package: String,
    pub version: String,
}

pub fn cel_issue_references(source: &str) -> Vec<(IssueProvider, String, u64)> {
    let mut out = Vec::new();
    for provider in [IssueProvider::Github, IssueProvider::Gitlab] {
        let prefix = match provider {
            IssueProvider::Github => "github.issues[\"",
            IssueProvider::Gitlab => "gitlab.issues[\"",
            IssueProvider::Jira => "jira.issues[\"",
        };
        let mut rest = source;
        while let Some(start) = rest.find(prefix) {
            let value = &rest[start + prefix.len()..];
            let Some(end) = value.find("\"]") else { break };
            let Some((repo, number)) = value[..end].rsplit_once('#') else {
                rest = &value[end..];
                continue;
            };
            if let Ok(number) = number.parse() {
                let reference = (provider.clone(), repo.to_owned(), number);
                if !out.contains(&reference) {
                    out.push(reference);
                }
            }
            rest = &value[end..];
        }
    }
    out
}

pub fn cel_release_references(source: &str) -> Vec<(IssueProvider, String)> {
    let mut out = vec![];
    for provider in [IssueProvider::Github, IssueProvider::Gitlab] {
        let prefix = match provider {
            IssueProvider::Github => "github.releases[\"",
            IssueProvider::Gitlab => "gitlab.releases[\"",
            IssueProvider::Jira => "jira.releases[\"",
        };
        let mut rest = source;
        while let Some(start) = rest.find(prefix) {
            let value = &rest[start + prefix.len()..];
            let Some(end) = value.find("\"]") else { break };
            let reference = (provider.clone(), value[..end].to_owned());
            if !out.contains(&reference) {
                out.push(reference);
            }
            rest = &value[end..];
        }
    }
    out
}

pub fn cel_jira_references(source: &str) -> Vec<String> {
    let prefix = "jira.issues[\"";
    let mut out = vec![];
    let mut rest = source;
    while let Some(start) = rest.find(prefix) {
        let value = &rest[start + prefix.len()..];
        let Some(end) = value.find("\"]") else { break };
        if !out.contains(&value[..end].to_owned()) {
            out.push(value[..end].to_owned());
        }
        rest = &value[end..];
    }
    out
}

pub fn facts_json(issues: &[IssueFact], releases: &[ReleaseFact]) -> serde_json::Value {
    let mut root = serde_json::Map::new();
    for fact in issues {
        let provider = match fact.provider {
            IssueProvider::Github => "github",
            IssueProvider::Gitlab => "gitlab",
            IssueProvider::Jira => "jira",
        };
        let entry = root
            .entry(provider)
            .or_insert_with(|| serde_json::json!({"issues": {}}));
        let key = fact
            .key
            .clone()
            .unwrap_or_else(|| format!("{}#{}", fact.repository, fact.number));
        entry["issues"][key] = serde_json::json!({"closed": matches!(fact.state, IssueState::Closed), "open": matches!(fact.state, IssueState::Open)});
    }
    for fact in releases {
        let provider = match fact.provider {
            IssueProvider::Github => "github",
            IssueProvider::Gitlab => "gitlab",
            IssueProvider::Jira => "jira",
        };
        let entry = root
            .entry(provider)
            .or_insert_with(|| serde_json::json!({}));
        entry["releases"][&fact.repository] = serde_json::json!({"latest": fact.version, "major": fact.major, "minor": fact.minor, "patch": fact.patch});
    }
    serde_json::Value::Object(root)
}

pub fn issue_facts_json(facts: &[IssueFact]) -> serde_json::Value {
    facts_json(facts, &[])
}

#[cfg(test)]
mod tests {
    #[test]
    fn deduplicates_repeated_cel_references() {
        let issues = super::cel_issue_references(
            r#"github.issues["org/repo#1"].closed || github.issues["org/repo#1"].open"#,
        );
        assert_eq!(issues.len(), 1);
        let releases = super::cel_release_references(
            r#"github.releases["org/repo"].latest == github.releases["org/repo"].latest"#,
        );
        assert_eq!(releases.len(), 1);
    }
}
