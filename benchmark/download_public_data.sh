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

case "$MODE" in
  all)
    download_giab_hg002
    download_igsr_chr22
    ;;
  giab-hg002)
    download_giab_hg002
    ;;
  igsr-chr22)
    download_igsr_chr22
    ;;
  *)
    echo "usage: $0 [all|giab-hg002|igsr-chr22]" >&2
    exit 2
    ;;
esac

cat <<EOF
Public data cache: $OUT_DIR
GIAB HG002: $GIAB_HG002_URL
1000 Genomes chr22: $IGSR_CHR22_URL
EOF
