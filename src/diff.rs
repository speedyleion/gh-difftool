//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Launches a difftool to compare changes

use crate::Change;
use anyhow::Result;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tempfile::{Builder, TempDir};
use tokio::process::Command;

#[derive(Debug)]
pub struct Diff {
    program: String,
    temp_dir: TempDir,
}

#[derive(Debug, Default)]
pub struct Difftool {
    program: String,
    local: OsString,
    remote: OsString,
}

impl Difftool {
    fn new(program: impl AsRef<str>, local: OsString, remote: OsString) -> Self {
        Self {
            program: program.as_ref().to_string(),
            local,
            remote,
        }
    }

    pub async fn launch(&self) -> Result<()> {
        let mut command = Command::new(&self.program);
        command.arg(self.local.clone());
        command.arg(self.remote.clone());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.output().await?;
        // Some difftools, like bcompare, will return non zero status when there is a diff and 0
        // only when there are no changes.  This prevents us from trusting the status
        Ok(())
    }
}

impl Diff {
    pub fn new(program: impl AsRef<str>) -> Result<Self> {
        let temp_dir = Builder::new().prefix("gh-difftool").tempdir()?;
        Ok(Self {
            program: program.as_ref().to_string(),
            temp_dir,
        })
    }

    pub async fn difftool(&self, change: Change) -> Result<Difftool> {
        let new = self.new_file_contents(&change).await?;
        let original = self.create_temp_original(&change, &new)?;
        Ok(Difftool::new(
            &self.program,
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

        let contents = reqwest::get(&change.raw_url).await?.text().await?;
        fs::write(&file, contents)?;
        Ok(file)
    }

    fn create_temp_original(&self, change: &Change, new: impl AsRef<Path>) -> Result<PathBuf> {
        let dir = self.temp_dir.as_ref();
        let file = dir.join(format!("{}_{}", "base", &change.filename));
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
    use httpmock::prelude::GET;
    use httpmock::MockServer;
    use std::fs;
    use temp_testdir::TempDir;
    use textwrap::dedent;

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
        let expected = dedent(
            "
            line one
            line two
            line three
            ",
        );
        let change = Change {
            filename: "ignore_me".to_string(),
            raw_url: "sure".to_string(),
            patch: diff.to_string(),
        };
        let diff = Diff::new("stuff").unwrap();
        let original = diff.create_temp_original(&change, b).unwrap();
        assert_eq!(fs::read(&original).unwrap(), expected.into_bytes());
    }

    #[tokio::test]
    async fn get_new_content() {
        let contents = "line one\nline two";
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/one.c");
            then.status(200)
                .header("content-type", "text/html")
                .body(contents);
        });
        let change = Change {
            filename: "foo/bar/fish.ext".to_string(),
            raw_url: server.url("/one.c"),
            patch: "@@ -1,3 +1,3 @@\n doesn't matter".to_string(),
        };
        let diff = Diff::new("sure").unwrap();
        let new_file = diff.new_file_contents(&change).await.unwrap();

        mock.assert();
        assert_eq!(
            fs::read(&new_file).unwrap(),
            contents.to_string().into_bytes()
        );
    }

    #[tokio::test]
    async fn getting_a_second_set_of_new_content() {
        let contents = "something\nelse";

        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/some_raw_url/path");
            then.status(200)
                .header("content-type", "text/html")
                .body(contents);
        });

        let change = Change {
            filename: "foo/bar/fish.ext".to_string(),
            raw_url: server.url("/some_raw_url/path"),
            patch: "@@ -1,3 +1,3 @@\n doesn't matter".to_string(),
        };
        let diff = Diff::new("stuff").unwrap();
        let new_file = diff.new_file_contents(&change).await.unwrap();

        mock.assert();
        assert_eq!(
            fs::read(&new_file).unwrap(),
            contents.to_string().into_bytes()
        );
    }
}
