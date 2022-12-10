# gh-difftool

A difftool extension to the GitHub CLI, [gh](https://cli.github.com/).

Launches a difftool to show the differences of PRs. The files will be created
in a temporary directory with the base branch version of the files prefixed
with `base_`.

```shell
Usage: gh-difftool [OPTIONS]

Options:
  -t, --tool <TOOL>           The tool to use for diffing [env: GH_DIFFTOOL=]
      --repo <ORG/REPO_NAME>  The GitHub repo to diff, defaults to the GitHub remote of the current git repo
      --pr <PR>               The PR to diff, defaults to the one associated with the current branch
      --name-only             Show only the names of files that changed in a PR
  -h, --help                  Print help information
  -V, --version               Print version information
```

With no args, the tool will try to diff the current branch's PR.

When provided a PR number will diff that PR. When provided a repo (requires a
pr), will diff that repo's PR.

For instance one can do the following from anywhere and get a result.

```shell
gh difftool --repo speedyleion/gh-difftool --pr 10
```

## Installation

This can be installed like any other GitHub CLI extension,
<https://docs.github.com/en/github-cli/github-cli/using-github-cli-extensions>

```shell
gh extension install speedyleion/gh-difftool
```

> Note: Current installs only support 64bit Linux.

## Options

`--name-only`: Print only the names of files that changed in a PR, to stdout.

`--pr`: The PR number to diff. When not specified the `gh` command line will be
used to look up the current PR. An error will be output on stdout if this option
is omitted and the current branch is not associated with a PR.

`--repo`: The GitHub repository to diff, defaults to the GitHub remote of the
current git repository.

`-t, --tool`: The tool to use for diffing the files. This name should match
those used by git's difftool command or a custom one that the user has set
the `git.diff.path` value for.

## Configuration

By default, the tool to use will be derived from the current git configuration
<https://git-scm.com/docs/git-difftool>. The `diff.tool` git configuration
option will be used to determine the tool. Similar to git if `diff.tool` is
not set then `merge.tool` will be used. Unlike git, if no tool is set
`gh-difftool` will report an error.

Alternatively one can specify a tool to use via the command line argument `-t,
--tool` or by the environment variable `GH_DIFFTOOL`.

There are a handful of known difftools available in `gh-difftool`, (bc, bc3,
bc4, meld, gvimdiff). These known tools assume that the executable is available
in the `PATH`.

When the difftool is not in the `PATH`, it can be specified via
the `difftool.<tool>.path` git config option.

```ini
[difftool.sometool]
    path = /path/to/some/difftool
```

> Note: `gh-difftool` does *not* support the `difftool.<tool>.cmd` or
> the `difftool.trustExitCode` git config options. Exit codes are not trusted.

### Unsupported difftools

Only a handful of difftools supported by git are natively supported
by `gh-difftool`. If your difftool of choice is not supported you can explicitly
set the `difftool.<tool>.path` git config option as a workaround.

The difftool will be invoked as :

```shell
<tool> <base_version> <pr_version>
```

Where `<tool>` will be taken from `difftool.<tool>.path`. If this invocation
format doesn't work with your tool of choice you'll want to wrap the tool in a
launcher script or similar that matches this format. If the launcher script
happens to not work for the git difftool then you can make a wrapper difftool
that will be used only by `gh-difftool` and not by git.

1. Set the environment variable `GH_DIFFTOOL` to the name of the wrapper
   difftool, e.g. `set GH_DIFFTOOL=wrappertool`
2. Configure the `path` git config option for the wrapper difftool (often
   in `~/.gitconfig`)

    ```ini
    [difftool.wrappertool]
        path = /path/to/the/wrapper/script
    ```

git will only use the `difftool.wrappertool.path` value if git is invoked
with `wrappertool` as the git difftool. This will allow `gh-difftool` to
work and not have a negative side effect on git.

## Requires

- The GitHub CLI, [gh](https://cli.github.com/)
- The [patch](https://www.man7.org/linux/man-pages/man1/patch.1.html) utility
