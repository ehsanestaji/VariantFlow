# v1.9 Second Public FORMAT-Rich Cohort

This report adds a second public FORMAT-rich cohort so public FORMAT aggregate
claims do not depend only on the ENA Ovis aries cohort. The default target is
ENA `ERZ18456468` / `PRJEB60909`,
`Dutch_Genebank_Cattle_Y_merged.vcf.gz`: a 29-sample
`Bos taurus` Y-chromosome VCF from Dutch Genebank Cattle with
declared `FORMAT/AD`, `FORMAT/DP`, and `FORMAT/GQ`,
`131795380` bytes, and no bundled tabix index.

Mayo VCF-Miner remains the preferred human follow-up candidate because its page
lists 629-sample 1000 Genomes chr22 benchmark VCFs, but direct automated
downloads returned 403 during validation. This harness therefore uses the
validated ENA cattle cohort now and keeps the human candidate as a caveat, not
as measured evidence.

Every row compares VariantFlow against `bcftools filter` and claims speed only
when filtered core records match. Heavy-output mode is enabled by default for
tiers at or above `500000` records and for `full` tiers:
correctness streams `/dev/stdout` into core records only, while timed runs
write to `/dev/null`.

Repeated timing uses `hyperfine` when available
(`VCF_FAST_V19_RUNS=3`, `VCF_FAST_V19_WARMUP=1`). Peak RSS is
reported from GNU `/usr/bin/time -v` on Linux or BSD `/usr/bin/time -l` on
macOS. Competitor version for this run: `bcftools 1.23.1`.

Planned expressions:

- `ANY(FORMAT/DP > 20)`
- `ALL(FORMAT/GQ >= 30)`
- `N_PASS(FORMAT/AD[1] > 10) >= 2`
- selected-sample `FORMAT/DP > 20`
- `QUAL > 30 && ANY(FORMAT/DP > 20)`

| case | dataset | tier | exact VariantFlow command | exact competitor command | correctness result | runtime | variants/sec | peak RSS | claim decision | caveat |
|---|---|---:|---|---|---|---|---|---|---|---|
| ANY(FORMAT/DP > 20) | tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz | full requested / 5488549 actual | `./target/release/variantflow filter tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz --where ANY\(FORMAT/DP\ \>\ 20\) -o /dev/stdout ` | `bcftools filter -Ov -i N_PASS\(FMT/DP\[\*\]\>20\)\>0 tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz -o /dev/stdout ` | matched core records | VariantFlow 3.932480s +/- 0.160109s; bcftools 44.932166s +/- 27.338326s; speedup 11.43x | VariantFlow 1395697; bcftools 122152 | VariantFlow 8110080; bcftools 8454144 | measured faster on this public FORMAT-rich expression tier | source=https://ftp.sra.ebi.ac.uk/vol1/analysis/ERZ184/ERZ18456468/Dutch_Genebank_Cattle_Y_merged.vcf.gz; accession=ERZ18456468; samples=29; selected_sample=n/a; output policy: heavy-output mode: /dev/stdout core records only for correctness; /dev/null for timed runs |
| ALL(FORMAT/GQ >= 30) | tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz | full requested / 5488549 actual | `./target/release/variantflow filter tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz --where ALL\(FORMAT/GQ\ \>=\ 30\) -o /dev/stdout ` | `bcftools filter -Ov -i N_PASS\(FMT/GQ\[\*\]\>=30\)==29 tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz -o /dev/stdout ` | matched core records | VariantFlow 3.904642s +/- 1.062674s; bcftools 35.218803s +/- 22.172048s; speedup 9.02x | VariantFlow 1405647; bcftools 155841 | VariantFlow 8028160; bcftools 8486912 | measured faster on this public FORMAT-rich expression tier | source=https://ftp.sra.ebi.ac.uk/vol1/analysis/ERZ184/ERZ18456468/Dutch_Genebank_Cattle_Y_merged.vcf.gz; accession=ERZ18456468; samples=29; selected_sample=n/a; output policy: heavy-output mode: /dev/stdout core records only for correctness; /dev/null for timed runs |
| N_PASS(FORMAT/AD[1] > 10) >= 2 | tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz | full requested / 5488549 actual | `./target/release/variantflow filter tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz --where N_PASS\(FORMAT/AD\[1\]\ \>\ 10\)\ \>=\ 2 -o /dev/stdout ` | `bcftools filter -Ov -i N_PASS\(FMT/AD\[\*:1\]\>10\)\>=2 tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz -o /dev/stdout ` | matched core records | VariantFlow 9.778260s +/- 1.727708s; bcftools 14.308917s +/- 4.789541s; speedup 1.46x | VariantFlow 561301; bcftools 383575 | VariantFlow 8110080; bcftools 8470528 | measured faster on this public FORMAT-rich expression tier | source=https://ftp.sra.ebi.ac.uk/vol1/analysis/ERZ184/ERZ18456468/Dutch_Genebank_Cattle_Y_merged.vcf.gz; accession=ERZ18456468; samples=29; selected_sample=n/a; output policy: heavy-output mode: /dev/stdout core records only for correctness; /dev/null for timed runs |
| selected-sample FORMAT/DP > 20 | tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz | full requested / 5488549 actual | `./target/release/variantflow filter tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz --sample 139419 --where FORMAT/DP\ \>\ 20 -o /dev/stdout ` | `bcftools view -s 139419 tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz -Ou | bcftools filter -Ov -i FMT/DP\[0\]\>20 -o /dev/stdout` | matched core records | VariantFlow 0.946564s +/- 0.237143s; bcftools 24.549455s +/- 5.875007s; speedup 25.94x | VariantFlow 5798392; bcftools 223571 | VariantFlow 8093696; bcftools 10387456 | measured faster on this public FORMAT-rich expression tier | source=https://ftp.sra.ebi.ac.uk/vol1/analysis/ERZ184/ERZ18456468/Dutch_Genebank_Cattle_Y_merged.vcf.gz; accession=ERZ18456468; samples=29; selected_sample=139419; output policy: heavy-output mode: /dev/stdout core records only for correctness; /dev/null for timed runs |
| QUAL > 30 && ANY(FORMAT/DP > 20) | tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz | full requested / 5488549 actual | `./target/release/variantflow filter tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz --where QUAL\ \>\ 30\ \&\&\ ANY\(FORMAT/DP\ \>\ 20\) -o /dev/stdout ` | `bcftools filter -Ov -i QUAL\>30\ \&\&\ N_PASS\(FMT/DP\[\*\]\>20\)\>0 tests/output/public-data/Dutch_Genebank_Cattle_Y_merged.vcf.gz -o /dev/stdout ` | matched core records | VariantFlow 1.344447s +/- 0.323590s; bcftools 35.842463s +/- 14.105624s; speedup 26.66x | VariantFlow 4082384; bcftools 153130 | VariantFlow 8110080; bcftools 8404992 | measured faster on this public FORMAT-rich expression tier | source=https://ftp.sra.ebi.ac.uk/vol1/analysis/ERZ184/ERZ18456468/Dutch_Genebank_Cattle_Y_merged.vcf.gz; accession=ERZ18456468; samples=29; selected_sample=n/a; output policy: heavy-output mode: /dev/stdout core records only for correctness; /dev/null for timed runs |
