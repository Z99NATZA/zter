#!/bin/sh

set -eu

launcher_path=$(readlink -f -- "$0")
script_directory=$(CDPATH= cd -- "$(dirname -- "$launcher_path")" && pwd)
project_directory=$(CDPATH= cd -- "$script_directory/.." && pwd)

exec cargo run --manifest-path "$project_directory/Cargo.toml" -- "$@"
