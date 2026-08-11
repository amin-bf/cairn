#!/usr/bin/env bash
#
# Photograph one combination of the card prototype, for #133.
#
#   scripts/capture-card.sh <name> [width] [height]
#
# with the combination in the environment:
#
#   PROTO_PAGE=shipped|panel  PROTO_CARD=today|well|raised|outline|two
#   PROTO_BADGE=corner|below  PROTO_CONTENT=word|sentence|long|fa-word|fa-sentence|markdown
#   PROTO_HEIGHT=grow|fixed   PROTO_SCREEN=question|revealed|live
#
# The image lands at `target/capture/card/<width>x<height>/<name>.png`, which git ignores exactly
# as it ignores `capture-desktop.sh`'s output; a set worth keeping is copied out deliberately.
#
# **The combination rides the environment rather than the storyboard**, which is what lets one
# one-line storyboard serve every one of them: `capture-desktop.sh` exports the caller's environment
# into the nested session, so the binary is launched already showing the screen being photographed
# and there is nothing to drive with `xdotool`. The cost is one process per image, which at four
# seconds of settle is cheaper than the alternative of clicking through a variant switch.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"

name="${1:?usage: capture-card.sh <name> [width] [height]}"
width="${2:-1280}"
height="${3:-800}"

export CAIRN_BIN="$root/target/debug/card-prototype"
if [ ! -x "$CAIRN_BIN" ]; then
  echo "capture-card: no binary — run 'cargo build -p cairn-desktop --bin card-prototype'" >&2
  exit 1
fi

out="$root/target/capture/card/${width}x${height}"
mkdir -p "$out"

CAIRN_SHOTS="$out" "$here/capture-desktop.sh" \
  "$here/storyboards/card-prototype.txt" "$width" "$height" 4 >/dev/null 2>&1

# `shot card` writes a fixed name, so the combination is stamped on afterwards — one storyboard,
# many images, and the name is the caller's to choose.
if [ -e "$out/card.png" ]; then
  mv "$out/card.png" "$out/$name.png"
  echo "capture-card: $out/$name.png"
else
  echo "capture-card: NO IMAGE for $name — the run produced nothing" >&2
  exit 1
fi
