#!/usr/bin/env bash
#
# Photograph one combination of the controls prototype, for #134.
#
#   scripts/capture-controls.sh <name> [width] [height]
#
# with the combination in the environment:
#
#   PROTO_SCREEN=revealed|picker|caughtup|pointer|checkpoint|live
#   PROTO_GRADES=stacked|row|row4|rowplus   PROTO_WEIGHT=solid|faint|quiet
#   PROTO_PREVIEW=same|small|none           PROTO_ENTRANCE=counts|primary|primarylink|plain
#   PROTO_EMPTY=sentence|centred|bare       PROTO_CONTROL=<px>
#   PROTO_CHECKPOINT=replaces|over
#
# The image lands at `target/capture/controls/<width>x<height>/<name>.png`, which git ignores
# exactly as it ignores `capture-desktop.sh`'s output; a set worth keeping is copied out
# deliberately.
#
# One process per image, as `capture-card.sh` established: at four seconds of settle that is cheaper
# than clicking through a variant switch, and it means no storyboard has to know which axis moved.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"

name="${1:?usage: capture-controls.sh <name> [width] [height]}"
width="${2:-1280}"
height="${3:-800}"

export CAIRN_BIN="$root/target/debug/controls-prototype"
if [ ! -x "$CAIRN_BIN" ]; then
  echo "capture-controls: no binary — run 'cargo build -p cairn-desktop --bin controls-prototype'" >&2
  exit 1
fi

out="$root/target/capture/controls/${width}x${height}"
mkdir -p "$out"

CAIRN_SHOTS="$out" "$here/capture-desktop.sh" \
  "$here/storyboards/controls-prototype.txt" "$width" "$height" 4 >/dev/null 2>&1

# `shot controls` writes a fixed name, so the combination is stamped on afterwards — one storyboard,
# many images, and the name is the caller's to choose.
if [ -e "$out/controls.png" ]; then
  mv "$out/controls.png" "$out/$name.png"
  echo "capture-controls: $out/$name.png"
else
  echo "capture-controls: NO IMAGE for $name — the run produced nothing" >&2
  exit 1
fi
