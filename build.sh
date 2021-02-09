#!bash

# Move to the project root
ROOT=$(git rev-parse --show-toplevel)
cd $ROOT

# Build the Rust library to WASM
cd proj
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir . ~/.build/rust/wasm32-unknown-unknown/debug/proj.wasm
wasm-gc proj_bg.wasm
cd $ROOT

# Copy all files into the out/ directory
mv proj/proj.js out/
mv proj/proj_bg.wasm out/

# Make sure that the files in the www/ directory are served along with the compiled wasm
cd out/
ln -sf ../www/* .
cd $ROOT
