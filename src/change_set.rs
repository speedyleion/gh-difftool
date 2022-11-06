//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Set of changes that goes from one version of files to another

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Change {
    pub filename: String,
    pub raw_url: String,
    pub patch: String,
}

impl Change {
    pub fn reverse_apply<P1, P2>(&self, src: P1, dest: P2) -> Result<(), String>
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

#[derive(Default, PartialEq, Eq, Debug)]
pub struct ChangeSet {
    pub changes: Vec<Change>,
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
                "patch": "more_stuff"
              },
              {
                "filename": "yes/no/maybe.idk",
                "raw_url": "sure",
                "patch": "why not"
              },
              {
                "filename": "what/when/where.stuff",
                "raw_url": "idk",
                "patch": "I guess"
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
                    },
                    Change {
                        filename: String::from("yes/no/maybe.idk"),
                        raw_url: String::from("sure"),
                        patch: String::from("why not"),
                    },
                    Change {
                        filename: String::from("what/when/where.stuff"),
                        raw_url: String::from("idk"),
                        patch: String::from("I guess"),
                    }
                ]
            }
        );
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
        };
        let expected = dedent(
            "
            line one
            line two
            line three
            ",
        );
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
        };
        let expected = dedent(
            "
            line one
            line three
            ",
        );
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
        };

        assert!(
            matches!(change.reverse_apply(&b, &a), Err(message) if message.starts_with(&message_start))
        );
    }
}
