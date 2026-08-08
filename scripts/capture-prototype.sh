#!/usr/bin/env bash
#
# Photograph every variant of the Review prototype at both judging widths, for #124.
#
#   scripts/capture-prototype.sh [variants...]     # default: a b c d
#
# Images land in `target/capture/proto/<width>x<height>/<variant>-<n>-<screen>.png`, which is
# ignored by git, exactly like `capture-desktop.sh`'s output. A set worth keeping is copied out
# deliberately.
#
# Two widths, always, because the map holds **one responsive design** and the pair is what makes
# that claim checkable: 1280x800 is the width the design pass judges at, 560x860 is the app's own
# default window. A variant that only works at one of them has not answered the question.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"

variants=("$@")
[ ${#variants[@]} -eq 0 ] && variants=(a b c d)

export CAIRN_BIN="$root/target/debug/review-prototype"
if [ ! -x "$CAIRN_BIN" ]; then
  echo "capture-prototype: no binary — run 'cargo build -p cairn-desktop --bin review-prototype'" >&2
  exit 1
fi

for size in "1280 800" "560 860"; do
  read -r width height <<<"$size"
  for variant in "${variants[@]}"; do
    out="$root/target/capture/proto/${width}x${height}"
    mkdir -p "$out"
    echo "=== variant $variant at ${width}x${height} ==="
    # The first screen is whatever the app launches with; the storyboard exports and restarts for
    # the other three.
    CAIRN_SHOTS="$out" \
    PROTO_VARIANT="$variant" \
    PROTO_SCREEN="picker" \
      "$here/capture-desktop.sh" "$here/storyboards/review-prototype.txt" "$width" "$height" 4

    # `shot <name>` writes a fixed name, so the variant is stamped on afterwards rather than
    # threaded through the storyboard — one storyboard serves all four.
    for f in "$out"/[1-9]-*.png; do
      [ -e "$f" ] || continue
      mv "$f" "$out/${variant}-$(basename "$f")"
    done
  done
done

echo "capture-prototype: done -> $root/target/capture/proto"
