## VCF-Fast Benchmark Report

- Generated: 2026-05-04T11:34:11Z
- Environment: Docker image from this repository on Apple Silicon host
- Filter expression: `QUAL > 30`
- Dataset sizes: `10000 100000`
- hyperfine: hyperfine 1.19.0
- bcftools: bcftools 1.21

| records | Output equivalence | vcf-fast mean | bcftools mean | speedup | notes |
|---:|---|---:|---:|---:|---|
| 10000 | matches bcftools filtered core records | 0.005676s | 0.007371s | 1.30x |  |
| 100000 | matches bcftools filtered core records | 0.045900s | 0.058328s | 1.27x |  |

### Interpretation

For the supported `QUAL > 30` filter on deterministic synthetic VCF data, VCF-Fast produced the same filtered core records as bcftools and was faster in this Docker benchmark run.

This is an initial benchmark, not a universal performance claim. The next benchmark layer should cover compressed input, INFO filters, larger datasets, and public real-world VCFs.
