# Contributing to edgelord

## Setup

[Rust](https://rustup.rs/) is required to build edgelord.

```console
$ git clone https://github.com/usealtoal/edgelord
$ cd edgelord
$ cargo build
```

## Testing

```console
$ cargo test
```

## Code Style

- `cargo fmt` before committing
- `cargo clippy -- -D warnings` must pass
- Doc comments on all public items
- Follow existing patterns in the codebase

## Architecture

Edgelord uses hexagonal architecture. See [ARCHITECTURE.md](ARCHITECTURE.md) for details.

| Layer | Purpose |
|-------|---------|
| `domain/` | Pure types, no I/O |
| `port/` | Inbound and outbound contracts |
| `application/` | Use-case orchestration |
| `adapter/` | CLI, exchange, storage implementations |
| `infrastructure/` | Runtime wiring and bootstrap |

## Commits

Single-line, conventional commit format:

```
<type>(<scope>): <description>
```

**Types:** `feat`, `fix`, `refactor`, `docs`, `test`, `chore`

**Examples:**
```
feat(strategy): add market rebalancing detector
fix(executor): handle partial fill edge case
docs(readme): update installation instructions
```

## Pull Requests

- One concern per PR
- Describe what changed and why
- Add tests for new functionality
- Ensure CI passes

## Reporting Issues

Use [GitHub Issues](https://github.com/usealtoal/edgelord/issues). Include:

- edgelord version (`edgelord --version`)
- OS and architecture
- Steps to reproduce
