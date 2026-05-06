#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="${VCF_FAST_PUBLIC_DATA_DIR:-tests/output/public-data}"
MODE="${1:-all}"
mkdir -p "$OUT_DIR"

GIAB_HG002_URL="https://ftp-trace.ncbi.nlm.nih.gov/ReferenceSamples/giab/release/AshkenazimTrio/HG002_NA24385_son/NISTv4.2.1/GRCh38/HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz"
GIAB_HG002_TBI_URL="${GIAB_HG002_URL}.tbi"
IGSR_CHR22_URL="https://ftp.1000genomes.ebi.ac.uk/vol1/ftp/data_collections/1000G_2504_high_coverage/working/20220422_3202_phased_SNV_INDEL_SV/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz"
IGSR_CHR22_TBI_URL="${IGSR_CHR22_URL}.tbi"
FORMAT_TRIO_URL="https://sourceforge.net/projects/project123vcf/files/Benchmark_Data/NA12878.trio.hg19_multianno.vcf.gz/download"
FORMAT_WGS_TRIO_URL="https://zenodo.org/records/3697103/files/trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz?download=1"
FORMAT_WGS_TRIO_TBI_URL="https://zenodo.org/records/3697103/files/trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz.tbi?download=1"
FORMAT_OVIS453_URL="https://ftp.sra.ebi.ac.uk/vol1/analysis/ERZ324/ERZ324584/19.filtered_intersect.vcf.gz"
FORMAT_OVIS453_TBI_URL="${FORMAT_OVIS453_URL}.tbi"
FORMAT_CATTLE29_URL="https://ftp.sra.ebi.ac.uk/vol1/analysis/ERZ184/ERZ18456468/Dutch_Genebank_Cattle_Y_merged.vcf.gz"
FORMAT_HUMAN_CHM13_CHR22_URL="https://ddbj.nig.ac.jp/public/public-human-genomes/CHM13/JointCall/CHM13_autosome_PAR.chr22.vcf.gz"
FORMAT_HUMAN_CHM13_CHR22_CSI_URL="${FORMAT_HUMAN_CHM13_CHR22_URL}.csi"

download_if_missing() {
  local url="$1"
  local output="$2"

  if [[ -s "$output" ]]; then
    echo "cached $output"
    return
  fi

  echo "downloading $url"
  curl -L --fail --retry 3 --output "$output" "$url"
}

download_giab_hg002() {
  download_if_missing "$GIAB_HG002_URL" "$OUT_DIR/HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz"
  download_if_missing "$GIAB_HG002_TBI_URL" "$OUT_DIR/HG002_GRCh38_1_22_v4.2.1_benchmark.vcf.gz.tbi"
}

download_igsr_chr22() {
  download_if_missing "$IGSR_CHR22_URL" "$OUT_DIR/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz"
  download_if_missing "$IGSR_CHR22_TBI_URL" "$OUT_DIR/1kGP_high_coverage_Illumina.chr22.filtered.SNV_INDEL_SV_phased_panel.vcf.gz.tbi"
}

download_format_trio() {
  download_if_missing "$FORMAT_TRIO_URL" "$OUT_DIR/NA12878.trio.hg19_multianno.vcf.gz"
}

download_format_wgs_trio() {
  download_if_missing "$FORMAT_WGS_TRIO_URL" "$OUT_DIR/trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz"
  download_if_missing "$FORMAT_WGS_TRIO_TBI_URL" "$OUT_DIR/trio_NA12878-NA12891-NA12892_hs37d5_dbsnp.vcf.gz.tbi"
}

download_format_ovis453() {
  # ENA ERZ324584: "Sheep genomes variants high quality - 19"; 453 sheep
  # with FORMAT/AD and FORMAT/DP sample fields.
  download_if_missing "$FORMAT_OVIS453_URL" "$OUT_DIR/19.filtered_intersect.vcf.gz"
  download_if_missing "$FORMAT_OVIS453_TBI_URL" "$OUT_DIR/19.filtered_intersect.vcf.gz.tbi"
}

download_format_cattle29() {
  # ENA ERZ18456468 / PRJEB60909: Dutch Genebank Cattle Y merged VCF;
  # 29 cattle with declared FORMAT/AD, FORMAT/DP, and FORMAT/GQ fields.
  download_if_missing "$FORMAT_CATTLE29_URL" "$OUT_DIR/Dutch_Genebank_Cattle_Y_merged.vcf.gz"
}

download_format_human_chm13_chr22() {
  # DDBJ public-human-genomes CHM13 JointCall chr22: 3715 human samples with
  # declared FORMAT/AD, FORMAT/DP, and FORMAT/GQ fields. The VCF is about
  # 27 GB compressed, so the default downloader caches only the CSI index and
  # a URL manifest. Set VCF_FAST_ALLOW_HUGE_DOWNLOAD=1 to cache the full VCF.
  download_if_missing "$FORMAT_HUMAN_CHM13_CHR22_CSI_URL" "$OUT_DIR/CHM13_autosome_PAR.chr22.vcf.gz.csi"
  printf '%s\n' "$FORMAT_HUMAN_CHM13_CHR22_URL" >"$OUT_DIR/CHM13_autosome_PAR.chr22.vcf.gz.url"
  if [[ "${VCF_FAST_ALLOW_HUGE_DOWNLOAD:-0}" = "1" ]]; then
    download_if_missing "$FORMAT_HUMAN_CHM13_CHR22_URL" "$OUT_DIR/CHM13_autosome_PAR.chr22.vcf.gz"
  else
    echo "skipping 27 GB CHM13 human VCF; benchmark/run_v20_human_format_cohort.sh streams bounded tiers from the URL"
  fi
}

case "$MODE" in
  all)
    download_giab_hg002
    download_igsr_chr22
    download_format_trio
    download_format_wgs_trio
    download_format_ovis453
    download_format_cattle29
    download_format_human_chm13_chr22
    ;;
  giab-hg002)
    download_giab_hg002
    ;;
  igsr-chr22)
    download_igsr_chr22
    ;;
  format-trio)
    download_format_trio
    ;;
  format-wgs-trio)
    download_format_wgs_trio
    ;;
  format-ovis453)
    download_format_ovis453
    ;;
  format-cattle29)
    download_format_cattle29
    ;;
  format-human-chm13-chr22)
    download_format_human_chm13_chr22
    ;;
  *)
    echo "usage: $0 [all|giab-hg002|igsr-chr22|format-trio|format-wgs-trio|format-ovis453|format-cattle29|format-human-chm13-chr22]" >&2
    exit 2
    ;;
esac

cat <<EOF
Public data cache: $OUT_DIR
GIAB HG002: $GIAB_HG002_URL
1000 Genomes chr22: $IGSR_CHR22_URL
FORMAT-rich NA12878 trio: $FORMAT_TRIO_URL
Larger FORMAT-rich WGS trio: $FORMAT_WGS_TRIO_URL
FORMAT-rich 453 sheep cohort: $FORMAT_OVIS453_URL
FORMAT-rich 29 cattle cohort: $FORMAT_CATTLE29_URL
FORMAT-rich 3715 human CHM13 chr22 cohort: $FORMAT_HUMAN_CHM13_CHR22_URL
EOF
