# todo-or-die — Cross-Language Implementation Specification

## 1. Objective

Build a standalone, cross-language implementation of `todo-or-die`.

The tool should detect temporary code markers embedded in source-code comments and fail CI or local checks when their associated expiration conditions become true.

The implementation must not be tied to Rust compilation, macros, TypeScript linting, Python tooling, or any particular language ecosystem.

The core design should be:

```text
source files
    ↓
Tree-sitter
    ↓
comment nodes
    ↓
todo-or-die directive parser
    ↓
condition representation
    ↓
fact/provider resolution
    ↓
condition evaluation
    ↓
diagnostics
```

The executable should initially be implemented in Rust.

The CLI should be usable against arbitrary repositories:

```bash
tod check .
```

or:

```bash
todo-or-die check .
```

The final binary name may be decided during implementation, but internally prefer the project name `todo-or-die`.

---

# 2. Design Principles

The implementation should follow these principles.

## Language-independent

The core must not depend on compiler plugins, procedural macros, ESLint, Clippy, or similar tooling.

Language-specific integrations may be added later, but they should delegate to the same engine.

## Comments are the integration boundary

A todo-or-die directive lives inside a source-code comment.

Examples:

```rust
// TODO-OR-DIE: after 2027-01-01
```

```typescript
// TODO-OR-DIE: after 2027-01-01
```

```python
# TODO-OR-DIE: after 2027-01-01
```

```java
// TODO-OR-DIE: after 2027-01-01
```

```html
<!-- TODO-OR-DIE: after 2027-01-01 -->
```

Tree-sitter should be responsible for determining what text is actually a comment.

Do not detect directives using a repository-wide regex over raw source text.

For example:

```typescript
const value = "// TODO-OR-DIE: after 2020-01-01";
```

must NOT produce a directive.

---

# 3. MVP Scope

Implement the following features in the first usable version.

## CLI

Support:

```bash
todo-or-die check .
```

```bash
todo-or-die check src/
```

```bash
todo-or-die check file.rs
```

```bash
todo-or-die list .
```

```bash
todo-or-die explain path/to/file.ts:42
```

Minimum required commands:

```text
check
list
```

`explain` may be implemented after the core pipeline works.

---

# 4. Supported Conditions

For the first version, support a very small native DSL.

Required:

```text
after YYYY-MM-DD
```

Example:

```rust
// TODO-OR-DIE: after 2027-01-01
// Remove temporary authentication fallback.
```

Semantics:

```text
current_date >= configured_date
```

means the TODO has expired.

Also support an explicit CEL escape hatch:

```text
cel(<expression>)
```

Example:

```rust
// TODO-OR-DIE: cel(now >= timestamp("2027-01-01T00:00:00Z"))
```

Do not try to make all built-in syntax CEL.

The common syntax should remain human-readable.

Preferred architecture:

```text
Condition
├── After
├── GitHubIssue        future
├── PackageVersion     future
└── Cel
```

---

# 5. Future DSL

Design parsing so the following can later be added without breaking existing syntax.

Potential examples:

```text
TODO-OR-DIE: after 2027-01-01
```

```text
TODO-OR-DIE: github owner/repo#123 closed
```

```text
TODO-OR-DIE: package npm/react >= 20
```

```text
TODO-OR-DIE: package crates/serde >= 2
```

```text
TODO-OR-DIE: env FEATURE_X == "enabled"
```

```text
TODO-OR-DIE: cel(...)
```

The MVP only needs `after` and `cel`, but the AST must be extensible.

---

# 6. Directive Format

The canonical directive starts with:

```text
TODO-OR-DIE:
```

Matching should initially be case-sensitive.

Example:

```rust
// TODO-OR-DIE: after 2027-01-01
```

The next comment line or comment block may contain the human explanation.

Example:

```rust
// TODO-OR-DIE: after 2027-01-01
// Remove compatibility layer after the database migration.
fn legacy_adapter() {}
```

Represent this internally approximately as:

```rust
pub struct Todo {
    pub file: PathBuf,
    pub span: SourceSpan,
    pub directive_span: SourceSpan,
    pub message: Option<String>,
    pub condition: Condition,
}
```

And:

```rust
pub struct SourceSpan {
    pub start_byte: usize,
    pub end_byte: usize,

    pub start_line: usize,
    pub start_column: usize,

    pub end_line: usize,
    pub end_column: usize,
}
```

---

# 7. Condition AST

Use an explicit Rust enum.

For example:

```rust
pub enum Condition {
    After {
        date: chrono::NaiveDate,
    },

    Cel {
        source: String,
    },

    // Future
    GithubIssue {
        repository: String,
        number: u64,
        expected_state: GithubIssueState,
    },

    // Future
    PackageVersion {
        ecosystem: PackageEcosystem,
        package: String,
        requirement: String,
    },
}
```

Do not model conditions as arbitrary strings after parsing.

Parse once at the boundary and operate on typed values after that.

---

# 8. Tree-sitter Architecture

Use Tree-sitter strictly as the host-language parser.

Tree-sitter should answer:

> Where are the comments in this source file?

The todo-or-die parser should answer:

> Does this comment contain a todo-or-die directive, and what does that directive mean?

These should be separate modules.

Suggested modules:

```text
src/
├── cli/
├── discovery/
├── languages/
├── comments/
├── directive/
├── conditions/
├── providers/
├── evaluator/
├── diagnostics/
├── output/
└── config/
```

---

# 9. Language Detection

Language detection should primarily use file extension.

Example mapping:

```text
.rs          → Rust
.ts          → TypeScript
.tsx         → TSX
.js          → JavaScript
.jsx         → JSX
.py          → Python
.go          → Go
.java        → Java
.kt/.kts     → Kotlin
.c/.h        → C
.cpp/.hpp    → C++
.cs          → C#
.rb          → Ruby
.php         → PHP
.swift       → Swift
```

Do not attempt content-based language detection in the MVP.

Unsupported files should be skipped.

The result should optionally be visible under verbose logging.

---

# 10. Tree-sitter Grammar Strategy

Avoid shipping dozens of languages immediately.

Start with:

```text
Rust
TypeScript
JavaScript
Python
Go
Java
```

Prefer Tree-sitter crates where available.

Create a common abstraction:

```rust
pub trait LanguageAdapter {
    fn language(&self) -> tree_sitter::Language;

    fn comment_query(&self) -> &'static str;
}
```

or an equivalent static registry.

Example concept:

```rust
pub struct LanguageDefinition {
    pub id: LanguageId,
    pub extensions: &'static [&'static str],
    pub tree_sitter_language: fn() -> tree_sitter::Language,
    pub comment_query: &'static str,
}
```

Then maintain:

```rust
static LANGUAGES: &[LanguageDefinition] = &[...];
```

The scanner should not contain large `match` blocks spread throughout the codebase.

---

# 11. Comment Extraction

Use Tree-sitter queries where appropriate.

For languages where comments use a node named:

```text
comment
```

a query such as:

```scheme
(comment) @comment
```

may be enough.

However, do not assume every grammar has identical node names.

Keep the comment query part of each language definition.

Each extracted comment should preserve:

```rust
pub struct SourceComment {
    pub raw_text: String,
    pub span: SourceSpan,
    pub language: LanguageId,
}
```

Do not strip comment delimiters too early.

Instead have a normalized representation as well:

```rust
pub struct NormalizedComment {
    pub raw_text: String,
    pub content: String,
    pub span: SourceSpan,
}
```

Examples:

```text
// hello
```

becomes:

```text
hello
```

and:

```text
# hello
```

becomes:

```text
hello
```

and:

```text
/* hello */
```

becomes:

```text
hello
```

Normalization should be language-aware enough to support normal line and block comments.

---

# 12. Multi-line Comments

Support:

```rust
/*
 * TODO-OR-DIE: after 2027-01-01
 * Remove old implementation.
 */
```

Also support consecutive comment lines:

```rust
// TODO-OR-DIE: after 2027-01-01
// Remove old implementation.
```

For the MVP, the simplest acceptable behavior is:

* directive is parsed from a single comment node;
* message may be extracted from the remaining text in that same comment node.

Grouping consecutive separate line-comment AST nodes into one logical block is desirable but not required for the very first implementation.

However, structure the extraction code so this can be added cleanly.

---

# 13. Directive Parser

Do NOT use a parser generator for the MVP.

The syntax is intentionally tiny.

A simple hand-written parser is preferred.

Input:

```text
TODO-OR-DIE: after 2027-01-01
```

Output:

```rust
Directive {
    condition: Condition::After {
        date: ...
    },
    ...
}
```

Suggested flow:

```text
normalized comment
    ↓
find directive prefix
    ↓
trim
    ↓
parse condition keyword
    ↓
parse condition arguments
    ↓
typed Condition
```

Do not run CEL parsing unless the condition begins with:

```text
cel(
```

---

# 14. CEL Integration

Use CEL only as an advanced expression engine.

Do not make CEL responsible for:

```text
finding source comments
parsing todo-or-die directives
HTTP calls
GitHub authentication
package registry access
filesystem access
process execution
```

CEL evaluation should receive an already constructed environment.

Example:

```json
{
  "now": "2027-01-10T10:00:00Z",
  "repo": {
    "branch": "main"
  }
}
```

Conceptually:

```rust
pub struct EvaluationContext {
    pub now: DateTime<Utc>,
    pub facts: Facts,
}
```

The CEL implementation should be wrapped behind an interface so the actual CEL crate can be changed later.

For example:

```rust
pub trait ExpressionEvaluator {
    fn evaluate(
        &self,
        expression: &str,
        context: &EvaluationContext,
    ) -> Result<bool, EvaluationError>;
}
```

Do not leak CEL-specific types across the entire codebase.

---

# 15. Providers

External conditions will eventually need data from external systems.

Design that abstraction now even if the MVP only uses time.

Concept:

```rust
#[async_trait]
pub trait Provider {
    fn kind(&self) -> ProviderKind;

    async fn resolve(
        &self,
        requirements: &[Requirement],
    ) -> Result<Facts, ProviderError>;
}
```

Potential providers:

```text
ClockProvider
GitHubProvider
PackageRegistryProvider
EnvironmentProvider
GitProvider
```

Important:

Do not let every TODO independently perform network calls.

The evaluator should first collect requirements.

Example:

```text
100 TODOs
     ↓
collect requirements
     ↓
deduplicate
     ↓
provider batch resolution
     ↓
facts
     ↓
evaluate all TODOs
```

This is important for:

```text
performance
rate limiting
offline support
testing
deterministic evaluation
caching
```

---

# 16. Clock Abstraction

Never directly call `Utc::now()` from condition evaluation.

Create:

```rust
pub trait Clock {
    fn now(&self) -> DateTime<Utc>;
}
```

Production:

```rust
SystemClock
```

Tests:

```rust
FixedClock
```

This allows deterministic expiration tests.

---

# 17. Repository Discovery

When running:

```bash
todo-or-die check .
```

recursively walk files.

Use something like:

```text
ignore
```

or equivalent Rust ecosystem tooling so `.gitignore` is respected.

Ignore:

```text
.git/
node_modules/
target/
dist/
build/
vendor/
```

where appropriate.

Do not hardcode everything if `.gitignore` already provides the answer.

The `ignore` crate is preferred.

Support explicit files even if they would normally be ignored:

```bash
todo-or-die check ignored/generated.rs
```

if practical.

---

# 18. Configuration

Support a repository config file eventually.

Preferred name:

```text
.todo-or-die.toml
```

Possible initial schema:

```toml
version = 1

[scan]
exclude = [
  "generated/**",
  "vendor/**"
]

[output]
format = "human"
```

Future configuration could contain:

```toml
[github]
repository = "my-org/my-repo"
```

Do not require configuration for the MVP.

The following should work immediately:

```bash
todo-or-die check .
```

---

# 19. CLI Exit Codes

Use predictable exit codes.

Recommended:

```text
0 = scan successful and no expired TODOs
1 = one or more TODO conditions triggered
2 = configuration/parsing/runtime error
```

A malformed todo-or-die directive should be considered an error, not silently ignored.

Example:

```rust
// TODO-OR-DIE: after bananas
```

should fail with a useful diagnostic.

---

# 20. Diagnostics

Diagnostics are a major product feature.

Example:

```text
src/auth/session.ts:142:3

error: TODO expired 17 days ago

  142 │ // TODO-OR-DIE: after 2027-01-01
      │    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  143 │ // Remove temporary OAuth compatibility layer.

condition:
  after 2027-01-01

current date:
  2027-01-18
```

At minimum include:

```text
file
line
column
condition
message if available
reason condition triggered
```

Use source spans from Tree-sitter.

Do not make diagnostics depend on compiler integration.

---

# 21. Output Formats

The output subsystem should be abstracted.

Concept:

```rust
pub trait Reporter {
    fn report(&self, result: &CheckResult) -> Result<()>;
}
```

Initial formats:

```text
human
json
```

Future:

```text
sarif
github
gitlab
```

CLI:

```bash
todo-or-die check . --format human
```

```bash
todo-or-die check . --format json
```

JSON should be stable enough for CI tooling.

Example:

```json
{
  "version": 1,
  "expired": [
    {
      "file": "src/auth/session.ts",
      "line": 142,
      "column": 3,
      "condition": {
        "type": "after",
        "date": "2027-01-01"
      },
      "message": "Remove temporary OAuth compatibility layer."
    }
  ]
}
```

---

# 22. Suggested Rust Crates

Evaluate current versions before implementation.

Likely useful crates:

```text
clap
tree-sitter
tree-sitter-rust
tree-sitter-typescript
tree-sitter-javascript
tree-sitter-python
tree-sitter-go
tree-sitter-java
ignore
chrono
serde
serde_json
toml
thiserror
miette
async-trait
tokio
```

For diagnostics, strongly consider:

```text
miette
```

For CEL, investigate available Rust CEL implementations before selecting one.

The CEL dependency must remain behind the abstraction described earlier.

---

# 23. Workspace Structure

Prefer a Cargo workspace even if only one binary exists initially.

Example:

```text
todo-or-die/
├── Cargo.toml
├── crates/
│   ├── todo-or-die-core/
│   ├── todo-or-die-cli/
│   ├── todo-or-die-tree-sitter/
│   └── todo-or-die-cel/
├── tests/
│   ├── fixtures/
│   └── integration/
├── README.md
├── LICENSE
└── .github/
```

Responsibilities:

```text
todo-or-die-core
    domain types
    Condition
    Todo
    evaluation
    providers
    result types

todo-or-die-tree-sitter
    language registry
    parser initialization
    comment extraction
    source spans

todo-or-die-cel
    CEL adapter
    EvaluationContext → CEL values

todo-or-die-cli
    clap
    filesystem traversal
    config
    reporting
    exit codes
```

Avoid one giant binary crate.

---

# 24. Error Modeling

Use typed errors.

Do not use arbitrary string errors throughout the core.

Example:

```rust
#[derive(Debug, thiserror::Error)]
pub enum DirectiveParseError {
    #[error("unknown todo-or-die condition: {0}")]
    UnknownCondition(String),

    #[error("invalid date: {0}")]
    InvalidDate(String),

    #[error("invalid CEL expression")]
    InvalidCelExpression,
}
```

Other domains should have their own error types.

At CLI boundaries these can be rendered through `miette` or equivalent.

---

# 25. Core Pipeline

The main execution pipeline should look conceptually like this:

```rust
pub async fn check(
    paths: &[PathBuf],
    options: CheckOptions,
) -> Result<CheckResult> {
    let files = discover_files(paths, &options)?;

    let mut todos = Vec::new();

    for file in files {
        let language = detect_language(&file)?;

        let source = read_source(&file)?;

        let comments =
            extract_comments(language, &source)?;

        let directives =
            parse_directives(&comments)?;

        todos.extend(directives);
    }

    let requirements =
        collect_requirements(&todos);

    let facts =
        resolve_requirements(requirements).await?;

    let results =
        evaluate_todos(&todos, &facts)?;

    Ok(CheckResult::new(results))
}
```

Exact types may differ.

Maintain this separation of responsibilities.

---

# 26. Evaluation Semantics

A TODO has three possible states:

```rust
pub enum TodoState {
    Active,
    Triggered,
    EvaluationError,
}
```

Examples:

```text
after 2030-01-01
today = 2027

→ Active
```

```text
after 2027-01-01
today = 2027-01-02

→ Triggered
```

Invalid syntax should generally be a parse error before evaluation.

---

# 27. Testing Strategy

Testing is required from the beginning.

## Unit Tests

Test:

```text
directive parsing
date parsing
condition evaluation
comment normalization
language detection
source span calculation
```

Example:

```rust
#[test]
fn parses_after_condition() {
    ...
}
```

## Tree-sitter Fixtures

Maintain fixture files for every supported language.

Example:

```text
tests/fixtures/
├── rust/
│   ├── simple.rs
│   ├── strings.rs
│   └── multiline.rs
├── typescript/
├── javascript/
├── python/
├── go/
└── java/
```

Every language should test the critical false-positive case.

Example:

```typescript
const x = "// TODO-OR-DIE: after 2020-01-01";
```

must NOT be detected.

But:

```typescript
// TODO-OR-DIE: after 2020-01-01
```

must be detected.

## Integration Tests

Test CLI exit codes.

Example:

```text
repository with no TODOs
→ exit 0
```

```text
repository with future TODO
→ exit 0
```

```text
repository with expired TODO
→ exit 1
```

```text
repository with malformed directive
→ exit 2
```

---

# 28. Performance

The tool should be fast enough to run on every CI build.

Avoid:

```text
parsing unsupported files
network calls per TODO
loading duplicate Tree-sitter parsers unnecessarily
reading binary files
walking ignored directories
```

Potential optimizations:

```text
parallel file parsing
parser reuse
provider batching
provider caching
```

Do not prematurely optimize, but do not create architecture that prevents parallel scanning.

---

# 29. Security Model

CEL expressions are untrusted repository input.

They must NOT allow arbitrary:

```text
shell execution
filesystem access
network requests
environment mutation
process spawning
dynamic library loading
```

CEL should evaluate pure expressions against explicitly provided values/functions.

Provider credentials must never be exposed directly to CEL.

Example:

Bad:

```text
CEL
  ↓
GitHub HTTP API
```

Preferred:

```text
Provider
  ↓
Facts
  ↓
CEL
```

---

# 30. Determinism

Given:

```text
same source
same configuration
same facts
same clock
```

evaluation should produce identical results.

Keep external data fetching separate from evaluation specifically to preserve this property.

---

# 31. GitHub Integration — Phase 2

Do NOT block MVP on this feature.

Eventually support:

```text
// TODO-OR-DIE: github my-org/my-repo#123 closed
```

Possible internal requirement:

```rust
Requirement::GithubIssue {
    repository: "my-org/my-repo",
    number: 123,
}
```

Provider result:

```rust
GithubIssueFact {
    repository,
    number,
    state,
}
```

Requirements across TODOs should be deduplicated before API calls.

Authentication should support environment-based tokens.

For example:

```text
GITHUB_TOKEN
```

Do not put credentials inside `.todo-or-die.toml`.

---

# 32. Package Version Conditions — Phase 2/3

Eventually:

```text
// TODO-OR-DIE: package npm/react >= 20
```

```text
// TODO-OR-DIE: package crates/serde >= 2
```

Potential ecosystems:

```text
npm
crates
pypi
maven
go
nuget
```

Do not implement all registries at once.

Build the provider abstraction first.

---

# 33. SARIF — Phase 2

Add:

```bash
todo-or-die check . --format sarif
```

This should enable integration with code-scanning systems.

The underlying domain model should not know about SARIF.

SARIF belongs in:

```text
output/reporters/sarif
```

---

# 34. Potential Future Commands

Architecture should not prevent:

```bash
todo-or-die list .
```

```bash
todo-or-die check .
```

```bash
todo-or-die explain src/foo.rs:42
```

```bash
todo-or-die fix .
```

```bash
todo-or-die check --changed
```

```bash
todo-or-die check --since origin/main
```

```bash
todo-or-die check --format sarif
```

```bash
todo-or-die doctor
```

Do not implement all of them in MVP.

---

# 35. Potential `fix` Behavior

Future command:

```bash
todo-or-die fix src/foo.rs:42
```

Could remove only the directive while preserving the explanation.

Input:

```rust
// TODO-OR-DIE: after 2027-01-01
// Remove old workaround.
```

Output:

```rust
// Remove old workaround.
```

This is another reason source spans must be accurately preserved.

---

# 36. MVP Acceptance Criteria

The MVP is complete when all of the following work.

A user can create:

```rust
// TODO-OR-DIE: after 2020-01-01
fn old_workaround() {}
```

and run:

```bash
todo-or-die check .
```

The CLI:

1. discovers the Rust source file;
2. detects Rust from the extension;
3. parses it using Tree-sitter;
4. extracts the comment node;
5. detects the `TODO-OR-DIE` directive;
6. parses the `after` condition;
7. evaluates it against the current date;
8. determines it has expired;
9. prints a source-aware diagnostic;
10. exits with status code `1`.

The same must work for:

```text
Rust
TypeScript
JavaScript
Python
Go
Java
```

The following must NOT be detected:

```typescript
const x =
    "// TODO-OR-DIE: after 2020-01-01";
```

The following malformed directive:

```rust
// TODO-OR-DIE: after foo
```

must produce a parse diagnostic and exit `2`.

The following:

```rust
// TODO-OR-DIE: after 2100-01-01
```

must result in exit `0`.

---

# 37. Recommended Implementation Order

Implement in this order.

## Phase 1 — Workspace

Create:

```text
todo-or-die-core
todo-or-die-tree-sitter
todo-or-die-cli
```

Set up:

```text
clap
thiserror
chrono
serde
miette
ignore
```

Add CI:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Phase 2 — Domain Model

Implement:

```text
Todo
SourceSpan
Condition
TodoState
CheckResult
```

No filesystem or Tree-sitter logic yet.

## Phase 3 — Directive Parser

Implement:

```text
TODO-OR-DIE: after YYYY-MM-DD
```

Add exhaustive unit tests.

## Phase 4 — Tree-sitter

Implement:

```text
LanguageId
LanguageDefinition
language detection
comment extraction
comment normalization
```

Start with Rust only.

Then add:

```text
TypeScript
JavaScript
Python
Go
Java
```

using fixtures.

## Phase 5 — Filesystem Discovery

Implement recursive scanning using the `ignore` crate.

Feed supported source files into the Tree-sitter pipeline.

## Phase 6 — Evaluation

Implement:

```text
Clock
SystemClock
FixedClock
After evaluator
```

## Phase 7 — Diagnostics

Produce human-readable source diagnostics.

## Phase 8 — CLI

Complete:

```bash
todo-or-die check PATH...
todo-or-die list PATH...
```

Implement documented exit codes.

## Phase 9 — JSON Reporter

Add:

```bash
--format json
```

## Phase 10 — CEL

Create separate:

```text
todo-or-die-cel
```

crate.

Implement:

```text
cel(...)
```

with a deliberately restricted evaluation environment.

CEL should not delay the core `after` feature.

---

# 38. Coding Guidelines

Use idiomatic Rust.

Prefer explicit domain types over stringly typed values.

Prefer:

```rust
enum
```

over combinations of booleans.

Prefer:

```rust
Result<T, DomainError>
```

over panics.

Avoid:

```rust
unwrap()
expect()
```

outside tests unless the invariant is genuinely impossible to violate and clearly documented.

Parse external data at system boundaries.

After parsing, internal functions should operate on validated types.

Keep local mutation where it simplifies implementation, but avoid mutable global state.

Make impossible states difficult or impossible to represent.

---

# 39. Non-Goals for MVP

Do NOT implement:

```text
IDE plugin
VS Code extension
GitHub Action
GitLab component
compiler plugins
ESLint plugin
Clippy integration
automatic source modification
GitHub provider
package registries
distributed caches
daemon mode
LSP
web UI
```

until the CLI/core architecture is solid.

---

# 40. Architectural Rule

The most important architectural rule is:

```text
Tree-sitter understands the host language.

todo-or-die understands todo-or-die.

CEL understands expressions.

Providers understand external systems.
```

Do not mix these responsibilities.

The resulting dependency direction should resemble:

```text
                    CLI
                     │
                     ▼
                 Core Engine
              ┌──────┼───────┐
              ▼      ▼       ▼
        Tree-sitter  CEL   Providers
```

Core domain types should remain independent of CLI concerns.

---

# 41. First Deliverable

Implement the first vertical slice before expanding the architecture.

Target:

```rust
// TODO-OR-DIE: after 2020-01-01
fn legacy() {}
```

Command:

```bash
cargo run -p todo-or-die-cli -- check fixtures/rust
```

Expected behavior:

```text
error: TODO expired

tests/fixtures/rust/expired.rs:1:4

1 │ // TODO-OR-DIE: after 2020-01-01
  │    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
2 │ fn legacy() {}

condition triggered:
  current date is after 2020-01-01
```

Exit:

```text
1
```

Once that vertical slice is working and tested, add the remaining Tree-sitter grammars.

---

# 42. Definition of Done

The implementation is ready for an initial public release when:

* all six initial languages work;
* `.gitignore` is respected;
* false positives inside strings are covered by tests;
* `after` works deterministically;
* malformed directives produce diagnostics;
* `check` and `list` are implemented;
* human and JSON output work;
* CI passes format, Clippy, and tests;
* architecture allows CEL/providers without refactoring the scanner;
* README contains installation and usage examples;
* the binary can be installed with `cargo install`;
* no external services are required for the basic date condition.

The first public version should intentionally be small.

The core value proposition is:

> A language-independent executable specification for temporary code.

A developer should be able to put a TODO in practically any supported language and make CI automatically reject that TODO once the condition that justified it is no longer true.
