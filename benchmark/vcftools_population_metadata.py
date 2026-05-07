#!/usr/bin/env python3
"""Create two VCFtools population files from VCF samples and metadata."""

from __future__ import annotations

import argparse
import gzip
from collections import defaultdict
from pathlib import Path


def open_text(path: Path):
    if path.suffix == ".gz":
        return gzip.open(path, "rt", encoding="utf-8")
    return path.open("rt", encoding="utf-8")


def append_suffix(path: Path, suffix: str) -> Path:
    return Path(f"{path}{suffix}")


def read_samples(vcf: Path) -> list[str]:
    with open_text(vcf) as handle:
        for line in handle:
            if line.startswith("#CHROM"):
                return line.rstrip("\n").split("\t")[9:]
    raise SystemExit(f"{vcf} has no #CHROM header")


def read_metadata(path: Path) -> dict[str, str]:
    labels: dict[str, str] = {}
    if not path.exists():
        return labels
    with path.open("rt", encoding="utf-8") as handle:
        header = handle.readline().rstrip("\n").split("\t")
        try:
            sample_i = header.index("sample")
            pop_i = header.index("population")
        except ValueError as error:
            raise SystemExit("metadata must contain sample and population columns") from error
        for line in handle:
            if not line.strip():
                continue
            fields = line.rstrip("\n").split("\t")
            if len(fields) <= max(sample_i, pop_i):
                continue
            labels[fields[sample_i]] = fields[pop_i]
    return labels


def choose_populations(
    samples: list[str], labels: dict[str, str]
) -> tuple[str, str, list[str], list[str], str]:
    grouped: dict[str, list[str]] = defaultdict(list)
    for sample in samples:
        label = labels.get(sample)
        if label:
            grouped[label].append(sample)
    usable = sorted((label, values) for label, values in grouped.items() if len(values) >= 2)
    if len(usable) >= 2:
        first_label, first_samples = usable[0]
        second_label, second_samples = usable[1]
        return first_label, second_label, first_samples[:2], second_samples[:2], "metadata"
    if len(samples) < 4:
        raise SystemExit("need at least four VCF samples to derive fallback populations")
    return "AUTO_POP1", "AUTO_POP2", samples[:2], samples[2:4], "header-fallback"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--vcf", required=True, type=Path)
    parser.add_argument("--metadata", required=True, type=Path)
    parser.add_argument("--out-prefix", required=True, type=Path)
    args = parser.parse_args()

    samples = read_samples(args.vcf)
    labels = read_metadata(args.metadata)
    pop1_label, pop2_label, pop1, pop2, source = choose_populations(samples, labels)
    args.out_prefix.parent.mkdir(parents=True, exist_ok=True)
    pop1_path = append_suffix(args.out_prefix, ".pop1.txt")
    pop2_path = append_suffix(args.out_prefix, ".pop2.txt")
    meta_path = append_suffix(args.out_prefix, ".population-source.tsv")
    pop1_path.write_text("\n".join(pop1) + "\n", encoding="utf-8")
    pop2_path.write_text("\n".join(pop2) + "\n", encoding="utf-8")
    meta_path.write_text(
        "population_file\tlabel\tsource\tsample_count\n"
        f"{pop1_path}\t{pop1_label}\t{source}\t{len(pop1)}\n"
        f"{pop2_path}\t{pop2_label}\t{source}\t{len(pop2)}\n",
        encoding="utf-8",
    )
    print(f"{pop1_path}\t{pop2_path}\t{meta_path}\t{source}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
