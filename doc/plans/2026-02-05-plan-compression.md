# Historical Plan Compression Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Prepend a standardized historical-summary header to all pre‑2026‑02‑05 plan files in `doc/plans/`.

**Architecture:** Apply a consistent historical-summary header manually, using deterministic supersession rules and simple text extraction (goal + first task headings). Keep changes purely additive so the original plan content remains intact.

**Tech Stack:** Markdown

---

### Task 1: Inventory Eligible Plan Files

**Files:**
- Test: `doc/plans/` (file list)

**Step 1: List plan files before 2026-02-05**

Run:
```bash
ls doc/plans | grep '^2026-02-0[1-4].*\\.md$' | sort
```
Expected: list of plan files dated 2026-02-01 through 2026-02-04 only.

**Step 2: Sanity-check the count**

Run:
```bash
ls doc/plans | grep '^2026-02-0[1-4].*\\.md$' | wc -l
```
Expected: non-zero count.

---

### Task 2: Define Header Template + Superseded Mapping

**Files:**
- Modify: `doc/plans/*.md` (pre‑2026‑02‑05 only)

**Step 1: Use this header template**

```
> Status: Historical
> Superseded by: <filename or N/A>
> Summary:
> - Goal: <from plan Goal line if present>
> - Scope: <first ### Task heading or first ## section>
> Planned Outcomes:
> - <first two ### Task headings or first two ## sections>
```

**Step 2: Apply deterministic superseded rules**

Use these rules to fill `Superseded by`:
- If a `*-design.md` has a matching `*-impl.md`, superseded by that impl file.
- If a newer plan exists with the same slug, superseded by the newest one.
- Otherwise, `N/A`.

---

### Task 3: Apply Headers and Spot-Check

**Files:**
- Modify: `doc/plans/*.md` (pre‑2026‑02‑05 only)

**Step 1: Prepend headers**

For each file listed in Task 1, insert the header template directly after the title line.

**Step 2: Spot-check two files**

Run:
```bash
sed -n '1,12p' doc/plans/2026-02-01-phase-1-foundation.md
sed -n '1,12p' doc/plans/2026-02-04-cli-stats-improvements.md
```
Expected: the new header block appears after the title line.

**Step 3: Commit**

```bash
git add doc/plans/2026-02-0[1-4]*.md
git commit -m "docs: add historical summaries to older plans"
```

---

### Task 4: Final Verification

**Step 1: Check no pre‑2026‑02‑05 plans are missing headers**

Run:
```bash
grep -L \"^> Status: Historical\" doc/plans/2026-02-0[1-4]*.md
```
Expected: no output.

**Step 2: Commit (only if additional fixes were required)**

```bash
git add -A
git commit -m "chore: finalize plan compression"
```

---

## Notes
- Headers should appear once per file; skip if already present.
- “Planned Outcomes” is intentionally non‑committal (historical intent only).
