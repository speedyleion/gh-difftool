// Allowing dead code until this gets hooked up
#![allow(dead_code)]

use crate::cmd::Cmd;
        self.command.arg(OsStr::new("pr"));
        self.command.arg(OsStr::new("diff"));
#[cfg(test)]
    use std::io;
            fn arg(&mut self, arg: &OsStr) -> &mut Self;
        mock.expect_arg()
            .with(eq(OsStr::new("pr")))
            .times(1)
        mock.expect_arg()
            .with(eq(OsStr::new("diff")))
            .times(1)
        mock.expect_output().times(1).returning(|| {

    #[test]
    fn current_pr() {
        let expected = b"
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
        ";
        let mut mock = MockC::new();
        mock.expect_arg()
            .with(eq(OsStr::new("pr")))
            .times(1)
            .returning(|_| MockC::new());
        mock.expect_arg()
            .with(eq(OsStr::new("diff")))
            .times(1)
            .returning(|_| MockC::new());
        // No good way to check for pipes
        mock.expect_stdout().times(1).returning(|_| MockC::new());
        mock.expect_stderr().times(1).returning(|_| MockC::new());
        mock.expect_output().times(1).returning(|| {
            Ok(Output {
                status: ExitStatus::from_raw(0),
                stdout: expected.to_vec(),
                stderr: vec![],
            })
        });
        let mut gh = GhCli::new(mock);
        let message = gh.diff().unwrap();
        assert_eq!(message, String::from_utf8(expected.to_vec()).unwrap());
    }