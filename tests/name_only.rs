//          Copyright Nick G 2022.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use assert_cmd::Command;

/// These tests require a network connection to github

#[test]
fn pr_10() {
    let mut cmd = Command::cargo_bin("gh-difftool").unwrap();
    let assert = cmd
        .arg("--name-only")
        .arg("10")
        .arg("--repo")
        .arg("speedyleion/gh-difftool")
        .assert();
    assert.success().stdout(
        ".github/workflows/release.yml\n.gitignore\nCargo.toml\nREADME.md\nscripts/build_dist.h\n",
    );
}

#[test]
fn pr_4535_from_clap() {
    let mut cmd = Command::cargo_bin("gh-difftool").unwrap();
    let assert = cmd
        .arg("--name-only")
        .arg("4535")
        .arg("--repo")
        .arg("clap-rs/clap")
        .assert();
    assert
        .success()
        .stdout("Cargo.lock\nCargo.toml\nclap_complete/Cargo.toml\n");
}
