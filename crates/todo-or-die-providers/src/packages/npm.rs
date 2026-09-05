use crate::{PackageFact, ProviderError};
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
#[derive(Debug, Deserialize)]
struct Response {
    #[serde(rename = "dist-tags")]
    dist_tags: HashMap<String, String>,
}

pub async fn resolve(client: &Client, package: &str) -> Result<PackageFact, ProviderError> {
    let response = client
        .get(format!("https://registry.npmjs.org/{package}"))
        .header("user-agent", "todo-or-die")
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(ProviderError::Status(response.status()));
    }
    let version = response
        .json::<Response>()
        .await?
        .dist_tags
        .get("latest")
        .cloned()
        .ok_or_else(|| ProviderError::InvalidVersion("missing npm latest".into()))?;
    Ok(PackageFact {
        ecosystem: todo_or_die_core::PackageEcosystem::Npm,
        package: package.into(),
        version,
    })
}
