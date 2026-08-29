#!/bin/sh

set -eu

script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_directory=$(CDPATH= cd -- "$script_directory/.." && pwd)

: "${HOME:?HOME must be set}"

data_directory=${XDG_DATA_HOME:-"$HOME/.local/share"}
binary_directory=${ZTER_BIN_DIR:-"$HOME/.local/bin"}
application_id=io.github.z99natza.zter
legacy_application_id=io.github.znnn.zter
icon_theme_directory="$data_directory/icons/hicolor"
installed_binary="$binary_directory/zter"

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

install -Dm755 "$binary_source" "$installed_binary"
install -Dm644 \
    "$project_directory/data/$application_id.desktop" \
    "$data_directory/applications/$application_id.desktop"
install -Dm644 \
    "$project_directory/data/icons/hicolor/scalable/apps/$application_id.svg" \
    "$data_directory/icons/hicolor/scalable/apps/$application_id.svg"
rm -f -- \
    "$data_directory/applications/$legacy_application_id.desktop" \
    "$data_directory/icons/hicolor/scalable/apps/$legacy_application_id.svg"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$data_directory/applications"
fi

if command -v gtk4-update-icon-cache >/dev/null 2>&1; then
    gtk4-update-icon-cache -f -t "$icon_theme_directory"
elif command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$icon_theme_directory"
fi

printf 'zter: installed for the current user\n'
printf '  binary: %s\n' "$installed_binary"
printf '  launcher: %s\n' "$data_directory/applications/$application_id.desktop"
printf '  icon: %s\n' "$data_directory/icons/hicolor/scalable/apps/$application_id.svg"

stale_processes=
for process_executable in /proc/[0-9]*/exe; do
    [ -L "$process_executable" ] || continue
    executable=$(readlink "$process_executable" 2>/dev/null || true)
    [ "$executable" = "$installed_binary (deleted)" ] || continue
    process_id=${process_executable#/proc/}
    process_id=${process_id%/exe}
    stale_processes="${stale_processes}${stale_processes:+ }$process_id"
done

if [ -n "$stale_processes" ]; then
    printf 'zter: warning: running installed process(es) %s still use the previous binary\n' \
        "$stale_processes" >&2
    printf '  close all installed zter windows, then reopen the launcher to use this build\n' >&2
fi
