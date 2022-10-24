#!/bin/bash
 
# A script to compile the rust target compatible with
# https://github.com/cli/gh-extension-precompile

cargo build --release && mkdir dist && cp target/release/gh-difftool dist/gh-difftool_"$1"_linux-amd64

