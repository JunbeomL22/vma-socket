#!/bin/bash
# run.sh - Helper script to build and run VMA-enabled applications

# First, build without LD_PRELOAD
cargo build --example $1

# Then run with LD_PRELOAD
LD_PRELOAD=/usr/lib64/libvma.so.9.8.51 ./target/debug/examples/$1 "${@:2}"