//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Launches a difftool to

// Allowing dead code until this gets hooked up
#![allow(dead_code)]

use crate::cmd::Cmd;
use std::ffi::OsStr;
use std::process::Stdio;

#[derive(Debug, Default)]
pub struct Diff<C> {
    command: C,
}

impl<C: Cmd> Diff<C> {
    pub fn new(command: C) -> Self {
        Self { command }
    }

    pub fn launch<S1: AsRef<OsStr> + 'static, S2: AsRef<OsStr> + 'static>(
        &mut self,
        local: S1,
        remote: S2,
    ) -> Result<(), String> {
        self.command.arg(local);
        self.command.arg(remote);
        self.command.stdout(Stdio::piped());
        self.command.stderr(Stdio::piped());
        let output = self
            .command
            .output()
            .map_err(|e| format!("Failed launching difftool: {}", e))?;

        let status = output.status;
        if status.success() {
            Ok(())
        } else {
            Err(String::from_utf8(output.stderr)
                .map_err(|e| format!("Failed to convert error message: {}", e))?)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    use mockall::predicate::eq;
    use std::ffi::OsStr;
    use std::io;
    use std::os::unix::prelude::ExitStatusExt;
    use std::process::Stdio;
    use std::process::{ExitStatus, Output};

    mock! {
        C {}
        impl Cmd for C {
            fn arg<S: AsRef<OsStr> + 'static>(&mut self, arg: S) -> &mut Self;
            fn stdout(&mut self, cfg: Stdio) -> &mut Self;
            fn stderr(&mut self, cfg: Stdio) -> &mut Self;
            fn output(&mut self) -> io::Result<Output>;
        }
    }

    #[test]
    fn diff_launches_ok() {
        let local = "foo/baz/bar";
        let remote = "some/other/file";
        let mut mock = MockC::new();
        mock.expect_arg::<&str>()
            .with(eq(local))
            .times(1)
            .returning(|_| MockC::new());
        mock.expect_arg::<&str>()
            .with(eq(remote))
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

        let mut diff = Diff::new(mock);
        assert_eq!(diff.launch(local, remote), Ok(()));
    }

    #[test]
    fn diff_fails_to_launch() {
        let local = "foo/baz/bar";
        let remote = "some/other/file";
        let mut mock = MockC::new();
        mock.expect_arg::<&str>()
            .with(eq(local))
            .times(1)
            .returning(|_| MockC::new());
        mock.expect_arg::<&str>()
            .with(eq(remote))
            .times(1)
            .returning(|_| MockC::new());
        mock.expect_stdout().times(1).returning(|_| MockC::new());
        mock.expect_stderr().times(1).returning(|_| MockC::new());
        mock.expect_output().times(1).returning(|| {
            Ok(Output {
                status: ExitStatus::from_raw(1),
                stdout: vec![],
                stderr: b"an error message".to_vec(),
            })
        });

        let mut diff = Diff::new(mock);
        assert_eq!(
            diff.launch(local, remote),
            Err(String::from("an error message"))
        );
    }
}
