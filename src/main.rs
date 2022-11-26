//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

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

    /// The GitHub repo to diff, defaults to the GitHub remote of the current git repo
    #[arg(long = "repo", requires = "pr", value_names = ["ORG/REPO_NAME"])]
    repo: Option<String>,

    /// The PR to diff, defaults to the one associated with the current branch
    #[arg(long = "pr")]
    pr: Option<usize>,

    /// Show only the names of files that changed in a PR
    #[arg(long = "name-only")]
    name_only: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let difftool = cli.difftool.as_deref().unwrap_or("bcompare");

    let mut gh = gh_interface::GhCli::new(Command::new("gh"));
    let pr = match cli.pr {
        None => gh.current_pr()?,
        Some(pr) => pr,
    };

    let repo = match cli.repo {
        None => gh.current_repo()?,
        Some(repo) => repo,
    };

    let change_set = gh.change_set(repo, pr)?;

    if cli.name_only {
        for change in change_set.changes {
            let filename = change.filename;
            println!("{filename}");
        }
        return Ok(());
    }

    let diff = Diff::new(difftool)?;
    for change in change_set.changes {
        let mut difftool = diff.difftool(&change)?;
        difftool.launch()?;

    }
    Ok(())
}
