//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Reverse apply patches to files to get back to the original version

use patch::{ParseError, Patch};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub trait ReverseApply {
    fn reverse_apply<P1, P2>(&self, src: P1, dest: P2) -> Result<(), String>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>;
}

