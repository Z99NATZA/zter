#!/bin/sh

set -eu

script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_directory=$(CDPATH= cd -- "$script_directory/.." && pwd)

: "${HOME:?HOME must be set}"

data_directory=${XDG_DATA_HOME:-"$HOME/.local/share"}
binary_directory=${ZTER_BIN_DIR:-"$HOME/.local/bin"}
application_id=io.github.znnn.zter

case "$data_directory:$binary_directory" in
    /*:/*) ;;
    *)
        printf 'zter: install directories must be absolute paths\n' >&2
        exit 1
        ;;
esac

if [ "$data_directory" = / ] || [ "$binary_directory" = / ]; then
    printf 'zter: refusing to install directly under /\n' >&2
    exit 1
fi

if [ -n "${ZTER_INSTALL_BINARY:-}" ]; then
    binary_source=$ZTER_INSTALL_BINARY
else
    cargo build --release --manifest-path "$project_directory/Cargo.toml"
    binary_source="$project_directory/target/release/zter"
fi

if [ ! -f "$binary_source" ]; then
    printf 'zter: binary not found: %s\n' "$binary_source" >&2
    exit 1
fi

install -Dm755 "$binary_source" "$binary_directory/zter"
install -Dm644 \
    "$project_directory/data/$application_id.desktop" \
    "$data_directory/applications/$application_id.desktop"
install -Dm644 \
    "$project_directory/data/icons/hicolor/scalable/apps/$application_id.svg" \
    "$data_directory/icons/hicolor/scalable/apps/$application_id.svg"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$data_directory/applications"
fi

printf 'zter: installed for the current user\n'
printf '  binary: %s\n' "$binary_directory/zter"
printf '  launcher: %s\n' "$data_directory/applications/$application_id.desktop"
printf '  icon: %s\n' "$data_directory/icons/hicolor/scalable/apps/$application_id.svg"
