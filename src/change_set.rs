//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Set of changes that goes from one version of files to another

use anyhow::Result;
use patch::Patch;
use serde::{Deserialize, Serialize};

#[derive(Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Change {
    pub filename: String,
    pub raw_url: String,
    pub patch: String,
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Taco<'a> {
    pub stuff: &'a str,
}
impl<'a> TryFrom<&'a str> for Taco<'a> {
    type Error = anyhow::Error;

    fn try_from(stuff: &'a str) -> Result<Self> {
        Ok(Taco{ stuff})

    }
}

// impl<'a> TryFrom<&'a Change> for Patch<'a> {
//     type Error = anyhow::Error;
//
//     fn try_from(change: &'a Change) -> Result<Self> {
//         Ok(Patch::from_single(&change.raw_url)?)
//
//     }
// }

#[cfg(test)]
mod tests {
    use patch::Patch;
    use super::*;

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
                    patch: String::from("---Cargo.toml\n+++Cargo.toml\n@@ -6,3 +6,7 @@ edition = \"2021\"\n [dev-dependencies]\n assert_cmd = \"2.0.4\"\n mockall = \"0.11.2\"\n+textwrap = \"0.15.1\"\n+\n+[dependencies]\n+patch = \"0.6.0\"\n"),
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
    fn try_from_change() {
        let change = Change {
            filename: String::from("Cargo.toml"),
            raw_url: String::from("https://github.com/speedyleion/gh-difftool/raw/befb7bf69c3c8ba97c714d57c8dadd9621021c84/Cargo.toml"),
            patch: String::from("--- Cargo.toml\n+++ Cargo.toml\n@@ -6,3 +6,7 @@ edition = \"2021\"\n [dev-dependencies]\n assert_cmd = \"2.0.4\"\n mockall = \"0.11.2\"\n+textwrap = \"0.15.1\"\n+\n+[dependencies]\n+patch = \"0.6.0\"\n"),
        };
        let patch = Patch::from_single(&change.patch).unwrap();
        assert_eq!(patch.old.path, "Cargo.toml");
        assert_eq!(patch.new.path, "Cargo.toml");
    }
}
