#!/usr/bin/env python3
"""Create VariantFlow/VCFtools population files from official IGSR metadata."""

from __future__ import annotations

import argparse
import gzip
import hashlib
from collections import defaultdict
from pathlib import Path

KNOWN_1000G_SUPERPOPULATIONS = ("AFR", "AMR", "EAS", "EUR", "SAS")


def open_text(path: Path):
    if path.suffix == ".gz":
        return gzip.open(path, "rt", encoding="utf-8")
    return path.open("rt", encoding="utf-8")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_vcf_samples(vcf: Path) -> list[str]:
    with open_text(vcf) as handle:
        for line in handle:
            if line.startswith("#CHROM"):
                samples = line.rstrip("\n").split("\t")[9:]
                seen: set[str] = set()
                duplicates: list[str] = []
                for sample in samples:
                    if sample in seen and sample not in duplicates:
                        duplicates.append(sample)
                    seen.add(sample)
                if duplicates:
                    raise SystemExit(f"{vcf} #CHROM header duplicates VCF sample IDs: {', '.join(duplicates)}")
                return samples
    raise SystemExit(f"{vcf} has no #CHROM header")


def normalize_header(name: str) -> str:
    return name.strip().lower().replace(" ", "_").replace("-", "_")


def read_metadata(path: Path) -> dict[str, tuple[str, str]]:
    sample_columns = ("sample", "sample_name", "sample_id", "sample_id_1kg")
    population_columns = ("population", "pop", "population_code")
    superpopulation_columns = ("superpopulation", "super_pop", "super_population", "superpopulation_code")

    def has_required_columns(candidate_header: list[str]) -> bool:
        return (
            any(column in candidate_header for column in sample_columns)
            and any(column in candidate_header for column in population_columns)
            and any(column in candidate_header for column in superpopulation_columns)
        )

    with open_text(path) as handle:
        header: list[str] | None = None
        header_row_number = 0
        for header_row_number, line in enumerate(handle, start=1):
            stripped = line.strip()
            if not stripped or stripped.startswith("##"):
                continue
            if stripped.startswith("#"):
                stripped = stripped[1:]
            candidate_header = [normalize_header(value) for value in stripped.split("\t")]
            if has_required_columns(candidate_header):
                header = candidate_header
                break
        if header is None:
            raise SystemExit(
                f"{path} has no metadata header with required sample, population, and superpopulation columns"
            )

        def choose(candidates: tuple[str, ...]) -> int:
            for candidate in candidates:
                if candidate in header:
                    return header.index(candidate)
            raise SystemExit(f"{path} header must contain one of {candidates}; found {header}")

        sample_i = choose(sample_columns)
        pop_i = choose(population_columns)
        super_i = choose(superpopulation_columns)

        labels: dict[str, tuple[str, str]] = {}
        for row_number, line in enumerate(handle, start=header_row_number + 1):
            if not line.strip():
                continue
            fields = [field.strip() for field in line.rstrip("\n").split("\t")]
            if len(fields) <= max(sample_i, pop_i, super_i):
                raise SystemExit(f"{path} row {row_number} is missing required metadata fields")
            sample = fields[sample_i]
            population = fields[pop_i]
            superpopulation = fields[super_i]
            if not sample or not population or not superpopulation:
                raise SystemExit(f"{path} row {row_number} is missing required metadata fields")
            if sample in labels:
                raise SystemExit(f"{path} row {row_number} duplicates sample {sample!r}")
            labels[sample] = (population, superpopulation)
    return labels


def group_samples(
    vcf_samples: list[str],
    labels: dict[str, tuple[str, str]],
    group_level: str,
) -> tuple[dict[str, list[str]], list[str]]:
    index = 1 if group_level == "superpopulation" else 0
    groups: dict[str, list[str]] = defaultdict(list)
    unmatched: list[str] = []
    for sample in vcf_samples:
        label = labels.get(sample)
        if label is None:
            unmatched.append(sample)
            continue
        groups[label[index]].append(sample)
    return dict(groups), unmatched


def format_unmatched_error(unmatched: list[str]) -> str:
    preview = ", ".join(unmatched[:5])
    if len(unmatched) > 5:
        preview = f"{preview}, ..."
    return (
        f"{len(unmatched)} VCF samples are missing from metadata; "
        f"preview: {preview}. Use --allow-unmatched to exclude them explicitly."
    )


def write_population_files(
    groups: dict[str, list[str]],
    group_pair: str,
    out_prefix: Path,
    min_samples: int,
) -> tuple[Path, Path, str, str]:
    try:
        left_label, right_label = [label.strip() for label in group_pair.split(":", 1)]
    except ValueError as error:
        raise SystemExit("--groups must use LABEL1:LABEL2 syntax, for example AFR:EUR") from error
    left = groups.get(left_label, [])
    right = groups.get(right_label, [])
    if len(left) < min_samples or len(right) < min_samples:
        raise SystemExit(
            f"group pair {group_pair} has too few samples: "
            f"{left_label}={len(left)}, {right_label}={len(right)}, min={min_samples}"
        )
    out_prefix.parent.mkdir(parents=True, exist_ok=True)
    pop1 = Path(f"{out_prefix}.{left_label}.txt")
    pop2 = Path(f"{out_prefix}.{right_label}.txt")
    pop1.write_text("\n".join(left) + "\n", encoding="utf-8")
    pop2.write_text("\n".join(right) + "\n", encoding="utf-8")
    return pop1, pop2, left_label, right_label


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--vcf", required=True, type=Path)
    parser.add_argument("--metadata", required=True, type=Path)
    parser.add_argument("--out-prefix", required=True, type=Path)
    parser.add_argument("--groups", default="AFR:EUR")
    parser.add_argument("--group-level", choices=("population", "superpopulation"), default="superpopulation")
    parser.add_argument("--min-samples", type=int, default=2)
    parser.add_argument("--allow-unmatched", action="store_true")
    args = parser.parse_args()

    if args.min_samples < 1:
        raise SystemExit("--min-samples must be at least 1")

    samples = read_vcf_samples(args.vcf)
    metadata = read_metadata(args.metadata)
    grouped, unmatched = group_samples(samples, metadata, args.group_level)
    if unmatched and not args.allow_unmatched:
        raise SystemExit(format_unmatched_error(unmatched))

    pop1, pop2, left_label, right_label = write_population_files(
        grouped, args.groups, args.out_prefix, args.min_samples
    )
    source = Path(f"{args.out_prefix}.population-source.tsv")
    allow_unmatched = str(args.allow_unmatched).lower()
    vcf_sha256 = sha256_file(args.vcf)
    metadata_sha256 = sha256_file(args.metadata)
    unmatched_sample_ids = ",".join(unmatched) if unmatched else "."
    group_pair = f"{left_label}:{right_label}"
    metadata_policy = "no header-fallback"
    source.write_text(
        "population_file\tlabel\tlevel\tsource\tsample_count\t"
        "vcf_path\tmetadata_path\tgroup_pair\tunmatched_count\tallow_unmatched\t"
        "vcf_sha256\tmetadata_sha256\tunmatched_sample_ids\tmetadata_policy\trecord_type\n"
        f"{pop1}\t{left_label}\t{args.group_level}\tofficial IGSR metadata\t"
        f"{len(grouped[left_label])}\t{args.vcf}\t{args.metadata}\t{group_pair}\t"
        f"{len(unmatched)}\t{allow_unmatched}\t{vcf_sha256}\t{metadata_sha256}\t"
        f"{unmatched_sample_ids}\t{metadata_policy}\tpopulation\n"
        f"{pop2}\t{right_label}\t{args.group_level}\tofficial IGSR metadata\t"
        f"{len(grouped[right_label])}\t{args.vcf}\t{args.metadata}\t{group_pair}\t"
        f"{len(unmatched)}\t{allow_unmatched}\t{vcf_sha256}\t{metadata_sha256}\t"
        f"{unmatched_sample_ids}\t{metadata_policy}\tpopulation\n"
        f"unmatched samples\t.\t.\tofficial IGSR metadata\t{len(unmatched)}\t"
        f"{args.vcf}\t{args.metadata}\t{group_pair}\t{len(unmatched)}\t{allow_unmatched}\t"
        f"{vcf_sha256}\t{metadata_sha256}\t{unmatched_sample_ids}\t{metadata_policy}\tunmatched\n"
        f"settings\t.\t{args.group_level}\tofficial IGSR metadata\t.\t"
        f"{args.vcf}\t{args.metadata}\t{group_pair}\t{len(unmatched)}\t{allow_unmatched}\t"
        f"{vcf_sha256}\t{metadata_sha256}\t{unmatched_sample_ids}\t{metadata_policy}\tsettings\n",
        encoding="utf-8",
    )
    print(f"{pop1}\t{pop2}\t{source}\tofficial IGSR metadata; no header-fallback")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
