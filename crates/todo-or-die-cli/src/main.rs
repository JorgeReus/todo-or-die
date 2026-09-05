use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};
use todo_or_die_core::*;

fn return_state(fact: &todo_or_die_providers::IssueFact, expected: &IssueState) -> bool {
    std::mem::discriminant(&fact.state) == std::mem::discriminant(expected)
}
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[arg(long, global = true, default_value = "human")]
    format: String,
}
#[derive(Subcommand)]
enum Command {
    Check { paths: Vec<PathBuf> },
    List { paths: Vec<PathBuf> },
    Explain { location: String },
}
fn files(ps: &[PathBuf]) -> Vec<PathBuf> {
    let mut o = vec![];
    for p in ps {
        if p.is_file() {
            o.push(p.clone())
        } else {
            for e in ignore::WalkBuilder::new(p).hidden(false).build().flatten() {
                if e.file_type().is_some_and(|t| t.is_file()) {
                    o.push(e.into_path())
                }
            }
        }
    }
    o
}
#[derive(Deserialize, Default)]
struct Settings {
    network: Network,
    github: Host,
    gitlab: Host,
    jira: Host,
}
#[derive(Deserialize)]
struct Network {
    timeout_seconds: u64,
}
#[derive(Deserialize, Default)]
struct Host {
    api_url: Option<String>,
}
impl Default for Network {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
        }
    }
}
fn settings(paths: &[PathBuf]) -> Result<Settings, String> {
    let mut path = std::env::current_dir().map_err(|e| e.to_string())?;
    if !path.join(".todo-or-die.toml").exists() {
        if let Some(first) = paths.first() {
            path = if first.is_dir() {
                first.clone()
            } else {
                first.parent().unwrap_or(first).to_path_buf()
            };
        }
    }
    let path = path.join(".todo-or-die.toml");
    if !path.exists() {
        return Ok(Settings::default());
    }
    toml::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
#[tokio::main]
async fn main() -> ExitCode {
    let c = Cli::parse();
    let ps = match &c.command {
        Command::Check { paths: p } | Command::List { paths: p } if !p.is_empty() => p,
        Command::Explain { location } => {
            let file = location
                .rsplit_once(':')
                .map(|(file, _)| file)
                .unwrap_or(location);
            &vec![PathBuf::from(file)]
        }
        _ => &vec![PathBuf::from(".")],
    };
    let mut ts = vec![];
    for f in files(ps) {
        let Ok(s) = fs::read_to_string(&f) else {
            continue;
        };
        let comments = match todo_or_die_tree_sitter::extract_comments(&f, &s) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}: {}", f.display(), e);
                return ExitCode::from(2);
            }
        };
        for (index, x) in comments.iter().enumerate() {
            match parse_directive(x) {
                Ok(Some((condition, mut message))) => {
                    if message.is_none() {
                        if let Some(next) = comments.get(index + 1) {
                            if next.span.start_line <= x.span.end_line + 1
                                && !next.content.starts_with("TODO-OR-DIE:")
                            {
                                message = Some(next.content.clone());
                            }
                        }
                    }
                    ts.push(Todo {
                        file: f.clone(),
                        span: x.span.clone(),
                        source: x.raw_text.clone(),
                        condition,
                        message,
                    })
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!(
                        "{}:{}:{}: {}",
                        f.display(),
                        x.span.start_line + 1,
                        x.span.start_column + 1,
                        e
                    );
                    return ExitCode::from(2);
                }
            }
        }
    }
    if let Command::Explain { location } = &c.command {
        let Some((file, line)) = location.rsplit_once(':') else {
            eprintln!("explain location must be file:line");
            return ExitCode::from(2);
        };
        let Ok(line) = line.parse::<usize>() else {
            eprintln!("explain line must be a number");
            return ExitCode::from(2);
        };
        if let Some(todo) = ts
            .iter()
            .find(|todo| todo.file == Path::new(file) && todo.span.start_line + 1 == line)
        {
            println!(
                "{}:{}:{}: {:?}",
                todo.file.display(),
                line,
                todo.span.start_column + 1,
                todo.condition
            );
            println!("  {}", todo.source);
            if let Some(message) = &todo.message {
                println!("  {message}");
            }
            return ExitCode::SUCCESS;
        }
        eprintln!("no TODO found at {location}");
        return ExitCode::from(2);
    }
    let settings = match settings(ps) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid configuration: {error}");
            return ExitCode::from(2);
        }
    };
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(settings.network.timeout_seconds))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            eprintln!("failed to initialize HTTP client: {error}");
            return ExitCode::from(2);
        }
    };
    let mut ex: Vec<Todo> = vec![];
    let mut issue_cache: HashMap<String, todo_or_die_providers::IssueFact> = HashMap::new();
    let mut release_cache: HashMap<String, todo_or_die_providers::ReleaseFact> = HashMap::new();
    for t in &ts {
        let triggered = match &t.condition {
            Condition::Issue {
                provider,
                id: IssueId::RepositoryNumber { repository, number },
                state: expected,
            } => {
                let token = std::env::var(if matches!(provider, IssueProvider::Github) {
                    "GITHUB_TOKEN"
                } else {
                    "GITLAB_TOKEN"
                })
                .ok();
                let host = std::env::var(if matches!(provider, IssueProvider::Github) {
                    "GITHUB_API_URL"
                } else {
                    "GITLAB_API_URL"
                })
                .ok()
                .or_else(|| {
                    if matches!(provider, IssueProvider::Github) {
                        settings.github.api_url.clone()
                    } else {
                        settings.gitlab.api_url.clone()
                    }
                });
                let cache_key = format!("{provider:?}:{repository}#{number}");
                if let Some(fact) = issue_cache.get(&cache_key) {
                    return_state(fact, expected)
                } else {
                    match todo_or_die_providers::issues::resolve(
                        &client,
                        provider,
                        repository,
                        *number,
                        host.as_deref(),
                        token.as_deref(),
                    )
                    .await
                    {
                        Ok(fact) => {
                            let result = return_state(&fact, expected);
                            issue_cache.insert(cache_key, fact);
                            result
                        }
                        Err(e) => {
                            eprintln!("{}: {}", t.file.display(), e);
                            return ExitCode::from(2);
                        }
                    }
                }
            }
            Condition::Issue {
                provider: IssueProvider::Jira,
                id: IssueId::Key(key),
                state: expected,
            } => {
                let cache_key = format!("jira:{key}");
                if let Some(fact) = issue_cache.get(&cache_key) {
                    return_state(fact, expected)
                } else {
                    let host = std::env::var("JIRA_API_URL")
                        .ok()
                        .or_else(|| settings.jira.api_url.clone())
                        .ok_or_else(|| "Jira API URL is not configured".to_owned());
                    let token = std::env::var("JIRA_TOKEN").ok();
                    let fact = match host {
                        Ok(host) => {
                            todo_or_die_providers::issues::jira::resolve(
                                &client,
                                key,
                                &host,
                                token.as_deref(),
                            )
                            .await
                        }
                        Err(error) => {
                            Err(todo_or_die_providers::ProviderError::InvalidVersion(error))
                        }
                    };
                    match fact {
                        Ok(fact) => {
                            let result = return_state(&fact, expected);
                            issue_cache.insert(cache_key, fact);
                            result
                        }
                        Err(error) => {
                            eprintln!("{}: {}", t.file.display(), error);
                            return ExitCode::from(2);
                        }
                    }
                }
            }
            Condition::Cel { source } => {
                let mut facts = vec![];
                let mut releases = vec![];
                for (provider, repository, number) in
                    todo_or_die_providers::cel_issue_references(source)
                {
                    let cache_key = format!("{provider:?}:{repository}#{number}");
                    if let Some(fact) = issue_cache.get(&cache_key) {
                        facts.push(fact.clone());
                        continue;
                    }
                    let token = std::env::var(if matches!(provider, IssueProvider::Github) {
                        "GITHUB_TOKEN"
                    } else {
                        "GITLAB_TOKEN"
                    })
                    .ok();
                    let host = std::env::var(if matches!(provider, IssueProvider::Github) {
                        "GITHUB_API_URL"
                    } else {
                        "GITLAB_API_URL"
                    })
                    .ok();
                    let host = host.or_else(|| {
                        if matches!(provider, IssueProvider::Github) {
                            settings.github.api_url.clone()
                        } else {
                            settings.gitlab.api_url.clone()
                        }
                    });
                    match todo_or_die_providers::issues::resolve(
                        &client,
                        &provider,
                        &repository,
                        number,
                        host.as_deref(),
                        token.as_deref(),
                    )
                    .await
                    {
                        Ok(fact) => {
                            issue_cache.insert(cache_key, fact.clone());
                            facts.push(fact)
                        }
                        Err(e) => {
                            eprintln!("{}: {}", t.file.display(), e);
                            return ExitCode::from(2);
                        }
                    }
                }
                for key in todo_or_die_providers::cel_jira_references(source) {
                    let cache_key = format!("jira:{key}");
                    if let Some(fact) = issue_cache.get(&cache_key) {
                        facts.push(fact.clone());
                        continue;
                    }
                    let host = std::env::var("JIRA_API_URL")
                        .ok()
                        .or_else(|| settings.jira.api_url.clone());
                    let token = std::env::var("JIRA_TOKEN").ok();
                    match host {
                        Some(host) => match todo_or_die_providers::issues::jira::resolve(
                            &client,
                            &key,
                            &host,
                            token.as_deref(),
                        )
                        .await
                        {
                            Ok(fact) => {
                                issue_cache.insert(cache_key, fact.clone());
                                facts.push(fact)
                            }
                            Err(error) => {
                                eprintln!("{}: {}", t.file.display(), error);
                                return ExitCode::from(2);
                            }
                        },
                        None => {
                            eprintln!("{}: Jira API URL is not configured", t.file.display());
                            return ExitCode::from(2);
                        }
                    }
                }
                for (provider, repository) in todo_or_die_providers::cel_release_references(source)
                {
                    let cache_key = format!("{provider:?}:{repository}");
                    if let Some(fact) = release_cache.get(&cache_key) {
                        releases.push(fact.clone());
                        continue;
                    }
                    let token = std::env::var(if matches!(provider, IssueProvider::Github) {
                        "GITHUB_TOKEN"
                    } else {
                        "GITLAB_TOKEN"
                    })
                    .ok();
                    let host = std::env::var(if matches!(provider, IssueProvider::Github) {
                        "GITHUB_API_URL"
                    } else {
                        "GITLAB_API_URL"
                    })
                    .ok();
                    match todo_or_die_providers::releases::resolve(
                        &client,
                        &provider,
                        &repository,
                        host.as_deref(),
                        token.as_deref(),
                    )
                    .await
                    {
                        Ok(fact) => {
                            release_cache.insert(cache_key, fact.clone());
                            releases.push(fact)
                        }
                        Err(e) => {
                            eprintln!("{}: {}", t.file.display(), e);
                            return ExitCode::from(2);
                        }
                    }
                }
                match todo_or_die_cel::evaluate_with_facts(
                    source,
                    todo_or_die_providers::facts_json(&facts, &releases),
                ) {
                    Ok(value) => value,
                    Err(e) => {
                        eprintln!("{}: CEL: {}", t.file.display(), e);
                        return ExitCode::from(2);
                    }
                }
            }
            Condition::Package {
                ecosystem,
                package,
                requirement,
            } => {
                let fact =
                    match todo_or_die_providers::packages::resolve(&client, ecosystem, package)
                        .await
                    {
                        Ok(fact) => fact,
                        Err(e) => {
                            eprintln!("{}: {}", t.file.display(), e);
                            return ExitCode::from(2);
                        }
                    };
                match (
                    semver::Version::parse(&fact.version),
                    semver::VersionReq::parse(requirement),
                ) {
                    (Ok(version), Ok(requirement)) => requirement.matches(&version),
                    _ => {
                        eprintln!(
                            "{}: invalid package version or requirement",
                            t.file.display()
                        );
                        return ExitCode::from(2);
                    }
                }
            }
            condition => state(condition, today()) == TodoState::Triggered,
        };
        if triggered {
            ex.push(t.clone());
        }
    }
    let output = if matches!(c.command, Command::List { .. }) {
        &ts
    } else {
        &ex
    };
    if c.format == "json" {
        let field = if matches!(c.command, Command::List { .. }) {
            "todos"
        } else {
            "expired"
        };
        println!(
            "{}",
            serde_json::json!({"version":1,field:output.iter().map(|t|serde_json::json!({"file":t.file,"line":t.span.start_line+1,"column":t.span.start_column+1,"condition":t.condition,"message":t.message})).collect::<Vec<_>>() })
        );
    } else {
        for t in output {
            println!(
                "{}:{}:{}: {} ({:?})\n  {}\n  {}^",
                t.file.display(),
                t.span.start_line + 1,
                t.span.start_column + 1,
                if matches!(c.command, Command::List { .. }) {
                    "TODO"
                } else {
                    "error: TODO expired"
                },
                t.condition,
                t.source,
                " ".repeat(t.span.start_column)
            )
        }
    }
    if matches!(c.command, Command::List { .. }) || ex.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
