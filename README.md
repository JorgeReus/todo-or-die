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
| HTML | `.html`, `.htm` | `<!-- ... -->` |
| Svelte | `.svelte` | `<!-- ... -->`, `//`, `/* ... */` in scripts |

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
cargo run -p todo-or-die-cli -- explain src/main.rs:42
cargo run -p todo-or-die-cli -- check . --format json
```

After installation, the binary is named `todo-or-die`:

```sh
cargo install --path crates/todo-or-die-cli --bin todo-or-die
todo-or-die check .
```

For prebuilt release binaries, use Cargo Binstall:

```sh
cargo binstall todo-or-die
```

With Nix, install the pinned prebuilt release without compiling Rust:

```sh
nix profile install github:JorgeReus/todo-or-die
nix run github:JorgeReus/todo-or-die -- check .
```

Exit codes are `0` for no expired TODOs, `1` when TODOs have expired, and `2` for malformed directives or other errors.

## Provider conditions

GitHub and GitLab issues use `repository#number` identifiers:

```rust
// TODO-OR-DIE: github org/repo#123 closed
// TODO-OR-DIE: gitlab group/project#42 open
```

Jira uses an issue key and treats `done`, `closed`, and `resolved` as closed:

```java
// TODO-OR-DIE: jira PROJ-123 done
```

Release conditions are available through CEL:

```rust
// TODO-OR-DIE: cel(github.releases["org/repo"].latest().major >= 2)
// TODO-OR-DIE: cel(gitlab.releases["group/project"].latest().major >= 3)
```

CEL can combine facts from several providers in one expiration rule:

```rust
// TODO-OR-DIE: cel(
//   github.issues["org/repo#123"].closed &&
//   jira.issues["PROJ-456"].closed &&
//   github.releases["org/repo"].latest().major >= 2
// )
```

Provider facts are resolved before CEL runs. CEL only evaluates the facts
provided by todo-or-die; it cannot make additional shell, filesystem, or
network calls.

Provider-backed conditions require the provider API URL in `.todo-or-die.toml`.
Tokens are read from `GITHUB_TOKEN`, `GITLAB_TOKEN`, and `JIRA_TOKEN`. Environment
variables `GITHUB_API_URL`, `GITLAB_API_URL`, and `JIRA_API_URL` override the file.

Package conditions currently support npm and crates.io:

```rust
// TODO-OR-DIE: package npm/react >= 20
// TODO-OR-DIE: package crates/serde >= 2
```

These conditions trigger when the registry’s current version satisfies the
requirement:

```typescript
// TODO-OR-DIE: package npm/typescript >= 5.0.0
// Remove the temporary compiler workaround after TypeScript 5.
```

```rust
// TODO-OR-DIE: package crates/reqwest >= 0.13
// Remove the compatibility adapter after the dependency upgrade.
```

Package conditions use the public npm and crates.io registries. They are
currently evaluated by the native package syntax, not through CEL.

`.todo-or-die.toml` settings:

```toml
[network]
timeout_seconds = 30

[github]
api_url = "https://github.example.com/api/v3"

[gitlab]
api_url = "https://gitlab.example.com/api/v4"

[jira]
api_url = "https://jira.example.com"
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
- IDE plugins, GitHub Actions, or other integrations

Package-version conditions can build on the existing provider and CEL fact layers.
