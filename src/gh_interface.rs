//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Module for interacting with the github command line

use crate::change_set::ChangeSet;
use crate::cmd::Cmd;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};
use std::fmt::{Display, Formatter};
use std::io::{Error, ErrorKind};
use std::process::Stdio;

#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct PullRequest {
    /// A repo in the form of "OWNER/REPO".  The owner and repo from
    /// "https://github.com/OWNER/REPO/pull/123"
    pub repo: String,

    /// The pull request number
    pub number: usize,
}

impl PullRequest {
    pub fn new_from_cwd() -> Result<Self> {
        let mut gh = GhCli::new(std::process::Command::new("gh"));
        let repo = gh.current_repo()?;
        let number = gh.current_pr()?;
        Ok(Self { repo, number })
    }
}

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
struct PrNumber {
    number: usize,
}

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
struct Owner {
    login: String,
}

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
struct Repo {
    name: String,
    owner: Owner,
}

impl Display for Repo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.owner.login, self.name)
    }
}

#[derive(Debug, Default)]
pub struct GhCli<C> {
    command: C,
}

impl<C: Cmd> GhCli<C> {
    pub fn new(command: C) -> Self {
        Self { command }
    }

    fn run_command<I, T>(&mut self, args: I) -> Result<String>
    where
        I: IntoIterator<Item = T>,
        T: AsRef<OsStr>,
    {
        let mut command = self.command.new_from_self();
        for arg in args {
            command.arg(OsString::from(arg.as_ref()));
        }
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        let output = command.output()?;
        let status = output.status;
        if status.success() {
            Ok(String::from_utf8(output.stdout)?)
        } else {
            Err(Error::new(
                ErrorKind::Other,
                String::from_utf8(output.stderr)?,
            ))?
        }
    }

    pub fn change_set(&mut self, pr: &PullRequest) -> Result<ChangeSet> {
        let repo = &pr.repo;
        let number = pr.number;
        let pr_path = format!("/repos/{repo}/pulls/{number}/files");
        let output =
            self.run_command(["api", "-H", "Accept: application/vnd.github+json", &pr_path])?;
        ChangeSet::try_from(output.as_str())
    }

    pub fn current_pr(&mut self) -> Result<usize> {
        let output = self.run_command(["pr", "view", "--json", "number"])?;
        let pr: PrNumber = serde_json::from_str(output.as_str())?;
        Ok(pr.number)
    }

    pub fn current_repo(&mut self) -> Result<String> {
        let output = self.run_command(["repo", "view", "--json", "owner,name"])?;
        let repo: Repo = serde_json::from_str(output.as_str())?;
        Ok(repo.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change_set::{Change, ChangeSet};
    use mockall::mock;
    use mockall::predicate::eq;
    use std::ffi::OsString;
    use std::io;
    use std::os::unix::prelude::ExitStatusExt;
    use std::process::Stdio;
    use std::process::{ExitStatus, Output};

    mock! {
        C {}
        impl Cmd for C {
            fn arg(&mut self, arg: OsString) -> &mut Self;
            fn stdout(&mut self, cfg: Stdio) -> &mut Self;
            fn stderr(&mut self, cfg: Stdio) -> &mut Self;
            fn output(&mut self) -> io::Result<Output>;
            fn new_from_self(&self) -> Self;
        }
    }

    fn change_set_mock(status: i32, stdout: &str, stderr: &str) -> MockC {
        mocked_command(
            &[
                "api",
                "-H",
                "Accept: application/vnd.github+json",
                "/repos/speedyleion/gh-difftool/pulls/10/files",
            ],
            status,
            stdout.as_ref(),
            stderr.as_ref(),
        )
    }

    fn mocked_command(args: &[&str], status: i32, stdout: &str, stderr: &str) -> MockC {
        let mut mock = MockC::new();
        let stdout = stdout.to_string();
        let stderr = stderr.to_string();
        let args = args
            .into_iter()
            .map(|s| String::from(*s))
            .collect::<Vec<_>>();
        mock.expect_new_from_self().returning(move || {
            let mut mock = MockC::new();
            let args = args.clone();
            for arg in args {
                mock.expect_arg()
                    .with(eq(OsString::from(&arg)))
                    .times(1)
                    .returning(|_| MockC::new());
            }
            mock.expect_stdout().times(1).returning(|_| MockC::new());
            mock.expect_stderr().times(1).returning(|_| MockC::new());
            let stdout = stdout.as_bytes().to_vec();
            let stderr = stderr.as_bytes().to_vec();
            mock.expect_output().times(1).returning(move || {
                Ok(Output {
                    status: ExitStatus::from_raw(status),
                    stdout: stdout.clone(),
                    stderr: stderr.clone(),
                })
            });
            mock
        });
        mock
    }

    fn pr_number_mock(status: i32, stdout: impl AsRef<str>, stderr: impl AsRef<str>) -> MockC {
        mocked_command(
            &["pr", "view", "--json", "number"],
            status,
            stdout.as_ref(),
            stderr.as_ref(),
        )
    }

    fn repo_mock(status: i32, stdout: impl AsRef<str>, stderr: impl AsRef<str>) -> MockC {
        mocked_command(
            &["repo", "view", "--json", "owner,name"],
            status,
            stdout.as_ref(),
            stderr.as_ref(),
        )
    }

    // The first file in the output from
    // `gh api  -H "Accept: application/vnd.github+json"  /repos/speedyleion/gh-difftool/pulls/10/files`
    const ONE_FILE: &str = r#"
            [
              {
                "sha": "b0a3777df4afc764c34234524267970025d55467",
                "filename": "Cargo.toml",
                "status": "modified",
                "additions": 4,
                "deletions": 0,
                "changes": 4,
                "blob_url": "https://github.com/speedyleion/gh-difftool/blob/befb7bf69c3c8ba97c714d57c8dadd9621021c84/Cargo.toml",
                "raw_url": "https://github.com/speedyleion/gh-difftool/raw/befb7bf69c3c8ba97c714d57c8dadd9621021c84/Cargo.toml",
                "contents_url": "https://api.github.com/repos/speedyleion/gh-difftool/contents/Cargo.toml?ref=befb7bf69c3c8ba97c714d57c8dadd9621021c84",
                "patch": "@@ -6,3 +6,7 @@ edition = \"2021\"\n [dev-dependencies]\n assert_cmd = \"2.0.4\"\n mockall = \"0.11.2\"\n+textwrap = \"0.15.1\"\n+\n+[dependencies]\n+patch = \"0.6.0\""
                }
            ]
        "#;

    // The first 2 files in the output from
    // `gh api  -H "Accept: application/vnd.github+json"  /repos/speedyleion/gh-difftool/pulls/10/files`
    const TWO_FILES: &str = r#"
            [
              {
                "sha": "b0a3777df4afc764c34234524267970025d55467",
                "filename": "Cargo.toml",
                "status": "modified",
                "additions": 4,
                "deletions": 0,
                "changes": 4,
                "blob_url": "https://github.com/speedyleion/gh-difftool/blob/befb7bf69c3c8ba97c714d57c8dadd9621021c84/Cargo.toml",
                "raw_url": "https://github.com/speedyleion/gh-difftool/raw/befb7bf69c3c8ba97c714d57c8dadd9621021c84/Cargo.toml",
                "contents_url": "https://api.github.com/repos/speedyleion/gh-difftool/contents/Cargo.toml?ref=befb7bf69c3c8ba97c714d57c8dadd9621021c84",
                "patch": "@@ -6,3 +6,7 @@ edition = \"2021\"\n [dev-dependencies]\n assert_cmd = \"2.0.4\"\n mockall = \"0.11.2\"\n+textwrap = \"0.15.1\"\n+\n+[dependencies]\n+patch = \"0.6.0\""
                },
                {
                "sha": "cb71da67691cdf5f595b4e64d4feaf0bdd7798f6",
                "filename": "src/main.rs",
                "status": "modified",
                "additions": 1,
                "deletions": 0,
                "changes": 1,
                "blob_url": "https://github.com/speedyleion/gh-difftool/blob/befb7bf69c3c8ba97c714d57c8dadd9621021c84/src%2Fmain.rs",
                "raw_url": "https://github.com/speedyleion/gh-difftool/raw/befb7bf69c3c8ba97c714d57c8dadd9621021c84/src%2Fmain.rs",
                "contents_url": "https://api.github.com/repos/speedyleion/gh-difftool/contents/src%2Fmain.rs?ref=befb7bf69c3c8ba97c714d57c8dadd9621021c84",
                "patch": "@@ -1,4 +1,5 @@\n mod gh_interface;\n+mod patch;\n \n fn main() {\n     println!(\"Hello, world!\");"
                }
            ]
        "#;

    #[test]
    fn single_change_available() {
        let mock = change_set_mock(0, ONE_FILE, "");
        let mut gh = GhCli::new(mock);
        assert_eq!(gh.change_set(&PullRequest{ repo: "speedyleion/gh-difftool".to_string(), number: 10}).unwrap(),
            ChangeSet {
                changes: vec![Change {
                    filename: String::from("Cargo.toml"),
                    raw_url: String::from("https://github.com/speedyleion/gh-difftool/raw/befb7bf69c3c8ba97c714d57c8dadd9621021c84/Cargo.toml"),
                    patch: String::from("@@ -6,3 +6,7 @@ edition = \"2021\"\n [dev-dependencies]\n assert_cmd = \"2.0.4\"\n mockall = \"0.11.2\"\n+textwrap = \"0.15.1\"\n+\n+[dependencies]\n+patch = \"0.6.0\""),
                }]
            }
        );
    }

    #[test]
    fn change_set_available() {
        let mock = change_set_mock(0, TWO_FILES, "");
        let mut gh = GhCli::new(mock);
        assert_eq!(gh.change_set(&PullRequest{ repo: "speedyleion/gh-difftool".to_string(), number: 10}).unwrap(),
            ChangeSet {
                changes: vec![
                    Change {
                        filename: String::from("Cargo.toml"),
                        raw_url: String::from("https://github.com/speedyleion/gh-difftool/raw/befb7bf69c3c8ba97c714d57c8dadd9621021c84/Cargo.toml"),
                        patch: String::from("@@ -6,3 +6,7 @@ edition = \"2021\"\n [dev-dependencies]\n assert_cmd = \"2.0.4\"\n mockall = \"0.11.2\"\n+textwrap = \"0.15.1\"\n+\n+[dependencies]\n+patch = \"0.6.0\""),
                    },
                    Change {
                        filename: String::from("src/main.rs"),
                        raw_url: String::from("https://github.com/speedyleion/gh-difftool/raw/befb7bf69c3c8ba97c714d57c8dadd9621021c84/src%2Fmain.rs"),
                        patch: String::from("@@ -1,4 +1,5 @@\n mod gh_interface;\n+mod patch;\n \n fn main() {\n     println!(\"Hello, world!\");"),
                    },
                ]
            }
        );
    }
    #[test]
    fn no_pr_change_set_available() {
        // The output from a non existent pr
        let expected = r#"
            {
              "message": "Not Found",
              "documentation_url": "https://docs.github.com/rest/reference/pulls#list-pull-requests-files"
            }
        "#;
        let mock = change_set_mock(1, expected, "gh: Not Found (HTTP 404)");
        let mut gh = GhCli::new(mock);
        let error = gh
            .change_set(&PullRequest {
                repo: "speedyleion/gh-difftool".to_string(),
                number: 10,
            })
            .unwrap_err();
        let root_cause = error.root_cause();
        assert_eq!(format!("{}", root_cause), "gh: Not Found (HTTP 404)");
    }

    #[test]
    fn bad_json() {
        let bad_json = r#"
            [
        "#;
        let mock = change_set_mock(0, bad_json, "");
        let mut gh = GhCli::new(mock);
        let error = gh
            .change_set(&PullRequest {
                repo: "speedyleion/gh-difftool".to_string(),
                number: 10,
            })
            .unwrap_err();
        let root_cause = error.root_cause();
        assert_eq!(
            format!("{}", root_cause),
            "EOF while parsing a list at line 3 column 8"
        );
    }

    #[test]
    fn current_pr_number_is_10() {
        let pr_json = r#"
            {
                "number": 10
            }
        "#;
        let mock = pr_number_mock(0, pr_json, "");
        let mut gh = GhCli::new(mock);
        assert_eq!(gh.current_pr().unwrap(), 10);
    }

    #[test]
    fn current_pr_number_is_8() {
        let pr_json = r#"
            {
                "number": 8
            }
        "#;
        let mock = pr_number_mock(0, pr_json, "");
        let mut gh = GhCli::new(mock);
        assert_eq!(gh.current_pr().unwrap(), 8);
    }

    #[test]
    fn bad_json_for_current_pr() {
        let pr_json = r#"
            {
        "#;
        let mock = pr_number_mock(0, pr_json, "");
        let mut gh = GhCli::new(mock);
        let error = gh.current_pr().unwrap_err();
        let root_cause = error.root_cause();
        assert_eq!(
            format!("{}", root_cause),
            "EOF while parsing an object at line 3 column 8"
        );
    }

    #[test]
    fn failure_running_gh_pr_command() {
        let mock = pr_number_mock(1, "", "no pull requests found for branch \"what\"");
        let mut gh = GhCli::new(mock);
        let error = gh.current_pr().unwrap_err();
        let root_cause = error.root_cause();
        assert_eq!(
            format!("{}", root_cause),
            "no pull requests found for branch \"what\""
        );
    }

    #[test]
    fn repo_name_is_this_repo() {
        // Output of `gh repo view --json owner,name` on this repo
        let repo_json = r#"
            {
                "name": "gh-difftool",
                "owner": {
                    "id": "MDQ6VXNlcjE0MDA1Mzk=",
                    "login": "speedyleion"
                }
            }
        "#;
        let mock = repo_mock(0, repo_json, "");
        let mut gh = GhCli::new(mock);
        assert_eq!(
            gh.current_repo().unwrap(),
            String::from("speedyleion/gh-difftool")
        );
    }

    #[test]
    fn repo_name_is_foo_bar() {
        let repo_json = r#"
            {
                "name": "bar",
                "owner": {
                    "id": "surewhatever",
                    "login": "foo"
                }
            }
        "#;
        let mock = repo_mock(0, repo_json, "");
        let mut gh = GhCli::new(mock);
        assert_eq!(gh.current_repo().unwrap(), String::from("foo/bar"));
    }

    #[test]
    fn bad_json_for_current_repo() {
        let bad_json = r#"
            {
        "#;
        let mock = repo_mock(0, bad_json, "");
        let mut gh = GhCli::new(mock);
        let error = gh.current_repo().unwrap_err();
        let root_cause = error.root_cause();
        assert_eq!(
            format!("{}", root_cause),
            "EOF while parsing an object at line 3 column 8"
        );
    }

    #[test]
    fn failure_running_gh_repo_command() {
        let mock = repo_mock(1, "", "none of the git remotes configured for this repository point to a known GitHub host. To tell gh about a new GitHub host, please use `gh auth login`");
        let mut gh = GhCli::new(mock);
        let error = gh.current_repo().unwrap_err();
        let root_cause = error.root_cause();
        assert_eq!(
            format!("{}", root_cause),
            "none of the git remotes configured for this repository point to a known GitHub host. To tell gh about a new GitHub host, please use `gh auth login`"
        );
    }
}
