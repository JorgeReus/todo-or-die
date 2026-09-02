# todo-or-die

Temporary code should have an expiration date. `todo-or-die` finds those TODOs in source comments and makes CI fail when they expire.

## Features

- Detects `TODO-OR-DIE` directives in parsed source comments.
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

Detection is based on Tree-sitter comment nodes, so this does not create a TODO:

```javascript
const text = "// TODO-OR-DIE: after 2020-01-01";
```

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

Exit codes are `0` for no expired TODOs, `1` when TODOs have expired, and `2` for malformed directives or other errors.

## Development environment

The Nix flake provides the pinned Rust toolchain, Rustfmt, Clippy, and Tree-sitter:

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

- CEL expression evaluation
- GitHub issue or package-version conditions
- Repository configuration files
- SARIF output
- Automatic TODO removal or fixing
- `explain` command
- IDE plugins, GitHub Actions, or other integrations

These can build on the existing separation between language parsing, directive parsing, and condition evaluation.
