# todo-or-die

Temporary code should have an expiration date. `todo-or-die` finds those TODOs in source comments and makes CI fail when they expire.

## Features

- Detects `TODO-OR-DIE` directives in source comments with a language-aware lexer.
- Ignores matching text inside normal and multiline strings.
- Supports `after YYYY-MM-DD` expiration conditions.
- Scans a repository, directory, or individual file.
- Respects `.gitignore` when scanning directories.
- Reports human-readable or JSON output.
- Uses predictable exit codes for CI.

Example:

```rust
// TODO-OR-DIE: after 2027-01-01
// Remove the temporary compatibility workaround.
```

Before the date, the TODO is active. On or after the date, `check` reports it as expired and exits with status `1`.

## Supported languages

| Language | File extensions | Comment styles |
|---|---|---|
| Rust | `.rs` | `//`, `/* ... */` |
| TypeScript | `.ts`, `.tsx` | `//`, `/* ... */` |
| JavaScript | `.js`, `.jsx` | `//`, `/* ... */` |
| Python | `.py` | `#` |
| Go | `.go` | `//`, `/* ... */` |
| Java | `.java` | `//`, `/* ... */` |
| Kotlin | `.kt`, `.kts` | `//`, `/* ... */` |

The lexer understands strings and comments, so this does not create a TODO:

```javascript
const text = "// TODO-OR-DIE: after 2020-01-01";
```

CEL can combine multiple provider facts:

```rust
// TODO-OR-DIE: cel(github.issues["org/repo#123"].closed && github.releases["org/repo"].latest().major >= 2)
```

Issue and release facts are fetched before CEL evaluates the expression. Repeated issue references are fetched once per run. Release versions expose `latest`, `major`, `minor`, and `patch` through the `latest()` method.

## Quick start

With Nix and direnv:

```sh
direnv allow
todo-or-die check .
```

Without installing the binary yet:

```sh
cargo run -p todo-or-die-cli -- check .
cargo run -p todo-or-die-cli -- check src/main.rs
cargo run -p todo-or-die-cli -- list .
cargo run -p todo-or-die-cli -- check . --format json
```

After installation, the binary is named `todo-or-die`:

```sh
cargo install --path crates/todo-or-die-cli --bin todo-or-die
todo-or-die check .
```

Exit codes are `0` for no expired TODOs, `1` when TODOs have expired, and `2` for malformed directives or other errors.

Issue conditions use `GITHUB_TOKEN` or `GITLAB_TOKEN` when authentication is needed. For private installations, set `GITHUB_API_URL` or `GITLAB_API_URL` to the provider API base URL.

Package conditions currently support npm and crates.io:

```rust
// TODO-OR-DIE: package npm/react >= 20
// TODO-OR-DIE: package crates/serde >= 2
```

Optional `.todo-or-die.toml` settings:

```toml
[network]
timeout_seconds = 30

[github]
api_url = "https://github.example.com/api/v3"

[gitlab]
api_url = "https://gitlab.example.com/api/v4"
```

## Development environment

The Nix flake provides the pinned Rust toolchain, Rustfmt, and Clippy:

```sh
nix develop
```

The `.envrc` file loads the same environment automatically through direnv.

## Verification

```sh
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Not implemented yet

The current MVP does not yet include:

- SARIF output
- Automatic TODO removal or fixing
- `explain` command
- IDE plugins, GitHub Actions, or other integrations

Package-version conditions can build on the existing provider and CEL fact layers.
