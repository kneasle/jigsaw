#!/usr/bin/env python3

"""Build script for Jigsaw."""

import argparse
import subprocess
import os
import shutil
import json

RUST_CRATE_NAME = "jigsaw"
RUST_CRATE_PATH = ""


def exit_with_message(msg):
    print(msg)
    exit(1)


def get_wasm_location(cargo_build_stdout):
    for l in cargo_build_stdout.split("\n"):
        # Skip blank lines
        if l == "":
            continue
        # Parse the compiler message into JSON
        message = json.loads(l)
        # Skip if this build message is either not a release artefact or refers to the wrong crate
        if not (
            message["reason"] == "compiler-artifact"
            and message["target"]["name"] == RUST_CRATE_NAME
        ):
            continue

        # Now that we know this is the right message, we extract the location of the '*.wasm' file and
        # move it into the out directory
        output_files = [
            file_path for file_path in message["filenames"] if file_path.endswith(".wasm")
        ]
        # Sanity check that there is exactly one wasm file
        if len(output_files) == 0:
            exit_with_message("No wasm files emitted!")
        if len(output_files) > 1:
            exit_with_message(f"Multiple wasm files emitted: {output_files}")
        return output_files[0]
    exit_with_message("No compiler messages found for the wasm files")


# ===== ARG PARSING =====

parser = argparse.ArgumentParser(description="Build script for Jigsaw")
parser.add_argument(
    "-r", "--release", action="store_true", help="Switches the build between debug and release."
)
parser.add_argument(
    "-o",
    "--out-dir",
    type=str,
    default="out",
    help="Custom location for the output directory, relative to the project root.  Defaults to `out`.",
)
args = parser.parse_args()
is_release = args.release
out_dir_arg = args.out_dir

# TODO: Run dependency check, and install things if necessary

# ===== GET DIRECTORIES =====

# Use 'git' to find the location of the project root (so that this can be run from any subdirectory
# of the project)
this_files_dir = os.path.split(__file__)[0]
root_dir = subprocess.run(
    ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, cwd=this_files_dir
).stdout.strip()
web_dir = os.path.join(root_dir, "www")
rust_dir = os.path.join(root_dir, RUST_CRATE_PATH)
# Combine the root dir and the value --out-dir to get the out directory path
out_dir = out_dir_arg if os.path.isabs(out_dir_arg) else os.path.join(root_dir, out_dir_arg)

# Make sure the out directory exists before building anything
os.makedirs(out_dir, exist_ok=True)

# ===== BUILD WEB COMPONENTS (JS, HTML, CSS, etc) =====

for file_name in os.listdir(web_dir):
    source_path = os.path.join(web_dir, file_name)
    out_path = os.path.join(out_dir, file_name)
    # Remove the output file if it already exists
    if os.path.exists(out_path):
        os.remove(out_path)
    # Move the source file to the out dir
    if is_release:
        # In release mode, we actually copy the files
        shutil.copy2(source_path, out_path)
    else:
        # In debug mode, just symlink the files so that we don't have to re-build every time the web
        # stuff is changed
        # TODO: Generate a relative path to stop the symlinks being horrible to look at
        os.symlink(source_path, out_path)

# ===== BUILD RUST CODE =====

cargo_args = ["cargo", "build", "--target", "wasm32-unknown-unknown"] + (
    ["--release"] if is_release else []
)

# Run the cargo build process once to actually perform the build
cargo_build_proc = subprocess.run(
    cargo_args,
    cwd=rust_dir,
    text=True,
)
# Check that the compilation was successful
if cargo_build_proc.returncode != 0:
    exit_with_message("Rust build failed.  Stopping build.")

# Now that cargo ran normally, we run it again to get path of the wasm file
# TODO: Once cargo's `--out-dir` hits stable, all of this will become unnecessary
build_location_proc = subprocess.run(
    cargo_args + ["--message-format", "json"], cwd=rust_dir, capture_output=True, text=True
)
wasm_path = get_wasm_location(build_location_proc.stdout)

# Run wasm-bindgen to generate the Rust/JS interaction code
subprocess.run(
    [
        "wasm-bindgen",
        wasm_path,
        "--target",
        "no-modules",
        "--no-typescript",
        "--out-dir",
        out_dir,
    ]
)
