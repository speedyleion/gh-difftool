//          Copyright Nick G 2026.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Reports generated from a set of changes

use crate::change_set::ChangeSet;
use std::io::{self, Write};

/// Write the name of each changed file on its own line.
pub fn name_only(change_set: &ChangeSet, mut writer: impl Write) -> io::Result<()> {
    for change in &change_set.changes {
        writeln!(writer, "{}", change.filename)?;
    }
    Ok(())
}

/// Write a summary of the files and lines changed.
pub fn shortstat(change_set: &ChangeSet, mut writer: impl Write) -> io::Result<()> {
    let files = change_set.changes.len();
    if files == 0 {
        return Ok(());
    }

    let additions: usize = change_set
        .changes
        .iter()
        .map(|change| change.additions)
        .sum();
    let deletions: usize = change_set
        .changes
        .iter()
        .map(|change| change.deletions)
        .sum();

    write!(
        writer,
        " {files} file{} changed",
        if files == 1 { "" } else { "s" }
    )?;

    if additions > 0 || deletions == 0 {
        write!(
            writer,
            ", {additions} insertion{}(+)",
            if additions == 1 { "" } else { "s" }
        )?;
    }
    if deletions > 0 || additions == 0 {
        write!(
            writer,
            ", {deletions} deletion{}(-)",
            if deletions == 1 { "" } else { "s" }
        )?;
    }
    writeln!(writer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change_set::Change;
    use yare::parameterized;

    fn change(filename: &str, additions: usize, deletions: usize) -> Change {
        Change {
            filename: filename.to_string(),
            additions,
            deletions,
            ..Change::default()
        }
    }

    #[test]
    fn name_only_writes_each_filename_on_its_own_line() {
        let change_set = ChangeSet {
            changes: vec![
                Change {
                    filename: "one.txt".to_string(),
                    ..Change::default()
                },
                Change {
                    filename: "path/two.txt".to_string(),
                    ..Change::default()
                },
            ],
        };
        let mut output = Vec::new();

        name_only(&change_set, &mut output).unwrap();

        assert_eq!(output, b"one.txt\npath/two.txt\n");
    }

    #[parameterized(
        aggregated = {&[("one.txt", 1, 2), ("two.txt", 3, 0)], " 2 files changed, 4 insertions(+), 2 deletions(-)\n"},
        singular = {&[("one.txt", 1, 1)], " 1 file changed, 1 insertion(+), 1 deletion(-)\n"},
        additions_only = {&[("one.txt", 2, 0)], " 1 file changed, 2 insertions(+)\n"},
        deletions_only = {&[("one.txt", 0, 2)], " 1 file changed, 2 deletions(-)\n"},
        no_line_changes = {&[("renamed.txt", 0, 0)], " 1 file changed, 0 insertions(+), 0 deletions(-)\n"},
        empty = {&[], ""},
    )]
    fn shortstat_formats_changes(changes: &[(&str, usize, usize)], expected: &str) {
        let change_set = ChangeSet {
            changes: changes
                .iter()
                .map(|(filename, additions, deletions)| change(filename, *additions, *deletions))
                .collect(),
        };
        let mut output = Vec::new();

        shortstat(&change_set, &mut output).unwrap();

        assert_eq!(output, expected.as_bytes());
    }
}
