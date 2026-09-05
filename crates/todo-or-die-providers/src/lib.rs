use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;
use todo_or_die_core::{IssueProvider, IssueState, PackageEcosystem};

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("provider request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("provider returned HTTP status {0}")]
    Status(reqwest::StatusCode),
    #[error("provider returned invalid semantic version: {0}")]
    InvalidVersion(String),
}

#[derive(Debug, Clone)]
pub struct IssueFact {
    pub provider: IssueProvider,
    pub repository: String,
    pub number: u64,
    pub state: IssueState,
}

#[derive(Debug, Deserialize)]
struct IssueResponse {
    state: String,
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
#[derive(Debug, Deserialize)]
struct ReleaseResponse {
    tag_name: String,
}
#[derive(Debug, Deserialize)]
struct NpmResponse {
    #[serde(rename = "dist-tags")]
    dist_tags: std::collections::HashMap<String, String>,
}
#[derive(Debug, Deserialize)]
struct CrateResponse {
    #[serde(rename = "crate")]
    crate_data: CrateData,
}
#[derive(Debug, Deserialize)]
struct CrateData {
    max_version: String,
}

pub async fn resolve_package(
    client: &Client,
    ecosystem: &PackageEcosystem,
    package: &str,
) -> Result<PackageFact, ProviderError> {
    let url = match ecosystem {
        PackageEcosystem::Npm => format!("https://registry.npmjs.org/{package}"),
        PackageEcosystem::Crates => format!("https://crates.io/api/v1/crates/{package}"),
    };
    let response = client
        .get(url)
        .header("user-agent", "todo-or-die")
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(ProviderError::Status(response.status()));
    }
    let version = match ecosystem {
        PackageEcosystem::Npm => response
            .json::<NpmResponse>()
            .await?
            .dist_tags
            .get("latest")
            .cloned()
            .ok_or_else(|| ProviderError::InvalidVersion("missing npm latest".into()))?,
        PackageEcosystem::Crates => {
            response
                .json::<CrateResponse>()
                .await?
                .crate_data
                .max_version
        }
    };
    Ok(PackageFact {
        ecosystem: ecosystem.clone(),
        package: package.into(),
        version,
    })
}

pub async fn resolve_issue(
    client: &Client,
    provider: &IssueProvider,
    repository: &str,
    number: u64,
    host: Option<&str>,
    token: Option<&str>,
) -> Result<IssueFact, ProviderError> {
    let base = host
        .unwrap_or(match provider {
            IssueProvider::Github => "https://api.github.com",
            IssueProvider::Gitlab => "https://gitlab.com/api/v4",
        })
        .trim_end_matches('/');
    let url = match provider {
        IssueProvider::Github => format!("{base}/repos/{repository}/issues/{number}"),
        IssueProvider::Gitlab => format!(
            "{base}/projects/{}/issues/{number}",
            urlencoding::encode(repository)
        ),
    };
    let mut request = client.get(url).header("user-agent", "todo-or-die");
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let response = request.send().await?;
    if !response.status().is_success() {
        return Err(ProviderError::Status(response.status()));
    }
    let state = match response.json::<IssueResponse>().await?.state.as_str() {
        "closed" => IssueState::Closed,
        _ => IssueState::Open,
    };
    Ok(IssueFact {
        provider: provider.clone(),
        repository: repository.into(),
        number,
        state,
    })
}

pub async fn resolve_latest_release(
    client: &Client,
    provider: &IssueProvider,
    repository: &str,
    host: Option<&str>,
    token: Option<&str>,
) -> Result<ReleaseFact, ProviderError> {
    let base = host
        .unwrap_or(match provider {
            IssueProvider::Github => "https://api.github.com",
            IssueProvider::Gitlab => "https://gitlab.com/api/v4",
        })
        .trim_end_matches('/');
    let url = match provider {
        IssueProvider::Github => format!("{base}/repos/{repository}/releases/latest"),
        IssueProvider::Gitlab => format!(
            "{base}/projects/{}/releases/permalink/latest",
            urlencoding::encode(repository)
        ),
    };
    let mut request = client.get(url).header("user-agent", "todo-or-die");
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let response = request.send().await?;
    if !response.status().is_success() {
        return Err(ProviderError::Status(response.status()));
    }
    let version = response.json::<ReleaseResponse>().await?.tag_name;
    let parsed = semver::Version::parse(version.trim_start_matches('v'))
        .map_err(|_| ProviderError::InvalidVersion(version.clone()))?;
    Ok(ReleaseFact {
        provider: provider.clone(),
        repository: repository.into(),
        version,
        major: parsed.major,
        minor: parsed.minor,
        patch: parsed.patch,
    })
}

pub fn cel_issue_references(source: &str) -> Vec<(IssueProvider, String, u64)> {
    let mut out = Vec::new();
    for provider in [IssueProvider::Github, IssueProvider::Gitlab] {
        let prefix = match provider {
            IssueProvider::Github => "github.issues[\"",
            IssueProvider::Gitlab => "gitlab.issues[\"",
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

pub fn facts_json(issues: &[IssueFact], releases: &[ReleaseFact]) -> serde_json::Value {
    let mut root = serde_json::Map::new();
    for fact in issues {
        let provider = match fact.provider {
            IssueProvider::Github => "github",
            IssueProvider::Gitlab => "gitlab",
        };
        let entry = root
            .entry(provider)
            .or_insert_with(|| serde_json::json!({"issues": {}}));
        entry["issues"][format!("{}#{}", fact.repository, fact.number)] = serde_json::json!({"closed": matches!(fact.state, IssueState::Closed), "open": matches!(fact.state, IssueState::Open)});
    }
    for fact in releases {
        let provider = match fact.provider {
            IssueProvider::Github => "github",
            IssueProvider::Gitlab => "gitlab",
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
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    fn mock(body: &'static str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 1024];
            let _ = stream.read(&mut request);
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        });
        address
    }

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

    #[tokio::test]
    async fn resolves_issue_and_release_from_http_responses() {
        let client = Client::new();
        let issue_host = mock(r#"{"state":"closed"}"#);
        let issue = resolve_issue(
            &client,
            &IssueProvider::Github,
            "org/repo",
            1,
            Some(&issue_host),
            None,
        )
        .await
        .unwrap();
        assert!(matches!(issue.state, IssueState::Closed));
        let release_host = mock(r#"{"tag_name":"v2.3.4"}"#);
        let release = resolve_latest_release(
            &client,
            &IssueProvider::Github,
            "org/repo",
            Some(&release_host),
            None,
        )
        .await
        .unwrap();
        assert_eq!((release.major, release.minor, release.patch), (2, 3, 4));
    }
}
