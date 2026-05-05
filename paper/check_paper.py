#!/usr/bin/env python3
"""Validate the in-repository JOSS paper scaffold.

This is intentionally lightweight. It does not replace the Open Journals Inara
build, but it catches missing sections, missing references, and accidental
performance claims that are not tied to tracked benchmark reports.
"""

from __future__ import annotations

import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PAPER = ROOT / "paper" / "paper.md"
BIB = ROOT / "paper" / "paper.bib"


def read(path: Path) -> str:
    if not path.exists():
        raise SystemExit(f"missing required paper file: {path.relative_to(ROOT)}")
    return path.read_text(encoding="utf-8")


def word_count(markdown: str) -> int:
    without_yaml = re.sub(r"\A---.*?---", "", markdown, flags=re.DOTALL)
    without_refs = without_yaml.split("\n# References", 1)[0]
    words = re.findall(r"[A-Za-z0-9]+(?:[-'][A-Za-z0-9]+)?", without_refs)
    return len(words)


def require_tokens(label: str, text: str, tokens: tuple[str, ...]) -> None:
    missing = [token for token in tokens if token not in text]
    if missing:
        formatted = ", ".join(repr(token) for token in missing)
        raise SystemExit(f"{label} is missing required token(s): {formatted}")


def main() -> int:
    paper = read(PAPER)
    bib = read(BIB)

    required_sections = (
        "# Summary",
        "# Statement of need",
        "# State of the field",
        "# Software design",
        "# Research impact statement",
        "# AI usage disclosure",
        "# Acknowledgements",
        "# References",
    )
    require_tokens("paper.md", paper, required_sections)
    require_tokens(
        "paper.md",
        paper,
        (
            "benchmark/reports/v14-public-parallel-scale-benchmark.md",
            "benchmark/reports/v12-public-parallel-workflow-benchmark.md",
            "13.44x to 13.47x",
            "1.77x to 2.01x",
            "3.18x to 25.67x",
            "not a claim that VariantFlow replaces bcftools or GATK",
            "AI usage disclosure",
        ),
    )
    require_tokens(
        "paper.bib",
        bib,
        (
            "@article{bcftools",
            "@article{htslib",
            "@article{bioconda",
            "@misc{joss",
            "@misc{apache_arrow",
            "@misc{parquet",
            "@misc{duckdb",
            "@misc{vcf_spec",
        ),
    )

    count = word_count(paper)
    if not 750 <= count <= 1750:
        raise SystemExit(f"paper.md word count {count} is outside JOSS range 750-1750")

    print(f"paper.md word count: {count}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
