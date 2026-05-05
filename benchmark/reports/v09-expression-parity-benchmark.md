# VCF-Fast v0.9 Expression Parity Benchmark

## Status

This report tracks correctness and performance for v0.9 expression parity cases. Runtime wins are claimed only for measured rows whose filtered core records match the stated `bcftools filter` baseline. No runtime win is claimed outside the measured rows below. The native scope is arbitrary `INFO/<KEY>`, selected-sample `FORMAT/<KEY>` with `--sample`, and `ANY(FORMAT/<KEY>)` / `ALL(FORMAT/<KEY>)` sample aggregate predicates.

## Run Configuration

- Generated: 2026-05-05T17:07:40Z
- Dataset source: deterministic synthetic stress data from `benchmark/generate_stress_vcf.sh`
- Dataset shape: stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD
- Record tiers: `10000 100000`
- Repeated runs: `3`
- Warmup runs: `1`
- hyperfine: hyperfine 1.20.0
- bcftools: bcftools 1.23.1

## Measured Native Expression Cases

| case | dataset source | dataset size bytes | record count | exact VCF-Fast command | exact competitor command | competitor version | correctness result | runtime mean/stddev | speedup | variants/sec | peak RSS | caveat | claim decision |
| --- | --- | ---: | ---: | --- | --- | --- | --- | --- | ---: | --- | --- | --- | --- |
| Arbitrary INFO numeric | deterministic stress VCF | 8040115 | 10000 | `./target/release/vcf-fast filter tests/output/benchmark-results/v09-expression/data/v09-expression-stress-10000.vcf --where 'INFO/UNUSED7 > 300' -o tests/output/benchmark-results/v09-expression/fast-arbitrary-info-numeric-10000.vcf` | `bcftools filter -i 'INFO/UNUSED7>300' tests/output/benchmark-results/v09-expression/data/v09-expression-stress-10000.vcf -o tests/output/benchmark-results/v09-expression/bcftools-arbitrary-info-numeric-10000.vcf` | bcftools 1.23.1 | matches bcftools filtered core records | 0.011049s +/- 0.000375s vs 0.037854s +/- 0.001343s | 3.43x | 905059 / 264173 | n/a / n/a KB | synthetic stress expression evidence; public v0.9 expression rows still pending | measured faster on this deterministic stress expression case |
| Selected arbitrary FORMAT/AD | deterministic stress VCF | 8040115 | 10000 | `./target/release/vcf-fast filter tests/output/benchmark-results/v09-expression/data/v09-expression-stress-10000.vcf --sample SAMPLE_001 --where 'FORMAT/AD > 30' -o tests/output/benchmark-results/v09-expression/fast-selected-arbitrary-format-ad-10000.vcf` | `bcftools filter -i 'FMT/AD[0:*]>30' tests/output/benchmark-results/v09-expression/data/v09-expression-stress-10000.vcf -o tests/output/benchmark-results/v09-expression/bcftools-selected-arbitrary-format-ad-10000.vcf` | bcftools 1.23.1 | matches bcftools filtered core records | 0.008133s +/- 0.001921s vs 0.031750s +/- 0.000162s | 3.90x | 1229559 / 314961 | n/a / n/a KB | synthetic stress expression evidence; public v0.9 expression rows still pending | measured faster on this deterministic stress expression case |
| ANY sample aggregate FORMAT/AD | deterministic stress VCF | 8040115 | 10000 | `./target/release/vcf-fast filter tests/output/benchmark-results/v09-expression/data/v09-expression-stress-10000.vcf --where 'ANY(FORMAT/AD > 80)' -o tests/output/benchmark-results/v09-expression/fast-any-sample-aggregate-format-ad-10000.vcf` | `bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' tests/output/benchmark-results/v09-expression/data/v09-expression-stress-10000.vcf -o tests/output/benchmark-results/v09-expression/bcftools-any-sample-aggregate-format-ad-10000.vcf` | bcftools 1.23.1 | matches bcftools filtered core records | 0.010874s +/- 0.000238s vs 0.029448s +/- 0.002595s | 2.71x | 919625 / 339582 | n/a / n/a KB | synthetic stress expression evidence; public v0.9 expression rows still pending | measured faster on this deterministic stress expression case |
| ALL sample aggregate FORMAT/DP | deterministic stress VCF | 8040115 | 10000 | `./target/release/vcf-fast filter tests/output/benchmark-results/v09-expression/data/v09-expression-stress-10000.vcf --where 'ALL(FORMAT/DP > 20)' -o tests/output/benchmark-results/v09-expression/fast-all-sample-aggregate-format-dp-10000.vcf` | `bcftools filter -i 'N_PASS(FMT/DP>20)==N_SAMPLES' tests/output/benchmark-results/v09-expression/data/v09-expression-stress-10000.vcf -o tests/output/benchmark-results/v09-expression/bcftools-all-sample-aggregate-format-dp-10000.vcf` | bcftools 1.23.1 | matches bcftools filtered core records | 0.012385s +/- 0.001474s vs 0.032904s +/- 0.000416s | 2.66x | 807428 / 303914 | n/a / n/a KB | synthetic stress expression evidence; public v0.9 expression rows still pending | measured faster on this deterministic stress expression case |
| Arbitrary INFO numeric | deterministic stress VCF | 80473430 | 100000 | `./target/release/vcf-fast filter tests/output/benchmark-results/v09-expression/data/v09-expression-stress-100000.vcf --where 'INFO/UNUSED7 > 300' -o tests/output/benchmark-results/v09-expression/fast-arbitrary-info-numeric-100000.vcf` | `bcftools filter -i 'INFO/UNUSED7>300' tests/output/benchmark-results/v09-expression/data/v09-expression-stress-100000.vcf -o tests/output/benchmark-results/v09-expression/bcftools-arbitrary-info-numeric-100000.vcf` | bcftools 1.23.1 | matches bcftools filtered core records | 0.081003s +/- 0.000672s vs 0.312744s +/- 0.052403s | 3.86x | 1234522 / 319750 | n/a / n/a KB | synthetic stress expression evidence; public v0.9 expression rows still pending | measured faster on this deterministic stress expression case |
| Selected arbitrary FORMAT/AD | deterministic stress VCF | 80473430 | 100000 | `./target/release/vcf-fast filter tests/output/benchmark-results/v09-expression/data/v09-expression-stress-100000.vcf --sample SAMPLE_001 --where 'FORMAT/AD > 30' -o tests/output/benchmark-results/v09-expression/fast-selected-arbitrary-format-ad-100000.vcf` | `bcftools filter -i 'FMT/AD[0:*]>30' tests/output/benchmark-results/v09-expression/data/v09-expression-stress-100000.vcf -o tests/output/benchmark-results/v09-expression/bcftools-selected-arbitrary-format-ad-100000.vcf` | bcftools 1.23.1 | matches bcftools filtered core records | 0.054994s +/- 0.002616s vs 0.284622s +/- 0.025017s | 5.18x | 1818380 / 351343 | n/a / n/a KB | synthetic stress expression evidence; public v0.9 expression rows still pending | measured faster on this deterministic stress expression case |
| ANY sample aggregate FORMAT/AD | deterministic stress VCF | 80473430 | 100000 | `./target/release/vcf-fast filter tests/output/benchmark-results/v09-expression/data/v09-expression-stress-100000.vcf --where 'ANY(FORMAT/AD > 80)' -o tests/output/benchmark-results/v09-expression/fast-any-sample-aggregate-format-ad-100000.vcf` | `bcftools filter -i 'N_PASS(FMT/AD[*:*]>80)>0' tests/output/benchmark-results/v09-expression/data/v09-expression-stress-100000.vcf -o tests/output/benchmark-results/v09-expression/bcftools-any-sample-aggregate-format-ad-100000.vcf` | bcftools 1.23.1 | matches bcftools filtered core records | 0.093491s +/- 0.004457s vs 0.225686s +/- 0.001445s | 2.41x | 1069622 / 443094 | n/a / n/a KB | synthetic stress expression evidence; public v0.9 expression rows still pending | measured faster on this deterministic stress expression case |
| ALL sample aggregate FORMAT/DP | deterministic stress VCF | 80473430 | 100000 | `./target/release/vcf-fast filter tests/output/benchmark-results/v09-expression/data/v09-expression-stress-100000.vcf --where 'ALL(FORMAT/DP > 20)' -o tests/output/benchmark-results/v09-expression/fast-all-sample-aggregate-format-dp-100000.vcf` | `bcftools filter -i 'N_PASS(FMT/DP>20)==N_SAMPLES' tests/output/benchmark-results/v09-expression/data/v09-expression-stress-100000.vcf -o tests/output/benchmark-results/v09-expression/bcftools-all-sample-aggregate-format-dp-100000.vcf` | bcftools 1.23.1 | matches bcftools filtered core records | 0.086029s +/- 0.001044s vs 0.259169s +/- 0.003101s | 3.01x | 1162399 / 385849 | n/a / n/a KB | synthetic stress expression evidence; public v0.9 expression rows still pending | measured faster on this deterministic stress expression case |

## Required Report Fields

- dataset source
- dataset size
- record count
- exact VCF-Fast command
- exact competitor command
- competitor version
- correctness result
- runtime mean and standard deviation
- speedup
- variants per second
- peak RSS
- caveat

## Raw Artifacts

- Working datasets: `tests/output/benchmark-results/v09-expression/data`
- Hyperfine JSON files: `tests/output/benchmark-results/v09-expression/hyperfine-*.json`
- Peak RSS files: `tests/output/benchmark-results/v09-expression/rss-*.txt`
- Equivalence diffs: `tests/output/benchmark-results/v09-expression/equivalence-*.diff`
