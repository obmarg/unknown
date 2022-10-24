



mod cli;
mod config;
mod git;
mod hashing;
mod workspace;

// TODO: Consider using camino::Utf8PathBuf everywhere instead...

fn main() -> miette::Result<()> {
    cli::run()
}
