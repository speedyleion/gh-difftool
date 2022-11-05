//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! A common command trait to make it possible to mock Command

use std::ffi::OsStr;
use std::io;
use std::process::Command;
use std::process::Output;
use std::process::Stdio;

pub trait Cmd {
    fn arg(&mut self, arg: &OsStr) -> &mut Self;
    fn stdout(&mut self, cfg: Stdio) -> &mut Self;
    fn stderr(&mut self, cfg: Stdio) -> &mut Self;
    fn output(&mut self) -> io::Result<Output>;
    fn new_from_self(&self) -> Self;
}

impl Cmd for Command {
    fn arg(&mut self, arg: &OsStr) -> &mut Self {
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
    fn new_from_self(&self) -> Self {
        let program = self.get_program();
        Command::new(program)
    }
}
