fn main() {
    if let Err(error) = vcf_fast::cli::run_with_name("variantflow") {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
