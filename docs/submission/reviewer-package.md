# VariantFlow Reviewer Package

This file is the reviewer-party source of truth for independent submission
review. It is not a list of guaranteed journal reviewers; it is a conflict-aware
package for author discussion and journal submission forms.

## Suggested reviewer criteria

Suggested reviewers should have expertise in at least one of:

- VCF/BCF, HTSlib, `bcftools`, or variant-processing infrastructure;
- population and conservation/ecological genomics, especially VCFtools- or
  pixy-style diversity, differentiation, and missing-data-aware statistics;
- high-performance bioinformatics software, Rust/C/C++, compression, or
  columnar analytics;
- reproducible benchmarking for genomics tools.

Suggested reviewers must not have financial, supervisory, recent collaboration,
family, or institutional conflicts with any author.

## Suggested reviewers

Nominated by the corresponding author (J.-F. Mao) on 2026-08-31.

| Name | Institution | Email/profile | Expertise rationale | Conflict checked |
|---|---|---|---|---|
| Dr [redacted] | Swiss Federal Research Institute WSL, Biodiversity and Nature Conservation Biology / Ecological Genetics, Birmensdorf, Switzerland | [redacted] · [redacted] | Population and landscape genomics of forest trees and alpine plants; environmental association studies. Routine user of VCF-scale diversity and differentiation statistics, so well placed to judge the population-genetic scope and the missing-data handling. | Yes — no shared institution, no co-authorship found with either author |
| [redacted] | College of Life Sciences, Sichuan University, Chengdu, China | wangjing2019@scu.edu.cn · [redacted] | Plant genomics and adaptive evolution; large-cohort resequencing analyses. Positioned to assess whether the statistics implemented cover what population genomicists actually need day to day. | Yes — no shared institution, no co-authorship found with either author |
| Dr [redacted] | INRAE, UMR1202 BIOGECO, Cestas, France; Research Director | [redacted] · [redacted] | Ecological and population genetics and genomics of forest trees; coordinates EVOLTREE and leads OPTFORESTS. Judges the applied value to the conservation and forest-genomics communities the paper names as beneficiaries. | Yes — no shared institution, no co-authorship found with either author |
| Prof. [redacted] | Department of Ecology and Genetics, Plant Ecology and Evolution, Uppsala University, Sweden | [redacted] · [redacted] | Plant population genetics and evolutionary genomics; long-standing work on diversity and demographic inference from resequencing data. Well placed on estimator correctness, including the missing-data-aware pi and dxy validation against pixy. | Yes — different institution from the authors (Uppsala, not Umeå), no co-authorship found with either author |

**Conflict-check basis.** Nominations were made by the corresponding author, who
declares no financial, supervisory, recent-collaboration, family, or
institutional conflict with any of the four. This was corroborated by an
automated Crossref search for co-authorship between J.-F. Mao and each nominee,
which returned no shared publications; the apparent Jing Wang matches were
false positives from unrelated fields (catalysis, motor control) involving
different individuals with the same common surnames. Automated search is
corroborating rather than authoritative, and the declaration rests with the
authors.

**Coverage note.** The four span Switzerland, China, France, and Sweden, and
cover forest-tree population genomics, plant adaptive evolution, and
conservation genomics. All four are users of population-genetic statistics
rather than developers of variant-processing infrastructure. If the handling
editor prefers a reviewer from the tool-development side, candidates without
conflict would include the authors of bcftools/HTSlib, pixy, scikit-allel, or
vcfexpress.

## Opposed reviewers

| Name | Institution | Reason | Approved for submission form |
|---|---|---|---|
| None | — | — | — |

## Independent reviewer gate

The reviewer party should check:

- manuscript claims against `docs/public-benchmark-table.md`;
- source reports for exact commands, correctness, caveats, and versions;
- release/archive metadata;
- installation and reproducibility documentation;
- AI usage disclosure;
- conflict and authorship metadata;
- no broad "best VCF tool" or universal replacement claim.

The Reviewer Gate cannot pass while reviewer suggestions, opposed-reviewer
status, or conflict checks contain TODO placeholders.
