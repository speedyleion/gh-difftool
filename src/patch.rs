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

impl<'a> ReverseApply for Patch<'a> {
    fn reverse_apply<P1, P2>(&self, src: P1, dest: P2) -> Result<(), String>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let mut cmd = Command::new("patch");
        cmd.args([
            "-R",
            &src.as_ref().to_string_lossy(),
            "-o",
            &dest.as_ref().to_string_lossy(),
        ]);
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to start `patch` process: {}", e))?;
        let mut stdin = child.stdin.take().expect("failed to get stdin for `patch`");

        let mut contents = self.to_string();

        // Not sure how to force this in a minimum reproducible example.
        // When using patch and deleting things close to the end of the file it seems that missing
        // a newline at the end of the patch will cause it to fail. Always Adding a newline to the
        // end never seems to be an issue.
        contents.push('\n');

        // If one doesn't use a thread for writing stdin then it will block indefinitely
        std::thread::spawn(move || {
            stdin
                .write_all(contents.as_bytes())
                .expect("Failed to write to stdin");
        });

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to run the `patch` process to finish: {}", e))?;

        let status = output.status;
        if status.success() {
            Ok(())
        } else {
            Err(format!(
                "Failed to patch {:?} to {:?}: {}",
                src.as_ref(),
                dest.as_ref(),
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }
}

pub struct PatchSet<'a> {
    pub patches: Vec<Patch<'a>>,
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
    use std::fs;
    use temp_testdir::TempDir;
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

    #[test]
    fn reverse_apply() {
        let temp = TempDir::default().permanent();
        let a = temp.join("a");
        let b = temp.join("b");
        let newest = dedent(
            "
            line one
            line changed
            line three
            ",
        );
        fs::write(&b, newest).unwrap();
        let diff = dedent(
            "
            diff --git a/foo.txt b/foo.txt
            index 0c2aa38..0370c84 100644
            --- a/foo.txt
            +++ b/foo.txt
            @@ -1,3 +1,3 @@
             line one
            -line two
            +line changed
             line three
            ",
        );
        let expected = dedent(
            "
            line one
            line two
            line three
            ",
        );
        let patches = PatchSet::new(&diff).unwrap();
        let patch = &patches.patches[0];
        patch.reverse_apply(&b, &a).unwrap();
        assert_eq!(fs::read(&a).unwrap(), expected.into_bytes());
    }

    #[test]
    fn only_deleting_lines() {
        let temp = TempDir::default().permanent();
        let a = temp.join("a");
        let b = temp.join("b");
        let newest = dedent(
            "
            line one
            line two
            line three
            ",
        );
        fs::write(&b, newest).unwrap();
        let diff = dedent(
            "
            diff --git a/foo.txt b/foo.txt
            index 0c2aa38..0370c84 100644
            --- a/foo.txt
            +++ b/foo.txt
            @@ -1,2 +1,3 @@
             line one
            +line two
             line three
            ",
        );
        let expected = dedent(
            "
            line one
            line three
            ",
        );
        let patches = PatchSet::new(&diff).unwrap();
        let patch = &patches.patches[0];
        patch.reverse_apply(&b, &a).unwrap();
        assert_eq!(fs::read(&a).unwrap(), expected.into_bytes());
    }

    #[test]
    fn fail_to_apply() {
        let temp = TempDir::default().permanent();
        let a = temp.join("a");
        let b = temp.join("b");
        let newest = dedent(
            "
            ",
        );
        fs::write(&b, newest).unwrap();
        let diff = dedent(
            "
            diff --git a/foo.txt b/foo.txt
            index 0c2aa38..0370c84 100644
            --- a/foo.txt
            +++ b/foo.txt
            @@ -1,3 +1,3 @@
             line one
            +line changed
             line three
            ",
        );
        let patches = PatchSet::new(&diff).unwrap();
        let patch = &patches.patches[0];
        let message_start = format!("Failed to patch {:?} to {:?}: patch: **** malformed", b, a);
        assert!(
            matches!(patch.reverse_apply(&b, &a), Err(message) if message.starts_with(&message_start))
        );
    }
}
