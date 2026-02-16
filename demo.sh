#!/bin/sh


set -ex

cargo build --package base
cargo run
