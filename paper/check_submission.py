#!/usr/bin/env python3
"""Validate the VariantFlow paper-submission control package.

Default mode is strict and exits non-zero while submission-blocking placeholders
remain. Set VCF_FAST_SUBMISSION_ALLOW_BLOCKED=1 to validate the structure of a
deliberately blocked package during development.
"""

from __future__ import annotations

import os
import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "paper" / "submission.yml"
PUBLIC_TABLE = ROOT / "docs" / "public-benchmark-table.md"
MANUSCRIPT = ROOT / "paper" / "bioinformatics-application-note" / "main.tex"
ISSUE_TEMPLATES = (
    ROOT / ".github" / "ISSUE_TEMPLATE" / "paper-author-gate.yml",
    ROOT / ".github" / "ISSUE_TEMPLATE" / "paper-reviewer-gate.yml",
)


def read(path: Path) -> str:
    if not path.exists():
        raise SystemExit(f"missing required submission file: {path.relative_to(ROOT)}")
    return path.read_text(encoding="utf-8")


def require_tokens(label: str, text: str, tokens: tuple[str, ...]) -> list[str]:
    return [f"{label} missing {token!r}" for token in tokens if token not in text]


def manifest_list(manifest: str, section: str) -> list[str]:
    pattern = rf"^{re.escape(section)}:\n((?:  - .+\n)+)"
    match = re.search(pattern, manifest, flags=re.MULTILINE)
    if not match:
        return []
    return [line.strip()[2:] for line in match.group(1).splitlines()]


def todo_tokens(text: str) -> list[str]:
    return sorted(set(re.findall(r"\bTODO[A-Z0-9_]*\b", text)))


def broad_claim_violations(path: Path, text: str) -> list[str]:
    violations: list[str] = []
    patterns = (
        r"\bbest VCF tool\b",
        r"\bbest variant tool\b",
        r"\buniversally replaces\b",
        r"\breplaces bcftools\b",
        r"\breplaces VCFtools\b",
        r"\breplaces GATK\b",
    )
    for pattern in patterns:
        for match in re.finditer(pattern, text, flags=re.IGNORECASE):
            start = max(0, match.start() - 24)
            prefix = text[start : match.start()].lower()
            if "not " in prefix or "no " in prefix:
                continue
            violations.append(f"{path.relative_to(ROOT)} contains broad claim {match.group(0)!r}")
    return violations


def main() -> int:
    allow_blocked = os.environ.get("VCF_FAST_SUBMISSION_ALLOW_BLOCKED") == "1"
    manifest = read(MANIFEST)
    manuscript = read(MANUSCRIPT)
    public_table = read(PUBLIC_TABLE)

    errors: list[str] = []
    blockers: list[str] = []

    errors += require_tokens(
        "paper/submission.yml",
        manifest,
        (
            "target_journal: Bioinformatics",
            "article_type: Application Note",
            "submission_status: blocked",
            "author_gate_status: blocked",
            "reviewer_gate_status: blocked",
            "broad_best_tool_claim_allowed: false",
            "performance_claim_source: tracked_reports_only",
        ),
    )

    submission_docs = manifest_list(manifest, "submission_docs")
    benchmark_reports = manifest_list(manifest, "benchmark_reports")
    if len(submission_docs) < 6:
        errors.append("paper/submission.yml must list all submission docs")
    if len(benchmark_reports) < 5:
        errors.append("paper/submission.yml must list benchmark reports used by the paper")

    for rel in submission_docs + benchmark_reports:
        path = ROOT / rel
        if not path.exists():
            errors.append(f"manifest path does not exist: {rel}")

    for template in ISSUE_TEMPLATES:
        if not template.exists():
            errors.append(f"missing GitHub issue template: {template.relative_to(ROOT)}")

    docs_text = "\n".join(read(ROOT / rel) for rel in submission_docs if (ROOT / rel).exists())
    errors += require_tokens(
        "submission docs",
        docs_text,
        (
            "Author Gate",
            "Reviewer Gate",
            "Bioinformatics",
            "Application Note",
            "AI usage disclosure",
            "Release tag",
            "Archive DOI",
            "No broad \"best VCF tool\" claim",
        ),
    )
    errors += require_tokens(
        "Bioinformatics manuscript",
        manuscript,
        (
            "\\begin{abstract}",
            "Availability and implementation",
            "AI usage disclosure",
            "Funding",
            "Conflict of interest",
            "MIT OR Apache-2.0",
        ),
    )

    paper_claim_numbers = ("17.78", "13.47", "272.73", "9.88", "19.20")
    for number in paper_claim_numbers:
        if number in manuscript and number not in public_table:
            errors.append(
                f"manuscript uses performance value {number}, but docs/public-benchmark-table.md does not"
            )

    scanned_paths = [MANIFEST, MANUSCRIPT, PUBLIC_TABLE]
    scanned_paths += [ROOT / rel for rel in submission_docs if (ROOT / rel).exists()]
    for path in scanned_paths:
        text = read(path)
        errors += broad_claim_violations(path, text)
        todos = todo_tokens(text)
        if todos:
            blockers.append(f"{path.relative_to(ROOT)} contains placeholders: {', '.join(todos)}")

    gate_statuses = re.findall(r"^(author_gate_status|reviewer_gate_status|submission_status):\s*(\w+)", manifest, flags=re.MULTILINE)
    if any(status.lower() in {"passed", "ready", "approved"} for _, status in gate_statuses) and blockers:
        errors.append("submission gates cannot be marked passed while placeholders remain")

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    if blockers:
        print("submission package is structurally valid but blocked:")
        for blocker in blockers:
            print(f"- {blocker}")
        if allow_blocked:
            print("VCF_FAST_SUBMISSION_ALLOW_BLOCKED=1 set; treating blockers as expected.")
            return 0
        print("Set VCF_FAST_SUBMISSION_ALLOW_BLOCKED=1 for structural checks before final metadata is available.")
        return 1

    print("submission package is ready for strict submission review")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
