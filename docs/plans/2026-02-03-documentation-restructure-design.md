# Documentation Restructure Design

> Status: Historical (superseded)
> Superseded by: 2026-02-03-documentation-restructure-impl.md
> Summary:
> - Scope: Goal
> Planned Outcomes:
> - Goal
> - Design Decisions


> **Date:** 2026-02-03
> **Status:** Approved

## Goal

Restructure documentation to be professional, clean, and comprehensive. Create a polished README with styled header and badges, and organize full documentation in docs/.

## Design Decisions

1. **Tone:** Professional OSS with subtle personality
2. **Plans:** Keep existing docs/plans/ as-is, just note they exist (don't link individually)
3. **Math level:** Applied practitioner — formulas with intuition, worked examples, pseudocode, skip proofs
4. **Format:** Plain markdown files, mermaid for diagrams, no ASCII art
5. **Hosting:** None — GitHub renders markdown

## README.md Structure

```html
<div align="center">
  <img src="asset/banner.png" alt="edgelord" width="100%">

  <p><strong>Multi-strategy arbitrage detection and execution for prediction markets</strong></p>

  <p>
    <a href="..."><img src="..." alt="CI"></a>
    <a href="..."><img src="..." alt="License"></a>
    <img src="..." alt="Rust">
  </p>

  <hr width="60%">
</div>
```

Sections:
- What It Does (brief)
- Quick Start (4 commands max, link to full guide)
- How It Works (2 paragraphs + mermaid diagram, link to architecture)
- Documentation (links to docs/ sections)
- Status (phase checklist)
- License

## docs/ Structure

```
docs/
├── README.md              # Documentation home
├── getting-started.md     # Full setup guide
├── configuration.md       # All config options
├── architecture/
│   └── overview.md        # System design, data flow
├── strategies/
│   ├── overview.md        # Strategy system explanation
│   ├── single-condition.md
│   ├── market-rebalancing.md
│   └── combinatorial.md
├── research/              # (unchanged)
└── plans/                 # (unchanged, not linked)
```

## Strategy Doc Format

Each strategy document follows:

1. **What It Detects** — Plain language explanation
2. **Intuition** — Why this works, the market inefficiency
3. **The Math** — Key formulas with variable definitions
4. **Worked Example** — Real numbers through the pipeline
5. **How It's Used** — Implementation in our system, traits, pipeline integration
6. **Configuration** — TOML snippet with parameter explanations
7. **Limitations** — Honest assessment of when it fails

## Files Changed

| Action | File |
|--------|------|
| Replace | README.md |
| Create | docs/README.md |
| Create | docs/getting-started.md |
| Create | docs/configuration.md |
| Create | docs/architecture/overview.md |
| Create | docs/strategies/overview.md |
| Create | docs/strategies/single-condition.md |
| Create | docs/strategies/market-rebalancing.md |
| Create | docs/strategies/combinatorial.md |
| Delete | docs/architecture/system-design.md (merged into overview.md) |
| Keep | ARCHITECTURE.md (root) |
| Keep | CONTRIBUTING.md |
| Keep | docs/research/* |
| Keep | docs/plans/* |
