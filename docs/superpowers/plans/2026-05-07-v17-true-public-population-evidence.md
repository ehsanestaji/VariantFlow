# VariantFlow v1.7 True Public Population Evidence Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a reproducible true-public-population VCFtools evidence harness using official 1000 Genomes / IGSR population metadata and actual `10k`, `50k`, and `100k` biallelic-site tiers before any release claim.

**Architecture:** Add a new benchmark slice instead of weakening the current smoke/staged VCFtools harness. The new slice stages biallelic public human cohorts from a cached IGSR/1000G VCF, creates real population files from official sample metadata, runs VariantFlow and VCFtools workflows with resource metrics, and writes a report whose rows separate correctness, runtime, RSS, CPU seconds, and caveats. LD memory and VCFtools edge-semantics work are tracked as implementation tasks with tests before optimization claims.

**Tech Stack:** Rust CLI (`variantflow`), Bash benchmark harnesses, Python metadata/normalization helpers, VCFtools 0.1.16, bcftools/bgzip/tabix for staging, hyperfine for repeated timing, GNU time/resource helper where available.

---

## File Structure

- Create `benchmark/run_v17_true_population_evidence.sh`: new human-first VCFtools public-population benchmark harness.
- Create `benchmark/igsr_population_files.py`: parse official sample metadata and write deterministic population files.
- Create `benchmark/reports/v17-true-public-population-evidence.md`: tracked report scaffold and later measured rows.
- Modify `Makefile`: add `bench-vcftools-true-popgen` and syntax check for the new shell script.
- Modify `tests/benchmark_harness_tests.rs`: assert the new harness, report fields, metadata helper, real-population policy, and no broad replacement claim.
- Modify `tests/popgen_cli_tests.rs`: add edge-semantics tests for missing genotypes, keep/remove interactions, multiallelic rejection, and window boundaries.
- Optionally modify `src/engine/popgen.rs`: only if new edge tests reveal a VCFtools parity mismatch.
- Modify `docs/claim-matrix.md`: add a pending v1.7 true-population evidence row that makes no speed claim until measurements exist.
- Modify `docs/public-benchmark-table.md`: only after measured rows exist; do not update in the harness-only slice.

---

### Task 1: Add Harness Contract Tests

**Files:**
- Modify: `tests/benchmark_harness_tests.rs`
- Later create: `benchmark/run_v17_true_population_evidence.sh`
- Later create: `benchmark/igsr_population_files.py`
- Later create: `benchmark/reports/v17-true-public-population-evidence.md`
- Later modify: `Makefile`

- [ ] **Step 1: Add the failing benchmark harness test**

Append this test near the existing VCFtools benchmark tests in `tests/benchmark_harness_tests.rs`:

```rust
#[test]
fn v17_true_public_population_evidence_harness_is_declared() {
    let root = repo_root();
    let makefile = fs::read_to_string(root.join("Makefile")).expect("read Makefile");
    let script = fs::read_to_string(root.join("benchmark/run_v17_true_population_evidence.sh"))
        .expect("read v1.7 true public population benchmark script");
    let helper = fs::read_to_string(root.join("benchmark/igsr_population_files.py"))
        .expect("read IGSR population metadata helper");
    let report = fs::read_to_string(
        root.join("benchmark/reports/v17-true-public-population-evidence.md"),
    )
    .expect("read v1.7 true public population report");

    assert!(makefile.contains("bench-vcftools-true-popgen:"));
    assert!(makefile.contains("run_v17_true_population_evidence.sh"));
    assert!(makefile.contains("bash -n benchmark/run_v17_true_population_evidence.sh"));
    assert!(makefile.contains("python3 -m py_compile benchmark/igsr_population_files.py"));

    for required in [
        "VCF_FAST_V17_TRUE_POP_INPUT",
        "VCF_FAST_V17_TRUE_POP_METADATA",
        "VCF_FAST_V17_TRUE_POP_TIERS",
        "10000 50000 100000",
        "VCF_FAST_V17_TRUE_POP_GROUPS",
        "AFR:EUR",
        "prepare_true_public_biallelic_dataset",
        "bcftools view -m2 -M2 -v snps",
        "actual_records",
        "igsr_population_files.py",
        "population metadata source",
        "official",
        "no header-fallback",
        "frequency",
        "missingness",
        "HWE",
        "heterozygosity",
        "site pi",
        "window pi",
        "Tajima's D",
        "LD",
        "Weir-Cockerham Fst",
        "peak RSS KB",
        "CPU seconds",
        "CPU-hour estimate",
        "This report does not support a broad VCFtools replacement claim",
    ] {
        assert!(script.contains(required), "missing script text {required}");
    }

    for required in [
        "sample",
        "population",
        "superpopulation",
        "write_population_files",
        "unmatched samples",
        "AFR",
        "EUR",
        "EAS",
        "SAS",
        "AMR",
    ] {
        assert!(helper.contains(required), "missing helper text {required}");
    }

    for required in [
        "VariantFlow v1.7 True Public Population Evidence",
        "1000 Genomes / IGSR",
        "actual record count",
        "official population metadata",
        "population metadata source",
        "peak RSS KB",
        "CPU seconds",
        "CPU-hour estimate",
        "no broad VCFtools replacement claim",
    ] {
        assert!(report.contains(required), "missing report text {required}");
    }
}
```

- [ ] **Step 2: Run the focused failing test**

Run:

```bash
cargo test --test benchmark_harness_tests v17_true_public_population_evidence_harness_is_declared
```

Expected: FAIL because the new script, helper, and report do not exist yet.

- [ ] **Step 3: Commit the failing test**

```bash
git add tests/benchmark_harness_tests.rs
git commit -m "test: require true public population evidence harness"
```

---

### Task 2: Add IGSR Population Metadata Helper

**Files:**
- Create: `benchmark/igsr_population_files.py`
- Test: `tests/benchmark_harness_tests.rs`

- [ ] **Step 1: Create the metadata helper**

Create `benchmark/igsr_population_files.py` with this content:

```python
#!/usr/bin/env python3
"""Create VariantFlow/VCFtools population files from official IGSR metadata."""

from __future__ import annotations

import argparse
import gzip
from collections import defaultdict
from pathlib import Path


def open_text(path: Path):
    if path.suffix == ".gz":
        return gzip.open(path, "rt", encoding="utf-8")
    return path.open("rt", encoding="utf-8")


def read_vcf_samples(vcf: Path) -> list[str]:
    with open_text(vcf) as handle:
        for line in handle:
            if line.startswith("#CHROM"):
                return line.rstrip("\n").split("\t")[9:]
    raise SystemExit(f"{vcf} has no #CHROM header")


def normalize_header(name: str) -> str:
    return name.strip().lower().replace(" ", "_").replace("-", "_")


def read_metadata(path: Path) -> dict[str, tuple[str, str]]:
    with path.open("rt", encoding="utf-8") as handle:
        header = [normalize_header(value) for value in handle.readline().rstrip("\n").split("\t")]
        sample_columns = ("sample", "sample_name", "sample_id", "sample_id_1kg")
        population_columns = ("population", "pop", "population_code")
        superpopulation_columns = ("superpopulation", "super_pop", "super_population", "superpopulation_code")

        def choose(candidates: tuple[str, ...]) -> int:
            for candidate in candidates:
                if candidate in header:
                    return header.index(candidate)
            raise SystemExit(f"{path} header must contain one of {candidates}; found {header}")

        sample_i = choose(sample_columns)
        pop_i = choose(population_columns)
        super_i = choose(superpopulation_columns)

        labels: dict[str, tuple[str, str]] = {}
        for row_number, line in enumerate(handle, start=2):
            if not line.strip():
                continue
            fields = line.rstrip("\n").split("\t")
            if len(fields) <= max(sample_i, pop_i, super_i):
                raise SystemExit(f"{path} row {row_number} is missing required metadata fields")
            sample = fields[sample_i]
            population = fields[pop_i]
            superpopulation = fields[super_i]
            if sample in labels:
                raise SystemExit(f"{path} row {row_number} duplicates sample {sample!r}")
            labels[sample] = (population, superpopulation)
    return labels


def group_samples(
    vcf_samples: list[str],
    labels: dict[str, tuple[str, str]],
    group_level: str,
) -> tuple[dict[str, list[str]], list[str]]:
    index = 1 if group_level == "superpopulation" else 0
    groups: dict[str, list[str]] = defaultdict(list)
    unmatched: list[str] = []
    for sample in vcf_samples:
        label = labels.get(sample)
        if label is None:
            unmatched.append(sample)
            continue
        groups[label[index]].append(sample)
    return dict(groups), unmatched


def write_population_files(
    groups: dict[str, list[str]],
    group_pair: str,
    out_prefix: Path,
    min_samples: int,
) -> tuple[Path, Path, str, str]:
    try:
        left_label, right_label = group_pair.split(":", 1)
    except ValueError as error:
        raise SystemExit("--groups must use LABEL1:LABEL2 syntax, for example AFR:EUR") from error
    left = groups.get(left_label, [])
    right = groups.get(right_label, [])
    if len(left) < min_samples or len(right) < min_samples:
        raise SystemExit(
            f"group pair {group_pair} has too few samples: "
            f"{left_label}={len(left)}, {right_label}={len(right)}, min={min_samples}"
        )
    out_prefix.parent.mkdir(parents=True, exist_ok=True)
    pop1 = Path(f"{out_prefix}.{left_label}.txt")
    pop2 = Path(f"{out_prefix}.{right_label}.txt")
    pop1.write_text("\n".join(left) + "\n", encoding="utf-8")
    pop2.write_text("\n".join(right) + "\n", encoding="utf-8")
    return pop1, pop2, left_label, right_label


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--vcf", required=True, type=Path)
    parser.add_argument("--metadata", required=True, type=Path)
    parser.add_argument("--out-prefix", required=True, type=Path)
    parser.add_argument("--groups", default="AFR:EUR")
    parser.add_argument("--group-level", choices=("population", "superpopulation"), default="superpopulation")
    parser.add_argument("--min-samples", type=int, default=2)
    args = parser.parse_args()

    samples = read_vcf_samples(args.vcf)
    metadata = read_metadata(args.metadata)
    grouped, unmatched = group_samples(samples, metadata, args.group_level)
    pop1, pop2, left_label, right_label = write_population_files(
        grouped, args.groups, args.out_prefix, args.min_samples
    )
    source = Path(f"{args.out_prefix}.population-source.tsv")
    source.write_text(
        "population_file\tlabel\tlevel\tsource\tsample_count\n"
        f"{pop1}\t{left_label}\t{args.group_level}\tofficial IGSR metadata\t{len(grouped[left_label])}\n"
        f"{pop2}\t{right_label}\t{args.group_level}\tofficial IGSR metadata\t{len(grouped[right_label])}\n"
        f"unmatched samples\t.\t.\tofficial IGSR metadata\t{len(unmatched)}\n",
        encoding="utf-8",
    )
    print(f"{pop1}\t{pop2}\t{source}\tofficial IGSR metadata; no header-fallback")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
```

- [ ] **Step 2: Compile-check the helper**

Run:

```bash
python3 -m py_compile benchmark/igsr_population_files.py
```

Expected: PASS with no output.

- [ ] **Step 3: Run the harness contract test**

Run:

```bash
cargo test --test benchmark_harness_tests v17_true_public_population_evidence_harness_is_declared
```

Expected: still FAIL because the shell script, Make target, and report do not exist yet.

- [ ] **Step 4: Commit the helper**

```bash
git add benchmark/igsr_population_files.py
git commit -m "bench: add IGSR population metadata helper"
```

---

### Task 3: Add True Public Population Harness And Report Scaffold

**Files:**
- Create: `benchmark/run_v17_true_population_evidence.sh`
- Create: `benchmark/reports/v17-true-public-population-evidence.md`
- Modify: `Makefile`
- Test: `tests/benchmark_harness_tests.rs`

- [ ] **Step 1: Create the shell harness**

Create `benchmark/run_v17_true_population_evidence.sh` by copying the structure of `benchmark/run_vcftools_population_benchmarks.sh`, then make these concrete changes:

```bash
OUT_DIR="${VCF_FAST_V17_TRUE_POP_OUT_DIR:-tests/output/v17-true-population-evidence}"
REPORT="${VCF_FAST_V17_TRUE_POP_REPORT:-benchmark/reports/v17-true-public-population-evidence.md}"
PUBLIC_INPUT="${VCF_FAST_V17_TRUE_POP_INPUT:-}"
PUBLIC_METADATA="${VCF_FAST_V17_TRUE_POP_METADATA:-tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt}"
PUBLIC_TIERS="${VCF_FAST_V17_TRUE_POP_TIERS:-10000 50000 100000}"
PUBLIC_GROUPS="${VCF_FAST_V17_TRUE_POP_GROUPS:-AFR:EUR}"
PUBLIC_GROUP_LEVEL="${VCF_FAST_V17_TRUE_POP_GROUP_LEVEL:-superpopulation}"
POPULATION_HELPER="${VCF_FAST_V17_TRUE_POP_HELPER:-python3 benchmark/igsr_population_files.py}"
RESOURCE_RUNNER="${VCF_FAST_RESOURCE_RUNNER:-python3 benchmark/command_resource_metrics.py}"
RUNS="${VCF_FAST_V17_TRUE_POP_RUNS:-3}"
WARMUP="${VCF_FAST_V17_TRUE_POP_WARMUP:-1}"
```

Add this staging function to ensure actual tier counts are recorded:

```bash
actual_records() {
  stream_vcf_text "$1" | awk 'BEGIN { n=0 } /^#/ { next } { n++ } END { print n }'
}

prepare_true_public_biallelic_dataset() {
  local input="$1" output="$2" tier_limit="$3"
  if ! command -v bcftools >/dev/null 2>&1 || ! command -v bgzip >/dev/null 2>&1; then
    echo "blocked: true public cohort staging requires bcftools and bgzip" >&2
    return 1
  fi

  mkdir -p "$(dirname "$output")"
  local tmp_output
  tmp_output="$(mktemp "${output}.tmp.XXXXXX")"
  set +o pipefail
  bcftools view -m2 -M2 -v snps "$input" \
    | awk -v limit="$tier_limit" '
      /^#/ { print; next }
      seen < limit { print; seen++ }
      seen >= limit { exit }
    ' \
    | bgzip -c >"$tmp_output"
  local statuses=("${PIPESTATUS[@]}")
  set -o pipefail
  if [[ "${statuses[0]}" -ne 0 && "${statuses[0]}" -ne 141 ]]; then
    echo "bcftools view -m2 -M2 -v snps failed while staging true public biallelic dataset" >&2
    rm -f "$tmp_output" "$output"
    return 1
  fi
  if [[ "${statuses[1]}" -ne 0 || "${statuses[2]}" -ne 0 ]]; then
    echo "failed to write true public biallelic dataset" >&2
    rm -f "$tmp_output" "$output"
    return 1
  fi
  mv -f "$tmp_output" "$output"
  printf "%s" "$output"
}
```

Add this population-file call; do not use header fallback:

```bash
public_population_files() {
  local tier_input="$1" tier_limit="$2"
  $POPULATION_HELPER \
    --vcf "$tier_input" \
    --metadata "$PUBLIC_METADATA" \
    --groups "$PUBLIC_GROUPS" \
    --group-level "$PUBLIC_GROUP_LEVEL" \
    --out-prefix "$OUT_DIR/public-cohort-$tier_limit"
}
```

Add a blocker report path when `PUBLIC_INPUT` or `PUBLIC_METADATA` is missing:

```bash
if [[ -z "$PUBLIC_INPUT" || ! -f "$PUBLIC_INPUT" || ! -f "$PUBLIC_METADATA" ]]; then
  cat >"$REPORT" <<EOF
# VariantFlow v1.7 True Public Population Evidence

Status: blocked. Set \`VCF_FAST_V17_TRUE_POP_INPUT\` to a cached 1000 Genomes /
IGSR VCF/BCF and \`VCF_FAST_V17_TRUE_POP_METADATA\` to official sample metadata
with sample, population, and superpopulation columns.

This report requires official population metadata, actual record count fields,
peak RSS KB, CPU seconds, CPU-hour estimate, and correctness results before any
performance claim. It uses no header-fallback population files.

| tier | case | actual record count | sample count | population metadata source | runtime mean | speedup | VariantFlow peak RSS KB | VCFtools peak RSS KB | VariantFlow CPU seconds | VCFtools CPU seconds | VariantFlow CPU-hour estimate | VCFtools CPU-hour estimate | correctness result | caveats |
|---|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| public cohort 10000 | all cases | pending | pending | official population metadata required | pending | pending | pending | pending | pending | pending | pending | pending | pending | blocked: set VCF_FAST_V17_TRUE_POP_INPUT and VCF_FAST_V17_TRUE_POP_METADATA |
| public cohort 50000 | all cases | pending | pending | official population metadata required | pending | pending | pending | pending | pending | pending | pending | pending | pending | blocked: set VCF_FAST_V17_TRUE_POP_INPUT and VCF_FAST_V17_TRUE_POP_METADATA |
| public cohort 100000 | all cases | pending | pending | official population metadata required | pending | pending | pending | pending | pending | pending | pending | pending | pending | blocked: set VCF_FAST_V17_TRUE_POP_INPUT and VCF_FAST_V17_TRUE_POP_METADATA |

Claim decision: no broad VCFtools replacement claim.
EOF
  exit 77
fi
```

For measured rows, reuse `benchmark_pair` and `run_tier` from the existing VCFtools population harness, but change each public-tier caveat to include:

```bash
"true public biallelic human cohort; population metadata source: official IGSR metadata; no header-fallback; requested tier $tier_limit; actual records $actual_count"
```

- [ ] **Step 2: Add the report scaffold**

Create `benchmark/reports/v17-true-public-population-evidence.md`:

```markdown
# VariantFlow v1.7 True Public Population Evidence

Status: scaffold. Run `make bench-vcftools-true-popgen` with
`VCF_FAST_V17_TRUE_POP_INPUT` and `VCF_FAST_V17_TRUE_POP_METADATA` pointing to a
cached 1000 Genomes / IGSR cohort and official metadata.

This report requires official population metadata, actual record count, sample
count, runtime mean/stddev where available, peak RSS KB, CPU seconds, CPU-hour
estimate, exact commands, VCFtools version, correctness result, and caveats.
The harness uses no header-fallback population files.

| tier | case | actual record count | sample count | population metadata source | runtime mean | speedup | VariantFlow peak RSS KB | VCFtools peak RSS KB | VariantFlow CPU seconds | VCFtools CPU seconds | VariantFlow CPU-hour estimate | VCFtools CPU-hour estimate | correctness result | caveats |
|---|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| public cohort 10000 | all cases | pending | pending | official population metadata required | pending | pending | pending | pending | pending | pending | pending | pending | pending | not measured |
| public cohort 50000 | all cases | pending | pending | official population metadata required | pending | pending | pending | pending | pending | pending | pending | pending | pending | not measured |
| public cohort 100000 | all cases | pending | pending | official population metadata required | pending | pending | pending | pending | pending | pending | pending | pending | pending | not measured |

Claim decision: no broad VCFtools replacement claim. Only correctness-matched
measured rows may support scoped performance claims.
```

- [ ] **Step 3: Wire Makefile**

Modify the `.PHONY` line in `Makefile` to include:

```make
bench-vcftools-true-popgen
```

Add syntax and compile checks under `verify` where the other benchmark scripts are checked:

```make
	bash -n benchmark/run_v17_true_population_evidence.sh
	python3 -m py_compile benchmark/igsr_population_files.py
```

Add the target:

```make
bench-vcftools-true-popgen:
	./benchmark/run_v17_true_population_evidence.sh
```

- [ ] **Step 4: Run syntax and contract tests**

Run:

```bash
bash -n benchmark/run_v17_true_population_evidence.sh
python3 -m py_compile benchmark/igsr_population_files.py
cargo test --test benchmark_harness_tests v17_true_public_population_evidence_harness_is_declared
```

Expected: PASS.

- [ ] **Step 5: Commit the harness**

```bash
git add Makefile benchmark/run_v17_true_population_evidence.sh benchmark/igsr_population_files.py benchmark/reports/v17-true-public-population-evidence.md tests/benchmark_harness_tests.rs
git commit -m "bench: add true public population evidence harness"
```

---

### Task 4: Add Human Dataset Download/Metadata Notes

**Files:**
- Modify: `benchmark/download_public_data.sh`
- Modify: `docs/bioinformatics-workflows.md`
- Test: `tests/benchmark_harness_tests.rs`

- [ ] **Step 1: Add a downloader contract test**

Append to `tests/benchmark_harness_tests.rs`:

```rust
#[test]
fn true_population_downloader_documents_igsr_metadata_inputs() {
    let root = repo_root();
    let downloader = fs::read_to_string(root.join("benchmark/download_public_data.sh"))
        .expect("read public data downloader");
    let workflows = fs::read_to_string(root.join("docs/bioinformatics-workflows.md"))
        .expect("read workflow docs");

    for required in [
        "igsr-true-population",
        "1000 Genomes",
        "IGSR",
        "sample metadata",
        "population",
        "superpopulation",
        "VCF_FAST_V17_TRUE_POP_INPUT",
        "VCF_FAST_V17_TRUE_POP_METADATA",
    ] {
        assert!(downloader.contains(required) || workflows.contains(required), "missing {required}");
    }
}
```

- [ ] **Step 2: Run the failing test**

```bash
cargo test --test benchmark_harness_tests true_population_downloader_documents_igsr_metadata_inputs
```

Expected: FAIL until downloader/docs are updated.

- [ ] **Step 3: Update the downloader with an explicit target**

In `benchmark/download_public_data.sh`, add a target named `igsr-true-population` that prints or downloads the selected official sources. Use guarded download style already present in the file. The target must write large files under `tests/output/public-data` and document these variables:

```bash
VCF_FAST_V17_TRUE_POP_INPUT="tests/output/public-data/<cached-igsr-vcf>.vcf.gz"
VCF_FAST_V17_TRUE_POP_METADATA="tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt"
```

If the final official metadata URL is not yet chosen, make the target exit `77` with this exact message:

```bash
printf '%s%s\n' "blocked: choose official IGSR sample " \
  "metadata URL before true population evidence run" >&2
exit 77
```

This is acceptable because it blocks claims instead of fabricating metadata.

- [ ] **Step 4: Add workflow docs**

Append to `docs/bioinformatics-workflows.md`:

```markdown
## True Public Population Evidence

The serious VCFtools-replacement benchmark requires a cached 1000 Genomes / IGSR
VCF and official sample metadata with `sample`, `population`, and
`superpopulation` columns. Header-derived fallback groups are validation-only
and must not support public population-genetics claims.

```bash
benchmark/download_public_data.sh igsr-true-population
VCF_FAST_V17_TRUE_POP_INPUT="tests/output/public-data/<cached-igsr-vcf>.vcf.gz" \
VCF_FAST_V17_TRUE_POP_METADATA="tests/output/public-data/igsr-1000g-3202-sample-ped-population.txt" \
VCF_FAST_V17_TRUE_POP_TIERS="10000 50000 100000" \
VCF_FAST_V17_TRUE_POP_GROUPS="AFR:EUR" \
make bench-vcftools-true-popgen
```
```

- [ ] **Step 5: Run the test and commit**

```bash
cargo test --test benchmark_harness_tests true_population_downloader_documents_igsr_metadata_inputs
git add benchmark/download_public_data.sh docs/bioinformatics-workflows.md tests/benchmark_harness_tests.rs
git commit -m "docs: document true population public data inputs"
```

---

### Task 5: Add VCFtools Edge-Semantics Tests

**Files:**
- Modify: `tests/popgen_cli_tests.rs`
- Optionally create: `tests/data/popgen_edge_semantics.vcf`
- Optionally modify: `src/engine/popgen.rs`

- [ ] **Step 1: Add a compact edge fixture**

Create `tests/data/popgen_edge_semantics.vcf`:

```text
##fileformat=VCFv4.2
##FORMAT=<ID=GT,Number=1,Type=String,Description="Genotype">
#CHROM	POS	ID	REF	ALT	QUAL	FILTER	INFO	FORMAT	S1	S2	S3	S4
1	100	.	A	G	50	PASS	.	GT	0/0	0/1	1/1	./.
1	200	.	C	T	50	PASS	.	GT	0/.	0/1	0/0	1/1
1	300	.	G	A,C	50	PASS	.	GT	0/1	1/2	0/0	./.
1	400	.	T	C	50	PASS	.	GT	0/0	0/0	0/0	0/0
1	601	.	G	A	50	PASS	.	GT	0/1	0/1	1/1	0/0
```

- [ ] **Step 2: Add missing-genotype and keep/remove tests**

Append to `tests/popgen_cli_tests.rs`:

```rust
#[test]
fn popgen_edge_missing_genotypes_are_counted_consistently() {
    let dir = tempdir().unwrap();
    let prefix = dir.path().join("edge-missingness");

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "missingness",
            fixture("tests/data/popgen_edge_semantics.vcf").to_str().unwrap(),
            "-o",
            prefix.to_str().unwrap(),
        ])
        .assert()
        .success();

    let lmiss = fs::read_to_string(prefix.with_extension("lmiss")).unwrap();
    assert!(lmiss.contains("1\t100\t8\t0\t2\t0.25\n"));
    assert!(lmiss.contains("1\t200\t8\t0\t1\t0.125\n"));
}

#[test]
fn popgen_edge_keep_and_remove_are_applied_before_frequency() {
    let dir = tempdir().unwrap();
    let keep = dir.path().join("keep.txt");
    let remove = dir.path().join("remove.txt");
    let output = dir.path().join("edge.frq");
    fs::write(&keep, "S1\nS2\nS3\n").unwrap();
    fs::write(&remove, "S3\n").unwrap();

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "freq",
            fixture("tests/data/popgen_edge_semantics.vcf").to_str().unwrap(),
            "--keep",
            keep.to_str().unwrap(),
            "--remove",
            remove.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    assert!(
        fs::read_to_string(output)
            .unwrap()
            .contains("1\t100\t2\t4\tA:0.75\tG:0.25\n")
    );
}
```

- [ ] **Step 3: Add multiallelic policy and window-boundary tests**

Append to `tests/popgen_cli_tests.rs`:

```rust
#[test]
fn popgen_edge_window_boundaries_match_half_open_tier_policy() {
    let dir = tempdir().unwrap();
    let output = dir.path().join("edge.windowed.pi");

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "pi",
            fixture("tests/data/popgen_edge_semantics.vcf").to_str().unwrap(),
            "--window-size",
            "300",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .success();

    let text = fs::read_to_string(output).unwrap();
    assert!(text.contains("1\t1\t300\t"));
    assert!(text.contains("1\t301\t600\t"));
    assert!(text.contains("1\t601\t900\t"));
}

#[test]
fn popgen_edge_weir_cockerham_rejects_multiallelic_sites_with_clear_error() {
    let dir = tempdir().unwrap();
    let pop1 = dir.path().join("pop1.txt");
    let pop2 = dir.path().join("pop2.txt");
    let output = dir.path().join("edge.weir.fst");
    fs::write(&pop1, "S1\nS2\n").unwrap();
    fs::write(&pop2, "S3\nS4\n").unwrap();

    Command::cargo_bin("variantflow")
        .unwrap()
        .args([
            "fst",
            fixture("tests/data/popgen_edge_semantics.vcf").to_str().unwrap(),
            "--pop",
            pop1.to_str().unwrap(),
            "--pop",
            pop2.to_str().unwrap(),
            "--estimator",
            "weir-cockerham",
            "-o",
            output.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("multiallelic"));
}
```

- [ ] **Step 4: Run the edge tests**

```bash
cargo test --test popgen_cli_tests popgen_edge_
```

Expected: PASS if current behavior already matches; otherwise fix only the failing semantics in `src/engine/popgen.rs`.

- [ ] **Step 5: Commit edge tests and fixes**

```bash
git add tests/popgen_cli_tests.rs tests/data/popgen_edge_semantics.vcf src/engine/popgen.rs
git commit -m "test: cover VCFtools popgen edge semantics"
```

---

### Task 6: Add LD Memory Investigation Gate

**Files:**
- Modify: `benchmark/run_v17_true_population_evidence.sh`
- Modify: `benchmark/reports/v17-true-public-population-evidence.md`
- Modify: `docs/claim-matrix.md`

- [ ] **Step 1: Ensure LD rows expose memory caveats**

In `benchmark/run_v17_true_population_evidence.sh`, make the LD `benchmark_pair` caveat include this exact phrase:

```bash
"LD memory is a monitored optimization target; VariantFlow RSS must be compared with VCFtools before expanding LD claims"
```

- [ ] **Step 2: Add an LD memory section to the report scaffold**

Append to `benchmark/reports/v17-true-public-population-evidence.md`:

```markdown
## LD Memory Gate

LD rows are not only speed rows. VariantFlow must report peak RSS beside
VCFtools, because earlier staged evidence showed VariantFlow LD could be faster
while using substantially more memory. If larger public rows preserve the speed
win but show higher RSS, the claim matrix must keep that caveat visible.
```

- [ ] **Step 3: Add a pending claim-matrix row**

In `docs/claim-matrix.md`, add a row near the VCFtools workflows:

```markdown
| v1.7 true public population evidence | not yet proven | `benchmark/reports/v17-true-public-population-evidence.md` tracks planned 1000 Genomes / IGSR official-population VCFtools parity rows at 10k/50k/100k actual biallelic sites | VCFtools | no claim until correctness-matched measured rows exist; LD RSS is a monitored caveat |
```

- [ ] **Step 4: Run docs/harness tests**

```bash
cargo test --test benchmark_harness_tests v17_true_public_population_evidence_harness_is_declared
```

Expected: PASS.

- [ ] **Step 5: Commit LD gate docs**

```bash
git add benchmark/run_v17_true_population_evidence.sh benchmark/reports/v17-true-public-population-evidence.md docs/claim-matrix.md
git commit -m "docs: track LD memory gate for true population evidence"
```

---

### Task 7: Smoke Run And Full Verification

**Files:**
- No new files unless previous steps reveal small fixes.

- [ ] **Step 1: Run script syntax and Python checks**

```bash
bash -n benchmark/run_v17_true_population_evidence.sh
python3 -m py_compile benchmark/igsr_population_files.py benchmark/check_vcftools_parity.py benchmark/command_resource_metrics.py
```

Expected: PASS.

- [ ] **Step 2: Run focused tests**

```bash
cargo test --test benchmark_harness_tests v17_true_public_population_evidence_harness_is_declared
cargo test --test benchmark_harness_tests true_population_downloader_documents_igsr_metadata_inputs
cargo test --test popgen_cli_tests popgen_edge_
```

Expected: PASS.

- [ ] **Step 3: Run full standard verification**

```bash
make verify
cargo test --features htslib-static
cargo clippy --features htslib-static --all-targets -- -D warnings
```

Expected: all pass.

- [ ] **Step 4: Run blocked-mode harness smoke**

Without setting public input variables, run:

```bash
make bench-vcftools-true-popgen
```

Expected: exits `77` and writes a blocked report explaining that `VCF_FAST_V17_TRUE_POP_INPUT` and `VCF_FAST_V17_TRUE_POP_METADATA` are required. This is acceptable for machines without the large public cohort cached.

- [ ] **Step 5: Commit final verification fixes**

If Step 1-4 required any small fixes:

```bash
git add Makefile benchmark tests docs src
git commit -m "fix: stabilize true population evidence smoke checks"
```

If no files changed, do not create an empty commit.

---

### Task 8: Full Evidence Run, Only When Data Is Cached

**Files:**
- Generated ignored artifacts under `tests/output/...`
- Modify after measurement: `benchmark/reports/v17-true-public-population-evidence.md`
- Modify after measurement: `docs/claim-matrix.md`
- Modify after measurement: `docs/public-benchmark-table.md`
- Modify after measurement: `README.md`

- [ ] **Step 1: Cache official public inputs**

Run the chosen downloader or manual cache command, then confirm:

```bash
test -f "$VCF_FAST_V17_TRUE_POP_INPUT"
test -f "$VCF_FAST_V17_TRUE_POP_METADATA"
```

Expected: both commands exit `0`.

- [ ] **Step 2: Run balanced true-population evidence**

```bash
VCF_FAST_V17_TRUE_POP_INPUT="$VCF_FAST_V17_TRUE_POP_INPUT" \
VCF_FAST_V17_TRUE_POP_METADATA="$VCF_FAST_V17_TRUE_POP_METADATA" \
VCF_FAST_V17_TRUE_POP_TIERS="10000 50000 100000" \
VCF_FAST_V17_TRUE_POP_GROUPS="AFR:EUR" \
VCF_FAST_V17_TRUE_POP_RUNS=3 \
VCF_FAST_V17_TRUE_POP_WARMUP=1 \
make bench-vcftools-true-popgen
```

Expected: measured report rows for each workflow and tier, or explicit blocker rows with exact failures.

- [ ] **Step 3: Inspect actual record counts**

Run:

```bash
rg "public cohort (10000|50000|100000)" benchmark/reports/v17-true-public-population-evidence.md
```

Expected: the actual record count column distinguishes real tier size from requested tier label. Do not claim `100k` evidence if the actual record count is lower.

- [ ] **Step 4: Update claims only from passing rows**

Update `docs/claim-matrix.md`, `docs/public-benchmark-table.md`, and `README.md` only if:

- correctness result says passed for that row,
- actual record count is at or above the stated tier,
- population metadata source says official IGSR metadata,
- LD RSS caveat is visible when LD memory remains higher than VCFtools.

- [ ] **Step 5: Regenerate benchmark table and verify**

```bash
python3 benchmark/generate_public_benchmark_table.py
make verify
cargo test --features htslib-static
cargo clippy --features htslib-static --all-targets -- -D warnings
```

Expected: PASS.

- [ ] **Step 6: Commit measured evidence**

```bash
git add benchmark/reports/v17-true-public-population-evidence.md docs/claim-matrix.md docs/public-benchmark-table.md README.md
git commit -m "bench: add true public population evidence rows"
```

---

## Self-Review

Spec coverage:

- Larger real population cohort: covered by Tasks 2, 3, 4, and 8.
- Real biological population files: covered by `igsr_population_files.py` and no-header-fallback harness policy.
- True VCFtools replacement evidence: covered by Task 3 workflows and Task 8 measured run.
- RSS and CPU-hour reporting: covered by Task 3 using `command_resource_metrics.py`.
- VCFtools edge semantics: covered by Task 5.
- LD memory validation: covered by Task 6 and Task 8.
- PLINK/GATK: intentionally deferred until after this evidence row, matching the approved direction.
- Paper figures: intentionally deferred until measured rows exist.
- Release hardening: explicitly non-goal until evidence is stronger.

Placeholder scan:

- The plan includes no implementation placeholders for required code paths. The only blocked behavior is an explicit `exit 77` for missing large public inputs or undecided official metadata URL, which prevents false claims.

Type consistency:

- New environment variables consistently use the `VCF_FAST_V17_TRUE_POP_` prefix.
- The new Make target is consistently `bench-vcftools-true-popgen`.
- The new report path is consistently `benchmark/reports/v17-true-public-population-evidence.md`.
