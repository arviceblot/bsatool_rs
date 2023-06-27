# bsatool_rs

[![crates.io cli](https://img.shields.io/crates/v/bsatool_rs.svg)](https://crates.io/crates/bsatool_rs)
[![crates.io lib](https://img.shields.io/crates/v/bsatoollib.svg)](https://crates.io/crates/bsatoollib)

A rust implementation of the openmw bsatool.

> Note: Currently this project only supports BSA files compatible with TES III: Morrowind.

## Install

The easiest way right now is to install with [cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html):

```shell
cargo install bsatool_rs
```

### Library

There is also a library option for manipulating BSAs from other rust code available on crates.io and can be added to a project with:

```shell
cargo add bsatoollib
```

#### Example Usage

```rust
use bsatoollib as bsa;

// open an existing BSA file
let bsa = bsa::BSAFile::new("SomeFile.BSA").unwrap();

// print all file names in the BSA
for file in bsa.get_list().iter() {
    println!(file.name);
}
```

## Command line options

```shell
> bsatool_rs --help
A tool for working with BSA files

Usage: bsatool_rs <FILE> <COMMAND>

Commands:
  list         List the files presents in the given BSA file
  extract      Extract a file from the given BSA file
  extract-all  Extract all files from the given BSA file
  create       Create a new BSA file with given files for archiving
  help         Print this message or the help of the given subcommand(s)

Arguments:
  <FILE>  BSA file to use

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Licensing

Since bsatool_rs is derivative work of OpenMW's bsatool, it is released under the same license as the openmw code.
