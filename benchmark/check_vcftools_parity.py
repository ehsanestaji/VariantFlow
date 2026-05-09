#!/usr/bin/env python3
"""Normalize VariantFlow/VCFtools parity artifacts and compare them."""

from __future__ import annotations

import argparse
import math
import os
import subprocess
import tempfile
from collections.abc import Callable
from pathlib import Path

FLOAT_TOLERANCE = 1e-5
LD_R2_TOLERANCE = 1e-4
NORMALIZER_POLICY = (
    "exact keys and counts; numeric tolerance for floating statistics; "
    "undefined nan rows are ignored only when both tools mark the value undefined"
)


class ParityError(AssertionError):
    pass


def read_tsv(path: Path) -> list[list[str]]:
    return [line.rstrip("\n").split("\t") for line in path.read_text().splitlines()]


def assert_equal(name: str, left: object, right: object) -> None:
    if left != right:
        raise ParityError(f"{name} mismatch:\nVariantFlow={left!r}\nVCFtools={right!r}")


def assert_float_close(
    name: str, left: str, right: str, tolerance: float = FLOAT_TOLERANCE
) -> None:
    left_value = float(left)
    right_value = float(right)
    if not math.isclose(left_value, right_value, rel_tol=tolerance, abs_tol=tolerance):
        raise ParityError(f"{name} mismatch: {left_value} != {right_value}")


def compare_exact_file(name: str, left: Path, right: Path) -> None:
    assert_equal(name, left.read_text(), right.read_text())


def is_nan(value: str) -> bool:
    try:
        return math.isnan(float(value))
    except ValueError:
        return False


def assert_matching_nan_policy(name: str, key: tuple[str, ...], left: str, right: str) -> bool:
    left_is_nan = is_nan(left)
    right_is_nan = is_nan(right)
    if left_is_nan and right_is_nan:
        return True
    if left_is_nan != right_is_nan:
        raise ParityError(
            f"{name} row {key} undefined value mismatch: "
            f"VariantFlow={left!r}; VCFtools={right!r}"
        )
    return False


def split_allele_frequency(value: str) -> tuple[str, str]:
    try:
        allele, frequency = value.rsplit(":", 1)
    except ValueError as error:
        raise ParityError(f"malformed allele frequency value {value!r}") from error
    return allele, frequency


def read_named_tsv(path: Path) -> tuple[list[str], list[dict[str, str]]]:
    rows = read_tsv(path)
    if not rows:
        raise ParityError(f"{path} is empty")
    header = rows[0]
    for row_number, row in enumerate(rows[1:], start=2):
        if len(row) != len(header):
            raise ParityError(
                f"{path} row {row_number} has {len(row)} fields; "
                f"expected {len(header)} from header"
            )
    return header, [dict(zip(header, row)) for row in rows[1:]]


def read_tsv_header(path: Path) -> list[str]:
    with path.open() as handle:
        header = handle.readline()
    if not header:
        raise ParityError(f"{path} is empty")
    return header.rstrip("\n").split("\t")


def iter_named_tsv_pairs(
    left_path: Path, right_path: Path, name: str
) -> tuple[list[str], list[str], object]:
    left_handle = left_path.open()
    right_handle = right_path.open()

    def close_handles() -> None:
        left_handle.close()
        right_handle.close()

    left_header_line = left_handle.readline()
    right_header_line = right_handle.readline()
    if not left_header_line:
        close_handles()
        raise ParityError(f"{left_path} is empty")
    if not right_header_line:
        close_handles()
        raise ParityError(f"{right_path} is empty")

    left_header = left_header_line.rstrip("\n").split("\t")
    right_header = right_header_line.rstrip("\n").split("\t")

    def row_pairs():
        try:
            line_number = 1
            while True:
                line_number += 1
                left_line = left_handle.readline()
                right_line = right_handle.readline()
                if not left_line and not right_line:
                    break
                if not left_line:
                    raise ParityError(f"{name} row count mismatch: VariantFlow ended before VCFtools")
                if not right_line:
                    raise ParityError(f"{name} row count mismatch: VCFtools ended before VariantFlow")

                left_values = left_line.rstrip("\n").split("\t")
                right_values = right_line.rstrip("\n").split("\t")
                if len(left_values) != len(left_header):
                    raise ParityError(
                        f"{left_path} row {line_number} has {len(left_values)} fields; "
                        f"expected {len(left_header)} from header"
                    )
                if len(right_values) != len(right_header):
                    raise ParityError(
                        f"{right_path} row {line_number} has {len(right_values)} fields; "
                        f"expected {len(right_header)} from header"
                    )
                yield line_number, dict(zip(left_header, left_values)), dict(
                    zip(right_header, right_values)
                )
        finally:
            close_handles()

    return left_header, right_header, row_pairs()


def sort_normalized_ld_rows(
    source_path: Path,
    output_path: Path,
    chrom_column: str,
    r2_column: str,
) -> None:
    unsorted_path = output_path.with_suffix(output_path.suffix + ".unsorted")
    with source_path.open() as source_handle, unsorted_path.open("w") as output_handle:
        header_line = source_handle.readline()
        if not header_line:
            raise ParityError(f"{source_path} is empty")
        header = header_line.rstrip("\n").split("\t")
        positions = {
            "CHROM": header.index(chrom_column),
            "POS1": header.index("POS1"),
            "POS2": header.index("POS2"),
            "N_INDV": header.index("N_INDV"),
            "R2": header.index(r2_column),
        }
        for line_number, line in enumerate(source_handle, start=2):
            values = line.rstrip("\n").split("\t")
            if len(values) != len(header):
                raise ParityError(
                    f"{source_path} row {line_number} has {len(values)} fields; "
                    f"expected {len(header)} from header"
                )
            output_handle.write(
                "\t".join(
                    values[positions[column]]
                    for column in ("CHROM", "POS1", "POS2", "N_INDV", "R2")
                )
                + "\n"
            )

    sorted_path = output_path.with_suffix(output_path.suffix + ".sorted")
    env = {**os.environ, "LC_ALL": "C"}
    subprocess.run(
        [
            "sort",
            "-t",
            "\t",
            "-k1,1",
            "-k2,2n",
            "-k3,3n",
            str(unsorted_path),
            "-o",
            str(sorted_path),
        ],
        check=True,
        env=env,
    )
    with output_path.open("w") as final_output, sorted_path.open() as sorted_input:
        final_output.write("CHROM\tPOS1\tPOS2\tN_INDV\tR2\n")
        for line in sorted_input:
            final_output.write(line)
    unsorted_path.unlink(missing_ok=True)
    sorted_path.unlink(missing_ok=True)


def iter_ld_groups(path: Path):
    with path.open() as handle:
        header = handle.readline().rstrip("\n").split("\t")
        if header != ["CHROM", "POS1", "POS2", "N_INDV", "R2"]:
            raise ParityError(f"{path} has unexpected normalized LD header {header!r}")

        current_key: tuple[str, str, str] | None = None
        current_rows: list[tuple[str, str]] = []
        for line_number, line in enumerate(handle, start=2):
            values = line.rstrip("\n").split("\t")
            if len(values) != len(header):
                raise ParityError(
                    f"{path} row {line_number} has {len(values)} fields; "
                    f"expected {len(header)} from header"
                )
            key = (values[0], values[1], values[2])
            row = (values[3], values[4])
            if current_key is None:
                current_key = key
            if key != current_key:
                yield current_key, current_rows
                current_key = key
                current_rows = []
            current_rows.append(row)

        if current_key is not None:
            yield current_key, current_rows


def compare_ld_group(
    key: tuple[str, str, str],
    vf_rows: list[tuple[str, str]],
    vt_rows: list[tuple[str, str]],
) -> None:
    assert_equal(f"ld row group {key} count", len(vf_rows), len(vt_rows))
    vf_sorted = sorted(vf_rows, key=lambda row: (row[0], float(row[1])))
    vt_sorted = sorted(vt_rows, key=lambda row: (row[0], float(row[1])))
    for occurrence, ((vf_n, vf_r2), (vt_n, vt_r2)) in enumerate(
        zip(vf_sorted, vt_sorted), start=1
    ):
        assert_equal(f"ld row group {key} occurrence {occurrence} N_INDV", vf_n, vt_n)
        assert_float_close(
            f"ld row group {key} occurrence {occurrence} R2",
            vf_r2,
            vt_r2,
            tolerance=LD_R2_TOLERANCE,
        )


def index_rows(
    name: str,
    rows: list[dict[str, str]],
    key_columns: tuple[str, ...],
) -> dict[tuple[str, ...], dict[str, str]]:
    indexed: dict[tuple[str, ...], dict[str, str]] = {}
    for row in rows:
        try:
            key = tuple(row[column] for column in key_columns)
        except KeyError as error:
            raise ParityError(f"{name} is missing key column {error.args[0]!r}") from error
        if key in indexed:
            raise ParityError(f"{name} duplicate key {key!r}")
        indexed[key] = row
    return indexed


def index_rows_by_occurrence(
    name: str,
    rows: list[dict[str, str]],
    key_columns: tuple[str, ...],
) -> dict[tuple[str, ...], dict[str, str]]:
    indexed: dict[tuple[str, ...], dict[str, str]] = {}
    counts: dict[tuple[str, ...], int] = {}
    for row in rows:
        try:
            base_key = tuple(row[column] for column in key_columns)
        except KeyError as error:
            raise ParityError(f"{name} is missing key column {error.args[0]!r}") from error
        occurrence = counts.get(base_key, 0)
        counts[base_key] = occurrence + 1
        indexed[base_key + (str(occurrence),)] = row
    return indexed


def assert_matching_keys(
    name: str,
    left: dict[tuple[str, ...], dict[str, str]],
    right: dict[tuple[str, ...], dict[str, str]],
) -> list[tuple[str, ...]]:
    left_keys = set(left)
    right_keys = set(right)
    if left_keys != right_keys:
        missing = sorted(left_keys - right_keys)
        extra = sorted(right_keys - left_keys)
        raise ParityError(
            f"{name} row keys mismatch:\n"
            f"Only in VariantFlow={missing!r}\n"
            f"Only in VCFtools={extra!r}"
        )
    return sorted(left_keys)


def row_value(name: str, row: dict[str, str], column: str, source: str) -> str:
    try:
        return row[column]
    except KeyError as error:
        raise ParityError(f"{name} {source} row is missing column {column!r}") from error


def choose_column(name: str, header: list[str], candidates: tuple[str, ...], source: str) -> str:
    for column in candidates:
        if column in header:
            return column
    raise ParityError(
        f"{name} {source} header is missing one of {candidates!r}; found {header!r}"
    )


def compare_hardy(out_dir: Path) -> None:
    variantflow = read_tsv(out_dir / "variantflow.hwe")
    vcftools = read_tsv(out_dir / "vcftools-hardy.hwe")
    assert_equal("hardy row count", len(variantflow), len(vcftools))

    for index, (vf, vt) in enumerate(zip(variantflow[1:], vcftools[1:]), start=1):
        obs = vt[2].split("/")
        exp = vt[3].split("/")
        assert_equal(f"hardy row {index} chrom", vf[0], vt[0])
        assert_equal(f"hardy row {index} pos", vf[1], vt[1])
        assert_equal(f"hardy row {index} observed counts", vf[2:5], obs)
        for offset, (left, right) in enumerate(zip(vf[5:8], exp), start=1):
            assert_float_close(f"hardy row {index} expected {offset}", left, right, tolerance=5e-3)
        assert_float_close(f"hardy row {index} chisq", vf[8], vt[4], tolerance=1e-5)


def compare_het(out_dir: Path) -> None:
    variantflow = read_tsv(out_dir / "variantflow.het")
    vcftools = read_tsv(out_dir / "vcftools-het.het")
    assert_equal("het row count", len(variantflow), len(vcftools))

    for index, (vf, vt) in enumerate(zip(variantflow[1:], vcftools[1:]), start=1):
        assert_equal(f"het row {index} sample", vf[0], vt[0])
        assert_equal(f"het row {index} observed hom", vf[1], vt[1])
        assert_float_close(f"het row {index} expected hom", vf[2], vt[2], tolerance=5e-2)
        assert_equal(f"het row {index} sites", vf[3], vt[3])
        assert_float_close(f"het row {index} F", vf[4], vt[4], tolerance=1e-5)


def compare_freq(out_dir: Path) -> None:
    variantflow = read_tsv(out_dir / "variantflow.frq")
    vcftools = read_tsv(out_dir / "vcftools-freq.frq")
    assert_equal("freq row count", len(variantflow), len(vcftools))
    assert_equal("freq header", variantflow[0], vcftools[0])

    for index, (vf, vt) in enumerate(zip(variantflow[1:], vcftools[1:]), start=1):
        assert_equal(f"freq row {index} fixed columns", vf[:4], vt[:4])
        assert_equal(f"freq row {index} allele count", len(vf[4:]), len(vt[4:]))
        for allele_index, (left, right) in enumerate(zip(vf[4:], vt[4:]), start=1):
            left_allele, left_freq = split_allele_frequency(left)
            right_allele, right_freq = split_allele_frequency(right)
            assert_equal(
                f"freq row {index} allele {allele_index}",
                left_allele,
                right_allele,
            )
            if left_freq == "." or right_freq == ".":
                assert_equal(f"freq row {index} allele {allele_index} missing", left_freq, right_freq)
            else:
                assert_float_close(
                    f"freq row {index} allele {allele_index} frequency",
                    left_freq,
                    right_freq,
                )


def compare_missingness(
    name: str,
    variantflow_path: Path,
    vcftools_path: Path,
    key_columns: tuple[str, ...],
    allow_duplicate_keys: bool = False,
) -> None:
    vf_header, vf_rows = read_named_tsv(variantflow_path)
    vt_header, vt_rows = read_named_tsv(vcftools_path)
    assert_equal(f"{name} header", vf_header, vt_header)
    indexer = index_rows_by_occurrence if allow_duplicate_keys else index_rows
    vf_index = indexer(f"{name} VariantFlow", vf_rows, key_columns)
    vt_index = indexer(f"{name} VCFtools", vt_rows, key_columns)

    for key in assert_matching_keys(name, vf_index, vt_index):
        for column in vf_header:
            if column in key_columns:
                continue
            if column == "F_MISS":
                assert_float_close(
                    f"{name} row {key} {column}",
                    row_value(name, vf_index[key], column, "VariantFlow"),
                    row_value(name, vt_index[key], column, "VCFtools"),
                )
            else:
                assert_equal(
                    f"{name} row {key} {column}",
                    row_value(name, vf_index[key], column, "VariantFlow"),
                    row_value(name, vt_index[key], column, "VCFtools"),
                )


def compare_site_pi(out_dir: Path) -> None:
    vf_header, vf_rows = read_named_tsv(out_dir / "variantflow.sites.pi")
    vt_header, vt_rows = read_named_tsv(out_dir / "vcftools-pi.sites.pi")
    vf_pi_column = choose_column("site pi", vf_header, ("PI",), "VariantFlow")
    vt_pi_column = choose_column("site pi", vt_header, ("PI",), "VCFtools")
    vf_index = index_rows_by_occurrence("site pi VariantFlow", vf_rows, ("CHROM", "POS"))
    vt_index = index_rows_by_occurrence("site pi VCFtools", vt_rows, ("CHROM", "POS"))

    for key in assert_matching_keys("site pi", vf_index, vt_index):
        assert_float_close(
            f"site pi row {key} PI",
            row_value("site pi", vf_index[key], vf_pi_column, "VariantFlow"),
            row_value("site pi", vt_index[key], vt_pi_column, "VCFtools"),
        )


def compare_window_pi(out_dir: Path) -> None:
    vf_header, vf_rows = read_named_tsv(out_dir / "variantflow.windowed.pi")
    vt_header, vt_rows = read_named_tsv(out_dir / "vcftools-window-pi.windowed.pi")
    vf_pi_column = choose_column(
        "window pi", vf_header, ("PI", "PI_PER_VARIANT"), "VariantFlow"
    )
    vt_pi_column = choose_column("window pi", vt_header, ("PI",), "VCFtools")
    vf_index = index_rows("window pi VariantFlow", vf_rows, ("CHROM", "BIN_START", "BIN_END"))
    vt_index = index_rows("window pi VCFtools", vt_rows, ("CHROM", "BIN_START", "BIN_END"))

    for key in assert_matching_keys("window pi", vf_index, vt_index):
        assert_equal(
            f"window pi row {key} N_VARIANTS",
            row_value("window pi", vf_index[key], "N_VARIANTS", "VariantFlow"),
            row_value("window pi", vt_index[key], "N_VARIANTS", "VCFtools"),
        )
        assert_float_close(
            f"window pi row {key} PI",
            row_value("window pi", vf_index[key], vf_pi_column, "VariantFlow"),
            row_value("window pi", vt_index[key], vt_pi_column, "VCFtools"),
        )


def compare_tajima_d(out_dir: Path) -> None:
    vf_header, vf_rows = read_named_tsv(out_dir / "variantflow.Tajima.D")
    vt_header, vt_rows = read_named_tsv(out_dir / "vcftools-tajima-d.Tajima.D")
    vf_tajima_column = choose_column("tajima-d", vf_header, ("TAJIMA_D", "TajimaD"), "VariantFlow")
    vt_tajima_column = choose_column("tajima-d", vt_header, ("TajimaD", "TAJIMA_D"), "VCFtools")
    vf_index = index_rows("tajima-d VariantFlow", vf_rows, ("CHROM", "BIN_START"))
    vt_index = index_rows("tajima-d VCFtools", vt_rows, ("CHROM", "BIN_START"))

    for key in assert_matching_keys("tajima-d", vf_index, vt_index):
        vf_tajima = row_value("tajima-d", vf_index[key], vf_tajima_column, "VariantFlow")
        vt_tajima = row_value("tajima-d", vt_index[key], vt_tajima_column, "VCFtools")
        if assert_matching_nan_policy("tajima-d", key, vf_tajima, vt_tajima):
            continue
        assert_equal(
            f"tajima-d row {key} N_SNPS",
            row_value("tajima-d", vf_index[key], "N_SNPS", "VariantFlow"),
            row_value("tajima-d", vt_index[key], "N_SNPS", "VCFtools"),
        )
        assert_float_close(
            f"tajima-d row {key} TajimaD",
            vf_tajima,
            vt_tajima,
        )


def compare_ld(out_dir: Path) -> None:
    vf_source = out_dir / "variantflow.geno.ld"
    vt_source = out_dir / "vcftools-ld.geno.ld"
    vf_header = read_tsv_header(vf_source)
    vt_header = read_tsv_header(vt_source)
    vf_chrom_column = choose_column("ld", vf_header, ("CHROM", "CHR"), "VariantFlow")
    vt_chrom_column = choose_column("ld", vt_header, ("CHR", "CHROM"), "VCFtools")
    vf_r2_column = choose_column("ld", vf_header, ("R2", "R^2"), "VariantFlow")
    vt_r2_column = choose_column("ld", vt_header, ("R^2", "R2"), "VCFtools")

    with tempfile.TemporaryDirectory(prefix="variantflow-parity-ld-", dir=out_dir) as temp_dir:
        temp_path = Path(temp_dir)
        vf_sorted = temp_path / "variantflow.geno.ld.normalized.tsv"
        vt_sorted = temp_path / "vcftools-ld.geno.ld.normalized.tsv"
        sort_normalized_ld_rows(vf_source, vf_sorted, vf_chrom_column, vf_r2_column)
        sort_normalized_ld_rows(vt_source, vt_sorted, vt_chrom_column, vt_r2_column)
        vf_groups = iter_ld_groups(vf_sorted)
        vt_groups = iter_ld_groups(vt_sorted)
        while True:
            vf_next = next(vf_groups, None)
            vt_next = next(vt_groups, None)
            if vf_next is None and vt_next is None:
                break
            if vf_next is None:
                raise ParityError(f"ld key {vt_next[0]!r} exists only in VCFtools")
            if vt_next is None:
                raise ParityError(f"ld key {vf_next[0]!r} exists only in VariantFlow")
            vf_key, vf_rows = vf_next
            vt_key, vt_rows = vt_next
            assert_equal("ld key", vf_key, vt_key)
            compare_ld_group(vf_key, vf_rows, vt_rows)


def compare_weir_fst(out_dir: Path) -> None:
    vf_header, vf_rows = read_named_tsv(out_dir / "variantflow.weir.fst")
    vt_header, vt_rows = read_named_tsv(out_dir / "vcftools-weir-fst.weir.fst")
    vf_fst_column = choose_column(
        "weir fst",
        vf_header,
        ("WEIR_AND_COCKERHAM_FST", "WC_FST"),
        "VariantFlow",
    )
    vt_fst_column = choose_column(
        "weir fst", vt_header, ("WEIR_AND_COCKERHAM_FST", "WC_FST"), "VCFtools"
    )
    vf_index = index_rows_by_occurrence("weir fst VariantFlow", vf_rows, ("CHROM", "POS"))
    vt_index = index_rows_by_occurrence("weir fst VCFtools", vt_rows, ("CHROM", "POS"))

    for key in assert_matching_keys("weir fst", vf_index, vt_index):
        vf_fst = row_value("weir fst", vf_index[key], vf_fst_column, "VariantFlow")
        vt_fst = row_value("weir fst", vt_index[key], vt_fst_column, "VCFtools")
        if assert_matching_nan_policy("weir fst", key, vf_fst, vt_fst):
            continue
        assert_float_close(
            f"weir fst row {key} WEIR_AND_COCKERHAM_FST",
            vf_fst,
            vt_fst,
        )


def run_check(name: str, check: Callable[[], None]) -> str | None:
    try:
        check()
    except AssertionError as error:
        return f"[{name}] {error}"
    except Exception as error:
        return f"[{name}] {type(error).__name__}: {error}"
    return None


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("out_dir", type=Path)
    args = parser.parse_args()
    out_dir = args.out_dir

    checks = [
        (
            "freq",
            lambda: compare_freq(out_dir),
        ),
        (
            "site missingness",
            lambda: compare_missingness(
                "site missingness",
                out_dir / "variantflow-missingness.lmiss",
                out_dir / "vcftools-missing-site.lmiss",
                ("CHR", "POS"),
                allow_duplicate_keys=True,
            ),
        ),
        (
            "individual missingness",
            lambda: compare_missingness(
                "individual missingness",
                out_dir / "variantflow-missingness.imiss",
                out_dir / "vcftools-missing-indv.imiss",
                ("INDV",),
            ),
        ),
        ("hardy", lambda: compare_hardy(out_dir)),
        ("het", lambda: compare_het(out_dir)),
        ("site pi", lambda: compare_site_pi(out_dir)),
        ("window pi", lambda: compare_window_pi(out_dir)),
        ("tajima-d", lambda: compare_tajima_d(out_dir)),
        ("ld", lambda: compare_ld(out_dir)),
        ("weir fst", lambda: compare_weir_fst(out_dir)),
    ]
    failures = [failure for name, check in checks if (failure := run_check(name, check))]
    if failures:
        raise SystemExit("VCFtools parity failures:\n\n" + "\n\n".join(failures))
    print(
        "VCFtools parity checks passed for freq, missingness, hardy, het, "
        "pi, Tajima's D, LD, and Weir-Cockerham Fst. "
        f"Normalizer policy: {NORMALIZER_POLICY}."
    )


if __name__ == "__main__":
    main()
