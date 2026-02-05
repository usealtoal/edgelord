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
    match = re.match(r"^(\d{4}-\d{2}-\d{2})-(.+)\.md$", path.name)
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
    goal_lines = extract_lines(
        text,
        1,
        re.compile(r"^\s*\*\*Goal:\*\*\s*|^\s*Goal:\s*"),
    )
    task_lines = extract_lines(text, 2, re.compile(r"^\s*###\s*"))
    if not task_lines:
        task_lines = extract_lines(text, 2, re.compile(r"^\s*##\s*"))

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
    return "\n".join(lines) + "\n\n"


def already_has_header(text: str) -> bool:
    return "> Status: Historical" in text[:300]


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
        new_text = lines[0] + "\n\n" + header + "\n".join(lines[1:]) + "\n"
        plan.path.write_text(new_text, encoding="utf-8")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
