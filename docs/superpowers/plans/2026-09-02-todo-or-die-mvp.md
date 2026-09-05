# todo-or-die MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Rust MVP described in `PLAN.md`.

**Architecture:** A Cargo workspace separates domain/evaluation, Tree-sitter extraction, and CLI discovery/reporting. The CLI composes those pieces into `check` and `list`.

**Tech Stack:** Rust, Cargo, clap, chrono, serde, serde_json, tree-sitter grammars, ignore, thiserror.

## Global Constraints

- Support six initial languages: Rust, TypeScript, JavaScript, Python, Go, Java.
- Do not detect directives inside strings; Tree-sitter must identify comments.
- Exit codes: 0 clean, 1 expired, 2 error.
- Keep CEL and external providers out of the MVP.

### Task 1: Rust workspace and shell environment

**Files:** Create `Cargo.toml`, `crates/*/Cargo.toml`, `shell.nix`; remove Go scaffold files.

- [ ] Create the three workspace manifests with minimal dependencies.
- [ ] Add `shell.nix` exposing Rust/Cargo, pkg-config, and Tree-sitter build support.
- [ ] Verify `nix-shell --run 'cargo check --workspace'` when Nix is available.

### Task 2: Core domain and parser

**Files:** Create `crates/todo-or-die-core/src/lib.rs`, `src/directive.rs`, `src/evaluate.rs`.

- [ ] Define spans, comments, conditions, todos, states, results, and typed errors.
- [ ] Parse the exact `TODO-OR-DIE: after YYYY-MM-DD` syntax and reject malformed dates.
- [ ] Add an injectable clock and tests for active/expired conditions.

### Task 3: Tree-sitter extraction

**Files:** Create `crates/todo-or-die-tree-sitter/src/lib.rs`, `src/languages.rs`, `src/comments.rs`, fixtures under `tests/fixtures/`.

- [ ] Register six extensions and grammars.
- [ ] Parse source, query comment nodes, preserve source spans, and normalize line/block delimiters.
- [ ] Test each language and a directive-like string false positive.

### Task 4: CLI pipeline and reporters

**Files:** Create `crates/todo-or-die-cli/src/main.rs`, `src/lib.rs`, `src/report.rs`.

- [ ] Discover explicit files and recursively walk directories with `.gitignore` support.
- [ ] Compose extraction, parsing, evaluation, and reporting for `check` and `list`.
- [ ] Implement human and stable JSON output plus exit codes 0/1/2.
- [ ] Add CLI tests for clean, expired, malformed, and list cases.

### Task 5: Documentation and verification

**Files:** Create/update `README.md` and CI configuration if needed.

- [ ] Document installation, shell environment, commands, syntax, and exit codes.
- [ ] Run `cargo fmt --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, and `cargo test --workspace`.
