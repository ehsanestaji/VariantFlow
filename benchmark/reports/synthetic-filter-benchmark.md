## VCF-Fast Benchmark Report

- Generated: 2026-05-04T11:41:47Z
- Environment: Docker image from this repository on Apple Silicon host
- Dataset sizes: `10000 100000`
- hyperfine: hyperfine 1.19.0
- bcftools: bcftools 1.21

| case | records | input | Output equivalence | vcf-fast mean | bcftools mean | speedup | notes |
|---|---:|---|---|---:|---:|---:|---|
| QUAL plain | 10000 | plain | matches bcftools filtered core records | 0.005077s | 0.007052s | 1.39x |  |
| DP plain | 10000 | plain | matches bcftools filtered core records | 0.004380s | 0.006346s | 1.45x |  |
| AF plain | 10000 | plain | matches bcftools filtered core records | 0.010326s | 0.005646s | 0.55x |  |
| QUAL gzip input | 10000 | gzip | matches bcftools filtered core records | 0.003922s | 0.005740s | 1.46x |  |
| QUAL plain | 100000 | plain | matches bcftools filtered core records | 0.024121s | 0.041638s | 1.73x |  |
| DP plain | 100000 | plain | matches bcftools filtered core records | 0.023223s | 0.043561s | 1.88x |  |
| AF plain | 100000 | plain | matches bcftools filtered core records | 0.029306s | 0.047422s | 1.62x |  |
| QUAL gzip input | 100000 | gzip | matches bcftools filtered core records | 0.026289s | 0.045589s | 1.73x |  |

### Interpretation

For deterministic synthetic VCF data, VCF-Fast produced the same filtered core records as bcftools for supported QUAL, INFO/DP, INFO/AF, and gzip-input QUAL filter cases.

At 100k records, this run supports speedup claims from `1.62x` to `1.88x` across measured cases. The 10k AF result was slower and noisy, so the honest claim should focus on larger synthetic datasets until repeated measurements or profiling explain the small-input behavior.

This is still not a universal performance claim. The next benchmark layer should include larger datasets, compressed INFO filters, and public real-world VCFs.
