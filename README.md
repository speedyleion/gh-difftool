# gh-difftool

A difftool implementation for use with GitHub pull requests.

Launches a difftool to show the differences between the branch the PR is being
merged into and the current on disk files.  If there are no local modifications
this should be the same diffs that the GitHub web UI shows.

```shell
Usage: gh-difftool [OPTIONS]

Options:
  -t, --tool <DIFFTOOL>  The difftool command to run [env: DIFFTOOL=]
  -h, --help             Print help information
  -V, --version          Print version information
```

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
