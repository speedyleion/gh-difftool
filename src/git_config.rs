//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use anyhow::Result;
use git_config::File;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

// Looking at the Git source code the main entry point is
// https://github.com/git/git/blob/master/git-mergetool--lib.sh
// This will call into the various files in https://github.com/git/git/tree/master/mergetools
// to build up the command and arguments.
// In order to support all of the options that git provides we're going to *start* with just a few
// tool options
//
// Note: I haven't figured out how the config -> difftool maps to the `cmd` and `path` config
// options.
//

static DIFFTOOLS: Lazy<HashMap<&str, &str>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("bc", "bcompare");
    m.insert("bc3", "bcompare");
    m.insert("bc4", "bcompare");
    m.insert("meld", "meld");
    m.insert("gvimdiff", "gvimdiff");
    m
});

#[derive(Debug, displaydoc::Display, Eq, PartialEq)]
pub enum Error {
    /// "{0}" is not a git repository
    NotAGitRepository(PathBuf),
    /// No difftool configured for git
    NoDifftoolConfigured,
    /// Unknown difftool {0}
    UnknownDifftool(String),
}

impl std::error::Error for Error {}

/// A difftool from git
#[derive(Debug, Eq, PartialEq, Default)]
pub struct Difftool {
    tool: String,
    program: String,
}

impl Difftool {
    pub fn new(git_dir: impl AsRef<Path>, tool: Option<impl AsRef<str>>) -> Result<Self> {
        let tool = match tool {
            Some(tool) => tool.as_ref().to_string(),
            None => get_config_difftool(&git_dir)?,
        };

        let program = get_difftool_program(&git_dir, &tool)?;

        Ok(Self { tool, program })
    }

    pub async fn launch(&self, local: impl AsRef<OsStr>, remote: impl AsRef<OsStr>) -> Result<()> {
        let mut command = Command::new(&self.program);
        command.arg(local);
        command.arg(remote);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.output().await?;
        // Some difftools, like bcompare, will return non zero status when there is a diff and 0
        // only when there are no changes.  This prevents us from trusting the status
        Ok(())
    }
}

fn get_difftool_program(git_dir: impl AsRef<Path>, name: impl AsRef<str>) -> Result<String> {
    let config = git_config(git_dir)?;
    match config.string("difftool", Some(name.as_ref()), "path") {
        Some(path) => Ok(path.to_string()),
        None => Ok(lookup_known_tool_programs(&name)?),
    }
}

fn lookup_known_tool_programs(tool: impl AsRef<str>) -> Result<String> {
    let tool = tool.as_ref();
    let program = DIFFTOOLS
        .get(tool)
        .ok_or_else(|| Error::UnknownDifftool(tool.to_string()))?;
    Ok(program.to_string())
}

fn get_config_difftool(dir: impl AsRef<Path>) -> Result<String> {
    let config = git_config(dir)?;
    match config.string("diff", None, "tool") {
        Some(tool) => Ok(tool.to_string()),
        // Note: due to the global git config being found and the users diff setting being taken
        // form that this None branch isn't unit tested.
        None => {
            // Similar to git, we fall back to the merge tool if it's available
            match config.string("merge", None, "tool") {
                Some(tool) => Ok(tool.to_string()),
                None => Err(Error::NoDifftoolConfigured.into()),
            }
        }
    }
}

/// Find the git directory, `.git`, for the provided directory
///
/// This will walk up from the provided `dir` looking for the `.git` directory.
/// This does *not* properly handle `.git` files for worktrees and submodules
///
/// # Returns:
/// The full path to the `.git` directory if found. None if not found.
fn find_git_dir(dir: impl AsRef<Path>) -> Option<PathBuf> {
    let dir = dir.as_ref();
    for path in dir.ancestors() {
        let git = path.join(".git");
        if git.exists() {
            return Some(git);
        }
    }
    None
}

/// Get the git config for the repo at `dir`
///
/// # Arguments
/// * `dir` - The directory or sub-directory to a git repo
///
/// # Returns
/// The config `File` for the repo at `dir`.
///
/// # Error
/// If `dir` is not for a git repository
pub fn git_config(dir: impl AsRef<Path>) -> Result<File<'static>> {
    let git_dir =
        find_git_dir(&dir).ok_or_else(|| Error::NotAGitRepository(PathBuf::from(dir.as_ref())))?;
    Ok(File::from_git_dir(git_dir)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::current_dir;
    use std::fs;
    use temp_testdir::TempDir;
    use yare::parameterized;

    #[test]
    fn found_git_dir_in_current_dir() {
        let dir = current_dir().unwrap();
        let expected = dir.join(".git");
        assert_eq!(find_git_dir(dir), Some(expected));
    }

    #[test]
    fn found_git_dir_in_nested_dir() {
        let root_dir = current_dir().unwrap();
        let expected = root_dir.join(".git");
        let nested_dir = root_dir.join("src");

        assert_eq!(find_git_dir(nested_dir), Some(expected));
    }

    #[test]
    fn relative_path() {
        let expected = PathBuf::from(".git");
        let nested_dir = PathBuf::from("src");

        assert_eq!(find_git_dir(nested_dir), Some(expected));
    }

    #[test]
    fn getting_git_config() {
        let temp = TempDir::default().permanent();
        let git_dir = temp.join(".git");
        let config_file = git_dir.join("config");
        fs::create_dir_all(git_dir).unwrap();
        fs::write(&config_file, "[user]\n    name = Me\n").unwrap();
        let config = git_config(temp).unwrap();

        assert_eq!(
            config.string("user", None, "name").unwrap().to_string(),
            "Me".to_string()
        );
    }

    #[test]
    fn found_difftool_in_config() {
        let temp = TempDir::default().permanent();
        let git_dir = temp.join(".git");
        let config_file = git_dir.join("config");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(&config_file, "[diff]\n    tool = meld\n").unwrap();

        assert_eq!(get_config_difftool(&temp).unwrap(), "meld".to_string());
    }

    #[test]
    fn difftool_program_from_config() {
        let temp = TempDir::default().permanent();
        let git_dir = temp.join(".git");
        let config_file = git_dir.join("config");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(
            &config_file,
            "[difftool.makebelieve]\n    path = some/random/path",
        )
        .unwrap();

        assert_eq!(
            get_difftool_program(&temp, "makebelieve").unwrap(),
            "some/random/path".to_string()
        );
    }

    #[test]
    fn difftool_program_from_config_with_quotes() {
        let temp = TempDir::default().permanent();
        let git_dir = temp.join(".git");
        let config_file = git_dir.join("config");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(&config_file, "[difftool.magic]\n    path = \"my/cool/dir\"").unwrap();

        assert_eq!(
            get_difftool_program(&temp, "magic").unwrap(),
            "my/cool/dir".to_string()
        );
    }

    #[parameterized(
    bc = { "bc", "bcompare" },
    bc3 = { "bc", "bcompare" },
    bc4 = { "bc", "bcompare" },
    meld = { "meld", "meld" },
    gvimdiff = { "gvimdiff", "gvimdiff" },
    )]
    fn lookup_known_tool(tool: &str, program: &str) {
        assert_eq!(
            lookup_known_tool_programs(tool).unwrap(),
            program.to_string()
        );
    }

    #[test]
    fn difftool_from_config_overrides_local() {
        let temp = TempDir::default().permanent();
        let git_dir = temp.join(".git");
        let config_file = git_dir.join("config");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(&config_file, "[difftool.bc]\n    path = /does/not/exist").unwrap();

        assert_eq!(
            get_difftool_program(&temp, "bc").unwrap(),
            "/does/not/exist".to_string()
        );
    }

    #[parameterized(
    bc = { "bc", "yes" },
    madeup = { "madeup", "no" },
    )]
    fn new_difftool(tool: &str, program: &str) {
        let temp = TempDir::default().permanent();
        let git_dir = temp.join(".git");
        let config_file = git_dir.join("config");
        fs::create_dir_all(&git_dir).unwrap();
        let contents = format!("[difftool.{tool}]\n    path = {program}");
        fs::write(&config_file, &contents).unwrap();

        assert_eq!(
            Difftool::new(&temp, Some(tool)).unwrap(),
            Difftool {
                tool: tool.to_string(),
                program: program.to_string()
            }
        );
    }
}
