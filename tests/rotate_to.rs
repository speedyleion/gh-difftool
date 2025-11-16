//          Copyright Nick G 2023.
// Distributed under the Boost Software License, Version 1.0.
//    (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

use assert_cmd::cargo;

/// These tests require a network connection to github

#[test]
fn pr_10() {
    let mut cmd = cargo::cargo_bin_cmd!("gh-difftool");
    let assert = cmd
        .arg("--name-only")
        .arg("--rotate-to")
        .arg("Cargo.toml")
        .arg("10")
        .arg("--repo")
        .arg("speedyleion/gh-difftool")
        .assert();
    assert.success().stdout(
        "Cargo.toml\nREADME.md\nscripts/build_dist.h\n.github/workflows/release.yml\n.gitignore\n",
    );
}

#[test]
fn pr_4535_from_clap() {
    let mut cmd = cargo::cargo_bin_cmd!("gh-difftool");
    let assert = cmd
        .arg("--name-only")
        .arg("--rotate-to")
        .arg("clap_complete/Cargo.toml")
        .arg("4535")
        .arg("--repo")
        .arg("clap-rs/clap")
        .assert();
    assert
        .success()
        .stdout("clap_complete/Cargo.toml\nCargo.lock\nCargo.toml\n");
}

#[test]
fn non_existent_file() {
    let mut cmd = cargo::cargo_bin_cmd!("gh-difftool");
    let assert = cmd
        .arg("--name-only")
        .arg("--rotate-to")
        .arg("not.real")
        .arg("10")
        .arg("--repo")
        .arg("speedyleion/gh-difftool")
        .assert();
    assert
        .failure()
        .stderr("Error: No such path 'not.real' in the diff.\n");
}
