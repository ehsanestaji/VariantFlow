## VCF-Fast Benchmark Report

- Generated: 2026-05-04T18:39:08Z
- Environment: Docker image from this repository on Apple Silicon host
- Mode: `synthetic`
- Dataset source: synthetic generated data
- Dataset sizes: `1000000`
- hyperfine: hyperfine 1.19.0
- bcftools: bcftools 1.21
- Runs: 3 with 1 warmup

### Command Templates

- VCF-Fast filter: `./target/release/vcf-fast filter <input> --where '<expr>' -o <output>`
- bcftools filter: `bcftools filter -i '<expr>' <input> -o <output>`
- VCF-Fast convert TSV: `./target/release/vcf-fast convert <input> --to tsv -o <output.tsv>`
- bcftools query TSV: `bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' <input>`

| case | records | input | Output equivalence | vcf-fast mean | bcftools mean | speedup | vcf-fast variants/s | bcftools variants/s | vcf-fast peak RSS KB | bcftools peak RSS KB | notes |
|---|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| QUAL plain | 1000000 | plain | matches bcftools filtered core records | 0.251569s | 0.449716s | 1.79x | 3975053 | 2223626 | 2672 | 3152 |  |
| DP plain | 1000000 | plain | matches bcftools filtered core records | 0.227815s | 0.415466s | 1.82x | 4389527 | 2406936 | 2676 | 3152 |  |
| AF plain | 1000000 | plain | matches bcftools filtered core records | 0.305543s | 0.493503s | 1.62x | 3272862 | 2026330 | 2676 | 3136 |  |
| QUAL gzip input | 1000000 | gzip | matches bcftools filtered core records | 0.281826s | 0.499683s | 1.77x | 3548289 | 2001269 | 2672 | 3140 |  |
| Convert TSV | 1000000 | plain | matches normalized bcftools query TSV rows | 0.357374s | 0.560407s | 1.57x | 2798189 | 1784417 | 2672 | 2972 |  |

### Interpretation

For deterministic 1M-record synthetic VCF data, VCF-Fast produced the same filtered core records as bcftools for supported QUAL, INFO/DP, INFO/AF, and gzip-input QUAL filter cases. TSV conversion produced the same selected rows as `bcftools query` after normalizing numeric presentation.

This three-run container benchmark supports speedup claims from `1.62x` to `1.82x` for measured filter cases and `1.57x` for TSV conversion. Peak RSS measurements are reported from GNU `/usr/bin/time -v`; they are useful for trend tracking, not a full memory-efficiency claim yet.
