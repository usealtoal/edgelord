# Astral-Quality CLI Polish Design

## Overview

Bring edgelord's CLI to Astral-level quality (uv, ruff) with consistent styling, rich error rendering, and polished output.

## Color Vocabulary

| Element | Style | Example |
|---------|-------|---------|
| Action verbs | `bold().cyan()` | `"Checking".bold().cyan()` |
| Success | `bold().green()` | `"✓ Connected"` |
| Warnings | `yellow()` | `"⚠ Low balance"` |
| Errors | `bold().red()` | `"× Failed"` |
| Labels/keys | `dimmed()` | `"Network:"` |
| Values | default or `cyan()` | `"polygon"` |
| Hints | `dimmed()` with prefix | `"hint: run edgelord init"` |
| Progress | Braille spinner | `["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"]` |

## Output Module Enhancements

### New Functions

```rust
// Action verbs (Astral-style)
pub fn action(verb: &str, target: &str)       // "  Checking wallet balance..."
pub fn action_done(verb: &str, target: &str)  // "  ✓ Checked wallet balance"

// Tables
pub fn table_header(columns: &[(&str, usize)])
pub fn table_separator(widths: &[usize])
pub fn table_row(cells: &[&str], widths: &[usize])

// Hints
pub fn hint(message: &str)  // "  hint: run edgelord init"

// Multi-line content
pub fn lines(content: &str)

// Centralized JSON
pub fn json(value: serde_json::Value)
```

### Updated Spinner

Replace default spinner with Astral's braille pattern:
```rust
const BRAILLE: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
```

## Error Rendering

Use miette's full diagnostic rendering instead of `.to_string()`:

```rust
// main.rs
if let Err(e) = result {
    if let Some(report) = e.as_miette_report() {
        eprintln!("{:?}", report);
    } else {
        output::error(&e.to_string());
    }
    std::process::exit(1);
}
```

Output example:
```
  × connection failed: timeout after 30s

  help: check your network connection and exchange status

  ────[edgelord::connection]────
```

## Consistency Fixes

### JSON Output (12 instances)
Replace raw `println!(json!(...))` with `output::json()`:
- `stats/handler.rs` (5 places)
- `strategy.rs` (2 places)
- `check/health.rs` (1 place)
- `check/telegram.rs` (1 place)

### Table Formatting (7 instances)
Replace raw `println!` tables with `output::table_*()`:
- `stats/format.rs` - Strategy and daily breakdown tables
- `strategy.rs` - Strategy list table

### Help Text (8 instances)
Replace raw `println!` help with `output::hint()`:
- `stats/handler.rs:219`
- `strategy.rs:149-151`
- `check/telegram.rs:27,31`
- `wallet/status.rs:33`
- `status.rs:67`

### Acceptable Exceptions (keep as-is)
- Interactive prompts (`print!` for y/N)
- Banner ASCII art
- Raw CSV data output

## Implementation Phases

### Phase 1: Output Module Enhancements
- Add `action()`, `action_done()` functions
- Add `table_header()`, `table_separator()`, `table_row()` functions
- Add `hint()` function
- Add `lines()` function
- Add `json()` centralized JSON emitter
- Update spinner to braille pattern

### Phase 2: Error Rendering
- Update `main.rs` for miette rendering
- Add diagnostic detection helper
- Ensure errors have `#[diagnostic(help = "...")]`

### Phase 3: Command Handler Fixes
- Fix 12 JSON output instances
- Fix 7 table instances
- Fix 8 help text instances
- Fix spacing inconsistencies

### Phase 4: Action Verb Polish
- Update spinners to action verb pattern
- Audit commands for consistent verbs

## Verification

After each phase:
- `cargo build`
- `cargo test`
- `cargo clippy`
- Manual spot-check of output
