//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

//! Module for interacting with the github command line

use std::process::{Command, Stdio};

#[derive(Debug, Default)]
pub struct GhCli;

impl GhCli {
    pub fn diff() -> Result<String, String> {
        let mut cmd = Command::new("gh");
        cmd.args(["pr", "diff"]);
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        let output = cmd
            .output()
            .map_err(|e| format!("Failed running gh diff: {}", e))?;
        let status = output.status;
        if status.success() {
            Ok(String::from_utf8(output.stdout)
                .map_err(|e| format!("Failed to convert output to string {}", e))?)
        } else {
            Err(String::from_utf8(output.stderr)
                .map_err(|e| format!("Failed to convert output to string {}", e))?)
        }
    }
}

mod tests {
    use super::*;

    #[test]
    fn no_current_pr() {
        let message = GhCli::diff().err().unwrap();
        assert!(message.contains("no pull requests found for branch"));
    }
}
