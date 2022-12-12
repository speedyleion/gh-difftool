#!/bin/bash
 
# A script to compile the rust target compatible with
# https://github.com/cli/gh-extension-precompile

case "$OSTYPE" in
  linux-gnu) os="linux" ;;
  darwin*) os="darwin" ;;
  *) echo "Unsupported OS: $OSTYPE."; exit 1 ;;
esac

case $(uname -m) in
  x86_64) arch="amd64" ;;
  arm64) arch="arm64" ;;
  *) echo "Unsupported architecture $(uname -m)."; exit 1 ;;
esac


if [ $os == "darwin" ];
then
  # Cross compile for m1, not sure supporting x86_64 is worth it
  rustup target add aarch64-apple-darwin
  cargo build --target aarch64-apple-darwin --release && mkdir dist && find target -name gh-difftool
  # cargo build --target aarch64-apple-darwin --release && mkdir dist && cp target/release/gh-difftool dist/gh-difftool_"$1"_"$os"-arm64
else
  cargo build --release && mkdir dist && cp target/release/gh-difftool dist/gh-difftool_"$1"_"$os"-"$arch"
fi

