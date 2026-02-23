# Testing

## Quick Start

```console
$ cargo test                              # Unit tests (offline)
$ cargo test --features polymarket        # Include Polymarket tests
$ cargo test --features telegram          # Include Telegram tests
```

## Test Categories

| Category | Features | Requires |
|----------|----------|----------|
| Unit tests | (none) | Nothing |
| Polymarket unit | `polymarket` | Nothing |
| Telegram unit | `telegram` | Nothing |
| LLM integration | `integration-tests` | `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` |
| Polymarket integration | `polymarket-integration` | `POLYMARKET_PRIVATE_KEY` |
| Telegram integration | `telegram-integration` | `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID` |

## Running Integration Tests

Integration tests hit real APIs and require credentials:

```console
$ export ANTHROPIC_API_KEY="sk-..."
$ export OPENAI_API_KEY="sk-..."
$ cargo test --features integration-tests
```

Or run all tests including integration:

```console
$ cargo test --all-features
```

## CI Behavior

- **Every push/PR**: Unit tests only (`polymarket,telegram` features)
- **Release tags**: Full integration tests with API credentials

## Writing Tests

- Place unit tests in `#[cfg(test)] mod tests` within the source file
- Use `#[cfg(feature = "integration-tests")]` for tests requiring real APIs
- Use in-memory SQLite (`:memory:`) for database tests
- Mock external HTTP calls in unit tests

## Coverage by Layer

| Layer | Tests | Coverage |
|-------|-------|----------|
| Domain | ~90 | Core types, invariants |
| Port | ~20 | Trait contracts |
| Application | ~150 | Strategies, solvers |
| Adapter | ~550 | CLI, LLM, Telegram, Polymarket, SQLite |
| Infrastructure | ~300 | Factories, pools, reconnection |

Total: ~1100 tests
