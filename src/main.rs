mod cli;
mod config;
mod diagnostics;
mod git;
mod hashing;
mod workspace;

#[cfg(test)]
mod test_files;

fn main() -> miette::Result<()> {
    cli::run()
}
