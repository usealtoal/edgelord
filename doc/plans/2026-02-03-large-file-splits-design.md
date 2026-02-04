# Large File Splits Design

## Goal

Split the 8 files exceeding the 400-line limit established in ARCHITECTURE.md to improve maintainability and enforce consistent structure.

## Files to Split

| File | Current Lines | Target Max |
|------|---------------|------------|
| `service/subscription/priority.rs` | 753 | 300 |
| `service/governor/latency.rs` | 629 | 300 |
| `app/orchestrator.rs` | 592 | 200 |
| `strategy/rebalancing/mod.rs` | 556 | 200 |
| `service/governor/mod.rs` | 428 | 300 |
| `strategy/condition/single.rs` | 423 | 200 |
| `domain/score.rs` | 416 | 250 |
| `app/status.rs` | 406 | 200 |

## Design Decisions

### Pattern 1: Test Extraction

For files with 200+ lines of tests, extract tests to a dedicated file:
- `{module}/tests.rs` with `#[cfg(test)]` at module level
- Main file becomes cleaner, tests remain colocated

Files using this pattern:
- priority.rs (440 lines of tests)
- latency.rs (400 lines of tests)
- rebalancing/mod.rs (300 lines of tests)
- single.rs (270 lines of tests)
- score.rs (180 lines of tests)
- status.rs (230 lines of tests)

### Pattern 2: Responsibility Separation

For files with multiple distinct responsibilities, split by concern:

**orchestrator.rs** → `app/orchestrator/`
- `mod.rs` - Main App struct and run() method
- `builder.rs` - Registry construction functions
- `handler.rs` - Event handling functions
- `execution.rs` - Trade execution and position recording

**status.rs** → `app/status/`
- `mod.rs` - Re-exports
- `types.rs` - StatusFile, StatusConfig, StatusRuntime, StatusToday
- `writer.rs` - StatusWriter implementation

### Pattern 3: Type Extraction

For domain files with multiple types, extract each to its own file:

**score.rs** → `domain/score/`
- `mod.rs` - Re-exports
- `factors.rs` - ScoreFactors struct
- `weights.rs` - ScoreWeights struct
- `market_score.rs` - MarketScore struct + Ord impl

**governor/mod.rs** - Extract config types:
- `config.rs` - LatencyTargets, ScalingConfig, GovernorConfig
- `mod.rs` - AdaptiveGovernor trait + re-exports

## Target Structure

```
src/
├── app/
│   ├── orchestrator/
│   │   ├── mod.rs         (~180 lines)
│   │   ├── builder.rs     (~110 lines)
│   │   ├── handler.rs     (~150 lines)
│   │   └── execution.rs   (~120 lines)
│   └── status/
│       ├── mod.rs         (~40 lines)
│       ├── types.rs       (~80 lines)
│       ├── writer.rs      (~95 lines)
│       └── tests.rs       (~230 lines)
│
├── core/
│   ├── domain/
│   │   └── score/
│   │       ├── mod.rs         (~50 lines)
│   │       ├── factors.rs     (~70 lines)
│   │       ├── weights.rs     (~50 lines)
│   │       ├── market_score.rs (~90 lines)
│   │       └── tests.rs       (~180 lines)
│   │
│   ├── service/
│   │   ├── governor/
│   │   │   ├── mod.rs         (~200 lines)
│   │   │   ├── config.rs      (~150 lines)
│   │   │   ├── latency.rs     (~170 lines)
│   │   │   └── tests.rs       (~300 lines)
│   │   └── subscription/
│   │       ├── mod.rs         (~180 lines)
│   │       ├── priority.rs    (~180 lines)
│   │       └── tests.rs       (~260 lines)
│   │
│   └── strategy/
│       ├── condition/
│       │   ├── mod.rs         (~50 lines)
│       │   ├── single.rs      (~150 lines)
│       │   └── tests.rs       (~250 lines)
│       └── rebalancing/
│           ├── mod.rs         (~150 lines)
│           ├── strategy.rs    (~80 lines)
│           ├── detect.rs      (~110 lines)
│           └── tests.rs       (~290 lines)
```

## Implementation Order

1. **Phase 1: Test Extractions** (simpler, less risky)
   - Extract tests from priority.rs
   - Extract tests from latency.rs
   - Extract tests from score.rs
   - Extract tests from status.rs
   - Extract tests from single.rs
   - Extract tests from rebalancing/mod.rs

2. **Phase 2: Type Extractions**
   - Split score.rs into score/ module
   - Extract config.rs from governor/mod.rs

3. **Phase 3: Responsibility Splits**
   - Split orchestrator.rs into orchestrator/ module
   - Split status.rs into status/ module (if not done in Phase 1)
   - Split rebalancing/mod.rs further

## Success Criteria

- All files under 400 lines
- All tests pass
- No clippy warnings
- API remains unchanged (re-exports preserve paths)
