//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Set of changes that goes from one version of files to another

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Error, ErrorKind, Write};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Change {
    pub filename: String,
    pub raw_url: String,
    pub patch: String,
    pub status: String,
}

impl Change {
    pub fn reverse_apply<P1, P2>(&self, src: P1, dest: P2) -> Result<()>
    where
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        // This is not an ideal implementation, but it works for now.
        // When the file is removed (deleted) the Github API results in a `raw_url` which points to
        // the old version of the file, not an empty version. This kind of makes sense as an empty
        // file doesn't exist, the file itself doesn't exit. The problem comes in that the consumers
        // of Change happen to create the original and new files instead of letting change do it.
        // Because of this lack of encapsulation, change will swap out the new version for the old
        // version and write an empty new version
        if self.status == "removed" {
            fs::copy(&src, &dest)?;
            fs::write(src, "")?;
            return Ok(());
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
        let mut child = cmd.spawn()?;
        let mut stdin = child.stdin.take().expect("failed to get stdin for `patch`");

        let mut contents = self.patch.clone();

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
            Err(Error::new(
                ErrorKind::Other,
                format!(
                    "Failed to patch {:?} to {:?}: {}",
                    src.as_ref(),
                    dest.as_ref(),
                    String::from_utf8_lossy(&output.stderr)
                ),
            ))?
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
                Error::new(
                    ErrorKind::Other,
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
    const EOL: &'static str = "\n";

    /// Convert `filenames` to a vector of ['Change']
    ///
    /// The `raw_url` and the `patch` in the resultant ['Change']es will all be the same.
    fn filenames_to_changes(filenames: &[&str]) -> Vec<Change> {
        let raw_url = String::from("raw_url");
        let patch = String::from("patch");
        filenames
            .iter()
            .map(|f| Change {
                filename: (*f).to_owned(),
                raw_url: raw_url.to_owned(),
                patch: patch.to_owned(),
                status: String::from("modified"),
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
                    raw_url: String::from("https://github.com/speedyleion/gh-difftool/raw/befb7bf69c3c8ba97c714d57c8dadd9621021c84/Cargo.toml"),
                    patch: String::from("@@ -6,3 +6,7 @@ edition = \"2021\"\n [dev-dependencies]\n assert_cmd = \"2.0.4\"\n mockall = \"0.11.2\"\n+textwrap = \"0.15.1\"\n+\n+[dependencies]\n+patch = \"0.6.0\""),
                    status: String::from("modified"),
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
                "raw_url": "stuff",
                "patch": "more_stuff",
                "status": "modified"
              },
              {
                "filename": "yes/no/maybe.idk",
                "raw_url": "sure",
                "patch": "why not",
                "status": "modified"
              },
              {
                "filename": "what/when/where.stuff",
                "raw_url": "idk",
                "patch": "I guess",
                "status": "modified"
              }
            ]
        "#;

        assert_eq!(
            ChangeSet::try_from(multiple_files).unwrap(),
            ChangeSet {
                changes: vec![
                    Change {
                        filename: String::from("Cargo.toml"),
                        raw_url: String::from("stuff"),
                        patch: String::from("more_stuff"),
                        status: String::from("modified"),
                    },
                    Change {
                        filename: String::from("yes/no/maybe.idk"),
                        raw_url: String::from("sure"),
                        patch: String::from("why not"),
                        status: String::from("modified"),
                    },
                    Change {
                        filename: String::from("what/when/where.stuff"),
                        raw_url: String::from("idk"),
                        patch: String::from("I guess"),
                        status: String::from("modified"),
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
                    raw_url: String::from("stuff"),
                    patch: String::from("more_stuff"),
                    status: String::from("modified"),
                },
                Change {
                    filename: String::from("yes/no/maybe.idk"),
                    raw_url: String::from("sure"),
                    patch: String::from("why not"),
                    status: String::from("modified"),
                },
                Change {
                    filename: String::from("what/when/where.stuff"),
                    raw_url: String::from("idk"),
                    patch: String::from("I guess"),
                    status: String::from("modified"),
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
                        raw_url: String::from("stuff"),
                        patch: String::from("more_stuff"),
                        status: String::from("modified"),
                    },
                    Change {
                        filename: String::from("yes/no/maybe.idk"),
                        raw_url: String::from("sure"),
                        patch: String::from("why not"),
                        status: String::from("modified"),
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
            raw_url: "idk".to_string(),
            patch: diff.to_string(),
            status: String::from("modified"),
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
            raw_url: "idk".to_string(),
            patch: diff.to_string(),
            status: String::from("modified"),
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
            raw_url: "idk".to_string(),
            patch: diff.to_string(),
            status: String::from("modified"),
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
        let raw_url_contents = dedent(
            "
            line one
            line two
            line three
            ",
        );
        fs::write(&b, raw_url_contents).unwrap();

        let diff = "@@ -1,3 +0,0 @@\n-line one\n-line two\n-line three";
        let change = Change {
            filename: "what/when/where.stuff".to_string(),
            raw_url: "idk".to_string(),
            patch: diff.to_string(),
            status: String::from("removed"),
        };
        let expected = format!("{EOL}line one{EOL}line two{EOL}line three{EOL}");
        change.reverse_apply(&b, &a).unwrap();
        assert_eq!(fs::read(&a).unwrap(), expected.into_bytes());
    }
}
