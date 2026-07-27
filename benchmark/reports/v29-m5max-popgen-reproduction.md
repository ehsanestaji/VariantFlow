# VariantFlow v2.9 Pop-Gen Reproduction on Apple M5 Max

This report regenerates the population-genetics benchmarks reported in the
manuscript's Supplementary Table S3 **on the actual host named in Table S2**
(Apple M5 Max), because the previously tabulated S3 pop-gen numbers did not
reproduce on any tracked machine. Every row is correctness-checked (output
parity vs VCFtools) before the timing is reported.

## Configuration

- Host: Apple M5 Max, 14-core CPU, 128 GB unified memory, macOS (arm64)
- VariantFlow: 1.5.0 (`target/release/variantflow`)
- VCFtools: 0.1.17
- bcftools (staging only): 1.23.1
- Date: 2026-07-19
- Timing: wall-clock, `/usr/bin/time -l`; chr22 workflows = min of 2 runs;
  LD = median of 5 isolated runs (VariantFlow) / clean run (VCFtools);
  chr1 missingness = single run.

## Datasets (cached public IGSR 1000 Genomes high-coverage, GRCh38)

- chr22 biallelic SNV subset (`bcftools view -m2 -M2 -v snps`): **925,730 records**, 3,202 samples (the manuscript's "925k" tier).
- chr22 biallelic (`bcftools view -m2 -M2`): full-chromosome missingness (~1.07M records), 3,202 samples.
- chr1 biallelic: full-chromosome missingness (~6.47M records), 3,202 samples.

## Measured Rows (parity-verified)

VariantFlow computes site and individual missingness in a single pass; VCFtools
requires two commands (`--missing-site` + `--missing-indv`), so the VCFtools
missingness time is the sum of both. Fst uses a deterministic 50/50 sample split
(identical for both tools; representative for timing and parity).

| workflow | dataset | VariantFlow (s) | VCFtools (s) | speedup | VF RSS (MB) | VCFtools RSS (MB) | correctness |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| missingness | chr22 full (~1.07M) | 23.75 | 201.67 | 8.49x | 8.6 | 4.1 | matched (site+indv) |
| missingness | chr1 full (~6.47M) | 128.27 | 1031.13 | 8.04x | 8.7 | 4.1 | matched (site+indv) |
| LD r^2 (max-dist 500) | chr22 925k | 75.9 | 444.9 | 5.86x | 9.0 | 4.2 | 15,232,310 pairs byte-identical |
| frequency | chr22 925k | 23.56 | 86.77 | 3.68x | 8.7 | 4.1 | 925,730 records identical |
| heterozygosity | chr22 925k | 96.09 | 140.16 | 1.46x | 9.3 | 4.3 | matched |
| Hardy-Weinberg | chr22 925k | 85.77 | 105.55 | 1.23x | 9.0 | 4.7 | matched |
| site pi | chr22 925k | 86.11 | 92.70 | 1.08x | 9.0 | 4.1 | matched |
| Tajima's D | chr22 925k | 100.15 | 127.05 | 1.27x | 27.1 | 13.8 | matched |
| Weir-Cockerham Fst | chr22 925k | 84.90 | 92.14 | 1.09x | 9.3 | 4.7 | matched (50/50 split) |

## Key findings vs the previous Table S3 / v25 (Docker/Linux)

- **LD is 5.86x on the M5 Max, not 9.88x.** VariantFlow LD is stable at ~75 s on
  both hosts, but VCFtools LD is host-sensitive: ~445 s on the M5 Max vs ~796 s
  on the Docker/Linux box where v25 was measured. The 9.88x figure is a
  Docker/Linux result and does not hold on the M5 Max named in Table S2.
- **Missingness is ~8x** (chr22 8.49x, chr1 8.04x), stronger than the previously
  reported 4.78x/3.67x. VariantFlow peak RSS is constant (~8.6-8.7 MB)
  independent of chromosome size, confirming the streaming-memory claim.
- **Hardy-Weinberg is 1.23x faster on the M5 Max**, not "slightly slower" (that
  was a Docker/Linux, v25, 0.98x observation).
- **site pi (1.08x), Fst (1.09x), heterozygosity (1.46x), Tajima's D (1.27x)** are
  only marginally faster; the previous S3 pi (3.70x) and Fst (1.37x) figures do
  not reproduce. Correctness is unchanged (all outputs match VCFtools).

## Caveat

The previously reported S3 absolute times reproduced on no tracked machine.
These M5 Max numbers are single-host, single-thread wall-clock measurements with
output parity; they supersede the earlier untracked S3 pop-gen values.
Filter/index (v22/v24/v28) and Parquet/DuckDB (v26) workflows are unchanged and
tracked in their own reports.
