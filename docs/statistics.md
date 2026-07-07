# Statistics reference

VariantFlow computes population-genetics statistics that share one property: each
can be evaluated in a **single streaming pass** over sites or windows, without
holding the whole genotype matrix in memory. That streaming model is what lets it
scale to large cohorts on commodity CPUs, and it defines the boundary of what
VariantFlow covers (see [Out of scope](#out-of-scope) below).

Every command reads a VCF/BCF and writes a tab-separated (TSV) table. Sample
subsets are selected with `--keep` / `--remove` (a file of sample IDs, one per
line) where supported; population comparisons take one or more population files.
The column names below are the literal header lines VariantFlow emits.

The grouping here mirrors the paper's **Supplementary Table S5**, which organizes
statistics by their computational class: streaming site/window statistics
(VariantFlow's domain) versus matrix- or haplotype-wide statistics (deferred to
scikit-allel).

---

## Diversity & divergence

### `pi` — nucleotide diversity (π)

Average number of pairwise nucleotide differences per comparison. With
`--window-size` VariantFlow reports π per non-overlapping window; without it,
per-site π.

**Estimator.** Per biallelic site with allele count and sample size, π is the
expected number of differences between two randomly drawn alleles. The windowed
value sums the per-site contributions across the window.

```bash
variantflow pi input.vcf.gz \
    --window-size 10000 \
    --output pi.tsv
```

**Output columns**

- Windowed: `CHROM  BIN_START  BIN_END  N_VARIANTS  PI`
- Per-site (no `--window-size`): `CHROM  POS  PI`

**Equivalent:** VCFtools `--window-pi` / `--site-pi`; scikit-allel
`sequence_diversity`.

---

### `pixy` — missing-data-aware π and d~XY~

A faithful reimplementation of [pixy](https://pixy.readthedocs.io/): windowed
nucleotide diversity (π) within populations and absolute divergence (d~XY~)
between population pairs, computed so that **missing genotypes and invariant
sites are handled correctly**. This requires an **all-sites VCF** (invariant sites
included), not a variants-only file.

**Estimator.** For a population of *n* alleles at a site, let `c_a` be the count
of allele *a*. The number of pairwise comparisons is

$$\text{comparisons} = \binom{n}{2} = \frac{n(n-1)}{2},$$

and the number of pairwise differences is

$$\text{differences} = \frac{n^2 - \sum_a c_a^2}{2}.$$

The windowed value is the ratio of summed differences to summed comparisons over
all sites in the window:

$$\pi_\text{window} = \frac{\sum \text{differences}}{\sum \text{comparisons}}.$$

d~XY~ uses the same accounting across the two populations at each site. Reporting
the summed `count_diffs` and `count_comparisons` alongside the ratio is what makes
the result robust to missing data — windows are compared on equal footing.

```bash
variantflow pixy allsites.vcf.gz \
    --populations pops.txt \
    --window-size 10000 \
    --out-pi pi.txt \
    --out-dxy dxy.txt
```

`pops.txt` is a two-column, tab-separated file mapping each sample to a population
(the pixy populations-file format).

**Output columns**

- π file: `pop  chromosome  window_pos_1  window_pos_2  avg_pi  no_sites  count_diffs  count_comparisons`
- d~XY~ file: `pop1  pop2  chromosome  window_pos_1  window_pos_2  avg_dxy  no_sites  count_diffs  count_comparisons`

**Equivalent:** pixy (validated to exact agreement, including on missing data).

---

### `tajima-d` — Tajima's *D* (and Watterson's θ)

Tajima's *D* contrasts two estimators of θ — pairwise diversity (π) and the number
of segregating sites (Watterson's θ~w~) — to test for departures from neutrality.
It is reported per window.

**Estimator.** Within each window VariantFlow accumulates the per-site pairwise
diversity (the π component) and the count of segregating sites *S*. Watterson's θ
is *S / a₁* where *a₁ = Σ 1/i* over *i = 1 … n−1* for *n* sampled chromosomes;
Tajima's *D* is the standardized difference between the π-based and θ~w~-based
estimators. Watterson's θ is thus available as a **component of the same windowed
pass** (Supplementary Table S5 marks it `tajima-d†`).

```bash
variantflow tajima-d input.vcf.gz \
    --window-size 10000 \
    --output tajimad.tsv
```

**Output columns:** `CHROM  BIN_START  N_SNPS  TajimaD`

**Equivalent:** VCFtools `--TajimaD`; scikit-allel `tajima_d` (and
`watterson_theta` for the θ~w~ component).

---

### `sfs` — site frequency spectrum

A histogram of allele counts across sites: how many sites have each derived-allele
count. This is the empirical input to many demographic and selection inferences.

**Estimator.** The **unfolded** spectrum tallies sites by their *derived*-allele
count (requires correctly polarized ALT alleles). The **folded** spectrum
(`--folded`) tallies by *minor*-allele count instead, which needs no ancestral
information and is the safe default when polarization is uncertain.

```bash
# Folded SFS (minor-allele counts)
variantflow sfs input.vcf.gz --folded --output sfs.tsv
```

**Output columns:** `ALLELE_COUNT  N_SITES`

**Equivalent:** scikit-allel `sfs` / `sfs_folded` (validated to exact agreement).

---

## Differentiation & linkage

### `fst` — fixation index (F~ST~)

Between-population differentiation. VariantFlow implements two standard
estimators, selectable with `--estimator`:

- **Hudson** (`hudson`, the default) — the Hudson/Bhatia ratio-of-averages
  estimator.
- **Weir–Cockerham** (`weir-cockerham`) — the classic AMOVA-style estimator.

`--pop` is given once per population file (repeatable). Output is per-site F~ST~.

```bash
variantflow fst input.vcf.gz \
    --pop popA.txt \
    --pop popB.txt \
    --estimator weir-cockerham \
    --output fst.tsv
```

**Output columns**

- Hudson: `CHROM  POS  HUDSON_FST`
- Weir–Cockerham: `CHROM  POS  WEIR_AND_COCKERHAM_FST`

Sites where F~ST~ is undefined (e.g. monomorphic in both populations) are written
as `nan`.

**Equivalent:** VCFtools `--weir-fst-pop`; scikit-allel `hudson_fst` /
`weir_cockerham_fst`.

---

### `ld` — linkage disequilibrium (r²)

Correlation (r²) between genotypes at pairs of sites, optionally capped by genomic
distance with `--max-distance` to keep the pairwise work bounded.

```bash
variantflow ld input.vcf.gz \
    --max-distance 100000 \
    --output ld.tsv
```

**Output columns:** `CHR  POS1  POS2  N_INDV  R^2`

**Equivalent:** VCFtools `--geno-r2`; scikit-allel `rogers_huff_r` (r; r² is its
square).

---

## Frequencies & quality

### `freq` — allele frequencies

Per-site allele counts and frequencies, in the VCFtools `--freq` layout.

```bash
variantflow freq input.vcf.gz --output freq.tsv
```

**Output columns:** `CHROM  POS  N_ALLELES  N_CHR  {ALLELE:FREQ}` (one
`ALLELE:FREQ` field per allele).

**Equivalent:** VCFtools `--freq`; scikit-allel `allele_frequencies`.

---

### `missingness` — genotype missingness

Per-site and per-individual missing-genotype rates, matching VCFtools `--missing`.

```bash
variantflow missingness input.vcf.gz --output miss
```

**Output columns**

- Per-site (`.lmiss`): `CHR  POS  N_DATA  N_GENOTYPE_FILTERED  N_MISS  F_MISS`
- Per-individual (`.imiss`): `INDV  N_DATA  N_GENOTYPES_FILTERED  N_MISS  F_MISS`

**Equivalent:** VCFtools `--missing-site` / `--missing-indv`.

---

### `het` — heterozygosity and inbreeding (F)

Per-individual observed vs. expected homozygosity and the method-of-moments
inbreeding coefficient *F*, matching VCFtools `--het`.

```bash
variantflow het input.vcf.gz --output het.tsv
```

**Output columns:** `INDV  O_HOM  E_HOM  N_SITES  F`

**Equivalent:** VCFtools `--het`; scikit-allel `heterozygosity_*`.

---

### `hardy` — Hardy–Weinberg equilibrium

Per-site observed and expected genotype counts and a χ² test of Hardy–Weinberg
equilibrium, matching VCFtools `--hardy`.

```bash
variantflow hardy input.vcf.gz --output hardy.tsv
```

**Output columns:** `CHROM  POS  OBS_HOM_REF  OBS_HET  OBS_HOM_ALT  E_HOM_REF  E_HET  E_HOM_ALT  CHISQ_HWE`

**Equivalent:** VCFtools `--hardy`.

---

## Out of scope

Some population-genetics statistics require the **whole genotype or phased
haplotype matrix in memory simultaneously** and therefore fall outside
VariantFlow's streaming model. VariantFlow **defers these to scikit-allel by
design** rather than reimplementing them — it complements, and does not replace,
matrix-oriented libraries. This is the Group B / Group A split of Supplementary
Table S5.

Deferred to scikit-allel:

- **Selection scans** — EHH, iHS, XP-EHH, nSL (`ehh`, `ihs`, …).
- **Haplotype statistics** — Garud's *H*, haplotype diversity.
- **D- / f-statistics** — ABBA-BABA (Patterson's *D*), *f₂/f₃/f₄*.
- **Joint / scaled 2D SFS** — `joint_sfs`, `sfs_scaled`.
- **Ordination** — PCA, PCoA, distance matrices.

For any of these, export once with `variantflow convert --to parquet` (or load the
VCF directly) and use [scikit-allel](https://scikit-allel.readthedocs.io/). See
[Choosing the right tool](tool-comparison.md) for how VariantFlow and scikit-allel
fit together.
