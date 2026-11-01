//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Launches a difftool to compare changes

use crate::Change;
use crate::gh_interface;
use crate::git_config;
use anyhow::Result;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::{Builder, TempDir};

#[cfg(windows)]
const NULL_DEVICE: &str = "NUL";
#[cfg(not(windows))]
const NULL_DEVICE: &str = "/dev/null";

#[derive(Debug)]
pub struct Diff {
    difftool: git_config::Difftool,
    temp_dir: TempDir,
    /// When present, the new version of each file is the one in this working tree
    local_root: Option<PathBuf>,
}

#[derive(Debug)]
pub struct Difftool<'a> {
    tool: &'a git_config::Difftool,
    local: OsString,
    remote: OsString,
}

impl<'a> Difftool<'a> {
    fn new(tool: &'a git_config::Difftool, local: OsString, remote: OsString) -> Self {
        Self {
            tool,
            local,
            remote,
        }
    }

    pub async fn launch(&self) -> Result<()> {
        self.tool.launch(&self.local, &self.remote).await
    }
}

impl Diff {
    pub fn new(difftool: git_config::Difftool, local_root: Option<PathBuf>) -> Result<Self> {
        let temp_dir = Builder::new().prefix("gh-difftool").tempdir()?;
        Ok(Self {
            difftool,
            temp_dir,
            local_root,
        })
    }

    pub async fn difftool(&self, change: Change) -> Result<Difftool<'_>> {
        let new = self.new_file_contents(&change).await?;

        // Mimic git-difftool which uses the null device for files that don't exist
        let original = match change.status.as_str() {
            "added" => PathBuf::from(NULL_DEVICE),
            _ => self.create_temp_original(&change, &new)?,
        };
        let remote = match &self.local_root {
            Some(root) if has_symlinked_parent(root, &change.filename) => {
                // Like git, a file under a symlinked directory isn't part of the working tree
                PathBuf::from(NULL_DEVICE)
            }
            Some(root) => {
                let local = root.join(&change.filename);
                match fs::symlink_metadata(&local) {
                    Ok(meta) if meta.is_symlink() => {
                        // Like git, show a symlink as the path it points to rather than following it
                        fs::write(&new, fs::read_link(&local)?.as_os_str().as_encoded_bytes())?;
                        new
                    }
                    Ok(meta) if meta.is_file() => local,
                    // Submodules are directories locally, so they keep the temporary file of the commit sha
                    Ok(_) => new,
                    Err(e) if e.kind() == io::ErrorKind::NotFound => PathBuf::from(NULL_DEVICE),
                    Err(e) => return Err(e.into()),
                }
            }
            None if change.status == "removed" => PathBuf::from(NULL_DEVICE),
            None => new,
        };
        Ok(Difftool::new(
            &self.difftool,
            original.into_os_string(),
            remote.into_os_string(),
        ))
    }

    async fn new_file_contents(&self, change: &Change) -> Result<PathBuf> {
        let dir = self.temp_dir.as_ref();
        let file = dir.join(&change.filename);
        fs::create_dir_all(
            file.parent()
                .expect("Should always have a parent temp path"),
        )?;

        let contents = gh_interface::file_contents(change).await?;
        fs::write(&file, contents)?;
        Ok(file)
    }

    fn create_temp_original(&self, change: &Change, new: impl AsRef<Path>) -> Result<PathBuf> {
        let dir = self.temp_dir.as_ref();
        let old_file_name = change
            .previous_filename
            .as_ref()
            .unwrap_or(&change.filename);
        let file = dir.join(format!("{}_{}", "base", old_file_name));
        fs::create_dir_all(
            file.parent()
                .expect("Should always have a parent temp path"),
        )?;

        change.reverse_apply(new, &file)?;
        Ok(file)
    }
}

/// Whether any of the directories of `filename`, below `root`, is a symlink
fn has_symlinked_parent(root: &Path, filename: impl AsRef<Path>) -> bool {
    filename
        .as_ref()
        .ancestors()
        .skip(1)
        .filter(|dir| !dir.as_os_str().is_empty())
        .any(|dir| fs::symlink_metadata(root.join(dir)).is_ok_and(|meta| meta.is_symlink()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use httpmock::MockServer;
    use httpmock::prelude::GET;
    use std::fs;
    use temp_testdir::TempDir;
    use textwrap::dedent;

    fn difftool(dir: impl AsRef<Path>) -> git_config::Difftool {
        let dir = dir.as_ref();
        let git_dir = dir.join(".git");
        let config = git_dir.join("config");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(&config, "[difftool.bc]\n    path = bcomp").unwrap();
        git_config::Difftool::new(dir, Some("bc")).unwrap()
    }

    #[test]
    fn create_temp() {
        let temp = TempDir::default().permanent();
        let b = temp.join("b");
        let new = dedent(
            "
            line one
            line changed
            line three
            ",
        );
        fs::write(&b, new).unwrap();
        let diff = "@@ -1,3 +1,3 @@\n line one\n-line two\n+line changed\n line three";
        let expected = "\nline one\nline two\nline three\n".to_string();
        let change = Change {
            filename: "ignore_me".to_string(),
            contents_url: "sure".to_string(),
            additions: 0,
            deletions: 0,
            patch: Some(diff.to_string()),
            status: "modified".to_string(),
            previous_filename: None,
            sha: "why not".to_string(),
        };
        let diff = Diff::new(difftool(&temp), None).unwrap();
        let original = diff.create_temp_original(&change, b).unwrap();
        assert!(original.to_str().unwrap().ends_with(&change.filename));
        assert_eq!(fs::read(&original).unwrap(), expected.into_bytes());
    }

    #[test]
    fn renamed_diff() {
        let temp = TempDir::default().permanent();
        let b = temp.join("b");
        let new = dedent(
            "
            line one
            line changed
            line three
            ",
        );
        fs::write(&b, new).unwrap();
        let diff = "@@ -1,3 +1,3 @@\n line one\n-line two\n+line changed\n line three";
        let expected = "\nline one\nline two\nline three\n".to_string();
        let change = Change {
            filename: "ignore_me".to_string(),
            contents_url: "sure".to_string(),
            additions: 0,
            deletions: 0,
            patch: Some(diff.to_string()),
            status: "renamed".to_string(),
            previous_filename: Some("new_filename".to_string()),
            sha: "why not".to_string(),
        };
        let diff = Diff::new(difftool(&temp), None).unwrap();
        let original = diff.create_temp_original(&change, b).unwrap();
        assert!(
            original
                .to_str()
                .unwrap()
                .ends_with(change.previous_filename.as_ref().unwrap())
        );
        assert_eq!(fs::read(&original).unwrap(), expected.into_bytes());
    }

    #[tokio::test]
    async fn get_new_content() {
        let temp = TempDir::default();
        let contents = "line one\nline two";
        let encoded = STANDARD.encode(contents.as_bytes());
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/one.c");
            then.status(200)
                .header("content-type", "text/html")
                .body(format!(
                    "{{\"content\":\"{encoded}\", \"type\":\"file\", \"sha\": \"not used\"}}"
                ));
        });
        let change = Change {
            filename: "foo/bar/fish.ext".to_string(),
            contents_url: server.url("/one.c"),
            additions: 0,
            deletions: 0,
            patch: Some("@@ -1,3 +1,3 @@\n doesn't matter".to_string()),
            status: "modified".to_string(),
            previous_filename: None,
            sha: "not used".to_string(),
        };
        let diff = Diff::new(difftool(&temp), None).unwrap();
        let new_file = diff.new_file_contents(&change).await.unwrap();

        mock.assert();
        assert_eq!(
            fs::read(&new_file).unwrap(),
            contents.to_string().into_bytes()
        );
    }

    const MODIFIED: &str = "@@ -1,2 +1,2 @@\n line one\n-line two\n+line changed";

    /// Run [`Diff::difftool`] for "foo/bar/fish.ext" whose contents on GitHub are
    /// "line one\nline changed\n"
    ///
    /// # Returns
    /// The `(local, remote)` paths followed by the contents of the `local` and `remote` files, when
    /// they aren't the null device
    async fn difftool_paths(
        status: &str,
        patch: &str,
        local_root: Option<PathBuf>,
    ) -> (OsString, OsString, Option<Vec<u8>>, Option<Vec<u8>>) {
        let temp = TempDir::default();
        let encoded = STANDARD.encode("line one\nline changed\n");
        let server = MockServer::start();
        server.mock(|when, then| {
            when.method(GET).path("/fish");
            then.status(200).body(format!(
                "{{\"content\":\"{encoded}\", \"type\":\"file\", \"sha\": \"not used\"}}"
            ));
        });
        let change = Change {
            filename: "foo/bar/fish.ext".to_string(),
            contents_url: server.url("/fish"),
            additions: 1,
            deletions: 1,
            patch: Some(patch.to_string()),
            status: status.to_string(),
            previous_filename: None,
            sha: "not used".to_string(),
        };
        let diff = Diff::new(difftool(&temp), local_root).unwrap();
        let tool = diff.difftool(change).await.unwrap();
        let original = (tool.local != NULL_DEVICE).then(|| fs::read(&tool.local).unwrap());
        let new = (tool.remote != NULL_DEVICE).then(|| fs::read(&tool.remote).unwrap());
        (tool.local.clone(), tool.remote.clone(), original, new)
    }

    #[tokio::test]
    async fn remote_is_temp_file_without_local_root() {
        let (_, remote, original, _) = difftool_paths("modified", MODIFIED, None).await;
        assert!(PathBuf::from(remote).ends_with("foo/bar/fish.ext"));
        assert_eq!(original.unwrap(), b"line one\nline two\n");
    }

    #[tokio::test]
    async fn remote_is_local_file() {
        let root = TempDir::default();
        let file = root.join("foo/bar/fish.ext");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        // Local edits shouldn't affect the original
        fs::write(&file, "line one\nlocal edit\n").unwrap();

        let (_, remote, original, _) =
            difftool_paths("modified", MODIFIED, Some(root.to_path_buf())).await;
        assert_eq!(remote, file.into_os_string());
        assert_eq!(original.unwrap(), b"line one\nline two\n");
    }

    #[tokio::test]
    async fn remote_is_null_device_when_local_is_missing() {
        let root = TempDir::default();
        let (_, remote, original, _) =
            difftool_paths("modified", MODIFIED, Some(root.to_path_buf())).await;
        assert_eq!(remote, NULL_DEVICE);
        assert_eq!(original.unwrap(), b"line one\nline two\n");
    }

    #[tokio::test]
    async fn remote_is_temp_file_when_local_is_a_directory() {
        let root = TempDir::default();
        fs::create_dir_all(root.join("foo/bar/fish.ext")).unwrap();
        let (_, remote, _, _) =
            difftool_paths("modified", MODIFIED, Some(root.to_path_buf())).await;
        let remote = PathBuf::from(remote);
        assert!(remote.ends_with("foo/bar/fish.ext"));
        assert!(!remote.starts_with(&*root));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn remote_is_link_text_when_local_is_a_symlink() {
        let root = TempDir::default();
        let outside = TempDir::default();
        let target = outside.join("target.txt");
        fs::write(&target, "not the link text\n").unwrap();
        let link = root.join("foo/bar/fish.ext");
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let (_, remote, original, new) =
            difftool_paths("modified", MODIFIED, Some(root.to_path_buf())).await;
        assert!(!PathBuf::from(remote).starts_with(&*root));
        assert_eq!(new.unwrap(), target.into_os_string().into_encoded_bytes());
        assert_eq!(original.unwrap(), b"line one\nline two\n");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn remote_is_link_text_when_local_is_a_dangling_symlink() {
        let root = TempDir::default();
        let link = root.join("foo/bar/fish.ext");
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink("../does/not/exist", &link).unwrap();

        let (_, _, _, new) = difftool_paths("modified", MODIFIED, Some(root.to_path_buf())).await;
        assert_eq!(new.unwrap(), b"../does/not/exist");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn remote_is_null_device_when_a_parent_is_a_symlink() {
        let root = TempDir::default();
        let outside = TempDir::default();
        fs::create_dir_all(outside.join("bar")).unwrap();
        fs::write(outside.join("bar/fish.ext"), "line one\nline changed\n").unwrap();
        // The grandparent, so checking only the immediate parent isn't enough
        std::os::unix::fs::symlink(&*outside, root.join("foo")).unwrap();

        let (_, remote, original, _) =
            difftool_paths("modified", MODIFIED, Some(root.to_path_buf())).await;
        assert_eq!(remote, NULL_DEVICE);
        assert_eq!(original.unwrap(), b"line one\nline two\n");
    }

    #[tokio::test]
    async fn added_file_has_null_device_original() {
        let added = "@@ -0,0 +1,2 @@\n+line one\n+line changed";
        let (local, remote, _, _) = difftool_paths("added", added, None).await;
        assert_eq!(local, NULL_DEVICE);
        assert!(PathBuf::from(remote).ends_with("foo/bar/fish.ext"));
    }

    #[tokio::test]
    async fn removed_file_has_null_device_remote() {
        let removed = "@@ -1,2 +0,0 @@\n-line one\n-line changed";
        let (_, remote, original, _) = difftool_paths("removed", removed, None).await;
        assert_eq!(remote, NULL_DEVICE);
        assert_eq!(original.unwrap(), b"line one\nline changed\n");
    }

    #[tokio::test]
    async fn removed_file_uses_local_file_when_present() {
        let root = TempDir::default();
        let file = root.join("foo/bar/fish.ext");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "recreated locally\n").unwrap();

        let removed = "@@ -1,2 +0,0 @@\n-line one\n-line changed";
        let (_, remote, original, _) =
            difftool_paths("removed", removed, Some(root.to_path_buf())).await;
        assert_eq!(remote, file.into_os_string());
        assert_eq!(original.unwrap(), b"line one\nline changed\n");
    }

    #[tokio::test]
    async fn getting_a_second_set_of_new_content() {
        let temp = TempDir::default();
        let contents = "something\nelse";
        let encoded = STANDARD.encode(contents.as_bytes());
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET).path("/some_raw_url/path");
            then.status(200)
                .header("content-type", "text/html")
                .body(format!(
                    "{{\"content\":\"{encoded}\", \"type\":\"file\", \"sha\": \"not used\"}}"
                ));
        });

        let change = Change {
            filename: "foo/bar/fish.ext".to_string(),
            contents_url: server.url("/some_raw_url/path"),
            additions: 0,
            deletions: 0,
            patch: Some("@@ -1,3 +1,3 @@\n doesn't matter".to_string()),
            status: "modified".to_string(),
            previous_filename: None,
            sha: "not used".to_string(),
        };
        let diff = Diff::new(difftool(&temp), None).unwrap();
        let new_file = diff.new_file_contents(&change).await.unwrap();

        mock.assert();
        assert_eq!(
            fs::read(&new_file).unwrap(),
            contents.to_string().into_bytes()
        );
    }
}
