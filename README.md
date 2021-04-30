# Jigsaw

A visual, incremental composing tool for change ringing.  Since this program is currently my 3rd
year project, there is an orphan branch `report` which contains the LaTeX files of the project
report.

## Overview

The goal of this project is to design and build a visual application which will _aid_ composers of
pieces of [Change Ringing](https://en.wikipedia.org/wiki/Change_ringing) in their work, whilst
requiring as little change to their existing workflow as possible.  The main advantages provided
over pen/paper is that this program gives instant and correct feedback on loads of useful
information such as music, length and falseness (and other less important statistics like
'all-the-work-ness' which are still tedious to calculate manually).  This is very much still a
W.I.P. prototype, but here's a screenshot:

![Project screenshot](https://raw.githubusercontent.com/kneasle/jigsaw/report/screenshot-2021-03-16.png)

## Build Instruction

This project is mostly written in Rust, but runs a web GUI using JS (communicating with Rust through
wasm).  Therefore, the project can simply run as a static website with no additional dependencies -
indeed, when it becomes remotely usable I will simply add it to my website for people to use at
their leisure.  If you _do_ want to build it from source, then you will need to
[install Rust](https://www.rust-lang.org/tools/install) and then install the necessary tools with
the following commands:
```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

To build, run `build.py` from anywhere on your filesystem.  By default, it will place the build
files in `<repo root>/out/` but that can be overridden if necessary (run `build.py --help` for more
info).
