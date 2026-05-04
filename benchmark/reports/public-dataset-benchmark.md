## Public Dataset Benchmark Report

Status: GIAB HG002 public-small run completed locally on 2026-05-04. 1000 Genomes/IGSR public-region remains pending download/run.

### Pinned Sources

- GIAB HG002 GRCh38 v4.2.1:
  `https://ftp-trace.ncbi.nlm.nih.gov/ReferenceSamples/giab/release/AshkenazimTrio/HG002_NA24385_son/NISTv4.2.1/GRCh38/HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz`
- 1000 Genomes high-coverage chr22:
  `https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000G_2504_high_coverage/working/20220422_3202_phased_SNV_INDEL_SV/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz`

### GIAB HG002 Public-Small Result

- Mode: `public-small`
- Dataset source: GIAB HG002 v4.2.1 first 10000 records
- Input cache: `tests/output/public-data/HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz`
- Input compression: source gzip; harness also creates a plain 10k-record subset and a gzip subset
- hyperfine: hyperfine 1.19.0
- bcftools: bcftools 1.21
- Correctness: all generated `equivalence-*.diff` files were empty

### Command Templates

- VCF-Fast filter: `./target/release/vcf-fast filter <input> --where '<expr>' -o <output>`
- bcftools filter: `bcftools filter -i '<expr>' <input> -o <output>`
- VCF-Fast convert TSV: `./target/release/vcf-fast convert <input> --to tsv -o <output.tsv>`
- bcftools query TSV: `bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' <input>`
- Region slicing for IGSR mode: `bcftools view -H -r "$VCF_FAST_PUBLIC_REGION" <input>`

| case | records | input | Output equivalence | vcf-fast mean | bcftools mean | speedup | notes |
|---|---:|---|---|---:|---:|---:|---|
| QUAL plain | 10000 | plain | matches bcftools filtered core records | 0.062302s | 0.131183s | 2.11x | one-run smoke |
| QUAL gzip input | 10000 | gzip | matches bcftools filtered core records | 0.066260s | 0.137709s | 2.08x | one-run smoke |
| Convert TSV | 10000 | plain | matches normalized bcftools query TSV rows | 0.016483s | 0.018540s | 1.12x | `bcftools query -u` used because GIAB lacks `INFO/AF` |

### How To Reproduce

```bash
benchmark/download_public_data.sh giab-hg002
docker run --rm -v "$PWD:/work" \
  -e VCF_FAST_BENCH_MODE=public-small \
  -e VCF_FAST_BENCH_SIZES="10000" \
  -e VCF_FAST_BENCH_RUNS=1 \
  -e VCF_FAST_BENCH_WARMUP=0 \
  vcf-fast make bench-smoke
```

```bash
benchmark/download_public_data.sh igsr-chr22
VCF_FAST_BENCH_MODE=public-region VCF_FAST_BENCH_SIZES="10000" make bench-smoke
```

### Caveats

This is the first public-data smoke, not a full public benchmark suite. It uses one run, one GIAB 10k-record subset, and only the currently supported filter/convert cases. The 1000 Genomes/IGSR chr22 region benchmark is still pending.
