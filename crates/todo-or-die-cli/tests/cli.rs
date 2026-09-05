use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn run(source: &str) -> std::process::Output {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("todo-or-die-{}-{nonce}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    fs::write(dir.join("fixture.rs"), source).unwrap();
    let binary = std::env::var("CARGO_BIN_EXE_todo-or-die")
        .or_else(|_| std::env::var("CARGO_BIN_EXE_todo-or-die-cli"))
        .unwrap();
    let output = Command::new(binary)
        .args(["check", dir.to_str().unwrap()])
        .output()
        .unwrap();
    let _ = fs::remove_dir_all(dir);
    output
}

#[test]
fn expired_directive_exits_one() {
    assert_eq!(
        run("// TODO-OR-DIE: after 2000-01-01\nfn main() {}\n")
            .status
            .code(),
        Some(1)
    );
}

#[test]
fn malformed_directive_exits_two() {
    assert_eq!(run("// TODO-OR-DIE: after never\n").status.code(), Some(2));
}

#[test]
fn future_directive_exits_zero() {
    assert_eq!(
        run("// TODO-OR-DIE: after 2999-01-01\n").status.code(),
        Some(0)
    );
}

#[test]
fn explain_shows_todo_at_requested_line() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("todo-or-die-explain-{nonce}"));
    fs::create_dir_all(&dir).unwrap();
    let file = dir.join("fixture.rs");
    fs::write(&file, "fn main() {}\n// TODO-OR-DIE: after 2000-01-01\n").unwrap();
    let binary = std::env::var("CARGO_BIN_EXE_todo-or-die")
        .or_else(|_| std::env::var("CARGO_BIN_EXE_todo-or-die-cli"))
        .unwrap();
    let output = Command::new(binary)
        .args(["explain", &format!("{}:2", file.display())])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("after 2000-01-01"));
    let _ = fs::remove_dir_all(dir);
}
