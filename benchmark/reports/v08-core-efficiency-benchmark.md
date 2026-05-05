# v0.8 Core Efficiency Benchmark

## Status

Stress 1M repeated runs copied from `tests/output/benchmark-results/v08-stress-1m-after-byte-core.md`. Public heavy rows remain pending. This report must only be filled from generated local reports under `tests/output/benchmark-results`.

## Scope

v0.8 refactored the native core around borrowed byte-slice parsing:

- `RecordView` for core VCF columns.
- `InfoView` for cached INFO lookup and comma-separated numeric predicates.
- Byte-backed expression evaluation through `EvalContext`.
- Native filter and stats paths migrated away from line-level `String` parsing.

## Evidence Commands

```bash
make verify
cargo test --features htslib-static
cargo clippy --features htslib-static --all-targets -- -D warnings

VCF_FAST_BENCH_MODE=stress \
VCF_FAST_BENCH_SIZES="1000000" \
VCF_FAST_BENCH_RUNS=3 \
VCF_FAST_BENCH_WARMUP=1 \
VCF_FAST_BENCH_REPORT="tests/output/benchmark-results/v08-stress-1m-after-byte-core.md" \
make bench-smoke

VCF_FAST_BENCH_SIZES="1000000" \
VCF_FAST_BENCH_RUNS=3 \
VCF_FAST_BENCH_WARMUP=1 \
VCF_FAST_BENCH_REPORT="tests/output/benchmark-results/v08-public-heavy-1m-after-byte-core.md" \
make bench-heavy
```

## Required Report Fields

Each measured row copied into this report must include dataset source, dataset shape, record count, dataset size bytes, correctness result, runtime mean, runtime stddev, speedup, variants/sec, peak RSS, exact VCF-Fast command, exact competitor command, competitor version, caveat, and claim decision.

## Measured Results

Stress 1M rows below were generated with `VCF_FAST_BENCH_RUNS=3` and `VCF_FAST_BENCH_WARMUP=1` on 2026-05-05.

| case | dataset source | dataset shape | record count | dataset size bytes | correctness result | runtime mean | runtime stddev | speedup | variants/sec | peak RSS | exact VCF-Fast command | exact competitor command | competitor version | caveat | claim decision |
|---|---|---|---:|---:|---|---:|---:|---:|---|---|---|---|---|---|---|
| QUAL plain | stress synthetic data | stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD | 1000000 | 805816148 | matches bcftools filtered core records | 0.401128s vs 2.339407s | 0.050589s vs 0.047825s | 5.83x | 2492970 / 427459 | n/a / n/a | `./target/release/vcf-fast filter tests/output/benchmark-results/data/stress-1000000.vcf --where 'QUAL > 30' -o tests/output/benchmark-results/fast-qual-plain-1000000.vcf` | `bcftools filter -i 'QUAL>30' tests/output/benchmark-results/data/stress-1000000.vcf -o tests/output/benchmark-results/bcftools-qual-plain-1000000.vcf` | bcftools 1.23.1 | synthetic stress shape | measured win |
| DP plain | stress synthetic data | stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD | 1000000 | 805816148 | matches bcftools filtered core records | 0.581662s vs 2.209620s | 0.031943s vs 0.032148s | 3.80x | 1719212 / 452567 | n/a / n/a | `./target/release/vcf-fast filter tests/output/benchmark-results/data/stress-1000000.vcf --where 'DP > 40' -o tests/output/benchmark-results/fast-dp-plain-1000000.vcf` | `bcftools filter -i 'INFO/DP>40' tests/output/benchmark-results/data/stress-1000000.vcf -o tests/output/benchmark-results/bcftools-dp-plain-1000000.vcf` | bcftools 1.23.1 | synthetic stress shape | measured win |
| AF plain | stress synthetic data | stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD | 1000000 | 805816148 | matches bcftools filtered core records | 0.657395s vs 2.559560s | 0.032090s vs 0.043970s | 3.89x | 1521155 / 390692 | n/a / n/a | `./target/release/vcf-fast filter tests/output/benchmark-results/data/stress-1000000.vcf --where 'AF > 0.2' -o tests/output/benchmark-results/fast-af-plain-1000000.vcf` | `bcftools filter -i 'INFO/AF>0.2' tests/output/benchmark-results/data/stress-1000000.vcf -o tests/output/benchmark-results/bcftools-af-plain-1000000.vcf` | bcftools 1.23.1 | synthetic stress shape | measured win |
| FORMAT/DP > 20 | stress synthetic data | stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD | 1000000 | 805816148 | matches bcftools filtered core records | 0.439446s vs 2.469105s | 0.075616s vs 0.045398s | 5.62x | 2275592 / 405005 | n/a / n/a | `./target/release/vcf-fast filter tests/output/benchmark-results/data/stress-1000000.vcf --sample SAMPLE_001 --where 'FORMAT/DP > 20' -o tests/output/benchmark-results/fast-formatdp--20-1000000.vcf` | `bcftools filter -i 'FMT/DP[0]>20' tests/output/benchmark-results/data/stress-1000000.vcf -o tests/output/benchmark-results/bcftools-formatdp--20-1000000.vcf` | bcftools 1.23.1 | synthetic stress shape | measured win |
| FORMAT/GQ >= 30 | stress synthetic data | stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD | 1000000 | 805816148 | matches bcftools filtered core records | 0.415560s vs 2.372473s | 0.066831s vs 0.056718s | 5.71x | 2406391 / 421501 | n/a / n/a | `./target/release/vcf-fast filter tests/output/benchmark-results/data/stress-1000000.vcf --sample SAMPLE_001 --where 'FORMAT/GQ >= 30' -o tests/output/benchmark-results/fast-formatgq--30-1000000.vcf` | `bcftools filter -i 'FMT/GQ[0]>=30' tests/output/benchmark-results/data/stress-1000000.vcf -o tests/output/benchmark-results/bcftools-formatgq--30-1000000.vcf` | bcftools 1.23.1 | synthetic stress shape | measured win |
| FORMAT/GT == "0/1" | stress synthetic data | stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD | 1000000 | 805816148 | matches bcftools filtered core records | 0.328306s vs 2.049780s | 0.034634s vs 0.012393s | 6.24x | 3045939 / 487857 | n/a / n/a | `./target/release/vcf-fast filter tests/output/benchmark-results/data/stress-1000000.vcf --sample SAMPLE_001 --where 'FORMAT/GT == "0/1"' -o tests/output/benchmark-results/fast-formatgt--01-1000000.vcf` | `bcftools filter -i 'FMT/GT[0]="0/1"' tests/output/benchmark-results/data/stress-1000000.vcf -o tests/output/benchmark-results/bcftools-formatgt--01-1000000.vcf` | bcftools 1.23.1 | synthetic stress shape | measured win |
| QUAL gzip input | stress synthetic data | stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD | 1000000 | 805816148 | matches bcftools filtered core records | 0.798898s vs 2.509340s | 0.010275s vs 0.058440s | 3.14x | 1251724 / 398511 | n/a / n/a | `./target/release/vcf-fast filter tests/output/benchmark-results/data/stress-1000000.vcf.gz --where 'QUAL > 30' -o tests/output/benchmark-results/fast-qual-gzip-input-1000000.vcf` | `bcftools filter -i 'QUAL>30' tests/output/benchmark-results/data/stress-1000000.vcf.gz -o tests/output/benchmark-results/bcftools-qual-gzip-input-1000000.vcf` | bcftools 1.23.1 | synthetic stress shape | measured win |
| Convert TSV | stress synthetic data | stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD | 1000000 | 805816148 | matches normalized bcftools query TSV rows | 0.480159s vs 1.221087s | 0.008063s vs 0.015089s | 2.54x | 2082643 / 818942 | n/a / n/a | `./target/release/vcf-fast convert tests/output/benchmark-results/data/stress-1000000.vcf --to tsv -o tests/output/benchmark-results/fast-convert-tsv-1000000.tsv` | `bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' tests/output/benchmark-results/data/stress-1000000.vcf > tests/output/benchmark-results/bcftools-convert-tsv-1000000.tsv` | bcftools 1.23.1 | synthetic stress shape | measured win |
| Stats JSON | stress synthetic data | stress INFO fields=40, samples=16, FORMAT=GT:DP:GQ:AD | 1000000 | 805816148 | Stats JSON variants match bcftools stats records | 0.451201s vs 1.125941s | 0.002160s vs 0.008828s | 2.50x | 2216307 / 888146 | n/a / n/a | `./target/release/vcf-fast stats tests/output/benchmark-results/data/stress-1000000.vcf > tests/output/benchmark-results/fast-stats-json-1000000.json` | `bcftools stats tests/output/benchmark-results/data/stress-1000000.vcf > tests/output/benchmark-results/bcftools-stats-json-1000000.stats.txt` | bcftools 1.23.1 | synthetic stress shape | measured win |

## Claim Decision Rules

- If correctness matches and VCF-Fast is faster, mark claim decision as `measured win`.
- If correctness matches and VCF-Fast is slower, mark claim decision as `correctness match; optimization needed`.
- If correctness fails, mark claim decision as `no performance claim; correctness target`.
- If a tier fails for environmental reasons, mark claim decision as `deferred with failure reason`.
