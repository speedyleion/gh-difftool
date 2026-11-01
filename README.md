# gh-difftool

A difftool extension to the GitHub CLI, [gh](https://cli.github.com/).

Launches a difftool to show the differences of a pull request. The base branch
version of the files will be created in a temporary directory, prefixed with
`base_`.

When no pull request is given, the pull request for the current branch is used
and the base files are diffed against the local files in the working tree. Any
edits made in the difftool will be kept.

When a pull request is given, the base files are diffed against temporary 
versions of the pull request files.

```shell
Usage: gh-difftool [OPTIONS] [PR] [-- <FILES>...]

Arguments:
  [PR]
          The pull request to diff

          When omitted the pull request associated with the current branch will be used,
          and the base is diffed against the local files.
          A pull request can be supplied as argument in any of the following formats:
          - by number, e.g. "123"
          - by URL, e.g. "https://github.com/OWNER/REPO/pull/123"

  [FILES]...
          Specific files to diff.

          When not provided all of the files that changed in the pull request will be diffed

Options:
  -t, --tool <TOOL>
          The tool to use for diffing

          [env: GH_DIFFTOOL=]

  -R, --repo <OWNER/REPO>
          The GitHub repo to diff, defaults to the GitHub remote of the current git repo

      --name-only
          Show only the names of files that changed in a pull request

      --shortstat
          Show only the number of changed files as well as number of added and deleted lines

      --rotate-to <ROTATE_TO>
          Start showing the diff for the given file, the files before it will move to end.

          Applied before `--skip-to`. This behavior deviates from `git-difftool` which
          seems to ignore rotation when `--skip-to` is present.

      --skip-to <SKIP_TO>
          Start showing the diff for the given file, skipping all the files before it

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

With no args, the tool will try to diff the current branch's pull request.

When provided a pull request number or URL will diff that pull request. When
provided a repo (requires a pull request number), will diff that repo's pull
request.

For instance one can do the following from any cloned GitHub repo

```shell
gh difftool --repo speedyleion/gh-difftool 10
```

## Installation

This can be installed like any other GitHub CLI extension,
<https://docs.github.com/en/github-cli/github-cli/using-github-cli-extensions>

```shell
gh extension install speedyleion/gh-difftool
```

Current installs support:

- x86_64 Linux
- arm64 Android
- x86_64 Mac
- arm64 Mac
- x86_64 Windows

## Configuration

By default, the tool to use will be derived from the current git configuration
<https://git-scm.com/docs/git-difftool>. The `diff.tool` git configuration
option will be used to determine the tool. Similar to git, if `diff.tool` is
not set then `merge.tool` will be used. Unlike git, if neither option is set
`gh-difftool` will report an error.

Alternatively one can specify a tool to use via the command line argument `-t,
--tool` or by the environment variable `GH_DIFFTOOL`.

There are a handful of known difftools available in `gh-difftool`, (bc, bc3,
bc4, meld, gvimdiff). These known tools assume that the executable is available
in the `PATH`.

> Note: `gh-difftool` does *not* support the `difftool.trustExitCode` git
> config option. Exit codes are not trusted.

### Tool Path

When the difftool is not in the `PATH`, it can be specified via
the `difftool.<tool>.path` git config option.

```ini
[difftool.sometool]
    path = /path/to/some/difftool
```

### Tool Command

If your difftool of choice is not supported you can explicitly
use the `difftool.<tool>.cmd` option.

```ini
[difftool.sometool]
    cmd = /path/to/some/unsupported/difftool --extra-arg=foo $LOCAL $REMOTE
```

The `$LOCAL` and `$REMOTE` variables will be replaced with the paths to the
base and pull request versions of the files.

Unlike the standard git difftool, the
[`difftool.<tool>.cmd`](https://git-scm.com/docs/git-difftool#Documentation/git-difftool.txt-difftoollttoolgtcmd)
used with `gh-difftool` will *not* be run in a shell.

This means:

- Only the `$LOCAL` and `$REMOTE` variables will be replaced.
- the `$LOCAL` and `$REMOTE` variables need to be space separated.
  Use `--local $LOCAL`, not `--local=$LOCAL`.

## Requires

- The GitHub CLI, [gh](https://cli.github.com/)
