#!/usr/bin/env bash
#
# Photograph the framed app at the three widths #131 needs, into `docs/design/framed-<date>/`.
#
#   scripts/capture-frame-widths.sh <out-dir>
#
# **Three, not the usual two.** 1280x800 is the width the design pass judges at and 560x860 is the
# app's own default window — the pair every other run uses, because the map holds *one responsive
# design* and two widths is what makes that claim checkable. The third, 880x800, existed because #131
# introduced the app's second arrangement change: the editor drew two columns at or above
# `frame::TWO_COLUMN_MIN_WIDTH` and fell back below it, and a threshold with no capture either side
# of it is a claim.
#
# **#163 deleted the threshold**, so 880 no longer sits either side of anything — the editor is side
# by side at every width the desktop has. It is kept because a third width still costs one run and
# still catches what a pair cannot: with the switch gone what varies is a *gradient*, and a middle
# sample is how you see a gradient at all.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"
out="${1:?usage: capture-frame-widths.sh <out-dir>}"

for size in "1280 800" "880 800" "560 860"; do
  read -r width height <<<"$size"
  dir="$out/${width}x${height}"
  mkdir -p "$dir"
  echo "=== ${width}x${height} ==="
  CAIRN_SHOTS="$dir" "$here/capture-desktop.sh" "$here/storyboards/baseline.txt" \
    "$width" "$height" 6 2>&1 | grep -E "session: (shot|page frame)"
done

echo "capture-frame-widths: done -> $out"
