//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Launches a difftool to

use crate::cmd::Cmd;
use crate::Change;
use anyhow::Result;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::{Builder, TempDir};

#[derive(Debug)]
pub struct Diff {
    program: String,
    temp_dir: TempDir,
}

#[derive(Debug, Default)]
pub struct Difftool<C> {
    command: C,
    local: OsString,
    remote: OsString,
}

impl<C: Cmd> Difftool<C> {
    fn new(command: C, local: OsString, remote: OsString) -> Self {
        Self {
            command,
            local,
            remote,
        }
    }

    pub fn launch(&mut self) -> Result<()> {
        self.command.arg(self.local.clone());
        self.command.arg(self.remote.clone());
        self.command.stdout(Stdio::piped());
        self.command.stderr(Stdio::piped());
        self.command.output()?;
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

    pub fn difftool(&self, change: &Change) -> Result<Difftool<Command>> {
        let new = self.new_file_contents(change)?;
        let original = self.create_temp_original(change, &new)?;
        Ok(Difftool::new(
            Command::new(&self.program),
            original.into_os_string(),
            new.into_os_string(),
        ))
    }

    fn new_file_contents(&self, change: &Change) -> Result<PathBuf> {
        let dir = self.temp_dir.as_ref();
        let file = dir.join(&change.filename);
        fs::create_dir_all(
            file.parent()
                .expect("Should always have a parent temp path"),
        )?;

        let contents = reqwest::blocking::get(&change.raw_url)?.text()?;
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
    use mockall::mock;
    use mockall::predicate::eq;
    use std::fs;
    use std::io;
    use std::os::unix::prelude::ExitStatusExt;
    use std::process::Stdio;
    use std::process::{ExitStatus, Output};
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

    #[test]
    fn get_new_content() {
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
        let new_file = diff.new_file_contents(&change).unwrap();

        mock.assert();
        assert_eq!(
            fs::read(&new_file).unwrap(),
            contents.to_string().into_bytes()
        );
    }

    #[test]
    fn getting_a_second_set_of_new_content() {
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
        let new_file = diff.new_file_contents(&change).unwrap();

        mock.assert();
        assert_eq!(
            fs::read(&new_file).unwrap(),
            contents.to_string().into_bytes()
        );
    }

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

    #[test]
    fn difftool_launches_ok() {
        let local = OsString::from("foo/baz/bar");
        let remote = OsString::from("some/other/file");
        let mut mock = MockC::new();
        mock.expect_arg()
            .with(eq(local.clone()))
            .times(1)
            .returning(|_| MockC::new());
        mock.expect_arg()
            .with(eq(remote.clone()))
            .times(1)
            .returning(|_| MockC::new());
        mock.expect_stdout().times(1).returning(|_| MockC::new());
        mock.expect_stderr().times(1).returning(|_| MockC::new());
        mock.expect_output().times(1).returning(|| {
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: vec![],
                stderr: b"an error message".to_vec(),
            })
        });

        let mut difftool = Difftool::new(mock, local, remote);
        assert!(difftool.launch().is_ok());
    }
}
