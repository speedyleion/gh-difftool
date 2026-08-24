//          Copyright Nick G 2026.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use assert_cmd::cargo;

/// This test requires a network connection to GitHub.
#[test]
fn pr_10() {
    let mut cmd = cargo::cargo_bin_cmd!("gh-difftool");
    let assert = cmd
        .arg("--shortstat")
        .arg("10")
        .arg("--repo")
        .arg("speedyleion/gh-difftool")
        .assert();

    assert
        .success()
        .stdout(" 5 files changed, 43 insertions(+), 4 deletions(-)\n");
}
