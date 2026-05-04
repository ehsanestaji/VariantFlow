# VCF-Fast Compatibility Benchmark

Status: v0.5 compatibility proof is implemented; this report is an initial proof record, not a broad speed claim.

## Scope

This milestone adds optional HTSlib interop while keeping the default Rust-native path dependency-light.

Compatibility path is selected for:

- `.bcf` input
- `--region`
- `--compression bgzf`

Native path remains default for plain `.vcf` and `.vcf.gz` streams without region selection or explicit BGZF output.

## Correctness Evidence

| case | command shape | competitor/check | result | caveat |
|---|---|---|---|---|
| BCF input filter | `vcf-fast filter input.bcf --where "QUAL > 30" -o fast.vcf` | expected core records; CI should also compare with `bcftools filter` | matched supported core records in integration test | htslib path reconstructs valid VCF records, not original text |
| BCF input TSV | `vcf-fast convert input.bcf --to tsv -o variants.tsv` | expected TSV rows; benchmark path should compare `bcftools query` | matched expected rows in integration test | selected TSV columns only |
| Indexed VCF region filter | `vcf-fast filter input.vcf.gz --region 1:1-250 --where "QUAL > 30" -o fast.vcf` | tabix-indexed fixture and expected core records | matched expected region-filtered records | region fixture is tiny |
| Indexed BCF region stats | `vcf-fast stats input.bcf --region 1:1-250` | expected overlapping counts | matched variant/SNP/chromosome counts | richer stats parity still pending |
| BGZF output | `vcf-fast filter input.vcf --where "QUAL > 30" --compression bgzf -o output.vcf.gz` | `gzip -t` and `tabix -p vcf` | gzip-readable and tabix-indexable in integration test | performance not measured yet |

## Exact Verification Commands

```bash
cargo test
cargo test --features htslib-static --test compatibility_cli_tests
```

Docker/CI verification target:

```bash
docker run --rm -v "$PWD:/work" vcf-fast bash -lc 'make verify && cargo test --features htslib-static'
```

## Benchmark Fields To Track Next

Each repeated compatibility benchmark should report:

- dataset source
- input format
- output compression
- exact VCF-Fast command
- exact competitor command
- competitor version
- runtime
- speedup
- variants/sec
- peak RSS
- correctness result
- caveats

## Competitor Commands

Filter BCF input:

```bash
vcf-fast filter input.bcf --where "QUAL > 30" -o fast.vcf
bcftools filter -i 'QUAL>30' input.bcf -Ov -o bcftools.vcf
```

Indexed region filter:

```bash
vcf-fast filter input.vcf.gz --region chr22:1-20000000 --where "QUAL > 30" -o fast.vcf
bcftools view -r chr22:1-20000000 input.vcf.gz | bcftools filter -i 'QUAL>30' -Ov -o bcftools.vcf
```

TSV conversion:

```bash
vcf-fast convert input.bcf --to tsv -o fast.tsv
bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' input.bcf > bcftools.tsv
```

BGZF output:

```bash
vcf-fast filter input.vcf --where "QUAL > 30" --compression bgzf -o fast.vcf.gz
tabix -p vcf fast.vcf.gz
bcftools view fast.vcf.gz >/dev/null
```

## Current Caveat

v0.5 proves compatibility behavior and indexability. It does not yet prove that the htslib-backed path is faster than bcftools for BCF, BGZF, or indexed regions. The next evidence milestone should run repeated public compatibility benchmarks before any speed claim is added to the README.
