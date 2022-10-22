//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Launches a difftool to

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

    pub fn launch(&mut self, local: &OsStr, remote: &OsStr) -> Result<(), String> {
        self.command.arg(local);
        self.command.arg(remote);
        self.command.stdout(Stdio::piped());
        self.command.stderr(Stdio::piped());
        self.command
            .output()
            .map_err(|e| format!("Failed launching difftool: {}", e))?;
        // Some difftools, like bcompare, will return non zero status when there is a diff and 0
        // only when there are no changes.  This prevents us from trusting the status
        Ok(())
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
            fn arg(&mut self, arg: &OsStr) -> &mut Self;
            fn stdout(&mut self, cfg: Stdio) -> &mut Self;
            fn stderr(&mut self, cfg: Stdio) -> &mut Self;
            fn output(&mut self) -> io::Result<Output>;
        }
    }

    #[test]
    fn diff_launches_ok() {
        let local = OsStr::new("foo/baz/bar");
        let remote = OsStr::new("some/other/file");
        let mut mock = MockC::new();
        mock.expect_arg()
            .with(eq(local))
            .times(1)
            .returning(|_| MockC::new());
        mock.expect_arg()
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
}
