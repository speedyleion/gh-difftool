//          Copyright Nick G 2022.
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
    let mut cmd = cargo::cargo_bin_cmd!("gh-difftool");
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

// The test cases from rust-lang were found by using,
// https://docs.github.com/en/graphql/overview/explorer
// and walking through prs looking for ones that exceeded the 30 file page size
// query {
//   repository(owner:"rust-lang", name:"rust") {
//     pullRequests(first:100 after:"Y3Vyc29yOnYyOpHOAAHobA==") {
//       edges{
//         node{
//           changedFiles
//           number
//         }
//       }
//       pageInfo {
//         endCursor
//         startCursor
//       }
//     }
//   }
// }

#[test]
fn pr_426_from_rust() {
    let files = [
        "src/comp/front/lexer.rs",
        "src/comp/front/parser.rs",
        "src/comp/front/token.rs",
        "src/comp/pretty/pprust.rs",
        "src/test/compile-fail/bad-recv.rs",
        "src/test/run-fail/linked-failure.rs",
        "src/test/run-fail/task-comm-14.rs",
        "src/test/run-fail/trivial-message2.rs",
        "src/test/run-pass/acyclic-unwind.rs",
        "src/test/run-pass/basic-1.rs",
        "src/test/run-pass/basic-2.rs",
        "src/test/run-pass/basic.rs",
        "src/test/run-pass/comm.rs",
        "src/test/run-pass/decl-with-recv.rs",
        "src/test/run-pass/destructor-ordering.rs",
        "src/test/run-pass/lazychan.rs",
        "src/test/run-pass/many.rs",
        "src/test/run-pass/obj-dtor.rs",
        "src/test/run-pass/preempt.rs",
        "src/test/run-pass/rt-circular-buffer.rs",
        "src/test/run-pass/task-comm-0.rs",
        "src/test/run-pass/task-comm-10.rs",
        "src/test/run-pass/task-comm-11.rs",
        "src/test/run-pass/task-comm-15.rs",
        "src/test/run-pass/task-comm-16.rs",
        "src/test/run-pass/task-comm-3.rs",
        "src/test/run-pass/task-comm-4.rs",
        "src/test/run-pass/task-comm-5.rs",
        "src/test/run-pass/task-comm-6.rs",
        "src/test/run-pass/task-comm-7.rs",
        "src/test/run-pass/task-comm-8.rs",
        "src/test/run-pass/task-comm-9.rs",
        "src/test/run-pass/task-comm-chan-nil.rs",
        "src/test/run-pass/task-comm.rs",
        "src/test/run-pass/trivial-message.rs",
        "", // Needed for the trailing newline
    ];
    let mut cmd = cargo::cargo_bin_cmd!("gh-difftool");
    cmd.arg("--name-only")
        .arg("426")
        .arg("--repo")
        .arg("rust-lang/rust")
        .assert()
        .success()
        .stdout(files.join("\n"));
}

#[test]
fn pr_346_from_rust() {
    let mut cmd = cargo::cargo_bin_cmd!("gh-difftool");
    let assert = cmd
        .arg("--name-only")
        .arg("346")
        .arg("--repo")
        .arg("rust-lang/rust")
        .assert()
        .success();
    let stdout = std::str::from_utf8(&assert.get_output().stdout).unwrap();
    assert_eq!(stdout.lines().count(), 182);
}
