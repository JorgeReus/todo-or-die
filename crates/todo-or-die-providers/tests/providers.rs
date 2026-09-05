use reqwest::Client;
use todo_or_die_core::IssueState;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn resolves_github_issue_and_release_from_mock_server() {
    todo_or_die_providers::install_tls_provider();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/org/repo/issues/7"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"state": "closed"})),
        )
        .mount(&server)
        .await;
    let issue = todo_or_die_providers::issues::github::resolve(
        &Client::new(),
        "org/repo",
        7,
        &server.uri(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(issue.state, IssueState::Closed);
    Mock::given(method("GET"))
        .and(path("/repos/org/repo/releases/latest"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"tag_name": "v2.4.1"})),
        )
        .mount(&server)
        .await;
    let release = todo_or_die_providers::releases::github::resolve(
        &Client::new(),
        "org/repo",
        &server.uri(),
        None,
    )
    .await
    .unwrap();
    assert_eq!((release.major, release.minor, release.patch), (2, 4, 1));
}

#[tokio::test]
async fn resolves_jira_issue_from_mock_server() {
    todo_or_die_providers::install_tls_provider();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/rest/api/3/issue/PROJ-123"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({"fields": {"status": {"name": "Done"}}})),
        )
        .mount(&server)
        .await;
    let issue = todo_or_die_providers::issues::jira::resolve(
        &Client::new(),
        "PROJ-123",
        &server.uri(),
        None,
    )
    .await
    .unwrap();
    assert_eq!(issue.key.as_deref(), Some("PROJ-123"));
    assert_eq!(issue.state, IssueState::Closed);
}
