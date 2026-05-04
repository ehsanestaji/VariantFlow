fn main() {
    if let Err(error) = vcf_fast::cli::run() {
        eprintln!("{error:#}");
        std::process::exit(1);
    }
}
