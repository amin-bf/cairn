#!/usr/bin/env bash
#
# Count the soft-keyboard show/hide requests a gesture costs, on the handset (ADR-0026 §6).
#
# This is the measurement the vendored `egui-winit` patch is justified by, and the one ADR-0026 §6
# requires again whenever `scripts/verify-vendor.sh` is unhappy. It is a script rather than a
# procedure because the number only means something next to the number it replaced:
#
#   | gesture                                      | hides | shows |
#   |----------------------------------------------|-------|-------|
#   | tap the already-focused field ×3, unpatched   |   6   |  17   |
#   | tap the already-focused field ×3, patched     |   0   |   0   |
#   | one scroll gesture, show-only attempt         |   0   |  72   |
#   | one scroll gesture, patched                   |   0   |   0   |
#
# The counts come from Android's own `ImeTracker`, which records every request with an originating
# package — so this counts what the platform was asked for, not what the application believes it
# asked for. `AGENTS.md` client-stack rule 9: the emulator cannot answer this, the Pixel 8 Pro can.
#
# Usage:
#   scripts/measure-ime-requests.sh tap  <x> <y> [count]   # taps, default 3
#   scripts/measure-ime-requests.sh drag <x1> <y1> <x2> <y2>
#
# Find the coordinates from a screenshot: `adb exec-out screencap -p > /tmp/screen.png`.

set -euo pipefail

PACKAGE="${PACKAGE:-dev.cairn.app}"

if ! adb get-state >/dev/null 2>&1; then
  echo "no handset attached — the emulator cannot answer this (client-stack rule 9)" >&2
  exit 1
fi

# The dump holds a rolling history keyed by an increasing entry id, so the id in hand before the
# gesture is what separates this run from every previous one.
latest_entry_id() {
  adb shell dumpsys input_method 2>/dev/null \
    | sed -n 's/^ *#\([0-9]\+\) TYPE_.*/\1/p' \
    | sort -n | tail -1
}

before="$(latest_entry_id)"
before="${before:-0}"

case "${1:-}" in
  tap)
    x="$2"; y="$3"; count="${4:-3}"
    echo "tapping ($x, $y) ×$count"
    for _ in $(seq "$count"); do
      adb shell input tap "$x" "$y"
      sleep 0.7
    done
    ;;
  drag)
    echo "dragging ($2, $3) → ($4, $5)"
    adb shell input swipe "$2" "$3" "$4" "$5" 400
    ;;
  *)
    echo "usage: $0 tap <x> <y> [count] | drag <x1> <y1> <x2> <y2>" >&2
    exit 2
    ;;
esac

# The keyboard animates, and a request is recorded when it is made rather than when it settles.
sleep 2

adb shell dumpsys input_method 2>/dev/null \
  | awk -v pkg="$PACKAGE" -v since="$before" '
      /^ *#[0-9]+ TYPE_/ {
        id = $1; sub(/^#/, "", id)
        current = (id + 0 > since + 0 && index($0, pkg) > 0)
        if (current) {
          if ($0 ~ /TYPE_HIDE/) hides++
          if ($0 ~ /TYPE_SHOW/) shows++
          print "  " $0
        }
        next
      }
      END {
        printf "\n%s: %d hide requests, %d show requests\n", pkg, hides + 0, shows + 0
      }'
