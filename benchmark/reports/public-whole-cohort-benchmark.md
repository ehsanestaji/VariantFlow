# VCF-Fast Public Whole-Cohort Benchmark

Status: v0.6 evidence run completed in Docker for GIAB HG002 tiers, IGSR chr22 10k/100k tiers, public indexed-region 10k/100k runs, and compatibility benchmarks. IGSR whole-file 1M was attempted but deferred after the generated plain VCF exceeded 13 GB, which made it a heavy-run item rather than a balanced local tier.

## Scope

Modes covered by the v0.6 harness:

- `public-whole`: larger GIAB HG002 and IGSR chr22 record-count tiers from cached public VCFs.
- `public-region-repeated`: repeated indexed-region runs comparing `vcf-fast --region` with `bcftools view -r ... | bcftools filter`.
- `compatibility`: BCF input, BGZF output, indexed VCF/BCF region reads, and TSV conversion with `--features htslib-static`.

## Required Report Fields

Every measured row must include:

- dataset source URL
- dataset size bytes
- record count
- input format and compression
- exact VCF-Fast command
- exact competitor command
- competitor version
- runtime mean/stddev
- speedup
- variants/sec
- peak RSS
- correctness result
- caveats

## Public Dataset Sources

| dataset | dataset source URL | cache path | caveats |
|---|---|---|---|
| GIAB HG002 small variant benchmark v4.2.1 | `https://ftp-trace.ncbi.nlm.nih.gov/ReferenceSamples/giab/release/AshkenazimTrio/HG002_NA24385_son/NISTv4.2.1/GRCh38/HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz` | `tests/output/public-data/HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz` | Public data is downloaded locally and ignored by git. |
| IGSR / 1000 Genomes high-coverage chr22 | `https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000G_2504_high_coverage/working/20220422_3202_phased_SNV_INDEL_SV/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz` | `tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz` | Region defaults to `chr22:1-20000000`; adjust if a source uses different contig naming. |

## Local Evidence Commands

```bash
benchmark/download_public_data.sh all
VCF_FAST_PUBLIC_RECORD_TIERS="10000 100000 1000000" make bench-public
VCF_FAST_PUBLIC_RECORD_TIERS="10000 100000" VCF_FAST_BENCH_RUNS=3 make bench-public-region
VCF_FAST_BENCH_RUNS=3 make bench-compat
```

## Correctness Baselines

| operation | competitor baseline | correctness result |
|---|---|---|
| Filtering | `bcftools filter` | Core records must match for supported predicates. |
| TSV conversion | `bcftools query -u` | Normalized rows must match selected TSV columns. |
| Stats | `bcftools stats` | Overlapping simple counts must match. |
| Indexed region filtering | `bcftools view -r ... &#124; bcftools filter` | Region-filtered core records must match. |
| BGZF output | `tabix -p vcf` and `bcftools view` | Output must be gzip-readable, indexable, and viewable. |

## Measurement Table

| case | dataset source URL | dataset size bytes | record count | input format | input compression | exact VCF-Fast command | exact competitor command | competitor version | runtime mean/stddev | speedup | variants/sec | peak RSS | correctness result | caveats |
|---|---|---:|---:|---|---|---|---|---|---|---:|---:|---|---|---|
| GIAB QUAL plain | GIAB HG002 URL above | 7,108,218 | 10,000 | VCF | none | `vcf-fast filter public-whole-10000.vcf --where 'QUAL > 30' -o fast.vcf` | `bcftools filter -i 'QUAL>30' public-whole-10000.vcf -o bcftools.vcf` | bcftools 1.21 | 0.162685s +/- 0.023696s vs 0.293435s +/- 0.008818s | 1.80x | 61,468 / 34,079 | 2,868 / 3,696 KB | matched filtered core records | Docker run, 3 repeats |
| GIAB QUAL gzip | GIAB HG002 URL above | 7,108,218 | 10,000 | VCF | gzip | `vcf-fast filter public-whole-10000.vcf.gz --where 'QUAL > 30' -o fast.vcf` | `bcftools filter -i 'QUAL>30' public-whole-10000.vcf.gz -o bcftools.vcf` | bcftools 1.21 | 0.203908s +/- 0.011960s vs 0.350708s +/- 0.026949s | 1.72x | 49,042 / 28,514 | 2,852 / 4,144 KB | matched filtered core records | Docker run, 3 repeats |
| GIAB TSV | GIAB HG002 URL above | 7,108,218 | 10,000 | VCF | none | `vcf-fast convert public-whole-10000.vcf --to tsv -o fast.tsv` | `bcftools query -u -f ... public-whole-10000.vcf > bcftools.tsv` | bcftools 1.21 | 0.046296s +/- 0.007211s vs 0.036899s +/- 0.001959s | 0.80x | 216,001 / 271,010 | 2,860 / 3,692 KB | matched normalized TSV rows | bcftools faster at this tier |
| GIAB QUAL plain | GIAB HG002 URL above | 70,535,185 | 100,000 | VCF | none | `vcf-fast filter public-whole-100000.vcf --where 'QUAL > 30' -o fast.vcf` | `bcftools filter -i 'QUAL>30' public-whole-100000.vcf -o bcftools.vcf` | bcftools 1.21 | 0.986689s +/- 0.447644s vs 2.344839s +/- 0.681678s | 2.38x | 101,349 / 42,647 | 2,868 / 3,676 KB | matched filtered core records | high variance; repeat on quiet runner |
| GIAB QUAL gzip | GIAB HG002 URL above | 70,535,185 | 100,000 | VCF | gzip | `vcf-fast filter public-whole-100000.vcf.gz --where 'QUAL > 30' -o fast.vcf` | `bcftools filter -i 'QUAL>30' public-whole-100000.vcf.gz -o bcftools.vcf` | bcftools 1.21 | 1.849179s +/- 0.277410s vs 1.742410s +/- 0.200522s | 0.94x | 54,078 / 57,392 | 2,856 / 4,160 KB | matched filtered core records | bcftools slightly faster |
| GIAB TSV | GIAB HG002 URL above | 70,535,185 | 100,000 | VCF | none | `vcf-fast convert public-whole-100000.vcf --to tsv -o fast.tsv` | `bcftools query -u -f ... public-whole-100000.vcf > bcftools.tsv` | bcftools 1.21 | 0.291234s +/- 0.005559s vs 0.279663s +/- 0.008479s | 0.96x | 343,367 / 357,573 | 2,852 / 3,692 KB | matched normalized TSV rows | near parity, bcftools slightly faster |
| GIAB QUAL plain | GIAB HG002 URL above | 705,603,816 | 1,000,000 | VCF | none | `vcf-fast filter public-whole-1000000.vcf --where 'QUAL > 30' -o fast.vcf` | `bcftools filter -i 'QUAL>30' public-whole-1000000.vcf -o bcftools.vcf` | bcftools 1.21 | 15.144179s +/- 0.397243s vs 30.205909s +/- 4.363265s | 1.99x | 66,032 / 33,106 | 2,848 / 3,696 KB | matched filtered core records | strongest GIAB filter tier |
| GIAB QUAL gzip | GIAB HG002 URL above | 705,603,816 | 1,000,000 | VCF | gzip | `vcf-fast filter public-whole-1000000.vcf.gz --where 'QUAL > 30' -o fast.vcf` | `bcftools filter -i 'QUAL>30' public-whole-1000000.vcf.gz -o bcftools.vcf` | bcftools 1.21 | 15.727625s +/- 0.642162s vs 29.799889s +/- 4.570259s | 1.89x | 63,582 / 33,557 | 2,868 / 4,160 KB | matched filtered core records | gzip input win at 1M |
| GIAB TSV | GIAB HG002 URL above | 705,603,816 | 1,000,000 | VCF | none | `vcf-fast convert public-whole-1000000.vcf --to tsv -o fast.tsv` | `bcftools query -u -f ... public-whole-1000000.vcf > bcftools.tsv` | bcftools 1.21 | 2.680287s +/- 0.568676s vs 3.015807s +/- 0.632075s | 1.13x | 373,094 / 331,586 | 2,868 / 3,692 KB | matched normalized TSV rows | TSV win appears at 1M |
| IGSR QUAL plain | IGSR chr22 URL above | 139,093,220 | 10,000 | VCF | none | `vcf-fast filter public-whole-10000.vcf --where 'QUAL > 30' -o fast.vcf` | `bcftools filter -i 'QUAL>30' public-whole-10000.vcf -o bcftools.vcf` | bcftools 1.21 | 0.126696s +/- 0.000549s vs 0.614581s +/- 0.014740s | 4.85x | 78,929 / 16,271 | 2,852 / 4,128 KB | matched filtered core records | sample-rich public VCF subset |
| IGSR QUAL gzip | IGSR chr22 URL above | 139,093,220 | 10,000 | VCF | gzip | `vcf-fast filter public-whole-10000.vcf.gz --where 'QUAL > 30' -o fast.vcf` | `bcftools filter -i 'QUAL>30' public-whole-10000.vcf.gz -o bcftools.vcf` | bcftools 1.21 | 0.172968s +/- 0.002183s vs 0.890341s +/- 0.070906s | 5.15x | 57,814 / 11,232 | 2,868 / 4,592 KB | matched filtered core records | sample-rich public VCF subset |
| IGSR TSV | IGSR chr22 URL above | 139,093,220 | 10,000 | VCF | none | `vcf-fast convert public-whole-10000.vcf --to tsv -o fast.tsv` | `bcftools query -u -f ... public-whole-10000.vcf > bcftools.tsv` | bcftools 1.21 | 0.120053s +/- 0.006271s vs 0.146778s +/- 0.004587s | 1.22x | 83,297 / 68,130 | 2,860 / 4,124 KB | matched normalized TSV rows | TSV win at 10k |
| IGSR QUAL plain | IGSR chr22 URL above | 1,390,476,439 | 100,000 | VCF | none | `vcf-fast filter public-whole-100000.vcf --where 'QUAL > 30' -o fast.vcf` | `bcftools filter -i 'QUAL>30' public-whole-100000.vcf -o bcftools.vcf` | bcftools 1.21 | 1.266873s +/- 0.335789s vs 6.518391s +/- 0.783685s | 5.15x | 78,935 / 15,341 | 2,856 / 4,128 KB | matched filtered core records | strong selective-filter win |
| IGSR QUAL gzip | IGSR chr22 URL above | 1,390,476,439 | 100,000 | VCF | gzip | `vcf-fast filter public-whole-100000.vcf.gz --where 'QUAL > 30' -o fast.vcf` | `bcftools filter -i 'QUAL>30' public-whole-100000.vcf.gz -o bcftools.vcf` | bcftools 1.21 | 1.273274s +/- 0.051888s vs 7.271171s +/- 0.518772s | 5.71x | 78,538 / 13,753 | 2,868 / 4,592 KB | matched filtered core records | strongest IGSR whole-tier win |
| IGSR TSV | IGSR chr22 URL above | 1,390,476,439 | 100,000 | VCF | none | `vcf-fast convert public-whole-100000.vcf --to tsv -o fast.tsv` | `bcftools query -u -f ... public-whole-100000.vcf > bcftools.tsv` | bcftools 1.21 | 1.443653s +/- 0.182409s vs 1.256818s +/- 0.118613s | 0.87x | 69,269 / 79,566 | 2,848 / 4,120 KB | matched normalized TSV rows | bcftools faster at 100k |
| IGSR whole 1M | IGSR chr22 URL above | >13 GB generated before stop | 1,000,000 | VCF | none/gzip | deferred | deferred | bcftools 1.21 | deferred | n/a | n/a | n/a | not measured | balanced local run exceeded practical disk/runtime |
| IGSR region QUAL | IGSR chr22 URL above | 445,701,977 | 10,000 | VCF | BGZF indexed | `vcf-fast filter input.vcf.gz --region chr22:1-20000000 --where 'QUAL > 30' -o fast.vcf` | `bcftools view -r chr22:1-20000000 input.vcf.gz &#124; bcftools filter -i 'QUAL>30' -Ov -o bcftools.vcf` | bcftools 1.21 | 9.449717s +/- 0.034928s vs 13.937555s +/- 0.066139s | 1.47x | 1,058 / 717 | 8,264 / 4,876 KB | matched filtered core records | indexed-region filter win |
| IGSR region TSV | IGSR chr22 URL above | 445,701,977 | 10,000 | VCF | BGZF indexed | `vcf-fast convert input.vcf.gz --region chr22:1-20000000 --to tsv -o fast.tsv` | `bcftools view -r chr22:1-20000000 input.vcf.gz &#124; bcftools query -u -f ... > bcftools.tsv` | bcftools 1.21 | 19.429769s +/- 0.022455s vs 13.900563s +/- 0.141777s | 0.72x | 515 / 719 | 5,816 / 4,856 KB | matched normalized TSV rows | bcftools faster; htslib TSV path needs optimization |
| IGSR region stats | IGSR chr22 URL above | 445,701,977 | 10,000 | VCF | BGZF indexed | `vcf-fast stats input.vcf.gz --region chr22:1-20000000 > fast.json` | `bcftools view -r chr22:1-20000000 input.vcf.gz &#124; bcftools stats - > bcftools.txt` | bcftools 1.21 | 19.382118s +/- 0.022114s vs 13.914735s +/- 0.034030s | 0.72x | 516 / 719 | 5,816 / 4,856 KB | matched overlapping record count | bcftools faster |
| IGSR region QUAL | IGSR chr22 URL above | 445,701,977 | 100,000 | VCF | BGZF indexed | `vcf-fast filter input.vcf.gz --region chr22:1-20000000 --where 'QUAL > 30' -o fast.vcf` | `bcftools view -r chr22:1-20000000 input.vcf.gz &#124; bcftools filter -i 'QUAL>30' -Ov -o bcftools.vcf` | bcftools 1.21 | 9.455918s +/- 0.027795s vs 13.879006s +/- 0.075344s | 1.47x | 10,575 / 7,205 | 8,264 / 4,876 KB | matched filtered core records | indexed-region filter win |
| IGSR region TSV | IGSR chr22 URL above | 445,701,977 | 100,000 | VCF | BGZF indexed | `vcf-fast convert input.vcf.gz --region chr22:1-20000000 --to tsv -o fast.tsv` | `bcftools view -r chr22:1-20000000 input.vcf.gz &#124; bcftools query -u -f ... > bcftools.tsv` | bcftools 1.21 | 19.485915s +/- 0.063716s vs 13.884802s +/- 0.109858s | 0.71x | 5,132 / 7,202 | 5,808 / 4,868 KB | matched normalized TSV rows | bcftools faster |
| IGSR region stats | IGSR chr22 URL above | 445,701,977 | 100,000 | VCF | BGZF indexed | `vcf-fast stats input.vcf.gz --region chr22:1-20000000 > fast.json` | `bcftools view -r chr22:1-20000000 input.vcf.gz &#124; bcftools stats - > bcftools.txt` | bcftools 1.21 | 19.392235s +/- 0.024031s vs 14.006119s +/- 0.242270s | 0.72x | 5,157 / 7,140 | 5,796 / 4,876 KB | matched overlapping record count | bcftools faster |

## Caveats

v0.6 proves a stronger but still narrow claim: native selective filtering is repeatedly faster on measured GIAB/IGSR public tiers, especially IGSR sample-rich QUAL filtering. TSV conversion is mixed, and htslib-backed region TSV/stats trail `bcftools`. Whole-cohort 1M IGSR is deferred because the balanced local extraction produced a >13 GB intermediate before benchmarking.
