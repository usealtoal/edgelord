# Contributing

## Git Conventions

### Commits

Single-line, conventional commit format:

```
<type>(<scope>): <description>
```

**Types:**
- `feat` — New feature
- `fix` — Bug fix
- `refactor` — Code change that neither fixes a bug nor adds a feature
- `docs` — Documentation only
- `test` — Adding or updating tests
- `chore` — Maintenance tasks

**Examples:**
```
feat(detector): add single-condition arbitrage scanner
fix(executor): handle partial fill edge case
refactor(orderbook): simplify cache update logic
docs(readme): update architecture diagram
chore(deps): bump tokio to 1.35
```

### Branches

```
main              # Production-ready
feat/<name>       # Feature branches
fix/<name>        # Bug fix branches
```

---

## Code Style

### Principles

1. **Clarity over cleverness** — Code reads like intent, not a puzzle
2. **One module, one job** — Single responsibility, clear boundaries
3. **Types enforce correctness** — Invalid states should be unrepresentable
4. **No premature abstraction** — Three concrete cases before generalizing
5. **Minimal indirection** — Fewer layers, easier to trace

### Rust Specifics

- Prefer `Result` over `panic!` for recoverable errors
- Use `thiserror` for error types, not strings
- Avoid `.unwrap()` except in tests
- Keep functions short — if it scrolls, split it
- Name things for what they are, not what they do

### File Organization

- One public type per file when possible
- `mod.rs` re-exports only, no logic
- Tests live in `tests/` for integration, inline `#[cfg(test)]` for unit

---

## Documentation

- Doc comments (`///`) on all public items
- Explain *why*, not *what* — the code shows what
- Examples in doc comments for non-obvious APIs
