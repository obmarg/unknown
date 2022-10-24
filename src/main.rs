mod cli;
mod config;
mod git;
mod hashing;
mod workspace;

fn main() -> miette::Result<()> {
    cli::run()
}
