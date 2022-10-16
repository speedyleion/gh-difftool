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


trait Cmd {
    fn args<I: IntoIterator<Item = S>, S: AsRef<OsStr>>(&mut self, args: I) -> &mut Self;
    fn stdout<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self;
    fn stderr<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self;
    fn output(&mut self) -> io::Result<Output>;
}

impl Cmd for Command {
    fn args<I: IntoIterator<Item = S>, S: AsRef<OsStr>>(&mut self, args: I) -> &mut Self {
        self.args(args)
    }
    fn stdout<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
        self.stdout(cfg)
    }
    fn stderr<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
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
        self.command.args(["pr", "diff"]);
        self.command.stdout(Stdio::piped()).stderr(Stdio::piped());
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
    use std::fmt::Error;
    use super::*;
    use mockall::mock;
    use std::ffi::OsStr;
    use std::process::Output;
    use std::process::Stdio;

    mock! {
        Command {
            fn args<I: IntoIterator<Item = S> + 'static, S: AsRef<OsStr> + 'static>(&mut self, args: I) -> &mut Self;
            fn stdout<T: Into<Stdio> + 'static>(&mut self, cfg: T) -> &mut Self;
            fn stderr<T: Into<Stdio> + 'static>(&mut self, cfg: T) -> &mut Self;
            fn output(&mut self) -> io::Result<Output>;
        }
    }

    #[test]
    fn no_current_pr() {
        let mut mock = MockCommand::new();
        let mut gh = GhCli::new(mock);
        let message = gh.diff().err().unwrap();
        assert!(message.contains("no pull requests found for branch"));
    }
}
