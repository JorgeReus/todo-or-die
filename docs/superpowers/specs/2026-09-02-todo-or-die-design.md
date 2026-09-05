# todo-or-die MVP Design

## Goal

Replace the Go scaffold with a Rust CLI that finds `TODO-OR-DIE:` directives in real source comments and evaluates `after YYYY-MM-DD` conditions.

## Architecture

Use a Cargo workspace with independent core, Tree-sitter, and CLI crates. Tree-sitter extracts comment nodes; the domain parser handles directives; the evaluator uses an injectable clock; reporters render human or JSON output.

## Scope

Implement `check` and `list` for Rust, TypeScript, JavaScript, Python, Go, and Java. Respect `.gitignore`, report malformed directives as errors, use exit codes 0/1/2, and provide deterministic tests. Add `shell.nix` with Rust and build dependencies. Defer CEL, providers, config, SARIF, and `explain`.

## Testing

Unit-test parsing, normalization, language detection, spans, and date evaluation. Add language fixtures proving comments are detected and directive-like strings are ignored. Add CLI integration coverage for active, expired, and malformed directives.
