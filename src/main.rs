mod change_set;
mod cmd;
mod diff;
mod gh_interface;

use clap::Parser;
use std::ffi::{OsStr, OsString};
use std::process::Command;
use tempfile::NamedTempFile;
use crate::change_set::Change;

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

fn normalize_file_name<S: AsRef<str>>(filename: S) -> OsString {
    OsString::from(&filename.as_ref()[2..])
}

fn diff_one(change: &Change, difftool: impl AsRef<str>) -> Result<(), String> {
    let original = create_temp_original(change)?;
    let new = normalize_file_name(&change.filename);

    let mut difftool = diff::Diff::new(Command::new(OsStr::new(difftool.as_ref())));
    difftool.launch(original.path().as_os_str(), &new)
}

fn create_temp_original(change: &Change) -> Result<NamedTempFile, String> {
    let file = NamedTempFile::new().map_err(|e| format!("Failed getting temp file: {}", e))?;

    // The first 2 characters are "b/" from git's diff output
    let normalized_path = normalize_file_name(&change.filename);
    change.reverse_apply(normalized_path, file.path())?;
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
        let diff = dedent(&format!(
            "
            diff --git a/foo.txt b/foo.txt
            index 0c2aa38..0370c84 100644
            --- a/foo.txt
            +++ b/{}
            @@ -1,3 +1,3 @@
             line one
            -line two
            +line changed
             line three
            ",
            b.to_str().unwrap()
        ));
        let expected = dedent(
            "
            line one
            line two
            line three
            ",
        );
        let change = Change{ filename: "foo".to_string(), raw_url: "sure".to_string(), patch: diff};
        let original = create_temp_original(&change).unwrap();
        assert_eq!(fs::read(&original.path()).unwrap(), expected.into_bytes());
    }

    #[test]
    fn strip_diff_prefix() {
        let with_prefix = "a/what/is/up";
        assert_eq!(
            normalize_file_name(with_prefix),
            OsString::from("what/is/up")
        );
    }
}
