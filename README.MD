# CARGO-TRIM

**Project status & info:**

| Travis Build Status | Dependency Status | License | Crates Version |
| :-----------------: | :---------------: | :-----: | :------------: |
| [![Travis Build Status][build_badge]][build_link] [![FOSSA Status](https://app.fossa.io/api/projects/git%2Bgithub.com%2Fiamsauravsharma%2Fcargo-trim.svg?type=shield)](https://app.fossa.io/projects/git%2Bgithub.com%2Fiamsauravsharma%2Fcargo-trim?ref=badge_shield)
| [![Dependency status][deps_badge]][deps_link] | [![License: MIT][license_badge]][license_link] | [![Crate][cratesio_badge]][cratesio_link] |

**Rust & OS support:**

| Stable |  Beta | Nightly | Linux |  OSX  | Windows |
| :----: | :---: | :-----: | :---: | :---: | :-----: |
| [![Stable support][stable_supported_badge]][git_link] | [![Beta support][beta_supported_badge]][git_link] | [![Nightly support][nightly_supported_badge]][git_link] | [![Linux support][linux_supported_badge]][git_link] | [![OSX support][osx_supported_badge]][git_link] | [![Windows support][windows_supported_badge]][git_link] |

Binary application for cleaning __\$CARGO_HOME/registry__  & __\$CARGO_HOME/git__ folder support orphan cleaning and old cleaning
Default \$CARGO_HOME is set as \$HOME/.cargo

### Install
Initially you should have rustc and cargo installed for a detail information of how to install rust-lang visit https://rust-lang.org

For installation of cargo-trim:-
You can install from crates.io registry
```
    cargo install cargo-trim
```
__OR__

For bleeding edge latest development version:
```
    cargo install --git https://github.com/iamsauravsharma/cargo-trim
```

By default cargo-trim print out colored output if you want to use a non colored output you can enable out no-color feature
```
    cargo install cargo-trim --features no-color
```

### Command output
All cargo-trim can be run using `cargo trim <command>`
`cargo trim --help` will produce out following output which list out all of available output for cargo-trim
```
cargo-trim 0.4.2
Saurav Sharma <appdroiddeveloper@gmail.com>
Binary application to cleanup $CARGO_HOME cache

USAGE:
    cargo trim [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
    -a, --all             Clean up all .cargo/registry & .cargo/git follow config file data
    -c, --clear           Clear config file data
    -n, --dry-run         Run command in dry run mode to see what would be removed
    -f, --force           Force clear cache without reading conf file
    -h, --help            Prints help information
    -l, --light           Light cleanup repos by removing git checkout and registry source but stores git db and
                          registry archive for future compilation without internet requirement
    -o, --old-clean       Clean old cache crates
    -x, --orphan-clean    Clean orphan cache crates i.e all crates which are not present in lock file generated till now
                          use cargo trim -u to guarantee your all project generate lock file
    -q, --query           Return size of different .cargo/cache folders
    -u, --update          Generate and Update Cargo.lock file present inside config directory folder path
    -V, --version         Prints version information

OPTIONS:
    -e, --exclude <crate>...              add listed crates to default conf file exclude list [use $TRIM_EXCLUDE
                                          environment variable for creating exclude list without editing conf file]
    -g, --gc <git compress>               Git compress to reduce size of .cargo [possible values: all, index, git, git-
                                          checkout, git-db]
    -i, --include <crate>...              add listed crates to default conf file include list [use $TRIM_INCLUDE
                                          environment variable for creating include list without editing conf file]
    -r, --remove <crate>...               Remove provided crates from registry or git
    -s, --set-directory <Directory>...    Set directory of Rust project [use TRIM_DIRECTORY environment variable for
                                          creating directory list without editing conf file]
    -t, --top <number>                    Show certain number of top crates which have highest size
    -w, --wipe <folder>                   Wipe folder [possible values: git, checkouts, db, registry, cache, index, src]

SUBCOMMANDS:
    clear          Clear current working directory from cargo cache config [alias: "clr"]
    completions    Generate tab-completions scripts for shell [alias: "comp"]
    config         Query config file data [alias: "c"]
    git            Perform operation only to git related cache file [alias: "g"]
    help           Prints this message or the help of the given subcommand(s)
    init           Initialize current working directory as cargo trim directory [alias: "in"]
    list           List out crates [alias: "l"]
    registry       Perform operation only to registry related cache file [alias: "reg"]
    remove         Remove values from config file [alias: "rm"]
```

### Config file location
cargo-trim store its config file to config directory of OS and name config file as `cargo_trim_config.json`.
In file cargo-trim stores out information about directory of rust projects and exclude and include crates list for cargo trim.

[git_link]: https://github.com/iamsauravsharma/cargo-trim

[build_badge]: https://img.shields.io/travis/com/iamsauravsharma/cargo-trim.svg?logo=travis
[build_link]: https://travis-ci.com/iamsauravsharma/cargo-trim

[deps_badge]: https://deps.rs/repo/github/iamsauravsharma/cargo-trim/status.svg
[deps_link]: https://deps.rs/repo/github/iamsauravsharma/cargo-trim

[license_badge]: https://img.shields.io/github/license/iamsauravsharma/cargo-trim.svg
[license_link]: LICENSE

[cratesio_badge]: https://img.shields.io/crates/v/cargo-trim.svg
[cratesio_link]: https://crates.io/crates/cargo-trim

[stable_supported_badge]: https://img.shields.io/badge/stable-supported-brightgreen.svg?logo=rust
[beta_supported_badge]: https://img.shields.io/badge/beta-supported-brightgreen.svg?logo=rust
[nightly_supported_badge]: https://img.shields.io/badge/nightly-supported-brightgreen.svg?logo=rust
[stable_not_supported_badge]: https://img.shields.io/badge/stable-not%20supported-red.svg?logo=rust
[beta_not_supported_badge]: https://img.shields.io/badge/beta-not%20supported-red.svg?logo=rust
[nightly_not_supported_badge]: https://img.shields.io/badge/nightly-not%20supported-red.svg?logo=rust

[linux_supported_badge]: https://img.shields.io/badge/linux-supported-brightgreen.svg?logo=linux
[osx_supported_badge]: https://img.shields.io/badge/osx-supported-brightgreen.svg?logo=apple
[windows_supported_badge]: https://img.shields.io/badge/windows-supported-brightgreen.svg?logo=windows
[linux_not_supported_badge]: https://img.shields.io/badge/linux-not%20supported-red.svg?logo=linux
[osx_not_supported_badge]: https://img.shields.io/badge/osx-not%20supported-red.svg?logo=apple
[nightly_not_supported_badge]: https://img.shields.io/badge/windows-not%20supported-red.svg?logo=windows

## License
[![FOSSA Status](https://app.fossa.io/api/projects/git%2Bgithub.com%2Fiamsauravsharma%2Fcargo-trim.svg?type=large)](https://app.fossa.io/projects/git%2Bgithub.com%2Fiamsauravsharma%2Fcargo-trim?ref=badge_large)