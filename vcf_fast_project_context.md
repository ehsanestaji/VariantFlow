# VCF-Fast Project Context

## Motivation

Bioinformaticians use many tools daily in variant analysis pipelines. After variant calling, they frequently process VCF/BCF files using tools such as bcftools, vcftools, GATK, bedtools, cyvcf2, pysam, and custom scripts.

The problem is not that these tools are bad. Many are excellent and widely trusted. The problem is that VCF-based workflows are often slow, repetitive, and file-processing oriented.

VCF-Fast is motivated by the idea that genomic variant data should be processed more like a modern analytical dataset, not only like a text file.

---

## Key Insight

VCF is a good exchange format, but it is not an ideal computational format.

This project should not simply be “another faster VCF parser.”

The deeper idea is:

```text
VCF/BCF is the input/output format.
The internal computation should use typed, columnar, indexed, and vectorized structures.
```

Analogy:

```text
CSV is not the database.
DuckDB reads CSV, infers structure, and executes optimized queries.
```

Similarly:

```text
VCF should not be the execution model.
VCF-Fast should read VCF and execute optimized variant operations internally.
```

---

## Why This Matters

Variant files are central in:

- human genetics
- GWAS
- population genomics
- cancer genomics
- clinical genomics
- rare disease analysis
- plant genomics
- microbial genomics
- cohort comparison
- variant annotation pipelines

Common daily operations include:

```text
filtering
subsetting
merging
comparing
intersecting
annotating
computing statistics
converting formats
```

These operations are repeated in many pipelines, often on large files.

Even modest speedups can matter because these steps are performed repeatedly by many users.

---

## What Makes This Meaningful

The goal is not to wrap Claude, Codex, or another agent around existing tools.

Agentic coding tools can help implement the software faster, but the scientific contribution should come from:

```text
better parsing strategy
better internal data model
better memory layout
better compression-aware execution
better vectorized filtering
better indexing and set operations
better reproducible benchmarking
```

This makes the project a real computational tool, not an AI wrapper.

---

## Product Vision

VCF-Fast should become:

```text
A next-generation execution engine for genomic variant data.
```

In practical terms, it should feel like:

```text
bcftools + DuckDB-style execution + Arrow/Parquet compatibility
```

The tool should be useful both as:

1. A command-line tool for bioinformaticians.
2. A backend engine for future pipelines.
3. A library for Python/Rust-based variant analytics.
4. A benchmarkable research software project.

---

## Initial User Persona

Target users:

- bioinformaticians running variant pipelines
- computational biologists analyzing cohort VCFs
- PhD students and postdocs processing variant data
- genomics core facilities
- pipeline developers
- researchers comparing variant callsets

Typical pain points:

```text
"My VCF filtering step is slow."
"I need to compare two variant callsets."
"I want quick stats before downstream analysis."
"I need to subset samples from a large cohort VCF."
"I keep converting VCF to TSV just to analyze it."
"I want to run repeated queries without reparsing everything."
```

---

## Important Distinction

This is not:

```text
a new variant caller
a new aligner
a new genome assembler
a replacement for all of bcftools
a GUI wrapper
a chatbot for VCF files
```

This is:

```text
a faster computational engine for post-calling variant operations
```

---

## First Research Question

Can selective parsing and typed internal representation accelerate common VCF operations while preserving correctness and compatibility with standard tools?

Sub-questions:

1. How much time is spent parsing unused fields?
2. Can we preserve original VCF records while only parsing required fields?
3. Can filtering be accelerated using typed arrays and vectorized evaluation?
4. Can variant comparison be accelerated using canonical hash keys?
5. When does columnar conversion become worth the overhead?
6. Which operations benefit most from persistent Arrow/Parquet storage?

---

## First Product Hypothesis

The fastest useful MVP is:

```text
A streaming selective VCF filter that preserves original lines for output.
```

Why?

Because full VCF reconstruction is hard and slow. But for filtering, we can:

```text
read record
extract only needed fields
evaluate condition
write original line if condition passes
```

This gives correctness and speed at the same time.

---

## Suggested MVP Strategy

Start simple:

```text
Version 0.1:
streaming selective parser
QUAL filters
INFO/DP and INFO/AF filters
basic stats
valid VCF output
benchmark against bcftools
```

Then evolve:

```text
Version 0.2:
typed column batches
parallel chunks
vectorized filters
faster stats
```

Then:

```text
Version 0.3:
diff/intersect engine
hash-based variant keys
Arrow/Parquet export
```

Then:

```text
Version 0.4:
sample subsetting
FORMAT/GT parsing
Python API
```

---

## Why Rust

Rust is a strong choice because:

- memory safety
- speed close to C/C++
- modern package ecosystem
- good CLI tooling
- good parallelism support
- strong testing culture
- suitable for bioinformatics infrastructure

Suggested Rust ecosystem:

```text
noodles
rayon
clap
arrow-rs
parquet
serde
criterion
hyperfine for external benchmarks
```

---

## Benchmarking Philosophy

The goal is not to claim universal superiority.

The correct claim should be specific:

```text
VCF-Fast is faster for selected common operations under clearly defined conditions.
```

The benchmark must be fair, reproducible, and honest.

Compare against:

```text
bcftools
vcftools
GATK
cyvcf2
pysam
bedtools for interval operations
Hail or Glow for large-scale distributed contexts
```

Important:

- always check output correctness
- report failures and limitations
- separate cold-start one-time conversion from repeated-query performance
- include small, medium, and large datasets
- include compressed and uncompressed input
- include one-core and multi-core results

---

## Potential Novelty Claims

Possible paper/software claims:

1. Selective parsing reduces unnecessary VCF processing overhead.
2. Original-record preservation avoids expensive reconstruction.
3. Typed internal representation improves filtering and statistics.
4. Canonical variant keys improve diff/intersection operations.
5. Arrow/Parquet export enables repeated analytical workloads.
6. The tool provides a reproducible benchmark framework for VCF operations.

---

## Risks

### Risk 1: bcftools is already very fast

Mitigation:

- do not compete with all of bcftools
- target selected operations where selective parsing or repeated queries can win
- benchmark honestly

### Risk 2: VCF specification complexity

Mitigation:

- start with a constrained MVP
- preserve original records
- use existing parser libraries where useful
- expand gradually

### Risk 3: FORMAT/sample fields are difficult

Mitigation:

- postpone sample-subset and genotype-heavy operations
- first focus on site-level VCF fields

### Risk 4: Columnar conversion may be expensive

Mitigation:

- separate streaming mode and persistent columnar mode
- streaming mode for one-off operations
- columnar mode for repeated queries

---

## Recommended Positioning

Short version:

```text
VCF-Fast is a high-performance engine for genomic variant operations.
```

Long version:

```text
VCF-Fast treats VCF/BCF as exchange formats and executes common variant operations using selective parsing, typed internal representation, and vectorized or parallel operators.
```

Academic framing:

```text
VCF is not a database: toward a columnar execution engine for large-scale genomic variant processing.
```

GitHub tagline:

```text
Fast, typed, and benchmarked operations for genomic variant data.
```

---

## Possible README Opening

VCF-Fast is an experimental high-performance engine for genomic variant data. It accelerates common post-calling operations such as filtering, statistics, and callset comparison by avoiding unnecessary full-record parsing and by using typed internal representations.

Unlike traditional VCF tools that process records primarily as text lines, VCF-Fast treats VCF/BCF as exchange formats and executes operations using a modern data-processing architecture inspired by columnar analytical engines.

---

## Success Criteria

This project is successful if it can show:

```text
1. Correct output compared with bcftools for supported operations.
2. Faster runtime for at least one common operation.
3. Lower or comparable memory usage.
4. Clear benchmark scripts.
5. Reproducible Docker environment.
6. Simple CLI that real bioinformaticians can use.
```

---

## Immediate Next Task for Codex

Create the initial Rust project with:

```text
CLI structure
filter command
stats command placeholder
diff command placeholder
VCF reader
VCF writer
QUAL filtering
basic tests
Dockerfile
Makefile
benchmark skeleton
```

First target command:

```bash
vcf-fast filter tests/data/example.vcf \
  --where "QUAL > 30" \
  -o tests/output/filtered.vcf
```

The tool should preserve the VCF header and output valid VCF records.

