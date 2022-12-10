#!/bin/bash
 
# A script to compile the rust target compatible with
# https://github.com/cli/gh-extension-precompile

case "$OSTYPE" in
  linux-gnu) os="linux" ;;
  darwin) os="darwin" ;;
  *) echo "Unsupported OS: $OSTYPE."; exit 1 ;;
esac

case $(uname -m) in
  x86_64) arch="amd64" ;;
  arm) arch="arm64" ;;
  *) echo "Unsupported architecture $(uname -m)."; exit 1 ;;
esac

target="dist/gh-difftool_$1_$os-$arch"
echo "Success: $target"
#cargo build --release && mkdir dist && cp target/release/gh-difftool dist/gh-difftool_"$1"_linux-amd64

