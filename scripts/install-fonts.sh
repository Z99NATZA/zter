#!/bin/sh
# Install missing default font families for the current user.
set -eu

: "${HOME:?HOME must be set}"
data_directory=${XDG_DATA_HOME:-"$HOME/.local/share"}
case "$data_directory" in
    /) printf 'zter: refusing to install fonts under /\n' >&2; exit 1 ;;
    /*) ;;
    *) printf 'zter: font data directory must be absolute\n' >&2; exit 1 ;;
esac

require_command() {
    command -v "$1" >/dev/null 2>&1 || {
        printf 'zter: %s is required to install/check fonts\n' "$1" >&2
        exit 1
    }
}
require_command fc-list
has_family() {
    fc-list -f '%{family}\n' | tr ',' '\n' | grep -Fxq "$1"
}

need_jetbrains=false
need_thai=false
has_family 'JetBrainsMono Nerd Font' || need_jetbrains=true
has_family 'Noto Sans Thai UI' || need_thai=true
if ! "$need_jetbrains" && ! "$need_thai"; then
    printf 'zter: default fonts already installed\n'
    exit 0
fi

for tool in curl fc-scan fc-cache; do require_command "$tool"; done
staging=$(mktemp -d)
trap 'rm -rf -- "$staging"' EXIT
trap 'exit 1' HUP INT TERM
font_directory="$data_directory/fonts/zter"

fetch() {
    curl --fail --location --silent --show-error --retry 2 \
        --connect-timeout 15 --max-time 180 --proto '=https' --proto-redir '=https' \
        --output "$2" "$1"
}
fetch_font() {
    fetch "$1" "$staging/$2"
    if ! fc-scan -f '%{family}\n' "$staging/$2" | tr ',' '\n' | grep -Fxq "$3"; then
        printf 'zter: downloaded font does not contain family %s\n' "$3" >&2
        exit 1
    fi
}

# Pin upstream versions so repeated installs use the same font files.
nerd_base='https://raw.githubusercontent.com/ryanoasis/nerd-fonts/v3.4.0/patched-fonts/JetBrainsMono/Ligatures'
noto_base='https://raw.githubusercontent.com/google/fonts/47d54ca1ebb14fd864695f1d39dc734f39017c33/ofl/notosansthaiui'
if "$need_jetbrains"; then
    printf 'zter: downloading JetBrainsMono Nerd Font\n'
    for style in Regular Bold Italic BoldItalic; do
        name="JetBrainsMonoNerdFont-$style.ttf"
        fetch_font "$nerd_base/$style/$name" "$name" 'JetBrainsMono Nerd Font'
    done
    fetch "$nerd_base/Regular/OFL.txt" "$staging/JetBrainsMono-OFL.txt"
fi
if "$need_thai"; then
    printf 'zter: downloading Noto Sans Thai UI\n'
    fetch_font "$noto_base/NotoSansThaiUI%5Bwdth%2Cwght%5D.ttf" \
        'NotoSansThaiUI.ttf' 'Noto Sans Thai UI'
    fetch "$noto_base/OFL.txt" "$staging/NotoSansThaiUI-OFL.txt"
fi

# Validate all downloads before copying any files into the user's font directory.
mkdir -p "$font_directory"
for file in "$staging"/*; do install -m644 "$file" "$font_directory/"; done
fc-cache -f "$font_directory"
for family in 'JetBrainsMono Nerd Font' 'Noto Sans Thai UI'; do
    if ! has_family "$family"; then
        printf 'zter: fontconfig cannot find installed family %s\n' "$family" >&2
        exit 1
    fi
done
printf 'zter: default fonts installed in %s\n' "$font_directory"
