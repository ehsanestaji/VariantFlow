# VariantFlow Bioinformatics Workflows

VariantFlow is meant to be boring to install and precise to compare. Use it
where selective streaming or export-once analysis is a measured fit, and keep
`bcftools`/HTSlib/GATK in the workflow for operations outside the current claim
matrix.

## Install

From source:

```bash
cargo install --locked --path .
variantflow --version
vcf-fast --version
```

With Docker:

```bash
docker build -t variantflow .
docker run --rm -v "$PWD:/work" variantflow variantflow --version
```

With Bioconda, after the recipe is accepted:

```bash
conda install -c bioconda variantflow
```

The first Bioconda release is planned as the native engine by default. Build
from source with `--features htslib-static` when you need `.bcf`, indexed
`--region`, or explicit BGZF output compatibility paths.

## Common Filters

| Task | VariantFlow | bcftools equivalent |
|---|---|---|
| Site QUAL | `variantflow filter input.vcf.gz --where 'QUAL > 30' -o out.vcf` | `bcftools filter -i 'QUAL>30' input.vcf.gz -o out.vcf` |
| INFO depth | `variantflow filter input.vcf.gz --where 'INFO/DP > 40' -o out.vcf` | `bcftools filter -i 'INFO/DP>40' input.vcf.gz -o out.vcf` |
| AF vector index | `variantflow filter input.vcf.gz --where 'INFO/AF[1] > 0.2' -o out.vcf` | `bcftools filter -i 'INFO/AF[1]>0.2' input.vcf.gz -o out.vcf` |
| Selected sample AD | `variantflow filter input.vcf.gz --sample HG002 --where 'FORMAT/AD[1] > 10' -o out.vcf` | `bcftools filter -s HG002 -i 'FMT/AD[1]>10' input.vcf.gz -o out.vcf` |
| Cohort aggregate | `variantflow filter input.vcf.gz --where 'N_PASS(FORMAT/AD[1] > 10) >= 2' -o out.vcf` | `bcftools filter -i 'N_PASS(FMT/AD[*:1]>10)>=2' input.vcf.gz -o out.vcf` |

## Parquet + DuckDB

Export once when the next step is repeated analysis rather than another
line-preserving VCF operation:

```bash
variantflow convert input.vcf.gz --to parquet -o variants.parquet
duckdb -c "select CHROM, FILTER, count(*) from 'variants.parquet' group by CHROM, FILTER"
```

Current evidence supports VariantFlow plus DuckDB for measured repeated-query
workflows in `benchmark/reports/v12-public-parallel-workflow-benchmark.md`.

## Public benchmark reproduction

```bash
benchmark/download_public_data.sh all
make bench-v14
make bench-v12
```

Use smoke tiers for local checks:

```bash
VCF_FAST_V12_PUBLIC_TIERS=100 VCF_FAST_V12_STRESS_TIERS=100 make bench-v12
```

## Limitations

- Native filtering preserves passing records byte-for-byte; htslib-backed BCF,
  region, and BGZF output paths guarantee valid records, not byte identity.
- BCF TSV remains a tracked compatibility-path optimization gap.
- Public FORMAT-heavy aggregate evidence is still pending.
- GATK, VCFtools, Polars, and PyArrow baselines are optional future evidence
  rows, not default CI requirements.
