//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Module for interacting with the github command line

use std::process::Command;
use std::ffi::OsStr;
use std::io;
use std::process::Output;
use std::process::Stdio;

pub trait Cmd {
    fn arg<S: AsRef<OsStr> + 'static>(&mut self, arg: S) -> &mut Self;
    fn stdout(&mut self, cfg: Stdio) -> &mut Self;
    fn stderr(&mut self, cfg: Stdio) -> &mut Self;
    fn output(&mut self) -> io::Result<Output>;
}

impl Cmd for Command {
    fn arg<S: AsRef<OsStr> + 'static>(&mut self, arg: S) -> &mut Self {
        self.arg(arg)
    }
    fn stdout(&mut self, cfg: Stdio) -> &mut Self {
        self.stdout(cfg)
    }
    fn stderr(&mut self, cfg: Stdio) -> &mut Self {
        self.stderr(cfg)
    }
    fn output(&mut self) -> io::Result<Output> {
        self.output()
    }
}

#[derive(Debug, Default)]
pub struct GhCli<C> {
    command: C,
}

impl<C: Cmd> GhCli <C> {
    pub fn new(command: C) -> Self {
        Self{ command }
    }

    pub fn diff(&mut self) -> Result<String, String> {
        self.command.arg("pr");
        // self.command.arg("pr").arg("diff");
        self.command.stdout(Stdio::piped());
        self.command.stderr(Stdio::piped());
        let output = self.command
            .output()
            .map_err(|e| format!("Failed running gh diff: {}", e))?;
        let status = output.status;
        if status.success() {
            Ok(String::from_utf8(output.stdout)
                .map_err(|e| format!("Failed to convert output to string {}", e))?)
        } else {
            Err(String::from_utf8(output.stderr)
                .map_err(|e| format!("Failed to convert output to string {}", e))?)
        }
    }
}

mod tests {
    use super::*;
    use mockall::mock;
    use std::ffi::OsStr;
    use std::os::unix::prelude::ExitStatusExt;
    use std::process::{ExitStatus, Output};
    use std::process::Stdio;
    use mockall::predicate::eq;

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
    fn no_current_pr() {
        let mut mock = MockC::new();
        mock.expect_arg::<&str>().with(eq("pr"));
        mock.expect_arg::<&str>().with(eq("diff"));
        // No good way to check for pipes
        mock.expect_stdout().times(1);
        mock.expect_stderr().times(1);
        mock.expect_output().returning(|| {
            Ok(Output{ status: ExitStatus::from_raw(1), stdout: vec![], stderr: b"no pull requests found for branch".to_vec()})
        });

        let mut gh = GhCli::new(mock);
        let message = gh.diff().err().unwrap();
        assert!(message.contains("no pull requests found for branch"));
    }
}
