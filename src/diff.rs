//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Launches a difftool to

use anyhow::Result;
use crate::cmd::Cmd;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;
use crate::Change;

#[derive(Debug, Default)]
pub struct Diff {
    change: Change,
}

#[derive(Debug, Default)]
struct Difftool<C> {
    command: C,
}

impl<C: Cmd> Difftool<C> {
    pub fn new(command: C) -> Self {
        Self { command }
    }

    pub fn launch(&mut self, local: &OsStr, remote: &OsStr) -> Result<()> {
        self.command.arg(OsString::from(local));
        self.command.arg(OsString::from(remote));
        self.command.stdout(Stdio::piped());
        self.command.stderr(Stdio::piped());
        self.command.output()?;
        // Some difftools, like bcompare, will return non zero status when there is a diff and 0
        // only when there are no changes.  This prevents us from trusting the status
        Ok(())
    }
}

impl Diff {
    pub fn new(change: Change) -> Self {
        Self { change }
    }

    pub fn difftool(&self, program: impl AsRef<str>) -> Result<()> {
        let original = self.create_temp_original()?;
        let mut difftool = Difftool::new(Command::new(program.as_ref()));
        difftool.launch(original.path().as_os_str(), OsStr::new(&self.change.filename))
    }

    pub(crate) fn new_file_contents(&self) -> Result<NamedTempFile> {
        let file = NamedTempFile::new()?;
        fs::write(&file, "line one\nline two")?;
        Ok(file)
    }

    fn create_temp_original(&self) -> Result<NamedTempFile> {
        let file = NamedTempFile::new()?;

        self.change.reverse_apply(&self.change.filename, file.path())?;
        Ok(file)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use mockall::predicate::eq;
    use std::io;
    use std::os::unix::prelude::ExitStatusExt;
    use std::process::Stdio;
    use std::process::{ExitStatus, Output};
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
            filename: b.to_string_lossy().to_string(),
            raw_url: "sure".to_string(),
            patch: diff.to_string(),
        };
        let diff = Diff::new(change);
        let original = diff.create_temp_original().unwrap();
        assert_eq!(fs::read(&original.path()).unwrap(), expected.into_bytes());
    }

    #[test]
    fn get_new_content() {
        let change = Change {
            filename: "foo/bar/fish.ext".to_string(),
            raw_url: "sure".to_string(),
            patch: "@@ -1,3 +1,3 @@\n doesn't matter".to_string(),
        };
        let diff = Diff::new(change);
        let new_file = diff.new_file_contents().unwrap();

        let expected = "line one\nline two";
        assert_eq!(fs::read(&new_file.path()).unwrap(), expected.to_string().into_bytes());

    }

    #[test]
    fn getting_a_second_set_of_new_content() {
        let change = Change {
            filename: "foo/bar/fish.ext".to_string(),
            raw_url: "sure".to_string(),
            patch: "@@ -1,3 +1,3 @@\n doesn't matter".to_string(),
        };
        let diff = Diff::new(change);
        let new_file = diff.new_file_contents().unwrap();

        let expected = "something\nelse";
        assert_eq!(fs::read(&new_file.path()).unwrap(), expected.to_string().into_bytes());
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

        let mut difftool = Difftool::new(mock);
        assert!(difftool.launch(local.as_os_str(), remote.as_os_str()).is_ok());
    }
}
