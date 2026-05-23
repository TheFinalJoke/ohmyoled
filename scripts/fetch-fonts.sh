#!/usr/bin/env bash
# Populate ./fonts/ with the five fonts the renderers and tests need.
#
# The fonts are bundled in a GitHub Release on this repo so the working
# copy and `/usr/share/fonts/` install can both point at one canonical
# tarball — fonts are no longer tracked in git.
#
# Required fonts:
#   04B_03B_.TTF        body font (stock, sport, golf, f1, weather)
#   04b24.otf           big font (sport)
#   BMmini.TTF          weather temp readout
#   weathericons.ttf    weather glyph icons
#   4x6.bdf             time renderer
#
# Usage:
#   scripts/fetch-fonts.sh           # download + extract into ./fonts/
#   scripts/fetch-fonts.sh --force   # re-download even if fonts exist
#
# The script is idempotent: it exits 0 immediately if every expected
# font is already present, unless --force is passed.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FONTS_DIR="$REPO_ROOT/fonts"
TARBALL_URL="${OHMYOLED_FONTS_URL:-https://github.com/TheFinalJoke/ohmyoled/releases/download/fonts-v1/fonts.tar.gz}"
TARBALL_SHA256="${OHMYOLED_FONTS_SHA256:-5f123ded1322ff26d506f524c54e8df82eca9e4eef7eed9acf4a8123ab71b4e1}"

REQUIRED=(
    "04B_03B_.TTF"
    "04b24.otf"
    "BMmini.TTF"
    "weathericons.ttf"
    "4x6.bdf"
)

force=0
for arg in "$@"; do
    case "$arg" in
        --force|-f) force=1 ;;
        --help|-h)
            sed -n '2,/^$/p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "unknown arg: $arg" >&2
            exit 2
            ;;
    esac
done

mkdir -p "$FONTS_DIR"

if [ "$force" -eq 0 ]; then
    missing=0
    for f in "${REQUIRED[@]}"; do
        [ -f "$FONTS_DIR/$f" ] || missing=1
    done
    if [ "$missing" -eq 0 ]; then
        echo "fonts: all required files present in $FONTS_DIR"
        exit 0
    fi
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
echo "fonts: fetching $TARBALL_URL"
curl --fail --location --silent --show-error -o "$tmp/fonts.tar.gz" "$TARBALL_URL"

if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$tmp/fonts.tar.gz" | cut -d' ' -f1)"
    if [ "$actual" != "$TARBALL_SHA256" ]; then
        echo "fonts: sha256 mismatch (expected $TARBALL_SHA256, got $actual)" >&2
        exit 1
    fi
fi

tar -xzf "$tmp/fonts.tar.gz" -C "$FONTS_DIR"

for f in "${REQUIRED[@]}"; do
    if [ ! -f "$FONTS_DIR/$f" ]; then
        echo "fonts: extracted tarball is missing '$f'" >&2
        exit 1
    fi
done

echo "fonts: extracted ${#REQUIRED[@]} files into $FONTS_DIR"
