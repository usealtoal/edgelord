# Security

## Reporting Vulnerabilities

Report security vulnerabilities to rob@altoal.com. Do not open public issues for security concerns.

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

We will respond within 48 hours and work with you on a fix before public disclosure.

## Threat Model

### In Scope

Edgelord protects against:

- **Accidental secret exposure** — Private keys never logged or written to disk (use [dugout](https://crates.io/crates/dugout) or environment variables)
- **Configuration errors** — `check config` and `check live` validate before trading
- **Risk limit enforcement** — Position limits, exposure caps, and slippage guards
- **Network interruptions** — Automatic reconnection with exponential backoff

### Out of Scope

Edgelord does not protect against:

- **Compromised host machines** — If your machine is compromised, secrets are exposed
- **Stolen private keys** — Protect your wallet keys with proper operational security
- **Exchange vulnerabilities** — We rely on exchange security for order execution
- **Smart contract bugs** — Settlement depends on exchange contracts

## Best Practices

### Secrets Management

Use [dugout](https://crates.io/crates/dugout) for encrypted secrets:

```console
$ dugout set WALLET_PRIVATE_KEY
$ dugout run -- edgelord run --config config.toml
```

Or environment variables (less secure but simpler):

```console
$ export WALLET_PRIVATE_KEY=<key>
$ edgelord run --config config.toml
```

Never:
- Commit private keys to version control
- Store keys in plaintext config files
- Share keys across multiple systems

### Wallet Security

- Use a dedicated trading wallet with limited capital
- Keep long-term holdings in separate cold storage
- Start with conservative risk limits
- Monitor trades closely during initial deployment

### Production Deployment

- Run as non-root user
- Restrict SSH access (key-based only)
- Enable host firewall
- Keep dependencies updated
- Use `dry_run = true` during initial testing
