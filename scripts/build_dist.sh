#!/bin/bash
 
# A script to compile the rust target compatible with
# https://github.com/cli/gh-extension-precompile

ext=""
case "$OSTYPE" in
  linux-gnu) os="linux" ;;
  darwin*) os="darwin" ;;
  msys) os="windows" ext=".exe" ;;
  *) echo "Unsupported OS: $OSTYPE."; exit 1 ;;
esac

case $(uname -m) in
  x86_64) arch="amd64" ;;
  arm64) arch="arm64" ;;
  *) echo "Unsupported architecture $(uname -m)."; exit 1 ;;
esac

cargo build --release && mkdir dist && cp target/release/gh-difftool"$ext" dist/gh-difftool_"$1"_"$os"-"$arch""$ext"

