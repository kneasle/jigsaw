# Build the Rust library to WASM
cd proj
cargo build --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir . ~/.build/rust/wasm32-unknown-unknown/debug/proj.wasm
wasm-gc proj_bg.wasm

# Copy all files into the out/ directory
cd ..
mv proj/proj.js out/
mv proj/proj_bg.wasm out/

cd out/
ln -sf ../www/* .
cd ..
