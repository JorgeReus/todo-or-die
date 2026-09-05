use crate::{IssueFact, ProviderError};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Response {
    state: String,
}

pub async fn resolve(
    client: &Client,
    project: &str,
    number: u64,
    api_url: &str,
    token: Option<&str>,
) -> Result<IssueFact, ProviderError> {
    crate::install_tls_provider();
    let url = format!(
        "{}/projects/{}/issues/{number}",
        api_url.trim_end_matches('/'),
        urlencoding::encode(project)
    );
    let mut request = client.get(url).header("user-agent", "todo-or-die");
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let response = request.send().await?;
    if !response.status().is_success() {
        return Err(ProviderError::Status(response.status()));
    }
    Ok(IssueFact {
        provider: todo_or_die_core::IssueProvider::Gitlab,
        repository: project.into(),
        number,
        key: None,
        state: if response.json::<Response>().await?.state == "closed" {
            todo_or_die_core::IssueState::Closed
        } else {
            todo_or_die_core::IssueState::Open
        },
    })
}
