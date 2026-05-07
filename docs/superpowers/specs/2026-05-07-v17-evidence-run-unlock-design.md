# VariantFlow v1.7 Evidence Run Unlock Design

Date: 2026-05-07
Status: approved design direction; implementation plan not started

## Goal

Unlock the true public VCFtools population-genetics evidence run by wiring the
existing v1.7 benchmark harness to an official 1000 Genomes / IGSR metadata
source. The immediate target is human IGSR evidence only: finalize metadata,
cache it reproducibly, run `10k`, `50k`, and `100k` actual biallelic tiers, then
update claims only from correctness-matched measured rows.

LD memory optimization remains out of scope for this milestone. The new evidence
rows should tell us whether LD RSS needs surgery next.

## Chosen Source

Use the official 1000 Genomes high-coverage 3202-sample metadata file:

`https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000G_2504_high_coverage/20130606_g1k_3202_samples_ped_population.txt`

The file is small, stable, and directly aligned with the cached 3202-sample
high-coverage chr22 VCF family already used by the project. Its header is
whitespace-delimited:

```text
FamilyID SampleID FatherID MotherID Sex Population Superpopulation
```

The required fields for VariantFlow are:

- `SampleID` as the VCF sample identifier.
- `Population` as the fine population label.
- `Superpopulation` as the broad population label such as `AFR`, `EUR`, `EAS`,
  `SAS`, or `AMR`.

This is preferred over older Phase 3 panel files because it matches the 3202
sample high-coverage cohort used by the public chr22 VCF.

## Scope

### In Scope

- Extend `benchmark/igsr_population_files.py` to parse official IGSR
  whitespace-delimited metadata.
- Keep support for the existing TSV-style metadata helper behavior.
- Cache the official metadata through `benchmark/download_public_data.sh
  igsr-true-population`.
- Ensure the downloader also caches or points to the existing 1000 Genomes
  high-coverage chr22 VCF.
- Add metadata smoke checks:
  - VCF samples are covered by metadata.
  - no unmatched samples by default.
  - `AFR:EUR` population files are generated from official labels.
  - provenance records local paths, hashes, group settings, and no-header-
    fallback policy.
- Run the v1.7 true-population harness for `10k`, `50k`, and `100k` actual
  biallelic rows when local tools and data are available.
- Update `benchmark/reports/v17-true-public-population-evidence.md` with real
  measured rows only.
- Update `docs/claim-matrix.md`, `docs/public-benchmark-table.md`, and
  `README.md` only for correctness-matched measured rows.

### Out Of Scope

- LD memory optimization.
- Plant population evidence.
- PLINK and GATK baselines.
- Paper figure updates.
- Release, Bioconda, or tagged binaries.

Those remain next milestones after true human population evidence exists.

## Architecture

The milestone keeps the current harness architecture and only removes the
metadata blocker.

### Metadata Helper

`benchmark/igsr_population_files.py` becomes the single metadata normalization
entry point. It should accept:

- tab-delimited metadata with `sample`, `population`, and `superpopulation`
  aliases;
- whitespace-delimited IGSR PED/population metadata with `SampleID`,
  `Population`, and `Superpopulation`;
- plain text or gzip-compressed metadata.

The helper must remain strict:

- duplicate VCF samples fail;
- duplicate metadata sample IDs fail;
- blank required fields fail;
- unmatched VCF samples fail unless `--allow-unmatched` is explicitly passed;
- no header-derived fallback groups are allowed for serious evidence.

### Downloader

`benchmark/download_public_data.sh igsr-true-population` should stop being a
permanent blocker. It should download/cache:

- `1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz`
  and its `.tbi`, reusing the existing `download_igsr_chr22` logic;
- `20130606_g1k_3202_samples_ped_population.txt` as
  `tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt`;
- optionally a small manifest file with source URL and date.

The script should print the exact environment variables for the benchmark:

```bash
VCF_FAST_V17_TRUE_POP_INPUT="tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz"
VCF_FAST_V17_TRUE_POP_METADATA="tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt"
```

### Benchmark Harness

`benchmark/run_v17_true_population_evidence.sh` remains the execution path.
After metadata is cached, the benchmark should run:

```bash
VCF_FAST_V17_TRUE_POP_INPUT="tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz" \
VCF_FAST_V17_TRUE_POP_METADATA="tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt" \
VCF_FAST_V17_TRUE_POP_TIERS="10000 50000 100000" \
VCF_FAST_V17_TRUE_POP_GROUPS="AFR:EUR" \
VCF_FAST_V17_TRUE_POP_RUNS=3 \
VCF_FAST_V17_TRUE_POP_WARMUP=1 \
make bench-vcftools-true-popgen
```

Rows that fail correctness must be documented as failures, not speed results.
Rows whose actual biallelic record count is below the requested tier must not be
described as full-tier evidence.

## Data Flow

1. Downloader caches public VCF and official metadata under
   `tests/output/public-data`.
2. Harness stages bounded biallelic SNP tiers with `bcftools view -m2 -M2 -v
   snps` and `bgzip`, without giant plain VCF intermediates.
3. Metadata helper reads staged VCF samples and official metadata, then writes
   two VCFtools population files for `AFR` and `EUR`.
4. Harness runs VariantFlow and VCFtools workflows:
   - frequency;
   - missingness;
   - HWE;
   - heterozygosity;
   - site pi;
   - window pi;
   - Tajima's D;
   - LD;
   - Weir-Cockerham Fst.
5. Harness records correctness, runtime, RSS, CPU seconds, CPU-hour estimates,
   exact commands, versions, source paths, and caveats.
6. Claim docs are updated only after measured rows pass correctness.

## Error Handling

- Missing public VCF or metadata still exits blocked rather than writing claims.
- Metadata schema mismatch exits clearly and names the missing required fields.
- Unmatched samples exit clearly by default, with a preview of sample IDs.
- Missing `bcftools`, `bgzip`, `vcftools`, `hyperfine`, or resource helper exits
  blocked/failure before performance claims are produced.
- Docker-only VCFtools remains blocked for this true evidence harness until
  parity regeneration is implemented safely in Docker mode.

## Testing

### Unit And Script Tests

- Metadata helper parses the official whitespace-delimited IGSR file shape.
- Metadata helper still parses existing tab-delimited fixture metadata.
- Gzip metadata still works.
- Missing `SampleID`, `Population`, or `Superpopulation` fails.
- Duplicate VCF or metadata samples fail.
- `AFR:EUR` output files preserve VCF sample order.
- Downloader contract test confirms `igsr-true-population` caches the official
  metadata URL and prints the benchmark environment variables.

### Verification

Run:

```bash
make verify
cargo test --features htslib-static
cargo clippy --features htslib-static --all-targets -- -D warnings
```

### Evidence Run

Run only after local data and tools are available:

```bash
benchmark/download_public_data.sh igsr-true-population

VCF_FAST_V17_TRUE_POP_INPUT="tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz" \
VCF_FAST_V17_TRUE_POP_METADATA="tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt" \
VCF_FAST_V17_TRUE_POP_TIERS="10000 50000 100000" \
VCF_FAST_V17_TRUE_POP_GROUPS="AFR:EUR" \
VCF_FAST_V17_TRUE_POP_RUNS=3 \
VCF_FAST_V17_TRUE_POP_WARMUP=1 \
make bench-vcftools-true-popgen
```

## Success Criteria

- Official IGSR metadata is cached reproducibly.
- Metadata helper generates real `AFR` and `EUR` population files without
  unmatched VCF samples.
- True-population benchmark rows are measured for `10k`, `50k`, and `100k`
  actual biallelic records, or the exact blocker is documented.
- Correctness-matched rows update reports and claims.
- No broad VCFtools replacement claim is added.
- LD memory remains a caveat until RSS evidence justifies changing it.
