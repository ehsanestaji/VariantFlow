fn main() {
    if let Err(error) = vcf_fast::cli::run_with_name("vcf-fast") {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
