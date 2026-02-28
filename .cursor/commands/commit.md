# Commit changes following project rules

When creating or suggesting git commits, follow these rules. Full details in `AGENTS.md`.

## Message format

```
<type>(<scope>): <subject>

<body>

<footer>
```

- **Type** (required): `feat` | `fix` | `refactor` | `test` | `docs` | `chore` | `style`
- **Scope** (optional): `proxyapi` | `proxyapi_v2` | `proxy_v2_models` | `proxyapi_models` | `tauri-ui` | `document` | `config`
- **Subject** (required): imperative, lowercase start, ≤50 chars, no trailing period
- **Body** (optional): wrap at 72 chars; explain what and why
- **Footer** (optional): breaking changes, issue refs

## Principles

1. Single purpose per commit
2. Split unrelated changes into separate commits
3. Each commit should be independently meaningful
4. Prefer small, logical units

## Pre-commit (required)

**TypeScript/JavaScript**: Run `bun run format`; run `bun run lint`; stage any changed files; then commit.

**Rust**: Run `cargo fmt --all -- --check` (fix with `cargo fmt --all` if needed); run `cargo clippy --all-targets --all-features -- -D warnings` and fix all warnings; then commit.

## Post-commit (required)

After committing: write a summary to an MD file. The file must include:

1. **Title** (e.g. branch name or commit subject)
2. **Work content**: what was done — goals, changes, and outcomes in prose (PR-style). If tests were added or updated, mention that (e.g. "Tests were added for …" or "Test coverage includes …").

**Do not commit this MD file** (add to `.gitignore` or leave unstaged).

## Examples

```
feat(proxy_v2_models): add data type detection for HWP files

- Add HWP 3.0 and 5.0+ file signature detection
- Support multiple HWP MIME types
- Add comprehensive test coverage
```

```
refactor(proxyapi_v2): reorganize TLS handler modules

- Move legacy TLS support to separate module
- Improve code organization for better maintainability
```
