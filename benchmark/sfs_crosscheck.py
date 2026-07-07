#!/usr/bin/env python3
"""Cross-validate VariantFlow's `sfs` subcommand against scikit-allel.

For each biallelic site (REF + exactly one ALT), using only non-missing
genotypes:
  - alt allele count `ac1` = number of alt (index 1) alleles,
  - allele number `an`    = total non-missing alleles at the site.

Unfolded spectrum mirrors `allel.sfs(alt_counts)` (histogram of integer derived
allele counts across sites); folded spectrum mirrors
`allel.sfs_folded(ac_biallelic)`. Sites with `an == 0` are skipped, matching
VariantFlow.

This script:
  1. computes both spectra with scikit-allel,
  2. runs the built `variantflow sfs` binary (both --folded and unfolded),
  3. compares bin-by-bin (exact integer match expected),
  4. writes JSON to tests/output/sfs-validation/sfs-crosscheck.json.
"""
import argparse
import csv
import json
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np

try:
    import allel
except ImportError:
    print("ERROR: scikit-allel not installed")
    sys.exit(1)


def allel_spectra(vcf):
    """Return (unfolded, folded) SFS numpy arrays computed with scikit-allel."""
    callset = allel.read_vcf(
        vcf,
        fields=["calldata/GT", "variants/ALT"],
        numbers={"GT": 2, "ALT": 3},
    )
    if callset is None:
        raise RuntimeError(f"scikit-allel could not read {vcf}")

    gt = allel.GenotypeArray(callset["calldata/GT"])
    ac = gt.count_alleles()  # shape (n_variants, max_alleles)
    alt = callset["variants/ALT"]  # shape (n_variants, max_alt_tokens)

    # VariantFlow's biallelic definition: the ALT column is a single token (no
    # comma) so `allele_labels` yields exactly two labels [REF, ALT]. This is
    # true whenever there is no second ALT allele, i.e. alt[:, 1] is empty. Note
    # a monomorphic site with ALT="." also has a single token and is therefore
    # biallelic under VF, contributing to the ac == 0 bin.
    an = ac.sum(axis=1)
    biallelic = (alt[:, 1] == "") & (an > 0)

    # Alt (derived) allele counts at biallelic sites: allele index 1.
    alt_counts = ac[biallelic, 1]
    # Per-site allele number over non-missing gametes.
    an_bi = an[biallelic]

    unfolded = allel.sfs(alt_counts)

    # Folded: pass the 2-column allele-count subarray so scikit folds using the
    # site-specific allele number.
    ac_bi = ac[biallelic][:, :2]
    folded = allel.sfs_folded(ac_bi)

    return np.asarray(unfolded, dtype=np.int64), np.asarray(folded, dtype=np.int64), an_bi


def read_vf_spectrum(path):
    """Read a VF sfs TSV (ALLELE_COUNT\tN_SITES) into a numpy array indexed by bin."""
    with open(path) as fh:
        reader = csv.DictReader(fh, delimiter="\t")
        rows = [(int(r["ALLELE_COUNT"]), int(r["N_SITES"])) for r in reader]
    rows.sort()
    length = rows[-1][0] + 1 if rows else 0
    spectrum = np.zeros(length, dtype=np.int64)
    for ac, n in rows:
        spectrum[ac] = n
    return spectrum


def run_vf(binary, vcf, folded, out):
    cmd = [binary, "sfs", vcf, "-o", out]
    if folded:
        cmd.append("--folded")
    subprocess.run(cmd, check=True)
    return read_vf_spectrum(out)


def compare(name, allel_spec, vf_spec):
    n = max(len(allel_spec), len(vf_spec))
    a = np.zeros(n, dtype=np.int64)
    b = np.zeros(n, dtype=np.int64)
    a[: len(allel_spec)] = allel_spec
    b[: len(vf_spec)] = vf_spec
    max_abs_diff = int(np.max(np.abs(a - b))) if n else 0
    return {
        "spectrum": name,
        "len_allel": int(len(allel_spec)),
        "len_vf": int(len(vf_spec)),
        "total_sites_allel": int(allel_spec.sum()),
        "total_sites_vf": int(vf_spec.sum()),
        "max_abs_diff": max_abs_diff,
        "exact_match": bool(max_abs_diff == 0 and len(allel_spec) == len(vf_spec)),
    }


def default_input():
    candidates = [
        "tests/output/pixy-validation/allsites.vcf",
        "tests/output/public-data/"
        "1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz",
    ]
    for c in candidates:
        if Path(c).exists():
            return c
    return None


def generate_fixture(path):
    """Deterministic small biallelic VCF with a missing genotype."""
    lines = [
        "##fileformat=VCFv4.3",
        "#CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tS1\tS2\tS3",
        "1\t100\t.\tA\tG\t.\t.\t.\tGT\t0/0\t0/1\t0/0",
        "1\t200\t.\tA\tG\t.\t.\t.\tGT\t0/1\t1/1\t./.",
        "1\t300\t.\tA\tG\t.\t.\t.\tGT\t1/1\t1/1\t1/1",
        "1\t400\t.\tA\tG\t.\t.\t.\tGT\t0/0\t0/0\t0/0",
    ]
    Path(path).write_text("\n".join(lines) + "\n")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--input", default=None, help="VCF to cross-check (default: auto-detect)")
    ap.add_argument("--binary", default="target/debug/variantflow")
    ap.add_argument(
        "--out-json", default="tests/output/sfs-validation/sfs-crosscheck.json"
    )
    args = ap.parse_args()

    tmpdir = tempfile.mkdtemp()
    vcf = args.input or default_input()
    generated = False
    if vcf is None:
        vcf = str(Path(tmpdir) / "sfs_fixture.vcf")
        generate_fixture(vcf)
        generated = True

    if not Path(args.binary).exists():
        print(f"ERROR: binary not found: {args.binary}")
        print("Build it with: cargo build --features htslib-static")
        sys.exit(2)

    print(f"scikit-allel version: {allel.__version__}")
    print(f"Input: {vcf}" + (" (generated fixture)" if generated else ""))

    unfolded_allel, folded_allel, _an = allel_spectra(vcf)

    vf_unfolded = run_vf(args.binary, vcf, False, str(Path(tmpdir) / "unfolded.tsv"))
    vf_folded = run_vf(args.binary, vcf, True, str(Path(tmpdir) / "folded.tsv"))

    unfolded_cmp = compare("unfolded", unfolded_allel, vf_unfolded)
    folded_cmp = compare("folded", folded_allel, vf_folded)

    verdict = "PASS" if unfolded_cmp["exact_match"] and folded_cmp["exact_match"] else "FAIL"
    result = {
        "tool": "scikit-allel",
        "input": vcf,
        "input_generated": generated,
        "unfolded": unfolded_cmp,
        "folded": folded_cmp,
        "verdict": verdict,
    }

    out_json = Path(args.out_json)
    out_json.parent.mkdir(parents=True, exist_ok=True)
    out_json.write_text(json.dumps(result, indent=2))

    print(json.dumps(result, indent=2))
    print(f"\nVERDICT: {verdict}")
    return 0 if verdict == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
