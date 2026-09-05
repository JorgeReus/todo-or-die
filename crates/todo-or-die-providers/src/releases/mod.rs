pub mod github;
pub mod gitlab;

pub use github::resolve as resolve_github;
pub use gitlab::resolve as resolve_gitlab;

pub async fn resolve(
    client: &reqwest::Client,
    provider: &todo_or_die_core::IssueProvider,
    repository: &str,
    api_url: Option<&str>,
    token: Option<&str>,
) -> Result<crate::ReleaseFact, crate::ProviderError> {
    match provider {
        todo_or_die_core::IssueProvider::Github => {
            github::resolve(
                client,
                repository,
                api_url.ok_or(crate::ProviderError::MissingApiUrl)?,
                token,
            )
            .await
        }
        todo_or_die_core::IssueProvider::Gitlab => {
            gitlab::resolve(
                client,
                repository,
                api_url.ok_or(crate::ProviderError::MissingApiUrl)?,
                token,
            )
            .await
        }
        todo_or_die_core::IssueProvider::Jira => Err(crate::ProviderError::InvalidVersion(
            "Jira has no releases".into(),
        )),
    }
}
