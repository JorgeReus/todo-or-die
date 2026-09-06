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
- Explains findings with `explain file:line`.
- Resolves GitHub, GitLab, and Jira issue conditions.
- Resolves GitHub and GitLab releases through CEL.
- Resolves npm and crates.io package versions.
- Combines multiple provider facts in CEL expressions.
- Ships pinned prebuilt binaries through Cargo Binstall and Nix.

Examples:

```rust
// TODO-OR-DIE: after 2027-01-01
// Remove the temporary compatibility workaround.

// TODO-OR-DIE: github acme/payments#481 closed
// Remove this workaround when the upstream fix lands.

// TODO-OR-DIE: package crates/reqwest >= 0.13
// Remove the compatibility adapter after the dependency upgrade.
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

## Installation

Install a prebuilt binary with Cargo Binstall:

```sh
cargo binstall todo-or-die
```

Or install the pinned release with Nix (no Rust compilation):

```sh
nix profile install github:JorgeReus/todo-or-die
```

## Quick start

Once `todo-or-die` is installed, scan the current repository:

```sh
todo-or-die check .
```

### GitHub Actions

Use the action from a release tag. It downloads the matching prebuilt binary
for the runner’s operating system and architecture:

```yaml
- uses: JorgeReus/todo-or-die@v0.2.1
  with:
    path: .
```

Other useful commands:

```sh
todo-or-die list .
todo-or-die explain src/main.rs:42
todo-or-die check . --format json
```

Exit codes are `0` for no expired TODOs, `1` when TODOs have expired, and `2` for malformed directives or other errors.

## Provider conditions

### GitHub issues

```rust
// TODO-OR-DIE: github org/repo#123 closed
```

Set `GITHUB_TOKEN` for private repositories. Use `GITHUB_API_URL` for GitHub
Enterprise.

### GitLab issues

```go
// TODO-OR-DIE: gitlab group/project#42 open
```

Set `GITLAB_TOKEN` for private projects. Use `GITLAB_API_URL` for self-managed
GitLab.

### Jira issues

Jira uses an issue key; `done`, `closed`, and `resolved` are closed:

```java
// TODO-OR-DIE: jira PROJ-123 done
```

Set `JIRA_TOKEN` and `JIRA_API_URL` for the Jira instance.

### Releases

Release conditions are available through CEL:

```rust
// TODO-OR-DIE: cel(github.releases["org/repo"].latest().major >= 2)
// TODO-OR-DIE: cel(gitlab.releases["group/project"].latest().major >= 3)
```

More examples:

```rust
// TODO-OR-DIE: cel(gitlab.releases["group/project"].latest().patch >= 4)
// TODO-OR-DIE: cel(github.issues["org/repo#123"].closed || jira.issues["PROJ-456"].closed)
```

CEL can combine facts from several providers in one expiration rule:

```rust
// TODO-OR-DIE: cel(
//   github.issues["org/repo#123"].closed &&
//   jira.issues["PROJ-456"].closed &&
//   github.releases["org/repo"].latest().major >= 2
// )
```

Provider facts are resolved before CEL runs. CEL evaluates only facts provided
by todo-or-die; it cannot make additional shell or filesystem calls.

Provider-backed conditions require the provider API URL in `.todo-or-die.toml`.
Tokens are read from `GITHUB_TOKEN`, `GITLAB_TOKEN`, and `JIRA_TOKEN`. Environment
variables `GITHUB_API_URL`, `GITLAB_API_URL`, and `JIRA_API_URL` override the file.

### Packages

Package conditions currently support npm and crates.io:

```rust
// TODO-OR-DIE: package npm/react >= 20
// TODO-OR-DIE: package crates/serde >= 2
// TODO-OR-DIE: package npm/typescript ^5.0
// TODO-OR-DIE: package crates/reqwest ~0.12
// TODO-OR-DIE: package npm/react >= 18, < 20
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

### Configuration

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

Tokens are read from `GITHUB_TOKEN`, `GITLAB_TOKEN`, and `JIRA_TOKEN`.

## Development environment

The Nix flake provides the pinned Rust toolchain, Rustfmt, and Clippy:

```sh
nix develop
```
