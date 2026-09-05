use crate::{ProviderError, ReleaseFact};
use reqwest::Client;
use serde::Deserialize;
#[derive(Debug, Deserialize)]
struct Response {
    tag_name: String,
}

pub async fn resolve(
    client: &Client,
    repository: &str,
    api_url: &str,
    token: Option<&str>,
) -> Result<ReleaseFact, ProviderError> {
    let url = format!(
        "{}/repos/{repository}/releases/latest",
        api_url.trim_end_matches('/')
    );
    let mut request = client.get(url).header("user-agent", "todo-or-die");
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let response = request.send().await?;
    if !response.status().is_success() {
        return Err(ProviderError::Status(response.status()));
    }
    let version = response.json::<Response>().await?.tag_name;
    let parsed = semver::Version::parse(version.trim_start_matches('v'))
        .map_err(|_| ProviderError::InvalidVersion(version.clone()))?;
    Ok(ReleaseFact {
        provider: todo_or_die_core::IssueProvider::Github,
        repository: repository.into(),
        version,
        major: parsed.major,
        minor: parsed.minor,
        patch: parsed.patch,
    })
}
