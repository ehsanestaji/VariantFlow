nextflow.enable.dsl=2

params.input_vcf = params.input_vcf ?: 'input.vcf.gz'
params.filter_expr = params.filter_expr ?: 'QUAL > 30'

process VARIANTFLOW_FILTER {
    input:
    path input_vcf

    output:
    path 'filtered.vcf'

    script:
    """
    variantflow filter ${input_vcf} --where '${params.filter_expr}' -o filtered.vcf
    """
}

process VARIANTFLOW_EXPORT_PARQUET {
    input:
    path filtered_vcf

    output:
    path 'variants.parquet'

    script:
    """
    variantflow convert ${filtered_vcf} --to parquet -o variants.parquet
    """
}

workflow {
    filtered = VARIANTFLOW_FILTER(file(params.input_vcf))
    VARIANTFLOW_EXPORT_PARQUET(filtered)
}
