# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.2.3] - 2025-05-11

### Fixed

- Submodule diffs causing early exit with error

## [1.2.2] - 2024-12-07

### Added

- Suggest ensuring `patch` is installed when patch fails to spawn.

## [1.2.1] - 2024-10-21

### Fixed

- Files that were renamed and had a diff showed no differences. This was introduced
  in version 1.1.1.

## [1.2.0] - 2024-10-17

### Added

- Add support for the `difftool.<tool>.cmd` git config option.

## [1.1.1] - 2024-09-29

### Fixed

- Renamed files in a diff:
  - Files that had no diffs and were only renamed would cause a panic
  - Files that were renamed and had a diff were displayed with the new filename instead
    of the previous one.

## [1.1.0] - 2024-01-06

### Added

- Update to work with private repositories

## [1.0.0] - 2023-11-27

### Added

- Support for vimdiff as a difftool

## [0.1.15] - 2023-10-07

### Changed

- Reduced the default release binary sizes

## [0.1.14] - 2023-09-01

### Fixed

- Android binary build published to release page

## [0.1.13] - 2023-08-28

### Added

- Android binary build published to release page

## [0.1.12] - 2023-04-07

### Fixed

- Diffing of deleted files. Deleted files now result in a comparison of the old
  file against an empty file.

## [0.1.11] - 2023-02-05

### Fixed

- PRs with more than 30 files will now show all files in the difftool.

## [0.1.10] - 2023-01-28

### Fixed

- Windows binary builds published to release page

## [0.1.9] - 2023-01-28

### Added

- The `--skip-to` option which allows for one to start the diff from the given
  file, skipping all files before the specified one.
- The `--rotate-to` option which allows for one to start the diff from the given
  file, moving all files before the specified one to the end.
- Windows pre-built binaries.

### Changed

- The `--name-only` flag now takes into account the specified
  `FILES` and limits output to only those provided in `FILES` when present.

## [0.1.8] - 2023-01-08

### Added

- Ability to specify `FILES` as the last positional argument. This limits
  which files will be diffed to only those specified.

### Fixed

- BeyondCompare command on linux. When the default changed to `bcomp` this
  required a symlink to `bcompare`, or similar workaround, for linux. Now
  `bcomp` will be attempted and if it doesn't exist then `bcompare` will be
  used.

## [0.1.7] - 2022-12-20

### Fixed

- Incorrect reuse of the future used for launching the difftool
- Incorrect default of `bcompare` for beyond compare. `bcomp` is the command
  line to be used with version control systems.

## [0.1.6] - 2022-12-11

### Added

- The default difftool will be looked up through the git config. The `diff.tool`
  config option will be used to determine the default tool. The `-t, --tool`
  flag and the `GH_DIFFTOOL` environment variable still override the default.
- Ability to specify the pull request as a positional argument in one of the
  following ways:
  - by number, e.g. "123"
  - by URL, e.g. `https://github.com/OWNER/REPO/pull/123`

### Changed

- The environment variable for the tool has been renamed to `GH_DIFFTOOL`.
  Previously it was `DIFFTOOL`. The new name allows for namespacing to avoid
  possible collision.
- The `-t, --tool` and environment variable now expect the git name of
  the difftool program. Previously these values expected the path of the
  difftool program to run. For example: `bc` is the git difftool name for the
  [Beyond Compare](https://scootersoftware.com/) tool. `bc` is now the value
  that should be passed to `-t, --tool`. The Beyond Compare executable is
  named `bcompare`.
- The `--pr` flag has been removed. Pull requests can now be specified via a
  positional argument.

## [0.1.5] - 2022-12-02

### Added

- `--name-only` flag which will output the filenames of the changed files to
  stdout. This is similar to the `gh pr diff --name-only` command.

### Changed

- The diffing logic has been updated to run asynchronously. This allows for
  fetching other changes in the background while the difftool is open and being
  looked at. The changes are still presented to the user in the same order as
  before.

## [0.1.4] - 2022-11-20

### Changed

- Statically link in OpenSSL. Dynamically linking to OpenSSL was causing
  portability issues. Some distro versions use OpenSSL 1.1.# while others use
  OpenSSL 3.#.

## [0.1.3] - 2022-11-19

### Added

- Arguments to diff any pr. `--pr` and `--repo` have been added which allow for
  diffing other PRs and even other repos.

## [0.1.2] - 2022-11-06

### Changed

- The new file version in the diff is based on what GitHub knows about.  The new
  version of the file will also be placed in the temp directory.

## [0.1.1] - 2022-10-23

### Fixed

- Version in Cargo.toml

## [0.1.0] - 2022-10-23

Initial release

[unreleased]: https://github.com/speedyleion/gh-difftool/releases/tag/v1.2.3...HEAD
[1.2.3]: https://github.com/speedyleion/gh-difftool/releases/tag/v1.2.3
[1.2.2]: https://github.com/speedyleion/gh-difftool/releases/tag/v1.2.2
[1.2.1]: https://github.com/speedyleion/gh-difftool/releases/tag/v1.2.1
[1.2.0]: https://github.com/speedyleion/gh-difftool/releases/tag/v1.2.0
[1.1.1]: https://github.com/speedyleion/gh-difftool/releases/tag/v1.1.1
[1.1.0]: https://github.com/speedyleion/gh-difftool/releases/tag/v1.1.0
[1.0.0]: https://github.com/speedyleion/gh-difftool/releases/tag/v1.0.0
[0.1.15]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.15
[0.1.14]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.14
[0.1.13]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.13
[0.1.12]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.12
[0.1.11]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.11
[0.1.10]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.10
[0.1.9]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.9
[0.1.8]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.8
[0.1.7]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.7
[0.1.6]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.6
[0.1.5]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.5
[0.1.4]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.4
[0.1.3]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.3
[0.1.2]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.2
[0.1.1]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.1
[0.1.0]: https://github.com/speedyleion/gh-difftool/releases/tag/v0.1.0
