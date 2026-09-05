use crate::{IssueFact, ProviderError};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct JiraIssueResponse {
    fields: JiraFields,
}
#[derive(Debug, Deserialize)]
struct JiraFields {
    status: JiraStatus,
}
#[derive(Debug, Deserialize)]
struct JiraStatus {
    name: String,
}

pub async fn resolve(
    client: &Client,
    key: &str,
    api_url: &str,
    token: Option<&str>,
) -> Result<IssueFact, ProviderError> {
    let url = format!(
        "{}/rest/api/3/issue/{}",
        api_url.trim_end_matches('/'),
        urlencoding::encode(key)
    );
    let mut request = client.get(url).header("user-agent", "todo-or-die");
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    let response = request.send().await?;
    if !response.status().is_success() {
        return Err(ProviderError::Status(response.status()));
    }
    let status = response
        .json::<JiraIssueResponse>()
        .await?
        .fields
        .status
        .name;
    Ok(IssueFact {
        provider: todo_or_die_core::IssueProvider::Jira,
        repository: String::new(),
        number: 0,
        key: Some(key.into()),
        state: if matches!(
            status.to_ascii_lowercase().as_str(),
            "done" | "closed" | "resolved"
        ) {
            todo_or_die_core::IssueState::Closed
        } else {
            todo_or_die_core::IssueState::Open
        },
    })
}
