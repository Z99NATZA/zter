#!/bin/sh

set -eu

script_directory=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_directory=$(CDPATH= cd -- "$script_directory/.." && pwd)

: "${HOME:?HOME must be set}"

data_directory=${XDG_DATA_HOME:-"$HOME/.local/share"}
binary_directory=${ZTER_BIN_DIR:-"$HOME/.local/bin"}
application_id=io.github.z99natza.zter.Devel
legacy_application_id=io.github.znnn.zter.Devel
icon_theme_directory="$data_directory/icons/hicolor"

case "$data_directory:$binary_directory" in
    /*:/*) ;;
    *)
        printf 'zter: development install directories must be absolute\n' >&2
        exit 1
        ;;
esac

if [ "$data_directory" = / ] || [ "$binary_directory" = / ]; then
    printf 'zter: refusing to install development files directly under /\n' >&2
    exit 1
fi

install -d "$binary_directory"
ln -sfn -- "$project_directory/scripts/run-dev.sh" "$binary_directory/zter-devel"
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

printf 'zter: development desktop metadata installed for the current user\n'
printf '  runner: %s\n' "$binary_directory/zter-devel"
printf '  launcher metadata: %s\n' "$data_directory/applications/$application_id.desktop"
printf '  icon: %s\n' "$data_directory/icons/hicolor/scalable/apps/$application_id.svg"
