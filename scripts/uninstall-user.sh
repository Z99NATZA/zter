#!/bin/sh

set -eu

: "${HOME:?HOME must be set}"

data_directory=${XDG_DATA_HOME:-"$HOME/.local/share"}
binary_directory=${ZTER_BIN_DIR:-"$HOME/.local/bin"}
application_id=io.github.znnn.zter
icon_theme_directory="$data_directory/icons/hicolor"

case "$data_directory:$binary_directory" in
    /*:/*) ;;
    *)
        printf 'zter: install directories must be absolute paths\n' >&2
        exit 1
        ;;
esac

if [ "$data_directory" = / ] || [ "$binary_directory" = / ]; then
    printf 'zter: refusing to remove files directly under /\n' >&2
    exit 1
fi

rm -f -- \
    "$binary_directory/zter" \
    "$data_directory/applications/$application_id.desktop" \
    "$data_directory/icons/hicolor/scalable/apps/$application_id.svg"

if command -v update-desktop-database >/dev/null 2>&1 && \
    [ -d "$data_directory/applications" ]; then
    update-desktop-database "$data_directory/applications"
fi

if [ -d "$icon_theme_directory" ]; then
    if command -v gtk4-update-icon-cache >/dev/null 2>&1; then
        gtk4-update-icon-cache -f -t "$icon_theme_directory"
    elif command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$icon_theme_directory"
    fi
fi

printf 'zter: removed the user-local binary, launcher, and icon\n'
