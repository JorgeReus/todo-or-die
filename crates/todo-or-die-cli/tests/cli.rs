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
