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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change_set::Change;

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
}
