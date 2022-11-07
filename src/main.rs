mod change_set;
mod cmd;
mod diff;
mod gh_interface;

use crate::change_set::Change;
use crate::diff::Diff;
use anyhow::Result;
use clap::Parser;
use std::process::Command;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The difftool command to run
    #[arg(short = 't', long = "tool", env = "DIFFTOOL")]
    difftool: Option<String>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let difftool = cli.difftool.as_deref().unwrap_or("bcompare");

    run_diff(difftool)
}

fn run_diff(difftool: impl AsRef<str>) -> Result<()> {
    let mut gh = gh_interface::GhCli::new(Command::new("gh"));
    let change_set = gh.local_change_set()?;
    for change in change_set.changes {
        let mut diff = Diff::new(change)?;
        diff.difftool(&difftool)?;
    }
    Ok(())
}
