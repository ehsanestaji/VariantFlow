#!/usr/bin/env python3
"""Independent cross-check of VF missingness using scikit-allel."""

import json
import sys
from pathlib import Path

import numpy as np
import pandas as pd

try:
    import allel
except ImportError:
    print("ERROR: scikit-allel not installed")
    sys.exit(1)

CHR22 = "tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz"
VF_MISS = "tests/output/benchmark-results/v29-full-genome/chr22-miss-vf.imiss"
OUTDIR = Path("tests/output/correctness-comparison")
OUTDIR.mkdir(parents=True, exist_ok=True)

print(f"scikit-allel version: {allel.__version__}")
print(f"Input: {CHR22}")
print(f"VF reference: {VF_MISS}")
print()

# Read VF output
vf_df = pd.read_csv(VF_MISS, sep="\t")
print(f"VF output: {len(vf_df)} individuals")

# Read VCF with scikit-allel
print("\nReading VCF with scikit-allel (this takes a few minutes for 1M variants x 3202 samples)...")
callset = allel.read_vcf(
    CHR22,
    fields=["samples", "calldata/GT"],
    numbers={"GT": 2},
)

if callset is None:
    print("ERROR: Could not read VCF")
    sys.exit(1)

gt = allel.GenotypeArray(callset["calldata/GT"])
samples = callset["samples"]
n_variants, n_samples, ploidy = gt.shape
print(f"Loaded: {n_variants} variants x {n_samples} samples x {ploidy} ploidy")

# Compute per-individual missingness with scikit-allel
is_missing = gt.is_missing()  # shape: (n_variants, n_samples)
sa_n_miss = is_missing.sum(axis=0)  # count missing per individual
sa_n_data = n_variants  # total sites
sa_fmiss = sa_n_miss / sa_n_data

# Compare with VF
# VF output columns: INDV, N_DATA, N_GENOTYPES_FILTERED, N_MISS, F_MISS
vf_fmiss = vf_df["F_MISS"].values
vf_nmiss = vf_df["N_MISS"].values

# Verify sample count matches
assert n_samples == len(vf_df), f"Sample count mismatch: scikit-allel={n_samples}, VF={len(vf_df)}"

# Compare N_MISS (integer)
nmiss_match = (sa_n_miss == vf_nmiss).all()
nmiss_max_diff = int(np.abs(sa_n_miss - vf_nmiss).max())

# Compare F_MISS (float)
fmiss_max_diff = float(np.abs(sa_fmiss - vf_fmiss).max())
fmiss_mean_diff = float(np.abs(sa_fmiss - vf_fmiss).mean())
correlation = float(np.corrcoef(sa_fmiss, vf_fmiss)[0, 1]) if np.std(vf_fmiss) > 0 else 1.0

print(f"\n=== Results: VF vs scikit-allel per-individual missingness ===")
print(f"  N individuals: {n_samples}")
print(f"  N variants: {n_variants}")
print(f"  N_MISS exact integer match: {nmiss_match}")
print(f"  N_MISS max difference: {nmiss_max_diff}")
print(f"  F_MISS max |diff|: {fmiss_max_diff:.2e}")
print(f"  F_MISS mean |diff|: {fmiss_mean_diff:.2e}")
print(f"  F_MISS correlation: {correlation:.15f}")
print(f"  VERDICT: {'PASS' if fmiss_max_diff < 1e-10 else 'FAIL'}")

results = {
    "tool": "scikit-allel",
    "version": allel.__version__,
    "n_variants": int(n_variants),
    "n_samples": int(n_samples),
    "statistic": "per-individual missingness",
    "nmiss_exact_match": bool(nmiss_match),
    "nmiss_max_diff": nmiss_max_diff,
    "fmiss_max_abs_diff": fmiss_max_diff,
    "fmiss_mean_abs_diff": fmiss_mean_diff,
    "fmiss_correlation": correlation,
    "verdict": "PASS" if fmiss_max_diff < 1e-10 else "FAIL",
}

out_json = OUTDIR / "scikit-allel-crosscheck.json"
with open(out_json, "w") as f:
    json.dump(results, f, indent=2)
print(f"\nResults saved to {out_json}")
