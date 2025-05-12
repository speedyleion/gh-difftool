//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Module for interacting with the github command line

use crate::change_set::ChangeSet;
use crate::cmd::Cmd;
use crate::Change;
use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};
use std::fmt::{Display, Formatter};
use std::io::{Error, ErrorKind};
use std::process::Stdio;
use tokio::process::Command;

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

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
struct Content {
    #[serde(rename = "type")]
    type_: String,
    sha: String,
    content: Option<String>,
}

fn output_to_string(output: std::process::Output) -> Result<String> {
    if output.status.success() {
        Ok(String::from_utf8(output.stdout)?)
    } else {
        Err(Error::new(
            ErrorKind::Other,
            String::from_utf8(output.stderr)?,
        ))?
    }
}

async fn run_async_command<I, T>(args: I) -> Result<String>
where
    I: IntoIterator<Item = T>,
    T: AsRef<OsStr>,
{
    let mut command = Command::new("gh");
    for arg in args {
        command.arg(OsString::from(arg.as_ref()));
    }
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let output = command.output().await?;
    output_to_string(output)
}

pub async fn file_contents(change: &Change) -> Result<String> {
    let output = run_async_command([
        "api",
        "--method",
        "GET",
        "-H",
        "Accept: application/vnd.github+json",
        &change.contents_url,
    ])
    .await?;

    let content: Content = serde_json::from_str(output.as_str())?;

    if content.type_ == "submodule" {
        return Ok(content.sha);
    }

    // Not sure why, but the base64 encoded contents from github has newlines
    // in it, removing these newlines still leaves the newlines that are encoded
    // into the base64 string so the diff will still be good.
    let cleaned = content.content.unwrap_or_default().replace('\n', "");
    let bytes = STANDARD.decode(cleaned)?;
    Ok(String::from_utf8(bytes)?)
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
        output_to_string(output)
    }

    pub fn change_set(&mut self, pr: &PullRequest) -> Result<ChangeSet> {
        let repo = &pr.repo;
        let number = pr.number;
        let pr_path = format!("/repos/{repo}/pulls/{number}/files");

        // The `gh` command line supports a `--paginate` flag which could potentially do this all
        // for us. When using paginate `gh` increases the items per page to the max of 100.
        // Unfortunately this results in the `patch` property being omitted on the last couple of
        // entries. By doing it manually we keep the page size at 30 entries and are able to
        // maintain the `patch` property on the files.
        let (pages, mut changes) = self.changes_first_page(&pr_path)?;
        for page in 2..=pages {
            changes.extend(self.changes_subsequent_page(page, &pr_path)?);
        }
        Ok(ChangeSet { changes })
    }

    /// Get a page changes that is after the first page.
    ///
    /// Simplified logic that doesn't look at the link header
    fn changes_subsequent_page(&mut self, page: usize, pr_path: &str) -> Result<Vec<Change>> {
        let output = self.run_command([
            "api",
            "--method",
            "GET",
            "-F",
            &format!("page={page}"),
            pr_path,
        ])?;
        Ok(serde_json::from_str(output.as_str())?)
    }

    /// Get the first page of changes
    ///
    /// Will parse the link header, if present to provide the total number of pages available
    /// When no link header is present then only one page worth of changes exists
    fn changes_first_page(&mut self, pr_path: &str) -> Result<(usize, Vec<Change>)> {
        let output = self.run_command([
            "api",
            "--method",
            "GET",
            "--include",
            "-F",
            "page=1",
            pr_path,
        ])?;
        let pages = if let Some(link) = output.lines().find(|l| l.starts_with("Link:")) {
            Self::changes_page_count(
                link.strip_prefix("Link:")
                    .expect("Prefix should have existed due to find call"),
            )?
        } else {
            1
        };
        Ok((
            pages,
            serde_json::from_str(output.as_str().lines().last().ok_or_else(|| {
                Error::new(
                    ErrorKind::Other,
                    format!("Should have had multiple lines in {output}"),
                )
            })?)?,
        ))
    }

    /// Number of pages that make up all of the changes in a pr.
    fn changes_page_count(link_header: &str) -> Result<usize> {
        let header = parse_link_header::parse_with_rel(link_header)?;
        if let Some(entry) = header.get("last") {
            let page = entry.queries.get("page").expect("Malformed link header");
            Ok(page.parse().expect("Page is not a valid integer"))
        } else {
            panic!("Expected a total page count in the link header")
        }
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
    use httpmock::Method::GET;
    use httpmock::MockServer;
    use mockall::mock;
    use mockall::predicate::eq;
    use std::ffi::OsString;
    use std::io;
    #[cfg(unix)]
    use std::os::unix::prelude::ExitStatusExt;
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;
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
                "--method",
                "GET",
                "--include",
                "-F",
                "page=1",
                "/repos/speedyleion/gh-difftool/pulls/10/files",
            ],
            status,
            stdout,
            stderr,
        )
    }

    fn mocked_command(args: &[&str], status: i32, stdout: &str, stderr: &str) -> MockC {
        let mut mock = MockC::new();
        let stdout = stdout.to_string();
        let stderr = stderr.to_string();
        let args = args.iter().map(|s| String::from(*s)).collect::<Vec<_>>();
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
                    status: ExitStatus::from_raw(status.try_into().unwrap()),
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
        let mock = change_set_mock(0, &ONE_FILE.replace("\n", ""), "");
        let mut gh = GhCli::new(mock);
        assert_eq!(gh.change_set(&PullRequest{ repo: "speedyleion/gh-difftool".to_string(), number: 10}).unwrap(),
            ChangeSet {
                changes: vec![Change {
                    filename: String::from("Cargo.toml"),
                    contents_url: String::from("https://api.github.com/repos/speedyleion/gh-difftool/contents/Cargo.toml?ref=befb7bf69c3c8ba97c714d57c8dadd9621021c84"),
                    patch: Some("@@ -6,3 +6,7 @@ edition = \"2021\"\n [dev-dependencies]\n assert_cmd = \"2.0.4\"\n mockall = \"0.11.2\"\n+textwrap = \"0.15.1\"\n+\n+[dependencies]\n+patch = \"0.6.0\"".into()),
                    status: String::from("modified"),
                    previous_filename: None,
                    sha: String::from("b0a3777df4afc764c34234524267970025d55467"),
                }]
            }
        );
    }

    #[test]
    fn change_set_available() {
        let mock = change_set_mock(0, &TWO_FILES.replace("\n", ""), "");
        let mut gh = GhCli::new(mock);
        assert_eq!(gh.change_set(&PullRequest{ repo: "speedyleion/gh-difftool".to_string(), number: 10}).unwrap(),
            ChangeSet {
                changes: vec![
                    Change {
                        filename: String::from("Cargo.toml"),
                        contents_url: String::from("https://api.github.com/repos/speedyleion/gh-difftool/contents/Cargo.toml?ref=befb7bf69c3c8ba97c714d57c8dadd9621021c84"),
                        patch: Some("@@ -6,3 +6,7 @@ edition = \"2021\"\n [dev-dependencies]\n assert_cmd = \"2.0.4\"\n mockall = \"0.11.2\"\n+textwrap = \"0.15.1\"\n+\n+[dependencies]\n+patch = \"0.6.0\"".into()),
                        status: String::from("modified"),
                        previous_filename: None,
                        sha: String::from("b0a3777df4afc764c34234524267970025d55467"),
                    },
                    Change {
                        filename: String::from("src/main.rs"),
                        contents_url: String::from("https://api.github.com/repos/speedyleion/gh-difftool/contents/src%2Fmain.rs?ref=befb7bf69c3c8ba97c714d57c8dadd9621021c84"),
                        patch: Some("@@ -1,4 +1,5 @@\n mod gh_interface;\n+mod patch;\n \n fn main() {\n     println!(\"Hello, world!\");".into()),
                        status: String::from("modified"),
                        previous_filename: None,
                        sha: String::from("cb71da67691cdf5f595b4e64d4feaf0bdd7798f6"),
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
        let mock = change_set_mock(1, &expected.replace("\n", ""), "gh: Not Found (HTTP 404)");
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
        let mock = change_set_mock(0, &bad_json.replace("\n", ""), "");
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
            "EOF while parsing a list at line 1 column 21"
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

    // The output of
    // `gh api https://api.github.com/repos/speedyleion/gh-difftool/contents/Cargo.toml?ref=befb7bf69c3c8ba97c714d57c8dadd9621021c84`
    const CARGO_CONTENTS: &str = r#"
        {
          "name": "Cargo.toml",
          "path": "Cargo.toml",
          "sha": "b0a3777df4afc764c34234524267970025d55467",
          "size": 178,
          "url": "https://api.github.com/repos/speedyleion/gh-difftool/contents/Cargo.toml?ref=befb7bf69c3c8ba97c714d57c8dadd9621021c84",
          "html_url": "https://github.com/speedyleion/gh-difftool/blob/befb7bf69c3c8ba97c714d57c8dadd9621021c84/Cargo.toml",
          "git_url": "https://api.github.com/repos/speedyleion/gh-difftool/git/blobs/b0a3777df4afc764c34234524267970025d55467",
          "download_url": "https://raw.githubusercontent.com/speedyleion/gh-difftool/befb7bf69c3c8ba97c714d57c8dadd9621021c84/Cargo.toml",
          "type": "file",
          "content": "W3BhY2thZ2VdCm5hbWUgPSAiZ2gtZGlmZnRvb2wiCnZlcnNpb24gPSAiMC4x\nLjAiCmVkaXRpb24gPSAiMjAyMSIKCltkZXYtZGVwZW5kZW5jaWVzXQphc3Nl\ncnRfY21kID0gIjIuMC40Igptb2NrYWxsID0gIjAuMTEuMiIKdGV4dHdyYXAg\nPSAiMC4xNS4xIgoKW2RlcGVuZGVuY2llc10KcGF0Y2ggPSAiMC42LjAiCg==\n",
          "encoding": "base64",
          "_links": {
            "self": "https://api.github.com/repos/speedyleion/gh-difftool/contents/Cargo.toml?ref=befb7bf69c3c8ba97c714d57c8dadd9621021c84",
            "git": "https://api.github.com/repos/speedyleion/gh-difftool/git/blobs/b0a3777df4afc764c34234524267970025d55467",
            "html": "https://github.com/speedyleion/gh-difftool/blob/befb7bf69c3c8ba97c714d57c8dadd9621021c84/Cargo.toml"
          }
        }
        "#;

    // The output of `gh api https://api.github.com/repos/speedyleion/gh-difftool/contents/src%2Fmain.rs?ref=befb7bf69c3c8ba97c714d57c8dadd9621021c84`
    const MAIN_CONTENTS: &str = r#"
        {
          "name": "main.rs",
          "path": "src/main.rs",
          "sha": "cb71da67691cdf5f595b4e64d4feaf0bdd7798f6",
          "size": 75,
          "url": "https://api.github.com/repos/speedyleion/gh-difftool/contents/src/main.rs?ref=befb7bf69c3c8ba97c714d57c8dadd9621021c84",
          "html_url": "https://github.com/speedyleion/gh-difftool/blob/befb7bf69c3c8ba97c714d57c8dadd9621021c84/src/main.rs",
          "git_url": "https://api.github.com/repos/speedyleion/gh-difftool/git/blobs/cb71da67691cdf5f595b4e64d4feaf0bdd7798f6",
          "download_url": "https://raw.githubusercontent.com/speedyleion/gh-difftool/befb7bf69c3c8ba97c714d57c8dadd9621021c84/src/main.rs",
          "type": "file",
          "content": "bW9kIGdoX2ludGVyZmFjZTsKbW9kIHBhdGNoOwoKZm4gbWFpbigpIHsKICAg\nIHByaW50bG4hKCJIZWxsbywgd29ybGQhIik7Cn0K\n",
          "encoding": "base64",
          "_links": {
            "self": "https://api.github.com/repos/speedyleion/gh-difftool/contents/src/main.rs?ref=befb7bf69c3c8ba97c714d57c8dadd9621021c84",
            "git": "https://api.github.com/repos/speedyleion/gh-difftool/git/blobs/cb71da67691cdf5f595b4e64d4feaf0bdd7798f6",
            "html": "https://github.com/speedyleion/gh-difftool/blob/befb7bf69c3c8ba97c714d57c8dadd9621021c84/src/main.rs"
          }
        }
    "#;

    // The output of `gh api https://api.github.com/repos/deep-foundation/dev/contents/packages/deepcase?ref=81b77114b9b70a012c880dbf23aa48bb90a17501`
    const SUBMODULE_CONTENTS: &str = r#"
        {
          "name": "deepcase",
          "path": "packages/deepcase",
          "sha": "7c8ba583177b9e14cb85346f52e7b5536935a051",
          "size": 0,
          "url": "https://api.github.com/repos/deep-foundation/dev/contents/packages/deepcase?ref=81b77114b9b70a012c880dbf23aa48bb90a17501",
          "html_url": "https://github.com/deep-foundation/deepcase/tree/7c8ba583177b9e14cb85346f52e7b5536935a051",
          "git_url": "https://api.github.com/repos/deep-foundation/deepcase/git/trees/7c8ba583177b9e14cb85346f52e7b5536935a051",
          "download_url": null,
          "type": "submodule",
          "submodule_git_url": "https://github.com/deep-foundation/deepcase.git",
          "_links": {
            "self": "https://api.github.com/repos/deep-foundation/dev/contents/packages/deepcase?ref=81b77114b9b70a012c880dbf23aa48bb90a17501",
            "git": "https://api.github.com/repos/deep-foundation/deepcase/git/trees/7c8ba583177b9e14cb85346f52e7b5536935a051",
            "html": "https://github.com/deep-foundation/deepcase/tree/7c8ba583177b9e14cb85346f52e7b5536935a051"
          }
        }
    "#;

    #[tokio::test]
    async fn contents_of_cargo_toml() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/cargo_toml/contents");
            then.status(200)
                .header("content-type", "text/html")
                .body(CARGO_CONTENTS);
        });
        let change = Change {
            contents_url: server.url("/cargo_toml/contents"),
            ..Default::default()
        };
        let expected = r#"
            [package]
            name = "gh-difftool"
            version = "0.1.0"
            edition = "2021"

            [dev-dependencies]
            assert_cmd = "2.0.4"
            mockall = "0.11.2"
            textwrap = "0.15.1"

            [dependencies]
            patch = "0.6.0"
        "#;
        assert_eq!(
            file_contents(&change).await.unwrap(),
            textwrap::dedent(expected).trim_start()
        );
        mock.assert();
    }

    #[tokio::test]
    async fn contents_of_main() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/main/contents");
            then.status(200)
                .header("content-type", "text/html")
                .body(MAIN_CONTENTS);
        });
        let change = Change {
            contents_url: server.url("/main/contents"),
            ..Default::default()
        };
        let expected = r#"
            mod gh_interface;
            mod patch;

            fn main() {
                println!("Hello, world!");
            }
        "#;
        assert_eq!(
            file_contents(&change).await.unwrap(),
            textwrap::dedent(expected).trim_start()
        );
        mock.assert();
    }

    #[tokio::test]
    async fn contents_of_submodule() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/deepcase/contents");
            then.status(200)
                .header("content-type", "text/html")
                .body(SUBMODULE_CONTENTS);
        });
        let change = Change {
            contents_url: server.url("/deepcase/contents"),
            ..Default::default()
        };
        let result = file_contents(&change).await;
        mock.assert();
        assert_eq!(result.unwrap(), "7c8ba583177b9e14cb85346f52e7b5536935a051");
    }
}
