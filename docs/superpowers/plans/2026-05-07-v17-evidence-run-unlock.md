# v1.7 Evidence Run Unlock Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unlock true human 1000 Genomes / IGSR population-genetics evidence by caching official metadata, generating real population files, and running correctness-gated 10k/50k/100k VCFtools comparisons.

**Architecture:** Keep the existing v1.7 benchmark harness and population helper, but replace the metadata blocker with a pinned official IGSR source. The Python helper becomes the normalization boundary: it accepts official whitespace metadata and legacy TSV metadata, validates sample/population/superpopulation mappings, and writes deterministic population files plus provenance. The shell downloader and benchmark harness only stage data, gate required tools, invoke the helper, and write measured rows when correctness checks pass.

**Tech Stack:** Rust tests, Bash benchmark scripts, Python 3 metadata normalization, `bcftools`, `bgzip`, `hyperfine`, VCFtools 0.1.16, existing VariantFlow release binary.

---

## File Structure

- Modify `benchmark/igsr_population_files.py`
  - Add official whitespace metadata parsing.
  - Add `SampleID`, `Population`, and `Superpopulation` header aliases.
  - Preserve existing tab-delimited metadata support and provenance output.
- Modify `benchmark/download_public_data.sh`
  - Add pinned official 1000 Genomes high-coverage 3202-sample metadata URL.
  - Make `igsr-true-population` download chr22 VCF/TBI plus metadata and write a manifest.
  - Remove the deliberate blocker for true population evidence.
- Modify `benchmark/run_v17_true_population_evidence.sh`
  - Use the new metadata cache path by default.
  - Gate `hyperfine` and the resource runner before timings.
  - Keep report rows pending if external benchmark dependencies are missing.
- Modify `tests/benchmark_harness_tests.rs`
  - Add behavior tests for official whitespace metadata.
  - Update contract tests for the pinned URL, cache path, and unblocked downloader.
- Modify `docs/bioinformatics-workflows.md`
  - Update the true population example to point at the pinned metadata path.
- Modify `benchmark/reports/v17-true-public-population-evidence.md`
  - Keep the report as scaffold or update it with measured rows only when the evidence run succeeds.
- Modify `docs/claim-matrix.md`, `docs/public-benchmark-table.md`, and `README.md`
  - Update only if measured correctness-matched 10k/50k/100k rows are produced.

## Task 1: Add Contract And Behavior Tests For Official IGSR Metadata

**Files:**
- Modify: `tests/benchmark_harness_tests.rs`

- [ ] **Step 1: Add imports needed by the behavior test**

Add `Command` near the existing imports at the top of `tests/benchmark_harness_tests.rs`:

```rust
use std::process::Command;
```

- [ ] **Step 2: Add the official metadata behavior test**

Append this test near `v17_true_public_population_evidence_harness_is_declared`:

```rust
#[test]
fn igsr_population_helper_parses_official_whitespace_metadata() {
    let root = repo_root();
    let tmp = tempfile::tempdir().expect("create tempdir");
    let vcf = tmp.path().join("mini-igsr.vcf");
    let metadata = tmp.path().join("20130606_g1k_3202_samples_ped_population.txt");
    let out_prefix = tmp.path().join("pop/public");

    fs::write(
        &vcf,
        "##fileformat=VCFv4.2\n\
         #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tHG00096\tHG00097\tNA18486\tNA18487\n\
         22\t100\t.\tA\tG\t50\tPASS\t.\tGT\t0/0\t0/1\t1/1\t0/1\n",
    )
    .expect("write mini VCF");
    fs::write(
        &metadata,
        "FamilyID SampleID FatherID MotherID Sex Population Superpopulation\n\
         0 HG00096 0 0 1 GBR EUR\n\
         0 HG00097 0 0 2 GBR EUR\n\
         0 NA18486 0 0 1 YRI AFR\n\
         0 NA18487 0 0 2 YRI AFR\n",
    )
    .expect("write official-style metadata");

    let output = Command::new("python3")
        .current_dir(&root)
        .arg("benchmark/igsr_population_files.py")
        .arg("--vcf")
        .arg(&vcf)
        .arg("--metadata")
        .arg(&metadata)
        .arg("--out-prefix")
        .arg(&out_prefix)
        .arg("--groups")
        .arg("AFR:EUR")
        .output()
        .expect("run population helper");

    assert!(
        output.status.success(),
        "helper failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let afr = fs::read_to_string(tmp.path().join("pop/public.AFR.txt")).expect("read AFR pop");
    let eur = fs::read_to_string(tmp.path().join("pop/public.EUR.txt")).expect("read EUR pop");
    let source = fs::read_to_string(tmp.path().join("pop/public.population-source.tsv"))
        .expect("read population provenance");

    assert_eq!(afr, "NA18486\nNA18487\n");
    assert_eq!(eur, "HG00096\nHG00097\n");
    assert!(source.contains("official IGSR metadata"));
    assert!(source.contains("no header-fallback"));
    assert!(source.contains("metadata_sha256"));
    assert!(source.contains("vcf_sha256"));
}
```

- [ ] **Step 3: Add the unmatched-sample failure behavior test**

Append this test after the previous one:

```rust
#[test]
fn igsr_population_helper_rejects_unmatched_vcf_samples_without_override() {
    let root = repo_root();
    let tmp = tempfile::tempdir().expect("create tempdir");
    let vcf = tmp.path().join("mini-igsr.vcf");
    let metadata = tmp.path().join("metadata.txt");
    let out_prefix = tmp.path().join("pop/public");

    fs::write(
        &vcf,
        "##fileformat=VCFv4.2\n\
         #CHROM\tPOS\tID\tREF\tALT\tQUAL\tFILTER\tINFO\tFORMAT\tHG00096\tMISSING1\n\
         22\t100\t.\tA\tG\t50\tPASS\t.\tGT\t0/0\t0/1\n",
    )
    .expect("write mini VCF");
    fs::write(
        &metadata,
        "FamilyID SampleID FatherID MotherID Sex Population Superpopulation\n\
         0 HG00096 0 0 1 GBR EUR\n",
    )
    .expect("write official-style metadata");

    let output = Command::new("python3")
        .current_dir(&root)
        .arg("benchmark/igsr_population_files.py")
        .arg("--vcf")
        .arg(&vcf)
        .arg("--metadata")
        .arg(&metadata)
        .arg("--out-prefix")
        .arg(&out_prefix)
        .output()
        .expect("run population helper");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("VCF samples are missing from metadata"));
    assert!(stderr.contains("MISSING1"));
    assert!(stderr.contains("--allow-unmatched"));
}
```

- [ ] **Step 4: Tighten the v1.7 contract test**

Inside `v17_true_public_population_evidence_harness_is_declared`, add these assertions after the file reads:

```rust
    assert!(script.contains("igsr-1000g-3202-sample-ped-population.txt"));
    assert!(helper.contains("sampleid"));
    assert!(helper.contains("split_metadata_fields"));
```

Add a downloader read:

```rust
    let downloader = fs::read_to_string(root.join("benchmark/download_public_data.sh"))
        .expect("read public data downloader");
```

Then add these assertions before the report assertions:

```rust
    assert!(downloader.contains(
        "https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000G_2504_high_coverage/20130606_g1k_3202_samples_ped_population.txt"
    ));
    assert!(downloader.contains("igsr-1000g-3202-sample-ped-population.txt"));
    assert!(downloader.contains("download_igsr_true_population"));
    assert!(!downloader.contains(concat!(
        "blocked: choose official IGSR sample ",
        "metadata URL before true population evidence run"
    )));
```

- [ ] **Step 5: Run the targeted test and confirm it fails**

Run:

```bash
cargo test --test benchmark_harness_tests igsr_population_helper_parses_official_whitespace_metadata -- --nocapture
```

Expected: fails because `benchmark/igsr_population_files.py` does not parse whitespace-delimited official metadata yet.

- [ ] **Step 6: Run the updated contract test and confirm it fails**

Run:

```bash
cargo test --test benchmark_harness_tests v17_true_public_population_evidence_harness_is_declared -- --nocapture
```

Expected: fails because the downloader still contains the metadata blocker and the helper does not contain `split_metadata_fields`.

- [ ] **Step 7: Commit the failing tests**

Run:

```bash
git add tests/benchmark_harness_tests.rs
git commit -m "test: require official IGSR population metadata support"
```

Expected: commit succeeds with only the test file staged.

## Task 2: Parse Official Whitespace Metadata In The Population Helper

**Files:**
- Modify: `benchmark/igsr_population_files.py`

- [ ] **Step 1: Add a metadata field splitter**

Add this function after `normalize_header`:

```python
def split_metadata_fields(line: str) -> list[str]:
    stripped = line.rstrip("\n\r")
    if "\t" in stripped:
        return [field.strip() for field in stripped.split("\t")]
    return stripped.split()
```

- [ ] **Step 2: Extend header aliases**

Replace the alias tuples at the top of `read_metadata` with:

```python
    sample_columns = ("sample", "sample_name", "sample_id", "sampleid", "sample_id_1kg")
    population_columns = ("population", "pop", "population_code")
    superpopulation_columns = (
        "superpopulation",
        "super_pop",
        "super_population",
        "superpopulation_code",
    )
```

- [ ] **Step 3: Use the shared splitter for header detection**

Replace:

```python
            candidate_header = [normalize_header(value) for value in stripped.split("\t")]
```

with:

```python
            candidate_header = [normalize_header(value) for value in split_metadata_fields(stripped)]
```

- [ ] **Step 4: Use the shared splitter for data rows**

Replace:

```python
            fields = [field.strip() for field in line.rstrip("\n").split("\t")]
```

with:

```python
            fields = split_metadata_fields(line)
```

- [ ] **Step 5: Run Python syntax validation**

Run:

```bash
python3 -m py_compile benchmark/igsr_population_files.py
```

Expected: exits with code `0`.

- [ ] **Step 6: Run the helper behavior tests**

Run:

```bash
cargo test --test benchmark_harness_tests igsr_population_helper_ -- --nocapture
```

Expected: both helper tests pass.

- [ ] **Step 7: Commit the helper change**

Run:

```bash
git add benchmark/igsr_population_files.py
git commit -m "fix: parse official IGSR population metadata"
```

Expected: commit succeeds with the helper staged.

## Task 3: Replace The True-Population Downloader Blocker

**Files:**
- Modify: `benchmark/download_public_data.sh`

- [ ] **Step 1: Add official metadata constants**

Add these constants immediately after `IGSR_CHR22_TBI_URL`:

```bash
IGSR_3202_METADATA_URL="https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000G_2504_high_coverage/20130606_g1k_3202_samples_ped_population.txt"
IGSR_3202_METADATA_FILE="igsr-1000g-3202-sample-ped-population.txt"
```

- [ ] **Step 2: Replace `download_igsr_true_population`**

Replace the whole function with:

```bash
download_igsr_true_population() {
  local metadata_path="$OUT_DIR/$IGSR_3202_METADATA_FILE"
  local vcf_path="$OUT_DIR/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz"
  local manifest="$OUT_DIR/igsr-true-population-manifest.env"

  download_igsr_chr22
  download_if_missing "$IGSR_3202_METADATA_URL" "$metadata_path"

  cat >"$manifest" <<EOF
VCF_FAST_V17_TRUE_POP_INPUT="$vcf_path"
VCF_FAST_V17_TRUE_POP_METADATA="$metadata_path"
IGSR_CHR22_URL="$IGSR_CHR22_URL"
IGSR_3202_METADATA_URL="$IGSR_3202_METADATA_URL"
EOF

  cat <<EOF
True population evidence inputs cached:
  VCF_FAST_V17_TRUE_POP_INPUT="$vcf_path"
  VCF_FAST_V17_TRUE_POP_METADATA="$metadata_path"
  manifest="$manifest"
EOF
}
```

- [ ] **Step 3: Add the metadata URL to the final source summary**

Add this line before the final `EOF` in the end-of-script source summary:

```bash
1000 Genomes 3202-sample population metadata: $IGSR_3202_METADATA_URL
```

- [ ] **Step 4: Run shell syntax validation**

Run:

```bash
bash -n benchmark/download_public_data.sh
```

Expected: exits with code `0`.

- [ ] **Step 5: Run the updated contract test**

Run:

```bash
cargo test --test benchmark_harness_tests v17_true_public_population_evidence_harness_is_declared -- --nocapture
```

Expected: still fails only if the harness default metadata path has not yet been updated. If the test failure is from `igsr-1000g-3202-sample-ped-population.txt` missing in the harness, continue to Task 4.

- [ ] **Step 6: Commit the downloader change**

Run:

```bash
git add benchmark/download_public_data.sh
git commit -m "bench: cache official IGSR population metadata"
```

Expected: commit succeeds with the downloader staged.

## Task 4: Point The True-Population Harness At The Official Metadata Cache

**Files:**
- Modify: `benchmark/run_v17_true_population_evidence.sh`

- [ ] **Step 1: Update the default metadata path**

Replace:

```bash
PUBLIC_METADATA="${VCF_FAST_V17_TRUE_POP_METADATA:-tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt}"
```

with:

```bash
PUBLIC_METADATA="${VCF_FAST_V17_TRUE_POP_METADATA:-tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt}"
```

- [ ] **Step 2: Add a `hyperfine` gate**

After the existing `bcftools` and `bgzip` gate, insert:

```bash
if ! command -v hyperfine >/dev/null 2>&1; then
  write_blocked_report "True public cohort timings require hyperfine before benchmark rows can be generated."
  echo "True public population evidence blocked; report written to $REPORT"
  exit 77
fi
```

- [ ] **Step 3: Add a resource-runner gate**

After the new `hyperfine` gate, insert:

```bash
if ! $RESOURCE_RUNNER --help >/dev/null 2>&1; then
  write_blocked_report "True public cohort resource metrics require $RESOURCE_RUNNER before benchmark rows can be generated."
  echo "True public population evidence blocked; report written to $REPORT"
  exit 77
fi
```

- [ ] **Step 4: Make the blocked report name the new default path**

Inside `write_blocked_report`, replace the first paragraph after `Status: blocked` with:

```markdown
Set `VCF_FAST_V17_TRUE_POP_INPUT` to a cached 1000 Genomes / IGSR VCF/BCF and
`VCF_FAST_V17_TRUE_POP_METADATA` to official population metadata with sample,
population, and superpopulation columns. The default metadata cache path is
`tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt`.
```

- [ ] **Step 5: Run shell syntax validation**

Run:

```bash
bash -n benchmark/run_v17_true_population_evidence.sh
```

Expected: exits with code `0`.

- [ ] **Step 6: Run the updated contract test**

Run:

```bash
cargo test --test benchmark_harness_tests v17_true_public_population_evidence_harness_is_declared -- --nocapture
```

Expected: passes.

- [ ] **Step 7: Run a missing-input blocked smoke**

Run:

```bash
VCF_FAST_V17_TRUE_POP_INPUT=/tmp/variantflow-missing.vcf.gz \
VCF_FAST_V17_TRUE_POP_METADATA=/tmp/variantflow-missing-metadata.txt \
VCF_FAST_V17_TRUE_POP_REPORT=tests/output/v17-true-population-evidence/blocked-smoke.md \
make bench-vcftools-true-popgen
```

Expected: exits `77`, prints that true public evidence is blocked, and writes `tests/output/v17-true-population-evidence/blocked-smoke.md` with no measured claims.

- [ ] **Step 8: Commit the harness change**

Run:

```bash
git add benchmark/run_v17_true_population_evidence.sh tests/benchmark_harness_tests.rs
git commit -m "bench: point true population harness at official IGSR metadata"
```

Expected: commit succeeds with only the harness and tests staged.

## Task 5: Update User-Facing Workflow Documentation

**Files:**
- Modify: `docs/bioinformatics-workflows.md`

- [ ] **Step 1: Update the true population benchmark command**

Replace the metadata path in the true public VCFtools evidence example with:

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

- [ ] **Step 2: Add the source sentence**

Add this sentence immediately after the command block:

```markdown
The `igsr-true-population` downloader caches the pinned 1000 Genomes high-coverage 3202-sample metadata file `20130606_g1k_3202_samples_ped_population.txt` and the chr22 VCF/TBI under `tests/output/public-data`, which remains ignored by git.
```

- [ ] **Step 3: Check the documentation does not reference the old metadata path**

Run the stale-token search for the retired sample-metadata filename and the
retired metadata-blocker wording across `docs`, `benchmark`, and `tests`.
Expected: no matches.

- [ ] **Step 4: Commit documentation**

Run:

```bash
git add docs/bioinformatics-workflows.md
git commit -m "docs: document official IGSR metadata workflow"
```

Expected: commit succeeds with the workflow doc staged.

## Task 6: Download Public Inputs And Smoke The True Population Path

**Files:**
- Uses ignored paths under `tests/output/public-data` and `tests/output/v17-true-population-evidence`
- May update `benchmark/reports/v17-true-public-population-evidence.md` only with measured rows or a clear blocked report

- [ ] **Step 1: Cache the official true-population inputs**

Run:

```bash
benchmark/download_public_data.sh igsr-true-population
```

Expected: downloads or reuses:

```text
tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz
tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz.tbi
tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt
tests/output/public-data/igsr-true-population-manifest.env
```

- [ ] **Step 2: Verify the official metadata header and superpopulation labels**

Run:

```bash
head -n 3 tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt
awk 'NR>1 { counts[$7]++ } END { for (label in counts) print label, counts[label] }' tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt | sort
```

Expected header:

```text
FamilyID SampleID FatherID MotherID Sex Population Superpopulation
```

Expected labels include:

```text
AFR
AMR
EAS
EUR
SAS
```

- [ ] **Step 3: Run a 100-record smoke benchmark**

Run:

```bash
VCF_FAST_V17_TRUE_POP_INPUT="tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz" \
VCF_FAST_V17_TRUE_POP_METADATA="tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt" \
VCF_FAST_V17_TRUE_POP_TIERS="100" \
VCF_FAST_V17_TRUE_POP_RUNS=1 \
VCF_FAST_V17_TRUE_POP_WARMUP=0 \
VCF_FAST_V17_TRUE_POP_REPORT="tests/output/v17-true-population-evidence/smoke.md" \
make bench-vcftools-true-popgen
```

Expected:
- If `vcftools`, `bcftools`, `bgzip`, `hyperfine`, and the resource runner are installed, the command exits `0` and writes measured smoke rows.
- If a benchmark dependency is missing, the command exits `77` and writes a blocked report naming the missing dependency.

- [ ] **Step 4: Inspect smoke output for population provenance**

Run:

```bash
rg "official IGSR metadata|no header-fallback|AFR:EUR|actual record count|CPU-hour estimate" tests/output/v17-true-population-evidence/smoke.md
```

Expected: each string is present if the smoke report exists.

- [ ] **Step 5: Commit no generated public data**

Run:

```bash
git status --short
```

Expected: no `tests/output/public-data` files are tracked because benchmark artifacts remain ignored. If `benchmark/reports/v17-true-public-population-evidence.md` changed only to a blocked report from the smoke run, leave it unstaged until Task 7 determines whether real measured rows are available.

## Task 7: Run Full True Public Population Evidence And Update Claims Only From Measured Rows

**Files:**
- May modify: `benchmark/reports/v17-true-public-population-evidence.md`
- May modify: `docs/public-benchmark-table.md`
- May modify: `docs/claim-matrix.md`
- May modify: `README.md`

- [ ] **Step 1: Run the full 10k/50k/100k evidence command**

Run:

```bash
VCF_FAST_V17_TRUE_POP_INPUT="tests/output/public-data/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz" \
VCF_FAST_V17_TRUE_POP_METADATA="tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt" \
VCF_FAST_V17_TRUE_POP_TIERS="10000 50000 100000" \
VCF_FAST_V17_TRUE_POP_GROUPS="AFR:EUR" \
VCF_FAST_V17_TRUE_POP_RUNS=3 \
VCF_FAST_V17_TRUE_POP_WARMUP=1 \
make bench-vcftools-true-popgen
```

Expected:
- Exit `0` means measured rows were written to `benchmark/reports/v17-true-public-population-evidence.md`.
- Exit `77` means at least one required local benchmark dependency is missing; keep the tracked report in blocked/scaffold form and do not add performance claims.
- Any other non-zero exit means a correctness or staging failure; preserve the error output in the final report and do not add performance claims.

- [ ] **Step 2: Confirm correctness text and measured fields**

Run:

```bash
rg "correctness result|passed: make vcftools-parity|runtime mean|speedup|peak RSS KB|CPU-hour estimate|official IGSR metadata|no header-fallback" benchmark/reports/v17-true-public-population-evidence.md
```

Expected: all strings are present in the report.

- [ ] **Step 3: Confirm measured tiers exist before claim updates**

Run:

```bash
rg "\\| public cohort (10000|50000|100000) \\|" benchmark/reports/v17-true-public-population-evidence.md
```

Expected: rows exist for all three tiers if the full run completed. If the command shows only pending rows, stop this task after committing implementation changes and report the blocker.

- [ ] **Step 4: Regenerate the public benchmark table when measured rows exist**

Run:

```bash
make benchmark-table
```

Expected: `docs/public-benchmark-table.md` updates only from tracked benchmark reports.

- [ ] **Step 5: Update the claim matrix when measured rows exist**

Edit `docs/claim-matrix.md` so the VCFtools population-genetics section says:

```markdown
VariantFlow matches VCFtools on supported diploid biallelic parity fixtures and has true public 1000 Genomes / IGSR evidence rows for frequency, missingness, HWE, heterozygosity, site pi, window pi, Tajima's D, LD, and Weir-Cockerham Fst. Performance statements are limited to rows in `benchmark/reports/v17-true-public-population-evidence.md`; LD memory remains a monitored optimization target.
```

- [ ] **Step 6: Update README only if measured rows exist**

Add one cautious sentence to the Current Evidence section:

```markdown
VCFtools-style population summaries now include true public 1000 Genomes / IGSR rows with official AFR/EUR population files; claims remain scoped to the measured rows in `benchmark/reports/v17-true-public-population-evidence.md`.
```

- [ ] **Step 7: Commit measured report and claim updates**

If the full run completed with measured rows, run:

```bash
git add benchmark/reports/v17-true-public-population-evidence.md docs/public-benchmark-table.md docs/claim-matrix.md README.md
git commit -m "bench: add true public VCFtools population evidence"
```

Expected: commit succeeds with measured report and claim files staged. If the run exited `77`, skip this commit and document the missing dependency in the final response.

## Task 8: Final Verification And Branch Hygiene

**Files:**
- No new files expected beyond implementation, docs, and measured reports.

- [ ] **Step 1: Run full local verification**

Run:

```bash
make verify
```

Expected: exits with code `0`.

- [ ] **Step 2: Run htslib feature tests**

Run:

```bash
cargo test --features htslib-static
```

Expected: exits with code `0`.

- [ ] **Step 3: Run htslib clippy**

Run:

```bash
cargo clippy --features htslib-static --all-targets -- -D warnings
```

Expected: exits with code `0`.

- [ ] **Step 4: Check generated artifacts are not tracked**

Run:

```bash
git status --short --ignored | sed -n '1,120p'
```

Expected:
- tracked changes are either absent or limited to intentional source/docs/report files.
- ignored benchmark artifacts appear under `tests/output/...` only.

- [ ] **Step 5: Commit verification-only fixups if needed**

If verification required code or docs changes, commit them with:

```bash
git add benchmark/igsr_population_files.py benchmark/download_public_data.sh benchmark/run_v17_true_population_evidence.sh tests/benchmark_harness_tests.rs docs/bioinformatics-workflows.md benchmark/reports/v17-true-public-population-evidence.md docs/public-benchmark-table.md docs/claim-matrix.md README.md
git commit -m "fix: stabilize true public population evidence unlock"
```

Expected: commit succeeds only if verification produced intentional tracked changes.

- [ ] **Step 6: Final status**

Run:

```bash
git status --short --branch
```

Expected: branch is clean except ignored benchmark artifacts.

## Self-Review Checklist

- Spec coverage:
  - Official 1000 Genomes metadata URL is added to the downloader.
  - Official whitespace metadata is parsed by the helper.
  - Legacy TSV metadata remains supported by `split_metadata_fields`.
  - The true-population harness defaults to the official cache path.
  - Missing benchmark dependencies exit with blocked reports instead of measured claims.
  - 10k/50k/100k evidence run is specified with correctness gates.
  - Claims are updated only after measured correctness-matched rows exist.
- Placeholder scan:
  - This plan uses exact file paths, commands, commit messages, snippets, and expected results.
  - No unspecified implementation steps remain.
- Type consistency:
  - New helper function is consistently named `split_metadata_fields`.
  - The default cache path is consistently `tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt`.
  - The official metadata URL is consistently `https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000G_2504_high_coverage/20130606_g1k_3202_samples_ped_population.txt`.
