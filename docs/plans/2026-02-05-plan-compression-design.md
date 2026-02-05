# Historical Plan Compression Design

> **Status:** Proposed
> **Date:** 2026-02-05

## Intent

Compress older plan files by prepending a uniform, short header so readers can understand purpose and supersession at a glance without losing the original content. This keeps historical context while making the plans directory easier to scan.

## Scope

- Target: all files in `docs/plans/` dated before **2026-02-05**
- Action: prepend a standardized “Historical Summary” header
- No file moves, renames, or deletions
- Exclude plans dated 2026-02-05 or later

## Header Format

Place the following block immediately after the document title line:

```
> Status: Historical
> Superseded by: <filename or N/A>
> Summary:
> - Goal: <from plan "Goal:" line if present>
> - Scope: <first 1–2 task/section headings>
> Planned Outcomes:
> - <first 1–2 task/section headings>
```

Notes:
- Use “Historical (superseded)” only when a superseding file is known.
- “Planned Outcomes” is intentionally phrased to avoid implying execution.

## Superseded Mapping Rules

Deterministic, content-light rules to avoid subjective interpretation:

1. If a file ends with `-design.md` and a matching `-impl.md` exists with the same base name (ignoring the suffix), superseded by that `-impl.md`.
2. If a newer plan exists with the same slug (date prefix differs, remainder matches), superseded by the newest one.
3. Otherwise, `N/A`.

## Summary Extraction

Use simple heuristics derived directly from the plan text:

- **Goal**: take the first line starting with `**Goal:**` or `Goal:` if present.
- **Scope/Planned Outcomes**: take the first 1–2 `### Task` headings; if none, use the first 1–2 `##` section headings.

This keeps summaries factual and reproducible.

## Risks / Mitigations

- **Risk:** Incorrect supersession mapping for atypical names  
  **Mitigation:** deterministic rules, allow manual edits if needed.
- **Risk:** Low‑quality summary for plans without structured headers  
  **Mitigation:** fallback to section headings; keep concise.

## Success Criteria

- All pre‑2026‑02‑05 plan files have a consistent summary header.
- No content removed from the original plans.
- Superseded references are deterministic and reproducible.
