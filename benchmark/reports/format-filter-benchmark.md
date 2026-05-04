## VCF-Fast FORMAT Filter Benchmark

- Generated: `2026-05-04T20:57:23Z`
- Mode: `stress`
- Dataset source: stress synthetic data
- Dataset shape: `40` INFO fields, `16` samples, `FORMAT=GT:DP:GQ:AD`
- Dataset sizes: `100000 1000000`
- Selected sample: `SAMPLE_001`
- hyperfine: `hyperfine 1.19.0`
- bcftools: `bcftools 1.21`
- Source report: `tests/output/benchmark-results/benchmark-report.md`

### Command Templates

- VCF-Fast selected-sample FORMAT filter: `./target/release/vcf-fast filter <input> --sample SAMPLE_001 --where '<expr>' -o <output>`
- bcftools FORMAT filter: `bcftools filter -i '<expr>' <input> -o <output>`
- Stress benchmark mode: `VCF_FAST_BENCH_MODE=stress VCF_FAST_BENCH_SIZES="100000 1000000" make bench-smoke`

Measured expression pairs:

| case | VCF-Fast expression | bcftools expression |
|---|---|---|
| FORMAT/DP > 20 | `FORMAT/DP > 20` | `FMT/DP[0]>20` |
| FORMAT/GQ >= 30 | `FORMAT/GQ >= 30` | `FMT/GQ[0]>=30` |
| FORMAT/GT == "0/1" | `FORMAT/GT == "0/1"` | `FMT/GT[0]="0/1"` |

### Results

| case | records | correctness check | vcf-fast mean | bcftools mean | speedup | vcf-fast variants/s | bcftools variants/s | vcf-fast peak RSS KB | bcftools peak RSS KB |
|---|---:|---|---:|---:|---:|---:|---:|---:|---:|
| FORMAT/DP > 20 | 100000 | matched bcftools filtered core records | 0.474133s | 0.934324s | 1.97x | 210911 | 107029 | 2868 | 3276 |
| FORMAT/GQ >= 30 | 100000 | matched bcftools filtered core records | 0.427830s | 0.867322s | 2.03x | 233738 | 115297 | 2860 | 3268 |
| FORMAT/GT == "0/1" | 100000 | matched bcftools filtered core records | 0.283870s | 0.572324s | 2.02x | 352274 | 174726 | 2868 | 3272 |
| FORMAT/DP > 20 | 1000000 | matched bcftools filtered core records | 4.754894s | 9.799137s | 2.06x | 210310 | 102050 | 2856 | 3272 |
| FORMAT/GQ >= 30 | 1000000 | matched bcftools filtered core records | 4.365225s | 8.944129s | 2.05x | 229083 | 111805 | 2868 | 3276 |
| FORMAT/GT == "0/1" | 1000000 | matched bcftools filtered core records | 2.870647s | 5.705068s | 1.99x | 348354 | 175283 | 2868 | 3272 |

### Summary

On the measured 1M synthetic stress FORMAT cases, VCF-Fast matched bcftools filtered core records and ran `1.99x` to `2.06x` faster than `bcftools filter` for selected-sample predicates on `FORMAT/DP`, `FORMAT/GQ`, and `FORMAT/GT`.

### Caveats

- FORMAT evidence covers a single selected sample only: `SAMPLE_001`.
- The dataset is a synthetic stress shape, not a public cohort.
- This milestone does not prove multi-sample predicates, ANY/ALL semantics, arbitrary FORMAT keys, BCF, BGZF-output, tabix-compatible output, or indexed region reads.
