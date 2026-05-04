## Public Dataset Benchmark Report

Status: GIAB HG002 public-small and 1000 Genomes/IGSR public-region runs completed locally on 2026-05-04.

### Pinned Sources

- GIAB HG002 GRCh38 v4.2.1:
  `https://ftp-trace.ncbi.nlm.nih.gov/ReferenceSamples/giab/release/AshkenazimTrio/HG002_NA24385_son/NISTv4.2.1/GRCh38/HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz`
- 1000 Genomes high-coverage chr22:
  `https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000G_2504_high_coverage/working/20220422_3202_phased_SNV_INDEL_SV/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz`

### GIAB HG002 Public-Small Result

- Mode: `public-small`
- Dataset source: GIAB HG002 v4.2.1 first 10000 records
- hyperfine: hyperfine 1.19.0
- bcftools: bcftools 1.21
- Correctness: all generated `equivalence-*.diff` files were empty

| case | records | input | Output equivalence | vcf-fast mean | bcftools mean | speedup | notes |
|---|---:|---|---|---:|---:|---:|---|
| QUAL plain | 10000 | plain | matches bcftools filtered core records | 0.062302s | 0.131183s | 2.11x | one-run smoke |
| QUAL gzip input | 10000 | gzip | matches bcftools filtered core records | 0.066260s | 0.137709s | 2.08x | one-run smoke |
| Convert TSV | 10000 | plain | matches normalized bcftools query TSV rows | 0.016483s | 0.018540s | 1.12x | `bcftools query -u` used because GIAB lacks `INFO/AF` |

### 1000 Genomes/IGSR Public-Region Result

- Mode: `public-region`
- Dataset source: 1000 Genomes high-coverage chr22 region `chr22:1-20000000`
- Dataset sizes: `10000 100000`
- hyperfine: hyperfine 1.19.0
- bcftools: bcftools 1.21
- Runs: 3 with 1 warmup
- Correctness: all generated `equivalence-*.diff` files were empty

| case | records | input | Output equivalence | vcf-fast mean | bcftools mean | speedup | vcf-fast variants/s | bcftools variants/s | vcf-fast peak RSS KB | bcftools peak RSS KB | notes |
|---|---:|---|---|---:|---:|---:|---:|---:|---:|---:|---|
| QUAL plain | 10000 | plain | matches bcftools filtered core records | 0.052220s | 0.375829s | 7.20x | 191498 | 26608 | 2672 | 4260 |  |
| QUAL gzip input | 10000 | gzip | matches bcftools filtered core records | 0.079282s | 0.414785s | 5.23x | 126132 | 24109 | 2660 | 4572 |  |
| Convert TSV | 10000 | plain | matches normalized bcftools query TSV rows | 0.070793s | 0.079173s | 1.12x | 141257 | 126306 | 2676 | 4124 |  |
| QUAL plain | 100000 | plain | matches bcftools filtered core records | 0.423681s | 3.530514s | 8.33x | 236027 | 28324 | 2656 | 4272 |  |
| QUAL gzip input | 100000 | gzip | matches bcftools filtered core records | 0.773240s | 4.140379s | 5.35x | 129326 | 24152 | 2676 | 4592 |  |
| Convert TSV | 100000 | plain | matches normalized bcftools query TSV rows | 0.577931s | 0.639262s | 1.11x | 173031 | 156430 | 2676 | 4124 |  |

### Command Templates

- VCF-Fast filter: `./target/release/vcf-fast filter <input> --where '<expr>' -o <output>`
- bcftools filter: `bcftools filter -i '<expr>' <input> -o <output>`
- VCF-Fast convert TSV: `./target/release/vcf-fast convert <input> --to tsv -o <output.tsv>`
- bcftools query TSV: `bcftools query -u -f '%CHROM\t%POS\t%ID\t%REF\t%ALT\t%QUAL\t%FILTER\t%INFO/DP\t%INFO/AF\n' <input>`
- Region slicing for IGSR mode: `bcftools view -H -r "$VCF_FAST_PUBLIC_REGION" <input>`

### Caveats

These are public benchmark subsets, not whole-cohort claims. GIAB remains a one-run smoke. IGSR uses repeated runs and includes memory/throughput fields, but still covers only the currently supported QUAL filter and TSV conversion cases.
