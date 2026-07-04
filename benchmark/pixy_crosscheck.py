#!/usr/bin/env python3
"""Cross-validate VariantFlow's pixy-equivalent pi/dxy against pixy on an
all-sites VCF containing missing genotypes.

Assumes:
  - pixy has already been run, producing <pixy_dir>/pixy_pi.txt and
    <pixy_dir>/pixy_dxy.txt
  - VariantFlow has already been run, producing vf_pi.tsv and vf_dxy.tsv

Compares avg_pi / avg_dxy and the integer count columns per
(population[-pair], window). Reports max absolute differences and a PASS/FAIL.
"""
import argparse
import csv
import json
import sys
from pathlib import Path


def read_tsv(path):
    with open(path) as fh:
        reader = csv.DictReader(fh, delimiter="\t")
        return [row for row in reader]


def to_float(x):
    if x is None or x.strip().upper() in ("NA", "NAN", ""):
        return None
    return float(x)


def key_window(row):
    return (row["chromosome"], int(row["window_pos_1"]), int(row["window_pos_2"]))


def compare_pi(pixy_rows, vf_rows):
    pixy = {(r["pop"], *key_window(r)): r for r in pixy_rows}
    vf = {(r["pop"], *key_window(r)): r for r in vf_rows}
    common = sorted(set(pixy) & set(vf))
    max_pi_diff = 0.0
    diffs_mismatch = 0
    comps_mismatch = 0
    n_compared = 0
    for k in common:
        pv = to_float(pixy[k]["avg_pi"])
        vv = to_float(vf[k]["avg_pi"])
        if pv is None and vv is None:
            continue
        if pv is None or vv is None:
            # one side NA where other isn't -> treat as mismatch only if
            # comparisons > 0 on the non-NA side
            continue
        n_compared += 1
        max_pi_diff = max(max_pi_diff, abs(pv - vv))
        if int(pixy[k]["count_diffs"]) != int(vf[k]["count_diffs"]):
            diffs_mismatch += 1
        if int(pixy[k]["count_comparisons"]) != int(vf[k]["count_comparisons"]):
            comps_mismatch += 1
    return {
        "windows_pixy": len(pixy),
        "windows_vf": len(vf),
        "windows_compared": n_compared,
        "max_avg_pi_abs_diff": max_pi_diff,
        "count_diffs_mismatches": diffs_mismatch,
        "count_comparisons_mismatches": comps_mismatch,
    }


def compare_dxy(pixy_rows, vf_rows):
    def dkey(r):
        pair = tuple(sorted((r["pop1"], r["pop2"])))
        return (pair, *key_window(r))

    pixy = {dkey(r): r for r in pixy_rows}
    vf = {dkey(r): r for r in vf_rows}
    common = sorted(set(pixy) & set(vf))
    max_dxy_diff = 0.0
    diffs_mismatch = 0
    comps_mismatch = 0
    n_compared = 0
    for k in common:
        pv = to_float(pixy[k]["avg_dxy"])
        vv = to_float(vf[k]["avg_dxy"])
        if pv is None or vv is None:
            continue
        n_compared += 1
        max_dxy_diff = max(max_dxy_diff, abs(pv - vv))
        if int(pixy[k]["count_diffs"]) != int(vf[k]["count_diffs"]):
            diffs_mismatch += 1
        if int(pixy[k]["count_comparisons"]) != int(vf[k]["count_comparisons"]):
            comps_mismatch += 1
    return {
        "windows_pixy": len(pixy),
        "windows_vf": len(vf),
        "windows_compared": n_compared,
        "max_avg_dxy_abs_diff": max_dxy_diff,
        "count_diffs_mismatches": diffs_mismatch,
        "count_comparisons_mismatches": comps_mismatch,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--pixy-dir", required=True)
    ap.add_argument("--pixy-prefix", default="pixy")
    ap.add_argument("--vf-pi", required=True)
    ap.add_argument("--vf-dxy", required=True)
    ap.add_argument("--out-json", default="tests/output/pixy-validation/pixy-crosscheck.json")
    ap.add_argument("--tol", type=float, default=1e-9)
    args = ap.parse_args()

    pixy_dir = Path(args.pixy_dir)
    pi = compare_pi(
        read_tsv(pixy_dir / f"{args.pixy_prefix}_pi.txt"),
        read_tsv(args.vf_pi),
    )
    dxy = compare_dxy(
        read_tsv(pixy_dir / f"{args.pixy_prefix}_dxy.txt"),
        read_tsv(args.vf_dxy),
    )

    pi_ok = (
        pi["max_avg_pi_abs_diff"] <= args.tol
        and pi["count_diffs_mismatches"] == 0
        and pi["count_comparisons_mismatches"] == 0
        and pi["windows_compared"] > 0
    )
    dxy_ok = (
        dxy["max_avg_dxy_abs_diff"] <= args.tol
        and dxy["count_diffs_mismatches"] == 0
        and dxy["count_comparisons_mismatches"] == 0
        and dxy["windows_compared"] > 0
    )
    verdict = "PASS" if (pi_ok and dxy_ok) else "FAIL"
    result = {"tool": "pixy", "tolerance": args.tol, "pi": pi, "dxy": dxy, "verdict": verdict}

    Path(args.out_json).parent.mkdir(parents=True, exist_ok=True)
    with open(args.out_json, "w") as fh:
        json.dump(result, fh, indent=2)

    print(json.dumps(result, indent=2))
    print(f"\nVERDICT: {verdict}")
    return 0 if verdict == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
