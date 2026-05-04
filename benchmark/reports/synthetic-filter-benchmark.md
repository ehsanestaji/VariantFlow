## VCF-Fast Benchmark Report

- Generated: 2026-05-04T14:14:52Z
- Environment: Docker image from this repository on Apple Silicon host
- Mode: `synthetic`
- Dataset source: synthetic generated data
- Dataset sizes: `10000 100000`
- hyperfine: hyperfine 1.19.0
- bcftools: bcftools 1.21

### Command Templates

- VCF-Fast filter: `./target/release/vcf-fast filter <input> --where '<expr>' -o <output>`
- bcftools filter: `bcftools filter -i '<expr>' <input> -o <output>`
- VCF-Fast convert TSV: `./target/release/vcf-fast convert <input> --to tsv -o <output.tsv>`
- bcftools query TSV: `bcftools query -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' <input>`

| case | records | input | Output equivalence | vcf-fast mean | bcftools mean | speedup | notes |
|---|---:|---|---|---:|---:|---:|---|
| QUAL plain | 10000 | plain | matches bcftools filtered core records | 0.005365s | 0.005900s | 1.10x |  |
| DP plain | 10000 | plain | matches bcftools filtered core records | 0.004003s | 0.005883s | 1.47x |  |
| AF plain | 10000 | plain | matches bcftools filtered core records | 0.005879s | 0.007022s | 1.19x |  |
| QUAL gzip input | 10000 | gzip | matches bcftools filtered core records | 0.004501s | 0.007175s | 1.59x |  |
| Convert TSV | 10000 | plain | matches normalized bcftools query TSV rows | 0.006478s | 0.007482s | 1.16x |  |
| QUAL plain | 100000 | plain | matches bcftools filtered core records | 0.027100s | 0.048931s | 1.81x |  |
| DP plain | 100000 | plain | matches bcftools filtered core records | 0.024554s | 0.039862s | 1.62x |  |
| AF plain | 100000 | plain | matches bcftools filtered core records | 0.033278s | 0.049851s | 1.50x |  |
| QUAL gzip input | 100000 | gzip | matches bcftools filtered core records | 0.030057s | 0.049048s | 1.63x |  |
| Convert TSV | 100000 | plain | matches normalized bcftools query TSV rows | 0.043266s | 0.055611s | 1.29x |  |

### Interpretation

For deterministic synthetic VCF data, VCF-Fast produced the same filtered core records as bcftools for supported QUAL, INFO/DP, INFO/AF, and gzip-input QUAL filter cases. The TSV conversion produced the same selected rows as `bcftools query` after normalizing numeric presentation, because VCF-Fast preserves original textual values while bcftools may print equivalent floats without trailing zeroes.

At 100k records, this one-run container smoke supports speedup claims from `1.50x` to `1.81x` for measured filter cases and `1.29x` for TSV conversion. This is still not a broad superiority claim. Public real-world VCF runs, repeated measurements, larger inputs, compressed INFO-heavy filters, and more competitors are tracked as next evidence layers.
