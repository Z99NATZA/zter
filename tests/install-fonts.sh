#!/bin/sh
# Integration test: downloads real upstream fonts into an isolated fontconfig setup.
set -eu
project_directory=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
fixture=$(mktemp -d)
trap 'rm -rf -- "$fixture"' EXIT
trap 'exit 1' HUP INT TERM
export XDG_DATA_HOME="$fixture/data"
export FONTCONFIG_FILE="$fixture/fonts.conf"
cat > "$FONTCONFIG_FILE" <<XML
<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "urn:fontconfig:fonts.dtd">
<fontconfig><dir>$XDG_DATA_HOME/fonts</dir><cachedir>$fixture/cache</cachedir></fontconfig>
XML
mkdir -p "$fixture/bin"
cat > "$fixture/bin/curl" <<'SH'
#!/bin/sh
printf 'unexpected download\n' >&2
exit 99
SH
chmod +x "$fixture/bin/curl"

# Download failures must not install partial files.
if PATH="$fixture/bin:$PATH" sh "$project_directory/scripts/install-fonts.sh"; then
    printf 'expected a download failure\n' >&2
    exit 1
fi
test ! -d "$XDG_DATA_HOME/fonts/zter"

sh "$project_directory/scripts/install-fonts.sh"
test "$(find "$XDG_DATA_HOME/fonts/zter" -name '*.ttf' | wc -l)" -eq 5

# Installed families must work offline, without even invoking curl.
PATH="$fixture/bin:$PATH" sh "$project_directory/scripts/install-fonts.sh"

# A missing fallback is repaired without reinstalling the existing main font.
main_font="$XDG_DATA_HOME/fonts/zter/JetBrainsMonoNerdFont-Regular.ttf"
original_time=$(stat -c %Y "$main_font")
rm "$XDG_DATA_HOME/fonts/zter/NotoSansThaiUI.ttf"
fc-cache -f "$XDG_DATA_HOME/fonts/zter"
sh "$project_directory/scripts/install-fonts.sh"
test "$(stat -c %Y "$main_font")" = "$original_time"
test -f "$XDG_DATA_HOME/fonts/zter/NotoSansThaiUI.ttf"
printf 'Font installation integration checks passed\n'
