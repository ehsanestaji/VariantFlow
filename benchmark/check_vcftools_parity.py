#!/usr/bin/env python3
"""Normalize small VariantFlow/VCFtools parity artifacts and compare them."""

from __future__ import annotations

import argparse
import math
from pathlib import Path


def read_tsv(path: Path) -> list[list[str]]:
    return [line.rstrip("\n").split("\t") for line in path.read_text().splitlines()]


def assert_equal(name: str, left: object, right: object) -> None:
    if left != right:
        raise AssertionError(f"{name} mismatch:\nVariantFlow={left!r}\nVCFtools={right!r}")


def assert_float_close(name: str, left: str, right: str, tolerance: float = 1e-5) -> None:
    left_value = float(left)
    right_value = float(right)
    if not math.isclose(left_value, right_value, rel_tol=tolerance, abs_tol=tolerance):
        raise AssertionError(f"{name} mismatch: {left_value} != {right_value}")


def compare_exact_file(name: str, left: Path, right: Path) -> None:
    assert_equal(name, left.read_text(), right.read_text())


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


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("out_dir", type=Path)
    args = parser.parse_args()
    out_dir = args.out_dir

    compare_exact_file("freq", out_dir / "variantflow.frq", out_dir / "vcftools-freq.frq")
    compare_exact_file(
        "site missingness",
        out_dir / "variantflow-missingness.lmiss",
        out_dir / "vcftools-missing-site.lmiss",
    )
    compare_exact_file(
        "individual missingness",
        out_dir / "variantflow-missingness.imiss",
        out_dir / "vcftools-missing-indv.imiss",
    )
    compare_hardy(out_dir)
    compare_het(out_dir)
    print("VCFtools parity checks passed for freq, missingness, hardy, and het.")


if __name__ == "__main__":
    main()
