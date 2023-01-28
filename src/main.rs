//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

mod change_set;
mod cmd;
mod diff;
mod gh_interface;
mod git_config;

use crate::change_set::{Change, ChangeSet};
use crate::diff::{Diff, Difftool};
use crate::gh_interface::PullRequest;
use anyhow::Result;
use clap::{ArgAction, Parser};
use futures::stream::FuturesOrdered;
use futures::StreamExt;
use itertools::Itertools;
use std::collections::VecDeque;
use std::process::Command;
use url::Url;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The tool to use for diffing
    #[arg(short = 't', long = "tool", env = "GH_DIFFTOOL")]
    tool: Option<String>,

    /// The GitHub repo to diff, defaults to the GitHub remote of the current git repo
    #[arg(short = 'R', long = "repo", requires = "pr", value_names = ["OWNER/REPO"])]
    repo: Option<String>,

    /// The pull request to diff
    ///
    /// When omitted the pull request associated with the current branch will be used
    /// A pull request can be supplied as argument in any of the following formats:
    /// - by number, e.g. "123"
    /// - by URL, e.g. "https://github.com/OWNER/REPO/pull/123"
    #[arg(value_parser=parse_pr, verbatim_doc_comment)]
    pr: Option<PullRequest>,

    /// Show only the names of files that changed in a pull request
    #[arg(long = "name-only")]
    name_only: bool,

    /// Start showing the diff for the given file, skipping all the files before it.
    #[arg(long = "skip-to")]
    skip_to: Option<String>,

    /// Specific files to diff.
    ///
    /// When not provided all of the files that changed in the pull request
    /// will be diffed
    #[arg(last=true, action=ArgAction::Append)]
    files: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let mut gh = gh_interface::GhCli::new(Command::new("gh"));
    let mut pr = match cli.pr {
        Some(pr) => pr,
        None => PullRequest::new_from_cwd()?,
    };

    if let Some(repo) = cli.repo {
        pr.repo = repo;
    };

    let mut change_set = gh.change_set(&pr)?;

    let files = cli.files;
    if !files.is_empty() {
        change_set.filter_files(&files);
    }

    if let Some(filename) = cli.skip_to {
        change_set.skip_to(filename)?;
    }

    if cli.name_only {
        for change in change_set.changes {
            let filename = change.filename;
            println!("{filename}");
        }
        return Ok(());
    }

    // Important, do this after the name only check as name only doesn't need a difftool
    let difftool = git_config::Difftool::new(std::env::current_dir()?, cli.tool.as_deref())?;
    diff(difftool, change_set).await?;
    Ok(())
}

/// A thin wrapper around [Difftool::launch()]. It allows for a common future when there is nothing
/// to diff
async fn launch_difftool(difftool: Option<Difftool<'_>>) -> Result<()> {
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
async fn diff(difftool: git_config::Difftool, change_set: ChangeSet) -> Result<()> {
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
                result = &mut diff_future, if !done => {
                    //TODO need to make this error more useful. Getting errors
                    // with no context isn't nice, but it's better than not
                    // getting the errors.
                    if let Err(error) = result {
                        println!("{error:?}");
                    }

                    if let Some(diffthing) = diffs.pop_front() {
                        diff_future.set(launch_difftool(Some(diffthing)));
                    } else {
                        diff_future.set(launch_difftool(None));
                        done = true;
                    }
                },
                else => break,
            }
        }
    }
    Ok(())
}

#[derive(Debug, displaydoc::Display, Eq, PartialEq)]
pub enum Error {
    /// PR URL is not valid: {0}
    PrUrl(String),
}

impl std::error::Error for Error {}

/// Parse a PR from the command line
///
/// A pull request can be supplied as argument in any of the following formats:
/// - by number, e.g. "123"
/// - by URL, e.g. "https://github.com/OWNER/REPO/pull/123"
fn parse_pr(pr: &str) -> Result<PullRequest> {
    if let Ok(number) = pr.parse() {
        let mut gh = gh_interface::GhCli::new(Command::new("gh"));
        let repo = gh.current_repo()?;
        return Ok(PullRequest { repo, number });
    }
    let url = Url::parse(pr)?;
    let components = url
        .path_segments()
        .map(|c| c.collect::<Vec<_>>())
        .expect("Should only fail for cannot-be-a-base urls");
    let number = components
        .get(3)
        .ok_or_else(|| Error::PrUrl(pr.to_string()))?
        .parse()?;

    // Note since the "3" up above will error out, we know for sure we have 2 components
    let repo = components.into_iter().take(2).join("/");
    Ok(PullRequest { repo, number })
}

#[cfg(test)]
mod tests {
    use super::*;
    use yare::parameterized;

    #[parameterized(
    empty = {""},
    not_a_url = {"nothing/to/it"},
    domain_only = {"https://github.com"},
    pr_location_is_not_a_number = {"https://github.com/repo/owner/pull/not_a_number"},
    )]
    fn pr_url_parsing_errors(bad_url: &str) {
        assert!(parse_pr(bad_url).is_err())
    }

    #[parameterized(
    ten = {"10", 10},
    twelve = {"12", 12},
    five = {"5", 5}
    )]
    fn parse_pr_from_a_number(number_str: &str, expected: usize) {
        let result = parse_pr(number_str).unwrap();
        assert_eq!(
            result,
            PullRequest {
                repo: "speedyleion/gh-difftool".to_string(),
                number: expected
            }
        );
    }

    #[parameterized(
    gh_difftool = {"https://github.com/speedyleion/gh-difftool/pull/10", "speedyleion/gh-difftool", 10},
    custom_1 = {"https://some_host.what/an-owner/a-repo-name/pull/3", "an-owner/a-repo-name", 3},
    custom_2 = {"https://my_domain.com/the_best/bad_code/pull/21", "the_best/bad_code", 21},
    )]
    fn parse_pr_from_url(url: &str, expected_repo: &str, expected_number: usize) {
        let result = parse_pr(url).unwrap();
        assert_eq!(
            result,
            PullRequest {
                repo: expected_repo.to_string(),
                number: expected_number
            }
        );
    }
}
