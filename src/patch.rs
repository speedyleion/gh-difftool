        let mut contents = self.to_string();

        // Not sure how to force this in a minimum reproducible example.
        // When using patch and deleting things close to the end of the file it seems that missing
        // a newline at the end of the patch will cause it to fail. Always Adding a newline to the
        // end never seems to be an issue.
        contents.push('\n');
    // #[test]
    // fn parsing_from_api() {
    //     let mut patch_text = String::from("--- file.1\n+++ file.1\n@@ -6,3 +6,7 @@ edition = \"2021\"\n [dev-dependencies]\n assert_cmd = \"2.0.4\"\n mockall = \"0.11.2\"\n+textwrap = \"0.15.1\"\n+\n+[dependencies]\n+patch = \"0.6.0\"");
    //     if !patch_text.ends_with('\n') {
    //         patch_text.push('\n')
    //     }
    //     let patch = Patch::from_single(&patch_text).unwrap();
    //     assert_eq!(patch.old.path, "file.1");
    //     assert_eq!(patch.new.path, "file.1");
    // }

    // #[test]
    // fn new_patch_set() {
    //     let text = dedent(
    //         "
    //         diff --git a/file.1 b/file.1
    //         index ff02a34..7d8ab89 100644
    //         --- a/file.1
    //         +++ b/file.1
    //         @@ -6,3 +6,6 @@ context
    //          more context
    //
    //          even more conext
    //         +new stuff
    //         +some more new stuff
    //         diff --git a/path_2/file.2 b/path_2/file.2
    //         new file mode 100644
    //         index 0000000..dafde04
    //         --- /dev/null
    //         +++ b/path_2/file.2
    //         @@ -0,0 +1,41 @@
    //         +new stuff
    //         +new stuff
    //     ",
    //     );
    //
    //     let patches = PatchSet::new(&text).unwrap();
    //     assert_eq!(patches.patches.len(), 2);
    //     assert_eq!(patches.patches[0].old.path, "a/file.1");
    //     assert_eq!(patches.patches[0].new.path, "b/file.1");
    //     assert_eq!(patches.patches[1].old.path, "/dev/null");
    //     assert_eq!(patches.patches[1].new.path, "b/path_2/file.2");
    // }
    //
    // #[test]
    // fn reverse_apply() {
    //     let temp = TempDir::default().permanent();
    //     let a = temp.join("a");
    //     let b = temp.join("b");
    //     let newest = dedent(
    //         "
    //         line one
    //         line changed
    //         line three
    //         ",
    //     );
    //     fs::write(&b, newest).unwrap();
    //     let diff = dedent(
    //         "
    //         diff --git a/foo.txt b/foo.txt
    //         index 0c2aa38..0370c84 100644
    //         --- a/foo.txt
    //         +++ b/foo.txt
    //         @@ -1,3 +1,3 @@
    //          line one
    //         -line two
    //         +line changed
    //          line three
    //         ",
    //     );
    //     let expected = dedent(
    //         "
    //         line one
    //         line two
    //         line three
    //         ",
    //     );
    //     let patches = PatchSet::new(&diff).unwrap();
    //     let patch = &patches.patches[0];
    //     patch.reverse_apply(&b, &a).unwrap();
    //     assert_eq!(fs::read(&a).unwrap(), expected.into_bytes());
    // }
    //
    // #[test]
    // fn only_deleting_lines() {
    //     let temp = TempDir::default().permanent();
    //     let a = temp.join("a");
    //     let b = temp.join("b");
    //     let newest = dedent(
    //         "
    //         line one
    //         line two
    //         line three
    //         ",
    //     );
    //     fs::write(&b, newest).unwrap();
    //     let diff = dedent(
    //         "
    //         diff --git a/foo.txt b/foo.txt
    //         index 0c2aa38..0370c84 100644
    //         --- a/foo.txt
    //         +++ b/foo.txt
    //         @@ -1,2 +1,3 @@
    //          line one
    //         +line two
    //          line three
    //         ",
    //     );
    //     let expected = dedent(
    //         "
    //         line one
    //         line three
    //         ",
    //     );
    //     let patches = PatchSet::new(&diff).unwrap();
    //     let patch = &patches.patches[0];
    //     patch.reverse_apply(&b, &a).unwrap();
    //     assert_eq!(fs::read(&a).unwrap(), expected.into_bytes());
    // }
    //
    // #[test]
    // fn fail_to_apply() {
    //     let temp = TempDir::default().permanent();
    //     let a = temp.join("a");
    //     let b = temp.join("b");
    //     let newest = dedent(
    //         "
    //         ",
    //     );
    //     fs::write(&b, newest).unwrap();
    //     let diff = dedent(
    //         "
    //         diff --git a/foo.txt b/foo.txt
    //         index 0c2aa38..0370c84 100644
    //         --- a/foo.txt
    //         +++ b/foo.txt
    //         @@ -1,3 +1,3 @@
    //          line one
    //         +line changed
    //          line three
    //         ",
    //     );
    //     let patches = PatchSet::new(&diff).unwrap();
    //     let patch = &patches.patches[0];
    //     let message_start = format!("Failed to patch {:?} to {:?}: patch: **** malformed", b, a);
    //     assert!(
    //         matches!(patch.reverse_apply(&b, &a), Err(message) if message.starts_with(&message_start))
    //     );
    // }