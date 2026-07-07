# VariantFlow user guide: an end-to-end population-genetics analysis of 1000 Genomes chromosome 22

This guide walks a **complete population-genetics analysis** of human chromosome 22
from the **1000 Genomes Project**, from downloading raw data to computing and
reading standard population-genetics statistics. It is written for someone with
**basic command-line familiarity but no prior bioinformatics experience** — for
example, a first-year biology MSc student. Every command is copy-pasteable, and
each step explains *what* it does and *why*.

> **Time and disk warning.** The full chromosome-22 file is several gigabytes and
> some computations take a while. Wherever a step is slow, this guide offers a
> **small-region shortcut** (a ~1 Mb slice) so you can follow along in minutes.
> Do the small-region version first; scale up once you are comfortable.

---

## 1. Introduction

A **VCF** (Variant Call Format) file is the standard text table of genetic
variants: each row is a position in the genome where individuals differ, and each
column (after the fixed fields) is one sample's genotype.

**VariantFlow** is a fast command-line tool for **post-calling VCF operations** —
the analysis steps that happen *after* variants have been called. Its guiding idea
is **selective execution**: it only reads and computes the parts of the file that
your query actually needs, which makes filtering and per-site statistics fast even
on large files. It reproduces the numbers you would get from established tools
(VCFtools, pixy, scikit-allel) with a single self-contained binary.

By the end of this guide you will have:

- downloaded and indexed a real 1000 Genomes chromosome-22 VCF,
- reduced it to clean, biallelic SNPs,
- computed allele frequencies, missingness, heterozygosity, Hardy–Weinberg tests,
  nucleotide diversity (π), Tajima's D, linkage disequilibrium, and Fst between
  two human populations,
- and read what those numbers mean, cautiously.

Everything is run with one binary, invoked as `variantflow <command>`.

### Who this is for / when to use VariantFlow

This guide (and the tool) is aimed at **population and conservation / ecological
geneticists**, **plant and animal breeding** programs, **large-cohort human-genomics
QC**, and **pipeline developers** who want a single dependency-light binary that
produces VCFtools-compatible output and a fast Parquet export.

Use VariantFlow when you want **interactive turnaround on large VCFs on commodity
CPUs** — fast field-selective filtering, streamable site/window statistics, and a
one-time Parquet export. For the full generality of VCF surgery reach for
**bcftools**; **VCFtools** is the reference baseline VariantFlow's statistics are
validated against; **DuckDB** runs ad hoc SQL over the exported Parquet; and
**scikit-allel** handles matrix- and haplotype-wide statistics (selection scans,
PCA, D/f-statistics) that fall outside VariantFlow's streaming model. See
[Choosing the right tool](tool-comparison.md) for the full comparison.

---

## 2. Installation

### 2.1 Build VariantFlow from source

VariantFlow is written in **Rust**. You need the Rust toolchain (`cargo`). If you
do not have it:

```bash
# Install the Rust toolchain (rustup). Accept the defaults.
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc --version   # sanity check
cargo --version
```

Then build the release binary from the repository root:

```bash
# Standard build — pure-Rust VCF/BCF text path
cargo build --release

# The binary lands here:
./target/release/variantflow --version
```

If you need **BCF input or region-based queries backed by HTSlib** (for example,
`--region chr22:1-1000000` on a `.bcf` file), build with the HTSlib feature:

```bash
# Statically linked HTSlib backend (needs a C toolchain)
cargo build --release --features htslib-static
```

Confirm the tool works and see the available commands:

```bash
./target/release/variantflow --help
```

You should see these subcommands: `filter`, `stats`, `freq`, `missingness`,
`hardy`, `het`, `fst`, `pi`, `pixy`, `tajima-d`, `ld`, `index`, `diff`, `convert`.

> For readability the rest of this guide writes `variantflow` instead of
> `./target/release/variantflow`. Either add the binary to your `PATH`
> (`export PATH="$PWD/target/release:$PATH"`) or keep typing the full path.

### 2.2 Companion tools and dependencies

A few standard bioinformatics tools do the jobs VariantFlow deliberately does not:
downloading, indexing, and heavy VCF surgery (splitting multiallelic sites, etc.).

| Tool | Used for | Install |
|------|----------|---------|
| **bcftools / HTSlib (`tabix`, `bgzip`)** | index VCFs, subset to biallelic SNPs, slice regions | `conda install -c bioconda bcftools htslib` |
| **VCFtools** | optional cross-check of results | `conda install -c bioconda vcftools` |
| **wget** or **curl** | download the data | usually preinstalled; `conda install -c conda-forge wget` |

The easiest route is **[Bioconda](https://bioconda.github.io/)**:

```bash
# One-time channel setup (order matters)
conda config --add channels bioconda
conda config --add channels conda-forge

# Install the companions into a fresh environment
conda create -n popgen -c bioconda -c conda-forge bcftools htslib vcftools wget
conda activate popgen
```

Version-check everything:

```bash
bcftools --version
tabix --version      # part of HTSlib
bgzip  --version
vcftools --version
wget --version | head -n1
```

---

## 3. Getting the data

We use the **1000 Genomes Project 30× high-coverage** phased panel (3202 samples)
aligned to **GRCh38**, hosted by the IGSR / EBI at EBI's public FTP. We want the
chromosome-22 file.

> **Heads up:** this file is **large** (several GB) and downloading it can take a
> while on a normal connection. The commands below are correct for the full file;
> Section 3.2 shows how to grab just a **1 Mb slice** so you can proceed quickly.

### 3.1 Download the full chromosome-22 panel

```bash
mkdir -p data && cd data

BASE="https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000G_2504_high_coverage/working/20220422_3202_phased_SNV_INDEL_SV"
FILE="1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz"

# Download the VCF and its tabix index (.tbi)
wget -c "${BASE}/${FILE}"
wget -c "${BASE}/${FILE}.tbi"
```

`-c` lets `wget` **resume** a partial download if your connection drops.

If the `.tbi` index is not present alongside the file, build it yourself:

```bash
# Index a bgzipped VCF for fast region queries
tabix -p vcf "${FILE}"
```

Confirm the file opens and see how many records it holds:

```bash
variantflow stats "${FILE}"
```

### 3.2 Small-region shortcut (recommended first)

Slicing a **1 Mb window** with `tabix` gives you a small file (a few MB) that runs
every downstream step in seconds. This needs the `.tbi` index from Section 3.1.

```bash
# Extract chr22:20,000,000–21,000,000 into a small bgzipped VCF
tabix -h "${FILE}" chr22:20000000-21000000 | bgzip > chr22.slice.vcf.gz
tabix -p vcf chr22.slice.vcf.gz

variantflow stats chr22.slice.vcf.gz
```

> **Chromosome naming.** These files use `chr22` (with the `chr` prefix). If a
> region query returns nothing, check whether your file uses `22` instead and
> adjust (e.g. `22:20000000-21000000`). `variantflow stats` prints the
> contig names it sees.

For the rest of the guide, set one variable so you can switch between the slice and
the full file by editing a single line:

```bash
# Follow along fast with the slice:
VCF=chr22.slice.vcf.gz
# ...or run the full analysis:
# VCF="1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz"
```

---

## 4. Preprocessing

Raw call sets contain multiallelic sites, indels, and structural variants. Most
classic population-genetics statistics assume **biallelic SNPs** (exactly one
reference and one alternate allele). We clean up with `bcftools`, then optionally
apply quality filters with `variantflow filter`.

### 4.1 Reduce to biallelic SNPs (bcftools)

```bash
bcftools view -m2 -M2 -v snps "$VCF" -Oz -o chr22.snps.vcf.gz
tabix -p vcf chr22.snps.vcf.gz
```

Plain-language breakdown:

- `-m2 -M2` — keep sites with a **minimum and maximum of 2 alleles** (biallelic).
- `-v snps` — keep **SNPs only** (drop indels and structural variants).
- `-Oz` — write **bgzip-compressed** output; `-o` names it.
- `tabix -p vcf` — build the index so later region queries are fast.

### 4.2 Optional: select a subset of samples

Population statistics are usually computed within a defined group of individuals.
You control which samples enter a calculation with a **plain-text file, one sample
ID per line** (blank lines and `#` comments are ignored). VariantFlow's
per-sample commands accept `--keep` (use only these) or `--remove` (drop these).

We build population files from the 1000 Genomes sample panel in Section 5.8; for
now, here is the shape of a keep-list:

```bash
cat > keep.txt <<'EOF'
# one sample ID per line
HG00096
HG00097
HG00099
EOF
```

### 4.3 Quality filtering with `variantflow filter`

`variantflow filter` selects records with a **predicate expression** passed to
`--where`. The expression language understands the VCF fields `CHROM`, `POS`,
`QUAL`, `FILTER`, `INFO/<TAG>` (with array indexing like `INFO/AF[1]`), and
`FORMAT/<TAG>`, plus the aggregate `N_PASS(...)` for counting samples.

```bash
# Keep only PASS sites with a decent quality score
variantflow filter chr22.snps.vcf.gz \
  --where 'QUAL > 30 && FILTER == "PASS"' \
  -o chr22.clean.vcf.gz
tabix -p vcf chr22.clean.vcf.gz
```

More filter examples:

```bash
# Common variants only: alternate-allele frequency above 5%
variantflow filter chr22.snps.vcf.gz --where 'INFO/AF[0] > 0.05' -o chr22.common.vcf.gz

# Restrict to a region while filtering (needs an index)
variantflow filter chr22.snps.vcf.gz --region chr22:20000000-21000000 \
  --where 'QUAL > 30' -o chr22.region.vcf.gz

# Require at least 2 samples with a well-covered alternate allele
variantflow filter chr22.snps.vcf.gz \
  --where 'N_PASS(FORMAT/AD[1] > 10) >= 2' -o chr22.cohort.vcf.gz
```

> **Note on this dataset.** The 1000 Genomes high-coverage panel is already a
> filtered, phased release, so most sites are `PASS` and `QUAL` is not always
> informative. The filter step matters more on your own freshly called data; here
> it mostly demonstrates the syntax. For the analyses below we use the
> biallelic-SNP file (`chr22.snps.vcf.gz`), which is the important cleanup.

From here on, set:

```bash
SNPS=chr22.snps.vcf.gz
```

---

## 5. Population-genetics calculations

Each subsection gives **what it measures → the command → how to read the output**.
Outputs are tab-separated tables, easy to open in any spreadsheet or load in
Python/R.

### 5.1 Allele frequency (`freq`)

**What it measures.** How common each allele is at every site. The building block
for almost everything else.

```bash
variantflow freq "$SNPS" -o chr22.frq
```

Output columns: `CHROM  POS  N_ALLELES  N_CHR  {ALLELE:FREQ}`. `N_CHR` is the
number of chromosomes with data (twice the number of diploid samples with a
genotype). Example row:

```text
CHROM   POS     N_ALLELES   N_CHR   {ALLELE:FREQ}
chr22   20000123    2       6404    A:0.83  G:0.17
```

Here the alternate allele `G` has a frequency of 0.17. You can restrict to a group
with `--keep group.txt` / `--remove drop.txt`.

### 5.2 Missingness (`missingness`)

**What it measures.** How much genotype data is *absent* — per site and per
individual. High missingness can bias every other statistic.

```bash
variantflow missingness "$SNPS" -o chr22.miss
```

This writes two files from the prefix:

- `chr22.miss.lmiss` — **per-site**: `CHR POS N_DATA N_GENOTYPE_FILTERED N_MISS F_MISS`.
- `chr22.miss.imiss` — **per-individual**: `INDV N_DATA N_GENOTYPES_FILTERED N_MISS F_MISS`.

`F_MISS` is the fraction missing (0 = complete, 1 = all missing). On the phased
1000 Genomes panel this is essentially 0 everywhere — a useful sanity check that
the file is complete.

### 5.3 Individual heterozygosity (`het`)

**What it measures.** For each sample, the observed vs. expected number of
homozygous sites, summarised as an inbreeding coefficient **F**.

```bash
variantflow het "$SNPS" -o chr22.het
```

Columns: `INDV  O_HOM  E_HOM  N_SITES  F`. Interpretation of **F**:

- **F ≈ 0** — heterozygosity matches Hardy–Weinberg expectation (typical outbred).
- **F > 0** — fewer heterozygotes than expected (possible inbreeding, or population
  structure lumped together).
- **F < 0** — more heterozygotes than expected (can indicate contamination or
  mixing distinct groups).

### 5.4 Hardy–Weinberg equilibrium (`hardy`)

**What it measures.** At each site, whether observed genotype counts
(hom-ref / het / hom-alt) match Hardy–Weinberg proportions, with a chi-square test.

```bash
variantflow hardy "$SNPS" -o chr22.hwe
```

Columns: `CHROM POS OBS_HOM_REF OBS_HET OBS_HOM_ALT E_HOM_REF E_HET E_HOM_ALT CHISQ_HWE`.
A large `CHISQ_HWE` flags a departure from equilibrium. In a pooled multi-population
sample, apparent HWE departures are often just **population structure** (the
Wahlund effect), not genotyping error — interpret cautiously.

### 5.5 Nucleotide diversity π (`pi`, windowed)

**What it measures.** The average number of nucleotide differences per site between
two randomly chosen sequences — a core measure of within-population genetic
diversity. Reported in **windows** along the chromosome.

```bash
# 100 kb windows
variantflow pi "$SNPS" --window-size 100000 -o chr22.windowed.pi
```

Each row reports π for one window. Higher π = more diverse region; π varies along
the chromosome with mutation rate, recombination, and selection. Use `--keep` to
compute π within a single population (recommended — π mixed across populations is
inflated by between-group differences).

### 5.6 Tajima's D (`tajima-d`)

**What it measures.** Compares two estimates of diversity to detect departures from
neutral, constant-size evolution. Reported per window.

```bash
variantflow tajima-d "$SNPS" --window-size 100000 -o chr22.tajimaD.tsv
```

Rough reading (always cautious, and ideally within one population):

- **D ≈ 0** — consistent with neutral equilibrium.
- **D < 0** — excess of rare variants; consistent with **population expansion** or
  **purifying/positive selection** (a recent sweep).
- **D > 0** — excess of intermediate-frequency variants; consistent with
  **balancing selection** or a **population contraction/bottleneck**.

Genome-wide human samples often show mildly negative D reflecting population
growth — a value alone never proves selection.

### 5.7 Linkage disequilibrium (`ld`)

**What it measures.** Non-random association between alleles at nearby sites —
i.e., how correlated genotypes are as a function of distance.

```bash
# Consider pairs of sites up to 50 kb apart
variantflow ld "$SNPS" --max-distance 50000 -o chr22.geno.ld
```

Each row is a pair of sites with their LD statistic. LD generally **decays with
distance** (nearby sites are more correlated); strong long-range LD can mark
recombination coldspots or selection. Keep `--max-distance` modest — the number of
site pairs grows quickly and so does runtime and file size.

### 5.8 Fst between two populations (`fst`)

**What it measures.** Genetic differentiation between two populations: 0 = identical
allele frequencies, higher = more differentiated.

First, build the two population files from the 1000 Genomes **sample panel**, which
maps each sample to its population and super-population. Download it once:

```bash
wget -c https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000_genomes_project/1000genomes.sequence.index -O /dev/null 2>/dev/null || true

# The 3202-sample pedigree/panel with population labels:
PED="https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000G_2504_high_coverage/20130606_g1k_3202_samples_ped_population.txt"
wget -c "$PED" -O samples.panel
head -n2 samples.panel
```

The panel is whitespace-separated with a `SampleID` column and a `Population`
column (e.g. `GBR` = British in England/Scotland, `YRI` = Yoruba in Ibadan,
Nigeria). Extract two population lists — **one sample ID per line**, which is
exactly what `--pop` expects:

```bash
# Column positions can vary between panel files — inspect the header first:
head -n1 samples.panel
# Assuming a column named 'Population' (adjust the awk field number to match):

awk 'NR>1 && $0 ~ /GBR/ {print $2}' samples.panel > GBR.txt   # British
awk 'NR>1 && $0 ~ /YRI/ {print $2}' samples.panel > YRI.txt   # Yoruba
wc -l GBR.txt YRI.txt
```

> Different panel files order columns differently. Open the header with
> `head -n1 samples.panel`, find which column is the sample ID and which is the
> population, and set the `awk` fields accordingly. Each output file must end up as
> a plain list of sample IDs, one per line.

Now compute Fst (Hudson estimator by default; `--estimator weir-cockerham` is also
available):

```bash
variantflow fst "$SNPS" --pop GBR.txt --pop YRI.txt -o chr22.GBR_YRI.fst
```

Output columns: `CHROM POS HUDSON_FST` (per site). Interpretation:

- **Fst ≈ 0** — the two populations have nearly identical allele frequencies.
- **small positive Fst** (human continental comparisons are often ~0.1) — modest
  but real differentiation.
- **high Fst sites** — candidate loci under population-specific selection, though a
  single high value can also be noise.

Sites where an allele is monomorphic in both groups return `nan` (undefined) — that
is expected, not an error.

### 5.9 Missing-data-aware π and dxy (`pixy`)

**What it measures.** Windowed within-population diversity (π) and
between-population divergence (dxy) using the **pixy** estimator, which stays
**unbiased under missing data** by counting only non-missing pairwise comparisons.

**Key requirement:** `pixy` needs an **all-sites VCF** — one that includes
**invariant (monomorphic) sites**, not just variant sites. This is because correct
π/dxy is the ratio of differences to comparisons over *all* callable sites; a
variant-only file would inflate the estimates. The standard 1000 Genomes panel is
variant-only, so this command is meant for an all-sites call set (produced, e.g.,
by calling with `bcftools mpileup`/`call` and emitting all sites). Run it on such a
file:

```bash
# populations file: two whitespace/tab separated columns per line:
#   SampleID    PopulationName
# Example:
#   HG00096   GBR
#   NA18486   YRI
variantflow pixy allsites.vcf.gz \
  --populations pops.txt \
  --window-size 10000 \
  --out-pi   chr22.pixy_pi.tsv \
  --out-dxy  chr22.pixy_dxy.tsv
```

Note that `pixy` takes the input as a **positional argument** (not `--input`), and
writes its two outputs to `--out-pi` and `--out-dxy` (there is no `-o`). Read
`chr22.pixy_pi.tsv` (per-population π per window) and `chr22.pixy_dxy.tsv`
(per-population-pair dxy per window) the same way as Sections 5.5 and 5.8, but with
the reassurance that missing data has been handled correctly.

### 5.10 Site-frequency spectrum (`sfs`) — new / unverified

> **Verification note.** The `sfs` subcommand is present in the current source
> tree but is **not** in the prebuilt release binary used to verify this guide, so
> the commands below could **not** be run as-is. Rebuild from source
> (`cargo build --release`) to get it. Treat the exact flags as provisional.

**What it measures.** The site-frequency spectrum (SFS/AFS): a histogram of how
many variant sites have each allele count — the raw signal behind Tajima's D and
many demographic inferences.

```bash
# Unfolded (derived-allele) spectrum
variantflow sfs "$SNPS" -o chr22.sfs.tsv

# Folded (minor-allele) spectrum, when ancestral allele is unknown
variantflow sfs "$SNPS" --folded -o chr22.sfs_folded.tsv
```

Use `--folded` when you cannot polarise alleles into ancestral/derived; the folded
spectrum is indexed by minor-allele count. The unfolded output matches
`scikit-allel`'s `allel.sfs`, and the folded output matches `allel.sfs_folded`.

---

## 6. Interpretation

A short, cautious reading of what typical values suggest. **None of these is proof
on its own** — population-genetics signals have many confounders (structure,
demography, sample size, missing data), so treat single numbers as hypotheses.

- **High π** in a region means high local diversity. Across human populations,
  **African samples (e.g. YRI) typically show higher π** than non-African samples,
  reflecting the older, larger ancestral African population and the out-of-Africa
  bottleneck.
- **Negative Tajima's D** genome-wide is common in humans and consistent with
  **population expansion** (many rare variants). Localised, strongly negative D can
  hint at a recent selective sweep — but confirm with independent evidence.
- **Positive Tajima's D** suggests an excess of intermediate-frequency variants,
  consistent with **balancing selection** or a **bottleneck**.
- **Fst** between human continental groups is usually **modest** (often around
  0.1), the quantitative basis for the statement that most human genetic variation
  is *within* rather than *between* populations. Isolated high-Fst sites are
  selection candidates, not conclusions.
- **LD decaying with distance** is the expected baseline; unusually extended LD can
  flag low recombination or selection.
- **Missingness near zero** and **F near zero** on this curated panel are healthy
  sanity checks; large deviations on your own data usually mean a data-quality
  problem to fix *before* interpreting anything else.

---

## 7. Conclusion

Starting from a raw 1000 Genomes chromosome-22 VCF, you downloaded and indexed the
data, reduced it to clean biallelic SNPs, and computed a full suite of
population-genetics statistics — allele frequency, missingness, heterozygosity,
Hardy–Weinberg, nucleotide diversity, Tajima's D, linkage disequilibrium, Fst
between two human populations, and (on an all-sites file) the missing-data-aware
pixy estimates — all with a single tool and standard companions. The numbers
reproduce those from established tools, and the interpretations point toward the
classic story of human genetic variation: high diversity, modest between-population
differentiation, and signatures of past demographic change. Use these results as a
starting point for careful, hypothesis-driven analysis rather than as final claims.

---

## 8. References

1. Byrska-Bishop M, Evani US, Zhao X, et al. *High-coverage whole-genome
   sequencing of the expanded 1000 Genomes Project cohort including 602 trios.*
   **Cell** 185(18):3426–3440 (2022). doi:10.1016/j.cell.2022.08.004
2. Danecek P, Auton A, Abecasis G, et al. *The variant call format and VCFtools.*
   **Bioinformatics** 27(15):2156–2158 (2011). doi:10.1093/bioinformatics/btr330
3. Danecek P, Bonfield JK, Liddle J, et al. *Twelve years of SAMtools and BCFtools.*
   **GigaScience** 10(2):giab008 (2021). doi:10.1093/gigascience/giab008
4. Korunes KL, Samuk K. *pixy: Unbiased estimation of nucleotide diversity and
   divergence in the presence of missing data.* **Molecular Ecology Resources**
   21(4):1359–1368 (2021). doi:10.1111/1755-0998.13326
5. Miles A, et al. *scikit-allel: Explore and analyse genetic variation.*
   Zenodo software package. https://github.com/cggh/scikit-allel
