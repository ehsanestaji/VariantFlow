## Public Dataset Benchmark Report

Status: planned and script-backed, not yet run in this repository checkout.

### Pinned Sources

- GIAB HG002 GRCh38 v4.2.1:
  `https://ftp-trace.ncbi.nlm.nih.gov/ReferenceSamples/giab/release/AshkenazimTrio/HG002_NA24385_son/NISTv4.2.1/GRCh38/HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz`
- 1000 Genomes high-coverage chr22:
  `https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000G_2504_high_coverage/working/20220422_3202_phased_SNV_INDEL_SV/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz`

### How To Generate

```bash
benchmark/download_public_data.sh giab-hg002
VCF_FAST_BENCH_MODE=public-small VCF_FAST_BENCH_SIZES="10000" make bench-smoke
```

```bash
benchmark/download_public_data.sh igsr-chr22
VCF_FAST_BENCH_MODE=public-region VCF_FAST_BENCH_SIZES="10000" make bench-smoke
```

### Competitor Commands Captured By Harness

- Filtering baseline: `bcftools filter -i '<expr>' <input> -o <output>`
- TSV baseline: `bcftools query -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' <input>`
- Region slicing for IGSR mode: `bcftools view -H -r "$VCF_FAST_PUBLIC_REGION" <input>`

### Required Report Fields

Generated benchmark reports must include dataset source, competitor version, exact VCF-Fast and competitor commands, exact correctness comparison, runtime, speedup, and caveats.

### Caveat

This tracked file intentionally does not claim a public-data performance result yet. It records the pinned sources and reproducible commands; publish measured public results only after the data has been downloaded locally and the generated report has matched `bcftools` outputs.
