use crate::{PackageFact, ProviderError};
use reqwest::Client;
use serde::Deserialize;
#[derive(Debug, Deserialize)]
struct Response {
    #[serde(rename = "crate")]
    crate_data: CrateData,
}
#[derive(Debug, Deserialize)]
struct CrateData {
    max_version: String,
}

pub async fn resolve(client: &Client, package: &str) -> Result<PackageFact, ProviderError> {
    crate::install_tls_provider();
    let response = client
        .get(format!("https://crates.io/api/v1/crates/{package}"))
        .header("user-agent", "todo-or-die")
        .send()
        .await?;
    if !response.status().is_success() {
        return Err(ProviderError::Status(response.status()));
    }
    let version = response.json::<Response>().await?.crate_data.max_version;
    Ok(PackageFact {
        ecosystem: todo_or_die_core::PackageEcosystem::Crates,
        package: package.into(),
        version,
    })
}
