#!/bin/bash
set -e

wget -qO cross.tar.gz https://github.com/cross-rs/cross/releases/download/v"$CROSS_VERSION"/cross-x86_64-unknown-linux-gnu.tar.gz
tar -C "$HOME"/.cargo/bin -xzf cross.tar.gz
