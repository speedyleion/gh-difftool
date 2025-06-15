//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Launches a difftool to compare changes

use crate::Change;
use crate::gh_interface;
use crate::git_config;
use anyhow::Result;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{Builder, TempDir};

#[derive(Debug)]
pub struct Diff {
    difftool: git_config::Difftool,
    temp_dir: TempDir,
}

#[derive(Debug)]
pub struct Difftool<'a> {
    tool: &'a git_config::Difftool,
    local: OsString,
    remote: OsString,
}

impl<'a> Difftool<'a> {
    fn new(tool: &'a git_config::Difftool, local: OsString, remote: OsString) -> Self {
        Self {
            tool,
            local,
            remote,
        }
    }

    pub async fn launch(&self) -> Result<()> {
        self.tool.launch(&self.local, &self.remote).await
    }
}

impl Diff {
    pub fn new(difftool: git_config::Difftool) -> Result<Self> {
        let temp_dir = Builder::new().prefix("gh-difftool").tempdir()?;
        Ok(Self { difftool, temp_dir })
    }

    pub async fn difftool(&self, change: Change) -> Result<Difftool> {
        let new = self.new_file_contents(&change).await?;
        let original = self.create_temp_original(&change, &new)?;
        Ok(Difftool::new(
            &self.difftool,
            original.into_os_string(),
            new.into_os_string(),
        ))
    }

    async fn new_file_contents(&self, change: &Change) -> Result<PathBuf> {
        let dir = self.temp_dir.as_ref();
        let file = dir.join(&change.filename);
        fs::create_dir_all(
            file.parent()
                .expect("Should always have a parent temp path"),
        )?;

        let contents = gh_interface::file_contents(change).await?;
        fs::write(&file, contents)?;
        Ok(file)
    }

    fn create_temp_original(&self, change: &Change, new: impl AsRef<Path>) -> Result<PathBuf> {
        let dir = self.temp_dir.as_ref();
        let old_file_name = change
            .previous_filename
            .as_ref()
            .unwrap_or(&change.filename);
        let file = dir.join(format!("{}_{}", "base", old_file_name));
        fs::create_dir_all(
            file.parent()
                .expect("Should always have a parent temp path"),
        )?;

        change.reverse_apply(new, &file)?;
        Ok(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    const EOL: &'static str = "\r\n";
    #[cfg(not(windows))]
    const EOL: &str = "\n";

    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use httpmock::MockServer;
    use httpmock::prelude::GET;
    use std::fs;
    use temp_testdir::TempDir;
    use textwrap::dedent;

    fn difftool(dir: impl AsRef<Path>) -> git_config::Difftool {
        let dir = dir.as_ref();
        let git_dir = dir.join(".git");
        let config = git_dir.join("config");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(&config, "[difftool.bc]\n    path = bcomp").unwrap();
        git_config::Difftool::new(dir, Some("bc")).unwrap()
    }

    #[test]
    fn create_temp() {
        let temp = TempDir::default().permanent();
        let b = temp.join("b");
        let new = dedent(
            "
            line one
            line changed
            line three
            ",
        );
        fs::write(&b, new).unwrap();
        let diff = "@@ -1,3 +1,3 @@\n line one\n-line two\n+line changed\n line three";
        let expected = format!("{EOL}line one{EOL}line two{EOL}line three{EOL}");
        let change = Change {
            filename: "ignore_me".to_string(),
            contents_url: "sure".to_string(),
            patch: Some(diff.to_string()),
            status: "modified".to_string(),
            previous_filename: None,
            sha: "why not".to_string(),
        };
        let diff = Diff::new(difftool(&temp)).unwrap();
        let original = diff.create_temp_original(&change, b).unwrap();
        assert!(original.to_str().unwrap().ends_with(&change.filename));
        assert_eq!(fs::read(&original).unwrap(), expected.into_bytes());
    }

    #[test]
    fn renamed_diff() {
        let temp = TempDir::default().permanent();
        let b = temp.join("b");
        let new = dedent(
            "
            line one
            line changed
            line three
            ",
        );
        fs::write(&b, new).unwrap();
        let diff = "@@ -1,3 +1,3 @@\n line one\n-line two\n+line changed\n line three";
        let expected = format!("{EOL}line one{EOL}line two{EOL}line three{EOL}");
        let change = Change {
            filename: "ignore_me".to_string(),
            contents_url: "sure".to_string(),
            patch: Some(diff.to_string()),
            status: "renamed".to_string(),
            previous_filename: Some("new_filename".to_string()),
            sha: "why not".to_string(),
        };
        let diff = Diff::new(difftool(&temp)).unwrap();
        let original = diff.create_temp_original(&change, b).unwrap();
        assert!(
            original
                .to_str()
                .unwrap()
                .ends_with(change.previous_filename.as_ref().unwrap())
        );
        assert_eq!(fs::read(&original).unwrap(), expected.into_bytes());
    }

    #[tokio::test]
    async fn get_new_content() {
        let temp = TempDir::default();
        let contents = "line one\nline two";
        let encoded = STANDARD.encode(contents.as_bytes());
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/one.c");
            then.status(200)
                .header("content-type", "text/html")
                .body(format!(
                    "{{\"content\":\"{encoded}\", \"type\":\"file\", \"sha\": \"not used\"}}"
                ));
        });
        let change = Change {
            filename: "foo/bar/fish.ext".to_string(),
            contents_url: server.url("/one.c"),
            patch: Some("@@ -1,3 +1,3 @@\n doesn't matter".to_string()),
            status: "modified".to_string(),
            previous_filename: None,
            sha: "not used".to_string(),
        };
        let diff = Diff::new(difftool(&temp)).unwrap();
        let new_file = diff.new_file_contents(&change).await.unwrap();

        mock.assert();
        assert_eq!(
            fs::read(&new_file).unwrap(),
            contents.to_string().into_bytes()
        );
    }

    #[tokio::test]
    async fn getting_a_second_set_of_new_content() {
        let temp = TempDir::default();
        let contents = "something\nelse";
        let encoded = STANDARD.encode(contents.as_bytes());
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/some_raw_url/path");
            then.status(200)
                .header("content-type", "text/html")
                .body(format!(
                    "{{\"content\":\"{encoded}\", \"type\":\"file\", \"sha\": \"not used\"}}"
                ));
        });

        let change = Change {
            filename: "foo/bar/fish.ext".to_string(),
            contents_url: server.url("/some_raw_url/path"),
            patch: Some("@@ -1,3 +1,3 @@\n doesn't matter".to_string()),
            status: "modified".to_string(),
            previous_filename: None,
            sha: "not used".to_string(),
        };
        let diff = Diff::new(difftool(&temp)).unwrap();
        let new_file = diff.new_file_contents(&change).await.unwrap();

        mock.assert();
        assert_eq!(
            fs::read(&new_file).unwrap(),
            contents.to_string().into_bytes()
        );
    }
}
