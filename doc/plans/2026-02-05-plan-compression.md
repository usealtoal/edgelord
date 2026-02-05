# Historical Plan Compression Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Prepend a standardized historical-summary header to all pre‑2026‑02‑05 plan files in `doc/plans/`.

**Architecture:** Use a small script to compute deterministic supersession targets and extract summary lines from each plan, then prepend the header block. Keep changes purely additive and idempotent so re-running the script is safe.

**Tech Stack:** Bash, Python 3

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

### Task 2: Add Deterministic Header Prepend Script

**Files:**
- Create: `scripts/compress_plans.py`

**Step 1: Write a small script (no execution yet)**

Create `scripts/compress_plans.py`:
```python
#!/usr/bin/env python3
from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
PLANS_DIR = ROOT / "doc" / "plans"
DATE_CUTOFF = "2026-02-05"


@dataclass(frozen=True)
class PlanMeta:
    path: Path
    date: str
    slug: str


def parse_plan(path: Path) -> PlanMeta | None:
    match = re.match(r"^(\\d{4}-\\d{2}-\\d{2})-(.+)\\.md$", path.name)
    if not match:
        return None
    date, slug = match.group(1), match.group(2)
    return PlanMeta(path=path, date=date, slug=slug)


def superseded_map(plans: list[PlanMeta]) -> dict[Path, str]:
    by_slug: dict[str, list[PlanMeta]] = {}
    for plan in plans:
        by_slug.setdefault(plan.slug, []).append(plan)

    for items in by_slug.values():
        items.sort(key=lambda p: p.date)

    superseded: dict[Path, str] = {}
    for plan in plans:
        superseded_by = "N/A"
        if plan.slug.endswith("-design"):
            impl_slug = plan.slug.replace("-design", "-impl")
            impls = by_slug.get(impl_slug, [])
            if impls:
                superseded_by = impls[-1].path.name
        if superseded_by == "N/A":
            newer = [p for p in by_slug.get(plan.slug, []) if p.date > plan.date]
            if newer:
                superseded_by = newer[-1].path.name
        superseded[plan.path] = superseded_by
    return superseded


def extract_lines(text: str, count: int, pattern: re.Pattern[str]) -> list[str]:
    lines = []
    for line in text.splitlines():
        if pattern.search(line):
            cleaned = pattern.sub("", line).strip()
            if cleaned:
                lines.append(cleaned)
        if len(lines) >= count:
            break
    return lines


def extract_summary(text: str) -> tuple[list[str], list[str]]:
    goal_lines = extract_lines(text, 1, re.compile(r"^\\s*\\*\\*Goal:\\*\\*\\s*|^\\s*Goal:\\s*"))
    task_lines = extract_lines(text, 2, re.compile(r"^\\s*###\\s*"))
    if not task_lines:
        task_lines = extract_lines(text, 2, re.compile(r"^\\s*##\\s*"))

    summary = []
    if goal_lines:
        summary.append(f"Goal: {goal_lines[0]}")
    summary.extend([f"Scope: {t}" for t in task_lines[:1]])
    planned = task_lines[:2] if task_lines else []
    return summary, planned


def header_block(status: str, superseded_by: str, summary: list[str], planned: list[str]) -> str:
    lines = [
        f"> Status: {status}",
        f"> Superseded by: {superseded_by}",
        "> Summary:",
    ]
    if summary:
        lines.extend([f"> - {line}" for line in summary])
    else:
        lines.append("> - Goal: N/A")
    lines.append("> Planned Outcomes:")
    if planned:
        lines.extend([f"> - {line}" for line in planned])
    else:
        lines.append("> - N/A")
    return "\\n".join(lines) + "\\n\\n"


def already_has_header(text: str) -> bool:
    return text.splitlines()[:6].count("> Status: Historical") > 0 or "> Status: Historical" in text[:300]


def main() -> int:
    plans = []
    for path in PLANS_DIR.glob("*.md"):
        meta = parse_plan(path)
        if not meta:
            continue
        if meta.date >= DATE_CUTOFF:
            continue
        plans.append(meta)

    superseded = superseded_map(plans)

    for plan in plans:
        text = plan.path.read_text(encoding="utf-8")
        if already_has_header(text):
            continue
        status = "Historical"
        superseded_by = superseded.get(plan.path, "N/A")
        if superseded_by != "N/A":
            status = "Historical (superseded)"
        summary, planned = extract_summary(text)
        header = header_block(status, superseded_by, summary, planned)

        lines = text.splitlines()
        if not lines:
            continue
        new_text = lines[0] + "\\n\\n" + header + "\\n".join(lines[1:]) + "\\n"
        plan.path.write_text(new_text, encoding="utf-8")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

**Step 2: Commit the script**

```bash
git add scripts/compress_plans.py
git commit -m "chore: add plan compression helper script"
```

---

### Task 3: Run Script and Spot-Check

**Files:**
- Modify: `doc/plans/*.md` (pre‑2026‑02‑05 only)

**Step 1: Run the script**

Run:
```bash
python3 scripts/compress_plans.py
```
Expected: no output, files updated.

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
- The script is idempotent; re-running it won’t duplicate headers.
- “Planned Outcomes” is intentionally non‑committal (historical intent only).
