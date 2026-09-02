use clap::{Parser, Subcommand};
use std::{fs, path::PathBuf, process::ExitCode};
use todo_or_die_core::*;
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
fn main() -> ExitCode {
    let c = Cli::parse();
    let ps = match &c.command {
        Command::Check { paths: p } | Command::List { paths: p } if !p.is_empty() => p,
        _ => &vec![PathBuf::from(".")],
    };
    let mut ts = vec![];
    for f in files(ps) {
        let Ok(s) = fs::read_to_string(&f) else {
            continue;
        };
        for x in match todo_or_die_tree_sitter::extract_comments(&f, &s) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}: {}", f.display(), e);
                return ExitCode::from(2);
            }
        } {
            match parse_directive(&x) {
                Ok(Some((condition, message))) => ts.push(Todo {
                    file: f.clone(),
                    span: x.span,
                    condition,
                    message,
                }),
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
    let ex: Vec<_> = ts
        .iter()
        .filter(|t| state(&t.condition, today()) == TodoState::Triggered)
        .collect();
    if c.format == "json" {
        println!(
            "{}",
            serde_json::json!({"version":1,"expired":ex.iter().map(|t|serde_json::json!({"file":t.file,"line":t.span.start_line+1,"column":t.span.start_column+1,"condition":t.condition,"message":t.message})).collect::<Vec<_>>() })
        );
    } else {
        for t in &ex {
            println!(
                "{}:{}:{}: error: TODO expired ({:?})",
                t.file.display(),
                t.span.start_line + 1,
                t.span.start_column + 1,
                t.condition
            )
        }
    }
    if matches!(c.command, Command::List { .. }) || ex.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}
