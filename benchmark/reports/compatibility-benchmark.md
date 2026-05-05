# VCF-Fast Compatibility Benchmark

Status: v0.5 compatibility proof is implemented, v0.6 repeated local compatibility benchmarks are measured, and v0.7 optimized typed htslib TSV/stats paths. This is still not a broad speed claim: compatibility is correctness-positive, with mixed performance versus `bcftools` depending on path.

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

## v0.6 Repeated Benchmark Fields

Each repeated compatibility benchmark should report:

- dataset source URL
- dataset size bytes
- record count
- input format
- input compression
- output compression
- exact VCF-Fast command
- exact competitor command
- competitor version
- runtime mean/stddev
- speedup
- variants/sec
- peak RSS
- correctness result
- caveats

## v0.6 Measurement Table

| case | dataset source URL | dataset size bytes | record count | input format | input compression | output compression | exact VCF-Fast command | exact competitor command | competitor version | runtime mean/stddev | speedup | variants/sec | peak RSS | correctness result | caveats |
|---|---|---:|---:|---|---|---|---|---|---|---|---:|---:|---|---|---|
| BCF input filter | local synthetic compatibility dataset | 136,642 | 10,000 | BCF | BGZF | VCF text | `vcf-fast filter compatibility-10000.bcf --where "QUAL > 30" -o fast.vcf` | `bcftools filter -i 'QUAL>30' compatibility-10000.bcf -o bcftools.vcf` | bcftools 1.21 | 0.007559s +/- 0.000239s vs 0.005122s +/- 0.000128s | 0.68x | 1,322,926 / 1,952,362 | 3,040 / 3,152 KB | matched filtered core records | bcftools faster |
| BCF input TSV | local synthetic compatibility dataset | 136,642 | 10,000 | BCF | BGZF | TSV | `vcf-fast convert compatibility-10000.bcf --to tsv -o fast.tsv` | `bcftools query -u -f ... compatibility-10000.bcf > bcftools.tsv` | bcftools 1.21 | 0.016074s +/- 0.000235s vs 0.007140s +/- 0.000664s | 0.44x | 622,123 / 1,400,560 | 3,196 / 2,956 KB | matched normalized TSV rows | bcftools faster |
| Indexed VCF region filter | local synthetic compatibility dataset | 92,191 | 10,000 | VCF | BGZF indexed | VCF text | `vcf-fast filter compatibility-10000.bgzf.vcf.gz --region 22:1-20000000 --where "QUAL > 30" -o fast.vcf` | `bcftools view -r 22:1-20000000 compatibility-10000.bgzf.vcf.gz &#124; bcftools filter -i 'QUAL>30' -Ov -o bcftools.vcf` | bcftools 1.21 | 0.004247s +/- 0.000136s vs 0.003035s +/- 0.000161s | 0.71x | 2,354,603 / 3,294,893 | 3,256 / 3,280 KB | matched region-filtered core records | sub-5ms timing warning |
| Indexed BCF region stats | local synthetic compatibility dataset | 136,642 | 10,000 | BCF | BGZF | JSON/text | `vcf-fast stats compatibility-10000.bcf --region 22:1-20000000 > fast.json` | `bcftools view -r 22:1-20000000 compatibility-10000.bcf &#124; bcftools stats - > bcftools.txt` | bcftools 1.21 | 0.003879s +/- 0.000300s vs 0.002104s +/- 0.000282s | 0.54x | 2,577,984 / 4,752,852 | 3,256 / 3,148 KB | matched overlapping record count | sub-5ms timing warning |
| BGZF output filter | local synthetic compatibility dataset | 442,754 | 10,000 | VCF | none | BGZF | `vcf-fast filter compatibility-10000.vcf --where "QUAL > 30" --compression bgzf -o fast.vcf.gz` | `bcftools filter -i 'QUAL>30' compatibility-10000.vcf -Oz -o bcftools.vcf.gz` | bcftools 1.21 | 0.012770s +/- 0.000339s vs 0.009726s +/- 0.001564s | 0.76x | 783,085 / 1,028,172 | 3,364 / 3,404 KB | matched filtered core records; output indexable | bcftools faster |
| BCF input filter | local synthetic compatibility dataset | 1,248,532 | 100,000 | BCF | BGZF | VCF text | `vcf-fast filter compatibility-100000.bcf --where "QUAL > 30" -o fast.vcf` | `bcftools filter -i 'QUAL>30' compatibility-100000.bcf -o bcftools.vcf` | bcftools 1.21 | 0.054230s +/- 0.008637s vs 0.038115s +/- 0.001821s | 0.70x | 1,843,998 / 2,623,639 | 3,044 / 3,152 KB | matched filtered core records | bcftools faster |
| BCF input TSV | local synthetic compatibility dataset | 1,248,532 | 100,000 | BCF | BGZF | TSV | `vcf-fast convert compatibility-100000.bcf --to tsv -o fast.tsv` | `bcftools query -u -f ... compatibility-100000.bcf > bcftools.tsv` | bcftools 1.21 | 0.125744s +/- 0.005549s vs 0.050044s +/- 0.000454s | 0.40x | 795,267 / 1,998,242 | 3,340 / 2,956 KB | matched normalized TSV rows | bcftools much faster |
| Indexed VCF region filter | local synthetic compatibility dataset | 868,634 | 100,000 | VCF | BGZF indexed | VCF text | `vcf-fast filter compatibility-100000.bgzf.vcf.gz --region 22:1-20000000 --where "QUAL > 30" -o fast.vcf` | `bcftools view -r 22:1-20000000 compatibility-100000.bgzf.vcf.gz &#124; bcftools filter -i 'QUAL>30' -Ov -o bcftools.vcf` | bcftools 1.21 | 0.007102s +/- 0.000833s vs 0.005817s +/- 0.000272s | 0.82x | 14,080,541 / 17,190,992 | 3,412 / 3,292 KB | matched region-filtered core records | bcftools faster |
| Indexed BCF region stats | local synthetic compatibility dataset | 1,248,532 | 100,000 | BCF | BGZF | JSON/text | `vcf-fast stats compatibility-100000.bcf --region 22:1-20000000 > fast.json` | `bcftools view -r 22:1-20000000 compatibility-100000.bcf &#124; bcftools stats - > bcftools.txt` | bcftools 1.21 | 0.008714s +/- 0.000581s vs 0.003138s +/- 0.000205s | 0.36x | 11,475,786 / 31,867,431 | 3,272 / 3,148 KB | matched overlapping record count | bcftools much faster |
| BGZF output filter | local synthetic compatibility dataset | 4,521,517 | 100,000 | VCF | none | BGZF | `vcf-fast filter compatibility-100000.vcf --where "QUAL > 30" --compression bgzf -o fast.vcf.gz` | `bcftools filter -i 'QUAL>30' compatibility-100000.vcf -Oz -o bcftools.vcf.gz` | bcftools 1.21 | 0.085130s +/- 0.004199s vs 0.062655s +/- 0.000568s | 0.74x | 1,174,674 / 1,596,042 | 3,408 / 3,408 KB | matched filtered core records; output indexable | bcftools faster |

## v0.7 Optimization Measurement Table

Command:

```bash
VCF_FAST_BENCH_SIZES="100000 1000000" \
VCF_FAST_BENCH_RUNS=3 \
VCF_FAST_BENCH_WARMUP=1 \
VCF_FAST_COMPAT_REPORT="tests/output/benchmark-results/v07-compatibility-after-optimization.md" \
make bench-compat
```

v0.7 removes htslib TSV/stats full VCF text reconstruction and writes TSV cells directly from typed record fields. Correctness matched for every measured row below.

| case | record count | competitor | correctness result | vcf-fast mean | competitor mean | speedup | current result |
|---|---:|---|---|---:|---:|---:|---|
| BCF input filter | 100,000 | `bcftools filter` | matched filtered core records | `0.021240s` | `0.035391s` | `1.67x` faster | faster, noisy competitor run |
| BCF input TSV | 100,000 | `bcftools query -u` | matched normalized TSV rows | `0.063271s` | `0.034185s` | `0.54x` | improved but still slower |
| Indexed VCF region filter | 100,000 | `bcftools view -r` + `bcftools filter` | matched filtered core records | `0.002633s` | `0.019879s` | `7.55x` faster | sub-5ms timing caveat |
| Indexed BCF region stats | 100,000 | `bcftools view -r` + `bcftools stats` | matched overlapping record count | `0.008420s` | `0.005846s` | `0.69x` | slower on this noisy 100k run |
| BGZF output filter | 100,000 | `bcftools filter -Oz` | matched filtered core records; output indexable | `0.065759s` | `0.066873s` | `1.02x` faster | near parity |
| BCF input filter | 1,000,000 | `bcftools filter` | matched filtered core records | `0.166686s` | `0.174694s` | `1.05x` faster | near parity/slightly faster |
| BCF input TSV | 1,000,000 | `bcftools query -u` | matched normalized TSV rows | `0.419674s` | `0.208067s` | `0.50x` | bcftools remains faster |
| Indexed VCF region filter | 1,000,000 | `bcftools view -r` + `bcftools filter` | matched filtered core records | `0.021999s` | `0.027565s` | `1.25x` faster | faster |
| Indexed BCF region stats | 1,000,000 | `bcftools view -r` + `bcftools stats` | matched overlapping record count | `0.017451s` | `0.018354s` | `1.05x` faster | near parity/slightly faster |
| BGZF output filter | 1,000,000 | `bcftools filter -Oz` | matched filtered core records; output indexable | `0.571114s` | `0.527536s` | `0.92x` | bcftools slightly faster |

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

v0.7 improves compatibility behavior from uniformly slower to mixed: BCF filter, indexed region filter, indexed BCF stats, and BGZF output are now near parity or faster in some measured tiers. BCF TSV remains the clearest compatibility gap: direct typed TSV writing reduced overhead substantially, but `bcftools query` is still about `2.02x` faster at 1M on the synthetic BCF benchmark.
