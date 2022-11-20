# gh-difftool

A difftool extension to the GitHub CLI, [gh](https://cli.github.com/).

Launches a difftool to show the differences of PRs. The files will be created in a
temporary directory with the base branch version of the files prefixed with
`base_`.

```shell
Usage: gh-difftool [OPTIONS]

Options:
  -t, --tool <DIFFTOOL>       The difftool command to run [env: DIFFTOOL=]
      --repo <ORG/REPO_NAME>  The GitHub repo to diff, defaults to the GitHub remote of the current git repo
      --pr <PR>               The PR to diff, defaults to the one associated with the current branch
  -h, --help                  Print help information
  -V, --version               Print version information
```

When no args the tool will try to diff the current branch's PR.

When provided a PR number will diff that PR. When provided a repo (requires a
pr), will diff that repo's PR.

For instance one can do:

```shell
gh difftool --repo speedyleion/gh-difftool --pr 10
```
from any repo and get the same result.

## Installation

This can be installed like any other GitHub CLI extension,
<https://docs.github.com/en/github-cli/github-cli/using-github-cli-extensions>

```shell
gh extension install speedyleion/gh-difftool
```

> Note: Current installs only support 64bit Linux.

## Difftool

The default diff tool is `bcompare`. There is no good reason it is the default.
It happened to be what I had on my system.

The `-t, --tool` option or the environment variable `DIFFTOOL` can be set to
specify the diff tool to use.

The name `DIFFTOOL` was chosen to be generic similar to `EDITOR`.

The difftool will be invoked as :

```shell
<tool> <base_version> <on_disk_version>
```

If this doesn't work with your tool of choice you'll want to wrap it in a
launcher script or similar that matches this format.

## Requires

- The GitHub CLI, [gh](https://cli.github.com/)
- The [patch](https://www.man7.org/linux/man-pages/man1/patch.1.html) utility
- OpenSSL headers for building
