# Installation

VariantFlow is written in **Rust** and ships as a single self-contained binary.
The only build requirement is the Rust toolchain; companion bioinformatics tools
(bcftools, tabix) are optional and used for data preparation, not by VariantFlow
itself.

## 1. Install the Rust toolchain

If you do not already have `cargo`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustc --version   # sanity check
cargo --version
```

## 2. Build from source

From the repository root:

```bash
# Standard build — pure-Rust VCF/BCF text path
cargo build --release

# The binary lands here:
./target/release/variantflow --version
```

### HTSlib backend (optional)

If you need **BCF input or region-based queries backed by HTSlib** (for example,
`--region chr22:1-1000000` on a `.bcf` file), build with the HTSlib feature. The
`htslib-static` feature links HTSlib statically so the resulting binary has no
runtime dependency on a system `libhts`:

```bash
# Statically linked HTSlib backend (needs a C toolchain)
cargo build --release --features htslib-static
```

!!! note
    The static HTSlib build requires a working C toolchain (a C compiler,
    `make`, and the usual autotools) available on your `PATH`. On most Linux
    systems these come from `build-essential`; on macOS install the Xcode
    command-line tools with `xcode-select --install`.

## 3. Put the binary on your PATH (optional)

For readability the rest of the documentation writes `variantflow` rather than the
full `./target/release/variantflow` path. Either add the release directory to your
`PATH` or keep typing the full path:

```bash
export PATH="$PWD/target/release:$PATH"
variantflow --help
```

## 4. Companion tools and dependencies

A few standard bioinformatics tools handle the jobs VariantFlow deliberately does
*not*: downloading, indexing, and heavy VCF surgery (splitting multiallelic sites,
normalization). None are required to run VariantFlow, but you will want them for a
realistic workflow.

| Tool | Used for | Install |
|------|----------|---------|
| **bcftools / HTSlib (`tabix`, `bgzip`)** | index VCFs, subset to biallelic SNPs, slice regions, normalize | `conda install -c bioconda bcftools htslib` |
| **VCFtools** | optional cross-check of VariantFlow's statistics | `conda install -c bioconda vcftools` |
| **wget** / **curl** | download data | usually preinstalled; `conda install -c conda-forge wget` |

## 5. Verify the installation

```bash
variantflow --version          # prints the version string
variantflow --help             # lists all 15 subcommands
variantflow filter --help      # per-command help and options
```

You should see these subcommands: `filter`, `stats`, `freq`, `missingness`,
`hardy`, `het`, `fst`, `pi`, `pixy`, `tajima-d`, `ld`, `index`, `diff`, `convert`.

Once installed, continue with the [User Guide](user-guide.md) for an end-to-end
walkthrough, or the [Statistics reference](statistics.md) for the full catalogue
of population-genetics commands.
