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
W.I.P. prototype and many features are missing, but the latest commit to the `master` branch is always
made public [here](https://kneasle.github.io/jigsaw/firehose/).  Also here's a screenshot:

![Project screenshot](https://raw.githubusercontent.com/kneasle/jigsaw/report/screenshot-2021-04-27.png)

## Build Instruction

This project is mostly written in Rust, but runs a web GUI using JS (with the Rust code compiled to
WebAssembly).  To build it from source, you will need to
[install Rust](https://www.rust-lang.org/tools/install) and then install the necessary tools with
the following commands:
```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli
```

This will possibly take a few minutes to compile `wasm-bindgen-cli`.

Once the utilities are installed, you can build Jigsaw by running `build.py` (in the project root) from
anywhere on your filesystem.  By default, it will place the build
files in `<repo root>/out/` but that can be overridden if necessary (run `build.py --help` for more
info).  This generates a folder which can be served by a webserver, for example:

```bash
cd <out directory>
python3 -m http.server
```

This will print the port of the HTTP server, but Jigsaw will usually be found at `https://127.0.0.1:8000`.
