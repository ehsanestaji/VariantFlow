# VCF-Fast Project Specification

## Project Name

**VCF-Fast**  
Alternative names: **VariantDBX**, **VariantDuck**, **GenVarX**, **VCF-Engine**

## One-line Summary

A high-performance execution engine for genomic variant data that treats VCF/BCF as exchange formats, while internally using columnar, typed, vectorized, and parallel data structures for faster filtering, comparison, statistics, and sample subsetting.

---

## Core Idea

Most current variant-processing tools treat VCF/BCF primarily as files to be scanned and processed record-by-record.

VCF-Fast should instead treat VCF/BCF as an input/output format and convert the data into a fast internal representation:

```text
VCF/BCF input
    ↓
typed parser + selective field extraction
    ↓
columnar internal representation
    ↓
vectorized / parallel operators
    ↓
VCF/BCF/TSV/Parquet output
```

This is conceptually similar to how DuckDB does not treat CSV as the main computational model. CSV is loaded, typed, optimized, and queried efficiently. VCF-Fast applies this idea to genomic variants.

---

## Main Goal

Build a tool that makes common post-variant-calling operations faster and more reproducible:

```bash
vcf-fast filter
vcf-fast stats
vcf-fast diff
vcf-fast intersect
vcf-fast sample-subset
vcf-fast convert
```

The first MVP should focus on:

```bash
vcf-fast filter input.vcf.gz --where "QUAL > 30 && DP > 10 && AF > 0.01" -o output.vcf.gz
vcf-fast stats input.vcf.gz
vcf-fast diff a.vcf.gz b.vcf.gz
```

---

## Problem Being Solved

Bioinformaticians routinely process large VCF/BCF files after variant calling. Common tasks include:

- filtering variants
- comparing samples or cohorts
- intersecting variant sets
- computing statistics
- subsetting samples
- converting VCF into analysis-friendly formats
- preparing files for GWAS, population genetics, clinical analysis, or downstream annotation

These tasks are often bottlenecked by:

1. Text-heavy parsing of VCF.
2. Repeated decompression of `.vcf.gz`.
3. Row-oriented processing.
4. Complex nested FORMAT fields.
5. Inefficient repeated queries.
6. Limited vectorization.
7. Partial parallelism.
8. High memory overhead when parsing unused fields.

---

## Key Design Principle

Do not parse everything.

Only parse what is required by the requested operation.

Example:

```bash
vcf-fast filter input.vcf.gz --where "QUAL > 30 && DP > 10"
```

The engine should not fully parse all INFO/FORMAT fields. It should extract:

```text
CHROM
POS
REF
ALT
QUAL
INFO/DP or FORMAT/DP if required
```

This selective parsing is one of the key performance opportunities.

---

## Initial MVP Scope

### Supported input

- VCF
- `.vcf.gz`
- Optional: BCF later

### Supported output

- VCF
- `.vcf.gz`
- TSV
- JSON summary
- Optional: Arrow/Parquet later

### MVP commands

#### 1. `filter`

```bash
vcf-fast filter input.vcf.gz \
  --where "QUAL > 30 && DP > 10 && AF > 0.01" \
  -o filtered.vcf.gz
```

Supported fields for MVP:

```text
CHROM
POS
ID
REF
ALT
QUAL
FILTER
INFO/DP
INFO/AF
```

Later:

```text
FORMAT/GT
FORMAT/DP
FORMAT/GQ
sample-specific filters
```

#### 2. `stats`

```bash
vcf-fast stats input.vcf.gz
```

MVP statistics:

```text
number of variants
number of SNPs
number of indels
variants per chromosome
QUAL distribution
AF distribution if available
missing FILTER values
transition/transversion ratio
```

#### 3. `diff`

```bash
vcf-fast diff a.vcf.gz b.vcf.gz -o diff.tsv
```

MVP comparison key:

```text
CHROM + POS + REF + ALT
```

Outputs:

```text
shared variants
only in A
only in B
summary counts
```

---

## Later Commands

### `intersect`

```bash
vcf-fast intersect variants.vcf.gz regions.bed -o intersected.vcf.gz
```

### `sample-subset`

```bash
vcf-fast sample-subset cohort.vcf.gz --samples samples.txt -o subset.vcf.gz
```

### `convert`

```bash
vcf-fast convert input.vcf.gz --to parquet -o variants.parquet
vcf-fast convert input.vcf.gz --to arrow -o variants.arrow
```

### `query`

```bash
vcf-fast query variants.parquet \
  --where "chrom = '1' AND pos > 1000000 AND AF > 0.01"
```

---

## Internal Architecture

Recommended language:

```text
Rust
```

Recommended libraries:

```text
clap       - CLI
noodles    - VCF/BCF/BGZF parsing
rayon      - parallel processing
arrow-rs   - Arrow columnar arrays
parquet    - optional Parquet output
serde      - JSON summaries
anyhow     - error handling
thiserror  - typed errors
criterion  - benchmarking
```

Alternative implementation:

```text
C++ with htslib + Apache Arrow
```

But Rust is preferred for safety, modern ecosystem, and easier maintainability.

---

## Internal Modules

Suggested crate layout:

```text
vcf-fast/
├── Cargo.toml
├── README.md
├── src/
│   ├── main.rs
│   ├── cli.rs
│   ├── error.rs
│   ├── reader/
│   │   ├── mod.rs
│   │   ├── vcf.rs
│   │   └── bgzf.rs
│   ├── schema/
│   │   ├── mod.rs
│   │   ├── variant_record.rs
│   │   └── field_types.rs
│   ├── engine/
│   │   ├── mod.rs
│   │   ├── filter.rs
│   │   ├── stats.rs
│   │   ├── diff.rs
│   │   └── operators.rs
│   ├── expr/
│   │   ├── mod.rs
│   │   ├── parser.rs
│   │   └── evaluator.rs
│   ├── writer/
│   │   ├── mod.rs
│   │   ├── vcf_writer.rs
│   │   └── table_writer.rs
│   └── benchmark/
│       └── mod.rs
├── tests/
│   ├── data/
│   ├── test_filter.rs
│   ├── test_stats.rs
│   └── test_diff.rs
└── benches/
    ├── filter_bench.rs
    ├── stats_bench.rs
    └── diff_bench.rs
```

---

## Execution Model

### MVP execution

For first version:

```text
stream VCF records
    ↓
selectively parse required fields
    ↓
evaluate expression
    ↓
write passing records
```

This avoids needing a full Arrow engine in version 0.1.

### Version 0.2 execution

```text
read block/chunk
    ↓
convert required fields into typed column arrays
    ↓
apply vectorized filters
    ↓
emit passing records
```

### Version 0.3 execution

```text
persistent Arrow/Parquet backend
    ↓
repeated fast queries
    ↓
SQL-like API or Python API
```

---

## Expression Language

MVP examples:

```text
QUAL > 30
DP > 10
AF > 0.01
FILTER == "PASS"
CHROM == "1"
POS > 1000000
```

Boolean combinations:

```text
QUAL > 30 && DP > 10
AF > 0.01 || FILTER == "PASS"
```

Supported operators:

```text
>
>=
<
<=
==
!=
&&
||
```

Supported literals:

```text
integers
floats
strings
booleans
```

---

## Variant Key

For set operations, define canonical variant key:

```text
chromosome
position
reference allele
alternate allele
```

Implementation:

```text
VariantKey = hash(CHROM, POS, REF, ALT)
```

Important:

- multi-allelic variants must be handled carefully
- MVP can preserve record-level representation
- later versions can normalize/decompose multi-allelic records

---

## Correctness Requirements

Correctness is more important than speed.

VCF-Fast must preserve:

- VCF header
- sample columns
- INFO fields
- FORMAT fields
- original record order where possible
- valid VCF output

MVP can restrict optimized filtering to known fields, while preserving full original lines for output.

Important strategy:

```text
Parse only required fields for computation,
but keep original record text for output.
```

This is powerful because it gives speed without needing full reconstruction of VCF records.

---

## Benchmarking Competitors

Primary competitors:

```text
bcftools
vcftools
GATK SelectVariants / VariantFiltration
bedtools, for interval-based tasks
```

Python competitors:

```text
cyvcf2
pysam
scikit-allel
pandas-based parsing
```

Modern/dataframe competitors:

```text
bioframe
PyRanges
Hail
Glow
DuckDB/Parquet workflows
```

---

## Benchmark Tasks

### Task 1: Basic filtering

VCF-Fast:

```bash
vcf-fast filter input.vcf.gz --where "QUAL > 30" -o out.vcf.gz
```

bcftools:

```bash
bcftools filter -i 'QUAL>30' input.vcf.gz -Oz -o out.vcf.gz
```

Metrics:

```text
runtime
max memory
CPU utilization
output equivalence
```

---

### Task 2: INFO-based filtering

```bash
vcf-fast filter input.vcf.gz --where "DP > 10 && AF > 0.01"
```

Compare against:

```bash
bcftools filter -i 'INFO/DP>10 && INFO/AF>0.01'
```

---

### Task 3: Variant statistics

```bash
vcf-fast stats input.vcf.gz
bcftools stats input.vcf.gz
```

Compare:

```text
runtime
memory
agreement of common statistics
```

---

### Task 4: VCF diff

```bash
vcf-fast diff a.vcf.gz b.vcf.gz
bcftools isec a.vcf.gz b.vcf.gz
```

Compare:

```text
runtime
memory
shared variant count
unique variant count
```

---

### Task 5: Sample subsetting

```bash
vcf-fast sample-subset cohort.vcf.gz --samples samples.txt
bcftools view -S samples.txt cohort.vcf.gz
```

This is harder and can be post-MVP.

---

## Benchmark Datasets

Use public, reproducible datasets:

1. 1000 Genomes Project VCFs.
2. Genome in a Bottle small variant truth sets.
3. gnomAD public subsets.
4. Simulated VCFs of controlled size.
5. Synthetic stress-test VCFs with many INFO/FORMAT fields.

Dataset sizes:

```text
small: 10k variants
medium: 1M variants
large: 10M+ variants
very large: whole-genome cohort VCF
```

---

## Benchmark Methodology

Use a reproducible Docker container.

Measure with:

```bash
/usr/bin/time -v
hyperfine
perf stat
```

Record:

```text
wall-clock time
user time
system time
maximum resident set size
CPU utilization
disk read/write
compressed input size
uncompressed input size
number of variants processed
variants per second
```

Suggested benchmark command:

```bash
hyperfine \
  'vcf-fast filter input.vcf.gz --where "QUAL > 30" -o out.fast.vcf.gz' \
  'bcftools filter -i "QUAL>30" input.vcf.gz -Oz -o out.bcftools.vcf.gz'
```

Correctness check:

```bash
bcftools sort out.fast.vcf.gz -Oz -o out.fast.sorted.vcf.gz
bcftools sort out.bcftools.vcf.gz -Oz -o out.bcftools.sorted.vcf.gz
bcftools isec out.fast.sorted.vcf.gz out.bcftools.sorted.vcf.gz
```

---

## Performance Hypotheses

VCF-Fast can win by:

1. Selective parsing.
2. Preserving original lines for output.
3. Typed field extraction.
4. Parallel decompression and processing.
5. Vectorized filtering.
6. Avoiding unnecessary reconstruction of records.
7. Using efficient hash keys for variant comparison.
8. Optional persistent Arrow/Parquet representation for repeated queries.

Expected performance:

```text
simple filtering: 1.5x to 5x faster
INFO filtering: 2x to 10x faster depending on fields
stats: 2x to 10x faster for selected statistics
diff: potentially 2x to 20x depending on indexing/hash strategy
sample subsetting: difficult but valuable
```

These are hypotheses and must be validated experimentally.

---

## Scientific Contribution

Potential paper framing:

```text
VCF is not a database: a columnar and vectorized execution engine for large-scale genomic variant processing
```

Main claim:

```text
Current VCF tools are optimized file processors.
VCF-Fast is a variant-data execution engine.
```

Contributions:

1. A selective parser for variant operations.
2. A typed internal representation for VCF fields.
3. Vectorized filtering and statistics.
4. Fast set operations using canonical variant keys.
5. Reproducible benchmarks against established tools.
6. Optional Arrow/Parquet export for downstream analytics.

---

## MVP Milestones

### Milestone 1: Project skeleton

- Rust CLI
- input/output handling
- read VCF/VCF.GZ
- preserve headers
- stream records

### Milestone 2: Simple filter

- support `QUAL > number`
- preserve original records
- write valid VCF output
- benchmark against bcftools

### Milestone 3: INFO filters

- extract INFO/DP
- extract INFO/AF
- support combined expressions

### Milestone 4: Stats

- count variants
- count SNPs/indels
- count per chromosome
- transition/transversion ratio

### Milestone 5: Diff

- hash CHROM/POS/REF/ALT
- compare two VCF files
- output shared/unique variants

### Milestone 6: Benchmark suite

- Dockerfile
- benchmark script
- reports
- correctness tests

---

## Docker Requirement

The entire project should be reproducible inside Docker.

Required files:

```text
Dockerfile
docker-compose.yml optional
Makefile
benchmark/run_benchmarks.sh
benchmark/download_test_data.sh
```

Example commands:

```bash
docker build -t vcf-fast .
docker run --rm -v $PWD:/work vcf-fast cargo test
docker run --rm -v $PWD:/work vcf-fast ./benchmark/run_benchmarks.sh
```

---

## Deliverables

1. Rust CLI tool.
2. Dockerized reproducible environment.
3. Test datasets or scripts to download them.
4. Benchmark scripts.
5. Correctness tests against bcftools.
6. README with examples.
7. Initial technical report.
8. Optional Python bindings later.

---

## Definition of Done for MVP

The MVP is done when:

```text
vcf-fast filter can process .vcf.gz files,
apply QUAL/INFO filters,
preserve valid VCF output,
match bcftools output for supported filters,
and show benchmark results on at least 3 datasets.
```

