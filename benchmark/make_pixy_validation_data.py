#!/usr/bin/env python3
"""Generate a deterministic all-sites VCF with missing genotypes and two
populations, for cross-validating VariantFlow's pixy-equivalent pi/dxy
against pixy itself.

Produces:
  <outdir>/allsites.vcf            (plain, for VariantFlow)
  <outdir>/allsites.vcf.gz(.tbi)   (bgzipped+indexed, for pixy)
  <outdir>/populations.txt         (sample<TAB>pop)

The dataset intentionally mixes invariant sites, variant sites, and a
controlled fraction of missing genotypes, because pixy's central claim is
unbiased estimation *in the presence of missing data*.
"""
import argparse
import random
import subprocess
from pathlib import Path

CHROM = "chr1"
N_SITES = 5000        # every position 1..N_SITES is present (all-sites VCF)
N_PER_POP = 20        # diploid samples per population
POPS = ["popA", "popB"]
MISSING_RATE = 0.15   # fraction of genotypes set to ./.
VARIANT_RATE = 0.45   # fraction of sites that are polymorphic
SEED = 20260614


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--outdir", default="tests/output/pixy-validation")
    args = ap.parse_args()

    outdir = Path(args.outdir)
    outdir.mkdir(parents=True, exist_ok=True)
    rng = random.Random(SEED)

    samples = []
    pop_of = {}
    for pop in POPS:
        for i in range(N_PER_POP):
            name = f"{pop}_{i:02d}"
            samples.append(name)
            pop_of[name] = pop

    # populations file (pixy format: sample <tab> population)
    pop_file = outdir / "populations.txt"
    with open(pop_file, "w") as fh:
        for s in samples:
            fh.write(f"{s}\t{pop_of[s]}\n")

    bases = ["A", "C", "G", "T"]
    vcf_path = outdir / "allsites.vcf"
    with open(vcf_path, "w") as fh:
        fh.write("##fileformat=VCFv4.2\n")
        fh.write(f'##contig=<ID={CHROM},length={N_SITES}>\n')
        fh.write('##FORMAT=<ID=GT,Number=1,Type=String,Description="Genotype">\n')
        header = ["#CHROM", "POS", "ID", "REF", "ALT", "QUAL", "FILTER", "INFO", "FORMAT"]
        header.extend(samples)
        fh.write("\t".join(header) + "\n")

        for pos in range(1, N_SITES + 1):
            ref = rng.choice(bases)
            is_variant = rng.random() < VARIANT_RATE
            if is_variant:
                alt = rng.choice([b for b in bases if b != ref])
                # population-structured allele frequencies so dxy is non-trivial
                pA = rng.uniform(0.05, 0.6)
                pB = rng.uniform(0.05, 0.6)
                alt_field = alt
            else:
                alt = "."
                pA = pB = 0.0
                alt_field = "."

            cols = [CHROM, str(pos), ".", ref, alt_field, ".", "PASS", ".", "GT"]
            for s in samples:
                if rng.random() < MISSING_RATE:
                    cols.append("./.")
                    continue
                p = pA if pop_of[s] == "popA" else pB
                a1 = 1 if (is_variant and rng.random() < p) else 0
                a2 = 1 if (is_variant and rng.random() < p) else 0
                cols.append(f"{a1}/{a2}")
            fh.write("\t".join(cols) + "\n")

    # bgzip + tabix for pixy
    gz = outdir / "allsites.vcf.gz"
    with open(gz, "wb") as out:
        subprocess.run(["bgzip", "-c", str(vcf_path)], check=True, stdout=out)
    subprocess.run(["tabix", "-p", "vcf", str(gz)], check=True)

    print(f"Wrote {vcf_path} ({N_SITES} sites, {len(samples)} samples)")
    print(f"Wrote {gz} (+.tbi)")
    print(f"Wrote {pop_file}")
    print(f"Populations: {POPS} ({N_PER_POP} samples each)")
    print(f"Missing rate: {MISSING_RATE}, variant rate: {VARIANT_RATE}, seed: {SEED}")


if __name__ == "__main__":
    main()
