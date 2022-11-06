mod change_set;
mod cmd;
mod diff;
mod gh_interface;

use crate::change_set::Change;
use clap::Parser;
use std::ffi::OsStr;
use std::process::Command;
use tempfile::NamedTempFile;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The difftool command to run
    #[arg(short = 't', long = "tool", env = "DIFFTOOL")]
    difftool: Option<String>,
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    let difftool = cli.difftool.as_deref().unwrap_or("bcompare");

    run_diff(difftool)
}

fn run_diff(difftool: impl AsRef<str>) -> Result<(), String> {
    let mut gh = gh_interface::GhCli::new(Command::new("gh"));
    let change_set = gh.local_change_set().map_err(|e| format!("{e}"))?;
    for change in change_set.changes {
        diff_one(&change, &difftool)?;
    }
    Ok(())
}

fn diff_one(change: &Change, difftool: impl AsRef<str>) -> Result<(), String> {
    let original = create_temp_original(change)?;
    let mut difftool = diff::Diff::new(Command::new(OsStr::new(difftool.as_ref())));
    difftool.launch(original.path().as_os_str(), OsStr::new(&change.filename))
}

fn create_temp_original(change: &Change) -> Result<NamedTempFile, String> {
    let file = NamedTempFile::new().map_err(|e| format!("Failed getting temp file: {}", e))?;

    change.reverse_apply(&change.filename, file.path())?;
    Ok(file)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use temp_testdir::TempDir;
    use textwrap::dedent;

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
        let expected = dedent(
            "
            line one
            line two
            line three
            ",
        );
        let change = Change {
            filename: b.to_string_lossy().to_string(),
            raw_url: "sure".to_string(),
            patch: diff.to_string(),
        };
        let original = create_temp_original(&change).unwrap();
        assert_eq!(fs::read(&original.path()).unwrap(), expected.into_bytes());
    }
}
