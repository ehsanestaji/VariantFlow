# v2.0 Human FORMAT-Rich Cohort

This report adds a public human FORMAT-rich cohort benchmark. The default
target is the DDBJ CHM13 public-human-genomes JointCall chr22 VCF,
`CHM13_autosome_PAR.chr22.vcf.gz`: a 3715-sample
`Homo sapiens` cohort VCF with declared `FORMAT/AD`,
`FORMAT/DP`, and `FORMAT/GQ`, `27232829080` compressed bytes,
and a CSI index at `https://ddbj.nig.ac.jp/public/public-human-genomes/CHM13/JointCall/CHM13_autosome_PAR.chr22.vcf.gz.csi`.

The harness uses bounded streaming from the remote BGZF file by default and
does not cache the 27 GB VCF. Full remote runs require
`VCF_FAST_ALLOW_REMOTE_FULL=1` or a local `VCF_FAST_HUMAN_FORMAT_VCF`.

Every row compares VariantFlow against `bcftools filter` and claims speed only
when filtered core records match. Heavy-output mode is enabled by default for
tiers at or above `500000` records and for `full` tiers:
correctness streams `/dev/stdout` into core records only, while timed runs
write to `/dev/null`.

Repeated timing uses `hyperfine` when available
(`VCF_FAST_V20_RUNS=3`, `VCF_FAST_V20_WARMUP=1`). Peak RSS is
reported from GNU `/usr/bin/time -v` on Linux or BSD `/usr/bin/time -l` on
macOS. Competitor version for this run: `bcftools 1.23.1`.

Planned expressions:

- `ANY(FORMAT/DP > 20)`
- `ALL(FORMAT/GQ >= 30)`
- `N_PASS(FORMAT/AD[1] > 10) >= 10`
- selected-sample `FORMAT/DP > 20`
- `QUAL > 30 && ANY(FORMAT/DP > 20)`

| case | dataset | tier | exact VariantFlow command | exact competitor command | correctness result | runtime | variants/sec | peak RSS | claim decision | caveat |
|---|---|---:|---|---|---|---|---|---|---|---|
| ANY(FORMAT/DP > 20) | https://ddbj.nig.ac.jp/public/public-human-genomes/CHM13/JointCall/CHM13_autosome_PAR.chr22.vcf.gz | 1000 requested / 1000 actual | `./target/release/variantflow filter tests/output/benchmark-results/v20-human-format-cohort/human-format-cohort-1000.vcf.gz --where ANY\(FORMAT/DP\ \>\ 20\) -o /dev/stdout ` | `bcftools filter -Ov -i N_PASS\(FMT/DP\[\*\]\>20\)\>0 tests/output/benchmark-results/v20-human-format-cohort/human-format-cohort-1000.vcf.gz -o /dev/stdout ` | matched core records | VariantFlow 0.065679s +/- 0.003961s; bcftools 0.723888s +/- 0.072840s; speedup 11.02x | VariantFlow 15226; bcftools 1381 | VariantFlow 8159232; bcftools 12238848 | measured faster on this public FORMAT-rich expression tier | source=https://ddbj.nig.ac.jp/public/public-human-genomes/CHM13/JointCall/CHM13_autosome_PAR.chr22.vcf.gz; samples=3715; selected_sample=n/a; output policy: smoke/core-stream output; bounded streaming; does not cache the 27 GB VCF |
| ALL(FORMAT/GQ >= 30) | https://ddbj.nig.ac.jp/public/public-human-genomes/CHM13/JointCall/CHM13_autosome_PAR.chr22.vcf.gz | 1000 requested / 1000 actual | `./target/release/variantflow filter tests/output/benchmark-results/v20-human-format-cohort/human-format-cohort-1000.vcf.gz --where ALL\(FORMAT/GQ\ \>=\ 30\) -o /dev/stdout ` | `bcftools filter -Ov -i N_PASS\(FMT/GQ\[\*\]\>=30\)==3715 tests/output/benchmark-results/v20-human-format-cohort/human-format-cohort-1000.vcf.gz -o /dev/stdout ` | matched core records | VariantFlow 0.058859s +/- 0.001249s; bcftools 0.391526s +/- 0.001873s; speedup 6.65x | VariantFlow 16990; bcftools 2554 | VariantFlow 8175616; bcftools 11665408 | measured faster on this public FORMAT-rich expression tier | source=https://ddbj.nig.ac.jp/public/public-human-genomes/CHM13/JointCall/CHM13_autosome_PAR.chr22.vcf.gz; samples=3715; selected_sample=n/a; output policy: smoke/core-stream output; bounded streaming; does not cache the 27 GB VCF |
| N_PASS(FORMAT/AD[1] > 10) >= 10 | https://ddbj.nig.ac.jp/public/public-human-genomes/CHM13/JointCall/CHM13_autosome_PAR.chr22.vcf.gz | 1000 requested / 1000 actual | `./target/release/variantflow filter tests/output/benchmark-results/v20-human-format-cohort/human-format-cohort-1000.vcf.gz --where N_PASS\(FORMAT/AD\[1\]\ \>\ 10\)\ \>=\ 10 -o /dev/stdout ` | `bcftools filter -Ov -i N_PASS\(FMT/AD\[\*:1\]\>10\)\>=10 tests/output/benchmark-results/v20-human-format-cohort/human-format-cohort-1000.vcf.gz -o /dev/stdout ` | matched core records | VariantFlow 0.080495s +/- 0.002354s; bcftools 0.518807s +/- 0.042953s; speedup 6.45x | VariantFlow 12423; bcftools 1927 | VariantFlow 8159232; bcftools 12140544 | measured faster on this public FORMAT-rich expression tier | source=https://ddbj.nig.ac.jp/public/public-human-genomes/CHM13/JointCall/CHM13_autosome_PAR.chr22.vcf.gz; samples=3715; selected_sample=n/a; output policy: smoke/core-stream output; bounded streaming; does not cache the 27 GB VCF |
| selected-sample FORMAT/DP > 20 | https://ddbj.nig.ac.jp/public/public-human-genomes/CHM13/JointCall/CHM13_autosome_PAR.chr22.vcf.gz | 1000 requested / 1000 actual | `./target/release/variantflow filter tests/output/benchmark-results/v20-human-format-cohort/human-format-cohort-1000.vcf.gz --sample HG00097 --where FORMAT/DP\ \>\ 20 -o /dev/stdout ` | `bcftools view -s HG00097 tests/output/benchmark-results/v20-human-format-cohort/human-format-cohort-1000.vcf.gz -Ou | bcftools filter -Ov -i FMT/DP\[0\]\>20 -o /dev/stdout` | matched core records | VariantFlow 0.046248s +/- 0.001341s; bcftools 0.585987s +/- 0.018272s; speedup 12.67x | VariantFlow 21623; bcftools 1707 | VariantFlow 8142848; bcftools 13582336 | measured faster on this public FORMAT-rich expression tier | source=https://ddbj.nig.ac.jp/public/public-human-genomes/CHM13/JointCall/CHM13_autosome_PAR.chr22.vcf.gz; samples=3715; selected_sample=HG00097; output policy: smoke/core-stream output; bounded streaming; does not cache the 27 GB VCF |
| QUAL > 30 && ANY(FORMAT/DP > 20) | https://ddbj.nig.ac.jp/public/public-human-genomes/CHM13/JointCall/CHM13_autosome_PAR.chr22.vcf.gz | 1000 requested / 1000 actual | `./target/release/variantflow filter tests/output/benchmark-results/v20-human-format-cohort/human-format-cohort-1000.vcf.gz --where QUAL\ \>\ 30\ \&\&\ ANY\(FORMAT/DP\ \>\ 20\) -o /dev/stdout ` | `bcftools filter -Ov -i QUAL\>30\ \&\&\ N_PASS\(FMT/DP\[\*\]\>20\)\>0 tests/output/benchmark-results/v20-human-format-cohort/human-format-cohort-1000.vcf.gz -o /dev/stdout ` | matched core records | VariantFlow 0.335309s +/- 0.053531s; bcftools 2.648581s +/- 1.469996s; speedup 7.90x | VariantFlow 2982; bcftools 378 | VariantFlow 8175616; bcftools 11730944 | measured faster on this public FORMAT-rich expression tier | source=https://ddbj.nig.ac.jp/public/public-human-genomes/CHM13/JointCall/CHM13_autosome_PAR.chr22.vcf.gz; samples=3715; selected_sample=n/a; output policy: smoke/core-stream output; bounded streaming; does not cache the 27 GB VCF |
