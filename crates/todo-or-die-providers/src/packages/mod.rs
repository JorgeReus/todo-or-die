pub mod crates_io;
pub mod npm;

pub use crates_io::resolve as resolve_crates_io;
pub use npm::resolve as resolve_npm;

pub async fn resolve(
    client: &reqwest::Client,
    ecosystem: &todo_or_die_core::PackageEcosystem,
    package: &str,
) -> Result<crate::PackageFact, crate::ProviderError> {
    match ecosystem {
        todo_or_die_core::PackageEcosystem::Npm => npm::resolve(client, package).await,
        todo_or_die_core::PackageEcosystem::Crates => crates_io::resolve(client, package).await,
    }
}
