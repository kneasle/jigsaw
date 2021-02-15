# 3rd Year Project

This repository contains source code & report for my 3rd year project.

## Build Instruction

This project is mostly written in Rust, but runs a web GUI using JS (communicating with Rust through
wasm).  Therefore, the project can simply run as a static website with no additional dependencies -
indeed, when it becomes remotely usable I will simply add it to my website for people to use at
their leisure.  If you _do_ want to build it from source, then you will need to
[install Rust](https://www.rust-lang.org/tools/install) and then install the necessary tools with
the following commands:
```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli wasm-gc
```

To build, run `sh build.sh` (it doesn't matter where in the code you are, running `sh ../build.sh`
from `proj/` will work just fine).
