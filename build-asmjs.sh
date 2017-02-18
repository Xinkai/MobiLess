#!/bin/sh

set -e

cargo build --target asmjs-unknown-emscripten --verbose --release --lib
emcc -O3 -g0 --memory-init-file 0 -s ALLOW_MEMORY_GROWTH=1 -s EXPORTED_FUNCTIONS="['_process_mobi_file']" target/asmjs-unknown-emscripten/release/libmobiless.a -o mobiless.js
