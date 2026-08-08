#!/usr/bin/env bash
#
# Photograph every frame of the page-frame prototype at both judging widths, for #131.
#
#   scripts/capture-frame.sh [frames...]     # default: f0 f1 f2 f3
#
# Images land in `target/capture/frame/<width>x<height>/<frame>-<n>-<screen>.png`, which git
# ignores exactly as it ignores `capture-desktop.sh`'s output. A set worth keeping is copied out
# deliberately.
#
# Two widths, always, because the map holds **one responsive design** and the pair is what makes
# that claim checkable: 1280x800 is the width the design pass judges at, 560x860 is the app's own
# default window. A frame that only works at one of them has not answered the question — and for
# *this* ticket the pair is the whole point, since three of the four frames differ only in what
# they do with width the narrow window does not have.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"

frames=("$@")
[ ${#frames[@]} -eq 0 ] && frames=(f0 f1 f2 f3)

export CAIRN_BIN="$root/target/debug/frame-prototype"
if [ ! -x "$CAIRN_BIN" ]; then
  echo "capture-frame: no binary — run 'cargo build -p cairn-desktop --bin frame-prototype'" >&2
  exit 1
fi

for size in "1280 800" "560 860"; do
  read -r width height <<<"$size"
  for frame in "${frames[@]}"; do
    out="$root/target/capture/frame/${width}x${height}"
    mkdir -p "$out"
    echo "=== frame $frame at ${width}x${height} ==="
    CAIRN_SHOTS="$out" \
    PROTO_FRAME="$frame" \
    PROTO_SCREEN="review" \
      "$here/capture-desktop.sh" "$here/storyboards/frame-prototype.txt" "$width" "$height" 4

    # `shot <name>` writes a fixed name, so the frame is stamped on afterwards rather than threaded
    # through the storyboard — one storyboard serves all four.
    for f in "$out"/[1-9]-*.png; do
      [ -e "$f" ] || continue
      mv "$f" "$out/${frame}-$(basename "$f")"
    done
  done
done

echo "capture-frame: done -> $root/target/capture/frame"
