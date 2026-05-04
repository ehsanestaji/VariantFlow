#!/usr/bin/env python3
"""Normalize TSV values used only for benchmark equivalence checks."""

from __future__ import annotations

import argparse
import sys


def normalize_number(value: str) -> str:
    if value in {"", "."}:
        return "."
    try:
        return f"{float(value):.15g}"
    except ValueError:
        return value


def normalize_number_list(value: str) -> str:
    if value in {"", "."}:
        return "."
    return ",".join(normalize_number(part) for part in value.split(","))


def normalize_row(row: str) -> str:
    columns = row.rstrip("\n").split("\t")
    if len(columns) < 9:
        return row.rstrip("\n")

    columns[5] = normalize_number(columns[5])
    columns[7] = normalize_number(columns[7])
    columns[8] = normalize_number_list(columns[8])
    return "\t".join(columns)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Normalize VCF-Fast and bcftools TSV rows for equivalence checks."
    )
    parser.add_argument("path")
    parser.add_argument(
        "--skip-header",
        action="store_true",
        help="skip the first line before normalizing rows",
    )
    args = parser.parse_args()

    with open(args.path, "r", encoding="utf-8") as handle:
        for index, line in enumerate(handle):
            if args.skip_header and index == 0:
                continue
            print(normalize_row(line))

    return 0


if __name__ == "__main__":
    sys.exit(main())
