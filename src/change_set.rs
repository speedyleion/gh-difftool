//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Set of changes that goes from one version of files to another

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Error, Write};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Change {
    pub filename: String,
    /// The previous_filename will be present for renamed files
    pub previous_filename: Option<String>,
    pub contents_url: String,
    /// Patches are *not* present for files that are only renamed
    /// and large binary diffs
    pub patch: Option<String>,
    pub status: String,
    // The sha of the file
    pub sha: String,
}

impl Change {
    pub fn reverse_apply<P1, P2>(&self, src: P1, dest: P2) -> Result<()>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        // This is not an ideal implementation, but it works for now.
        // When the file is removed (deleted) the Github API provides a `contents_url` which points to
        // the old version of the file, not an empty version. This makes sense as an empty file is
        // different than a file that doesn't exist. The problem comes in that the consumers of
        // [`Change`] happen to create the original and new files instead of letting [`Change`] do
        // it. Because of this lack of encapsulation, [`Change`] will swap out the new version for
        // the old version and write an empty new version
        if self.status == "removed" {
            fs::copy(&src, &dest)?;
            fs::write(src, "")?;
            return Ok(());
        }

        // Renamed files don't have a patch
        let Some(patch) = self.patch.as_ref() else {
            fs::copy(&src, &dest)?;
            return Ok(());
        };

        // Submodules diffs should only be the sha values
        // this logic is reluctantly split between here and the `gh_interface` module
        if let Some(sha) = self.get_submodule_commit_sha(patch) {
            return Ok(fs::write(dest, sha)?);
        }

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
            .context("Failed to spawn `patch`, is it installed?")?;
        let mut stdin = child.stdin.take().expect("failed to get stdin for `patch`");

        let mut contents = patch.clone();

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

        let output = child.wait_with_output()?;

        let status = output.status;
        if status.success() {
            Ok(())
        } else {
            Err(Error::other(
                format!(
                    "Failed to patch {:?} to {:?}: {}",
                    src.as_ref(),
                    dest.as_ref(),
                    String::from_utf8_lossy(&output.stderr)
                ),
            ))?
        }
    }

    // Will parse the patch to see if it conforms to a submodule patch
    // Unfortunately the true indicator if this is a submoudule is the `type`
    // on the `Content` struct. This may warrant a refactor
    fn get_submodule_commit_sha(&self, patch: &str) -> Option<String> {
        const SUBMODULE_PATCH_PREFIX: &str = "@@ -1 +1 @@\n-Subproject commit ";
        let possible_sha = patch.strip_prefix(SUBMODULE_PATCH_PREFIX);
        match (possible_sha, patch.ends_with(&self.sha)) {
            (Some(sha), true) => Some(sha.split_once('\n').unwrap_or(("", "")).0.to_string()),
            _ => None,
        }
    }
}

#[derive(Default, PartialEq, Eq, Debug)]
pub struct ChangeSet {
    pub changes: Vec<Change>,
}

impl ChangeSet {
    /// Will keep only changes related to `files`
    ///
    /// Any `files` which aren't in the current [`Changeset`] will be ignored.
    /// This ignoring of unmatched entries in `files` mimics the behavior of `git-difftool`.
    ///
    /// # Arguments
    /// * `files` - The files to keep the changes for
    pub fn filter_files<T: AsRef<str>>(&mut self, files: &[T]) -> &mut Self {
        let files = files.iter().map(T::as_ref).collect::<Vec<_>>();
        self.changes
            .retain(|c| files.contains(&c.filename.as_str()));
        self
    }

    /// Rotate to `file` in the changeset.
    ///
    /// Will rotate the files in the [`Changeset`] so that `file` is first and all files before
    /// `file` come at the end
    ///
    /// # Arguments
    /// * `file` - The file to rotate to
    ///
    /// # Errors
    /// When `file` does not exist in the [`Changeset`].
    pub fn rotate_to<T: AsRef<str>>(&mut self, file: T) -> Result<&mut Self> {
        let position = self.file_position(file)?;
        self.changes.rotate_left(position);
        Ok(self)
    }

    /// Skip to `file` in the changeset.
    ///
    /// Will remove any files prior to `file` in the [`Changeset`].
    ///
    /// # Arguments
    /// * `file` - The file to skip to
    ///
    /// # Errors
    /// When `file` does not exist in the [`Changeset`].
    pub fn skip_to<T: AsRef<str>>(&mut self, file: T) -> Result<&mut Self> {
        let position = self.file_position(file)?;
        self.changes = self.changes.split_off(position);
        Ok(self)
    }

    /// Position of `file` in the changeset.
    ///
    /// # Arguments
    /// * `file` - The file to get the position for
    ///
    /// # Errors
    /// When `file` does not exist in the [`Changeset`].
    fn file_position(&self, file: impl AsRef<str>) -> Result<usize> {
        let file = file.as_ref();
        Ok(self
            .changes
            .iter()
            .position(|c| c.filename.as_str() == file)
            .ok_or_else(|| {
                Error::other(
                    format!("No such path '{file}' in the diff."),
                )
            })?)
    }
}

impl TryFrom<&str> for ChangeSet {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        let changes = serde_json::from_str(value)?;
        Ok(Self { changes })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use temp_testdir::TempDir;
    use textwrap::dedent;
    use yare::parameterized;

    #[cfg(windows)]
    const EOL: &'static str = "\r\n";
    #[cfg(not(windows))]
    const EOL: &str = "\n";

    /// Convert `filenames` to a vector of ['Change']
    ///
    /// The `contents_url` and the `patch` in the resultant ['Change']es will all be the same.
    fn filenames_to_changes(filenames: &[&str]) -> Vec<Change> {
        let contents_url = String::from("contents_url");
        filenames
            .iter()
            .map(|f| Change {
                filename: (*f).to_owned(),
                contents_url: contents_url.to_owned(),
                sha: "sha".into(),
                patch: Some("patch".into()),
                status: String::from("modified"),
                previous_filename: None,
            })
            .collect::<Vec<_>>()
    }

    #[test]
    fn empty_changeset_parses() {
        let empty_json = r#"
            [
            ]
        "#;

        assert_eq!(
            ChangeSet::try_from(empty_json).unwrap(),
            ChangeSet::default()
        );
    }

    #[test]
    fn one_change_parses() {
        let one_file_json = r#"
            [
              {
                "sha": "b0a3777df4afc764c34234524267970025d55467",
                "filename": "Cargo.toml",
                "status": "modified",
                "additions": 4,
                "deletions": 0,
                "changes": 4,
                "blob_url": "https://github.com/speedyleion/gh-difftool/blob/befb7bf69c3c8ba97c714d57c8dadd9621021c84/Cargo.toml",
                "raw_url": "https://github.com/speedyleion/gh-difftool/raw/befb7bf69c3c8ba97c714d57c8dadd9621021c84/Cargo.toml",
                "contents_url": "https://api.github.com/repos/speedyleion/gh-difftool/contents/Cargo.toml?ref=befb7bf69c3c8ba97c714d57c8dadd9621021c84",
                "patch": "@@ -6,3 +6,7 @@ edition = \"2021\"\n [dev-dependencies]\n assert_cmd = \"2.0.4\"\n mockall = \"0.11.2\"\n+textwrap = \"0.15.1\"\n+\n+[dependencies]\n+patch = \"0.6.0\""
                }
            ]
        "#;

        assert_eq!(
            ChangeSet::try_from(one_file_json).unwrap(),
            ChangeSet {
                changes: vec![Change {
                    filename: String::from("Cargo.toml"),
                    contents_url: String::from("https://api.github.com/repos/speedyleion/gh-difftool/contents/Cargo.toml?ref=befb7bf69c3c8ba97c714d57c8dadd9621021c84"),
                    patch: Some("@@ -6,3 +6,7 @@ edition = \"2021\"\n [dev-dependencies]\n assert_cmd = \"2.0.4\"\n mockall = \"0.11.2\"\n+textwrap = \"0.15.1\"\n+\n+[dependencies]\n+patch = \"0.6.0\"".into()),
                    status: String::from("modified"),
                    previous_filename: None,
                    sha: String::from("b0a3777df4afc764c34234524267970025d55467"),
                }]
            }
        );
    }

    #[test]
    fn bad_json_errors() {
        let bad_json = r#"
            [
        "#;

        let error = ChangeSet::try_from(bad_json).unwrap_err();
        let root_cause = error.root_cause();
        assert_eq!(
            format!("{}", root_cause),
            "EOF while parsing a list at line 3 column 8"
        );
    }

    #[test]
    fn multiple_changes_parsed() {
        let multiple_files = r#"
            [
              {
                "filename": "Cargo.toml",
                "contents_url": "stuff",
                "patch": "more_stuff",
                "status": "modified",
                "sha": "sha1"
              },
              {
                "filename": "yes/no/maybe.idk",
                "contents_url": "sure",
                "patch": "why not",
                "status": "modified",
                "sha": "sha2"
              },
              {
                "filename": "what/when/where.stuff",
                "contents_url": "idk",
                "patch": "I guess",
                "status": "modified",
                "sha": "sha3"
              }
            ]
        "#;

        assert_eq!(
            ChangeSet::try_from(multiple_files).unwrap(),
            ChangeSet {
                changes: vec![
                    Change {
                        filename: String::from("Cargo.toml"),
                        contents_url: String::from("stuff"),
                        patch: Some("more_stuff".into()),
                        status: String::from("modified"),
                        previous_filename: None,
                        sha: String::from("sha1"),
                    },
                    Change {
                        filename: String::from("yes/no/maybe.idk"),
                        contents_url: String::from("sure"),
                        patch: Some("why not".into()),
                        status: String::from("modified"),
                        previous_filename: None,
                        sha: String::from("sha2"),
                    },
                    Change {
                        filename: String::from("what/when/where.stuff"),
                        contents_url: String::from("idk"),
                        patch: Some("I guess".into()),
                        status: String::from("modified"),
                        previous_filename: None,
                        sha: String::from("sha3"),
                    }
                ]
            }
        );
    }

    #[test]
    fn filter_files_from_changeset() {
        let mut changeset = ChangeSet {
            changes: vec![
                Change {
                    filename: String::from("Cargo.toml"),
                    contents_url: String::from("stuff"),
                    patch: Some("more_stuff".into()),
                    status: String::from("modified"),
                    previous_filename: None,
                    sha: String::from("what is up"),
                },
                Change {
                    filename: String::from("yes/no/maybe.idk"),
                    contents_url: String::from("sure"),
                    patch: Some("why not".into()),
                    status: String::from("modified"),
                    previous_filename: None,
                    sha: String::from("why not"),
                },
                Change {
                    filename: String::from("what/when/where.stuff"),
                    contents_url: String::from("idk"),
                    patch: Some("I guess".into()),
                    status: String::from("modified"),
                    previous_filename: None,
                    sha: String::from("I guess"),
                },
            ],
        };

        changeset.filter_files(&["yes/no/maybe.idk", "fake/file", "Cargo.toml"]);

        assert_eq!(
            changeset,
            ChangeSet {
                changes: vec![
                    Change {
                        filename: String::from("Cargo.toml"),
                        contents_url: String::from("stuff"),
                        patch: Some("more_stuff".into()),
                        status: String::from("modified"),
                        previous_filename: None,
                        sha: String::from("what is up"),
                    },
                    Change {
                        filename: String::from("yes/no/maybe.idk"),
                        contents_url: String::from("sure"),
                        patch: Some("why not".into()),
                        status: String::from("modified"),
                        previous_filename: None,
                        sha: String::from("why not"),
                    },
                ]
            }
        );
    }

    #[parameterized(
    first = {"Cargo.toml", &["Cargo.toml", "yes/no/maybe.idk", "what/when/where.stuff"]},
    middle = {"yes/no/maybe.idk", &["yes/no/maybe.idk", "what/when/where.stuff"]},
    last = {"what/when/where.stuff", &["what/when/where.stuff"]},
    )]
    fn skip_to_files(file: &str, expected: &[&str]) {
        let changes =
            filenames_to_changes(&["Cargo.toml", "yes/no/maybe.idk", "what/when/where.stuff"]);
        let mut changeset = ChangeSet { changes };

        changeset.skip_to(file).expect("Should be able to skip to");

        assert_eq!(
            changeset,
            ChangeSet {
                changes: filenames_to_changes(expected)
            },
        );
    }

    #[test]
    fn skip_to_non_existent_file_is_an_error() {
        let changes =
            filenames_to_changes(&["Cargo.toml", "yes/no/maybe.idk", "what/when/where.stuff"]);
        let mut changeset = ChangeSet { changes };

        let error = changeset
            .skip_to("foo")
            .expect_err("Should not find file in change");
        assert_eq!(error.to_string(), "No such path 'foo' in the diff.");
    }

    #[parameterized(
        first = {"Cargo.toml", &["Cargo.toml", "yes/no/maybe.idk", "what/when/where.stuff"]},
        middle = {"yes/no/maybe.idk", &["yes/no/maybe.idk", "what/when/where.stuff", "Cargo.toml"]},
        last = {"what/when/where.stuff", &["what/when/where.stuff", "Cargo.toml", "yes/no/maybe.idk"]},
    )]
    fn rotate_to_files(file: &str, expected: &[&str]) {
        let changes =
            filenames_to_changes(&["Cargo.toml", "yes/no/maybe.idk", "what/when/where.stuff"]);
        let mut changeset = ChangeSet { changes };

        changeset
            .rotate_to(file)
            .expect("Should be able to rotate to");

        assert_eq!(
            changeset,
            ChangeSet {
                changes: filenames_to_changes(expected)
            },
        );
    }

    #[test]
    fn rotate_to_non_existent_file_is_an_error() {
        let changes =
            filenames_to_changes(&["Cargo.toml", "yes/no/maybe.idk", "what/when/where.stuff"]);
        let mut changeset = ChangeSet { changes };

        let error = changeset
            .rotate_to("baz")
            .expect_err("Should not find file in change");
        assert_eq!(error.to_string(), "No such path 'baz' in the diff.");
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
        let diff = "@@ -1,3 +1,3 @@\n line one\n-line two\n+line changed\n line three";
        let change = Change {
            filename: "what/when/where.stuff".to_string(),
            contents_url: "idk".to_string(),
            patch: Some(diff.to_string()),
            status: String::from("modified"),
            previous_filename: None,
            sha: "I guess".to_string(),
        };
        let expected = format!("{EOL}line one{EOL}line two{EOL}line three{EOL}");
        change.reverse_apply(&b, &a).unwrap();
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
        let diff = "@@ -1,2 +1,3 @@\n line one\n+line two\n line three";
        let change = Change {
            filename: "what/when/where.stuff".to_string(),
            contents_url: "idk".to_string(),
            patch: Some(diff.to_string()),
            status: String::from("modified"),
            previous_filename: None,
            sha: "I guess".to_string(),
        };
        let expected = format!("{EOL}line one{EOL}line three{EOL}");
        change.reverse_apply(&b, &a).unwrap();
        assert_eq!(fs::read(&a).unwrap(), expected.into_bytes());
    }

    #[test]
    fn fail_to_apply() {
        let temp = TempDir::default().permanent();
        let a = temp.join("a");
        let b = temp.join("b");
        let newest = "\n";
        fs::write(&b, newest).unwrap();
        let diff = "@@ -1,3 +1,3 @@\n line one\n+line changed\n line three";
        let message_start = format!("Failed to patch {:?} to {:?}: patch: **** malformed", b, a);
        let change = Change {
            filename: "what/when/where.stuff".to_string(),
            contents_url: "idk".to_string(),
            patch: Some(diff.to_string()),
            status: String::from("modified"),
            previous_filename: None,
            sha: "I guess".to_string(),
        };

        let error = change.reverse_apply(&b, &a).unwrap_err();
        let root_cause = error.root_cause();
        let message = format!("{}", root_cause);
        assert!(message.starts_with(&message_start));
    }

    #[test]
    fn file_removed() {
        let temp = TempDir::default().permanent();
        let a = temp.join("a");
        let b = temp.join("b");
        let contents = dedent(
            "
            line one
            line two
            line three
            ",
        );
        fs::write(&b, contents).unwrap();

        let diff = "@@ -1,3 +0,0 @@\n-line one\n-line two\n-line three";
        let change = Change {
            filename: "what/when/where.stuff".to_string(),
            contents_url: "idk".to_string(),
            patch: Some(diff.to_string()),
            status: String::from("removed"),
            previous_filename: None,
            sha: "I guess".to_string(),
        };
        let expected = "\nline one\nline two\nline three\n".to_string();
        change.reverse_apply(&b, &a).unwrap();
        assert_eq!(fs::read(&a).unwrap(), expected.into_bytes());
        assert_eq!(fs::read(&b).unwrap(), "".as_bytes());
    }

    #[test]
    fn no_patch() {
        let temp = TempDir::default().permanent();
        let a = temp.join("a");
        let b = temp.join("b");
        let contents = dedent(
            "
            line one
            line two
            line three
            ",
        );
        fs::write(&b, contents).unwrap();

        let change = Change {
            filename: "what/when/where.stuff".to_string(),
            contents_url: "idk".to_string(),
            patch: None,
            status: String::from("renamed"),
            previous_filename: Some("foo/bar/baz/me.txt".into()),
            sha: "I guess".to_string(),
        };
        let expected = "\nline one\nline two\nline three\n".to_string();
        change.reverse_apply(&b, &a).unwrap();
        assert_eq!(fs::read(&a).unwrap(), expected.into_bytes());
    }

    #[test]
    fn submodule() {
        let temp = TempDir::default().permanent();
        let a = temp.join("a");
        let b = temp.join("b");
        fs::write(&b, "").unwrap();

        let diff = "@@ -1 +1 @@\n-Subproject commit 236682e946bc79ef30288013e144f9794a9f0ff1\n Subproject commit 7c8ba583177b9e14cb85346f52e7b5536935a051";

        let change = Change {
            filename: "a/submodule".to_string(),
            contents_url: "idk".to_string(),
            patch: Some(diff.to_string()),
            status: String::from("modified"),
            previous_filename: None,
            sha: "7c8ba583177b9e14cb85346f52e7b5536935a051".to_string(),
        };
        let expected = "236682e946bc79ef30288013e144f9794a9f0ff1".to_string();
        change.reverse_apply(&b, &a).unwrap();
        assert_eq!(fs::read(&a).unwrap(), expected.into_bytes());
    }
}
