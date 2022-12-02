//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

mod change_set;
mod cmd;
mod diff;
mod gh_interface;

use crate::change_set::{Change, ChangeSet};
use crate::diff::{Diff, Difftool};
use anyhow::Result;
use clap::Parser;
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use std::collections::VecDeque;
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

#[tokio::main]
async fn main() -> Result<()> {
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

    diff(difftool, change_set).await?;
    Ok(())
}

/// A thin wrapper around [Difftool::launch()]. It allows for a common future when there is nothing
/// to diff
async fn launch_difftool(difftool: Option<Difftool>) -> Result<()> {
    if let Some(difftool) = difftool {
        difftool.launch().await
    } else {
        Ok(())
    }
}

/// Launches a difftool for each change in `change_set`.
///
/// Similar to git-difftool only one change will be opened at a time in the difftool. The difftool
/// of the changes will be executed in the same order as the changes.
///
/// # Arguments
/// * `difftool` - The command name of the difftool to use
/// * `change_set` - The changes to run the difftool on
///
/// # Implementation Details
/// In an effort to speed up performance `async` behavior has been done. The logic uses 2 queues:
///
/// 1. a queue to download and create the temporary diff files
/// 2. a queue to launch the difftool on the next change ready for diffing
///
/// The reason for the 2 queues is to prevent launching multiple difftool instances. We only want
/// one instance up at a time until the user dismisses it. While the difftool is up and has not
/// been dismissed, the downloading and creation of temporary diff files will proceed.
async fn diff(difftool: impl AsRef<str>, change_set: ChangeSet) -> Result<()> {
    let diff = Diff::new(difftool)?;
    {
        let mut stream = FuturesOrdered::new();
        for change in change_set.changes {
            stream.push_back(diff.difftool(change));
        }

        // See https://tokio.rs/tokio/tutorial/select#resuming-an-async-operation on this pattern
        // Initialize to done, because the `launch_difftool(None)` will return a consumed future.
        let mut done = true;
        let diff_future = launch_difftool(None);
        tokio::pin!(diff_future);

        let mut diffs = VecDeque::new();

        loop {
            tokio::select! {
                Some(new_diff) = stream.next() => {
                    diffs.push_back(new_diff?);
                    // Be sure and set this back to false since the other branch will set to true
                    // if the `diffs` had happened to be empty last time through.
                    done = false;
                },
                _ = &mut diff_future, if !done => {
                    if let Some(diffthing) = diffs.pop_front() {
                        diff_future.set(launch_difftool(Some(diffthing)));
                    } else {
                        done = true;
                    }
                },
                else => break,
            }
        }
    }
    Ok(())
}
