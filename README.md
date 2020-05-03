# bsatool_rs

[![Build Status](https://travis-ci.org/arviceblot/bsatool_rs.svg?branch=master)](https://travis-ci.org/arviceblot/bsatool_rs)

A rust implementation of the openmw bsatool.

## Command line options

    bsatool_rs
    Inspect and extract files from Bethesda BSA archives

    USAGE:
        bsatool_rs <INPUT> [SUBCOMMAND]

    FLAGS:
        -h, --help       Prints help information
        -V, --version    Prints version information

    ARGS:
        <INPUT>    The input archive file to use

    SUBCOMMANDS:
        create        Create an archive file
        extract       Extract a file from the input archive
        extractall    Extract all files from the input archive
        help          Prints this message or the help of the given subcommand(s)
        list          List the files presents in the input archive

## Licensing

Since bsatool_rs is derivative work of OpenMW's bsatool, it is released under the same license as the openmw code.
