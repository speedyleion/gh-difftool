//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Reverse apply patches to files to get back to the original version

// Allowing dead code until this gets hooked up
#![allow(dead_code)]

use patch::{ParseError, Patch};
use std::path::Path;

pub trait ReverseApply {
    fn reverse_apply<P1, P2>(&self, src: P1, dest: P2) -> Result<(), ()>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>;
}

impl<'a> ReverseApply for Patch<'a> {
    fn reverse_apply<P1, P2>(&self, _src: P1, _dest: P2) -> Result<(), ()>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        Ok(())
    }
}

pub struct PatchSet<'a> {
    patches: Vec<Patch<'a>>,
}

impl<'a> PatchSet<'a> {
    pub fn new(patch_text: &'a str) -> Result<Self, ParseError> {
        let patches = Patch::from_multiple(patch_text)?;
        Ok(Self { patches })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use textwrap::dedent;

    #[test]
    fn new_patch_set() {
        let text = dedent(
            "
            diff --git a/file.1 b/file.1
            index ff02a34..7d8ab89 100644
            --- a/file.1
            +++ b/file.1
            @@ -6,3 +6,6 @@ context
             more context

             even more conext
            +new stuff
            +some more new stuff
            diff --git a/path_2/file.2 b/path_2/file.2
            new file mode 100644
            index 0000000..dafde04
            --- /dev/null
            +++ b/path_2/file.2
            @@ -0,0 +1,41 @@
            +new stuff
            +new stuff
        ",
        );

        let patches = PatchSet::new(&text).unwrap();
        assert_eq!(patches.patches.len(), 2);
        assert_eq!(patches.patches[0].old.path, "a/file.1");
        assert_eq!(patches.patches[0].new.path, "b/file.1");
        assert_eq!(patches.patches[1].old.path, "/dev/null");
        assert_eq!(patches.patches[1].new.path, "b/path_2/file.2");
    }
}
