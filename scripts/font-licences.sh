#!/bin/sh
# Copies the licence texts of every embedded font into a directory, so they ship inside the app
# package (the SIL Open Font License requires it): the bundled Inter and JetBrains Mono, craft-fonts
# and the PhotoComfy font pack. The iOS and Android build scripts call it for their app bundle.
#
#   scripts/font-licences.sh <destination directory>
#
# CRAFT_FONTS_DIR and PHOTOCOMFY_FONTS_DIR name the optional inputs. A pack file is named after
# its family directory (packaging/env.sh ships the same names on the desktop), e.g.
# OFL-noto-sans-sc.txt or LICENSE-lxgw-neo-xihei.md; the bundled ones keep their names.

set -eu

dest="$1"
repo="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$dest"

for lic in "$repo"/assets/fonts/OFL-*.txt; do
    cp "$lic" "$dest/"
done
for pack in "${CRAFT_FONTS_DIR:-}" "${PHOTOCOMFY_FONTS_DIR:-}"; do
    [ -n "$pack" ] || continue
    for lic in "$pack"/fonts/*/OFL.txt "$pack"/fonts/*/LICENSE.txt "$pack"/fonts/*/LICENSE.md; do
        [ -f "$lic" ] || continue
        family="$(basename "$(dirname "$lic")")"
        name="$(basename "$lic")"
        cp "$lic" "$dest/${name%.*}-$family.${name##*.}"
    done
done
