#!/usr/bin/env bash
#
# Runs *inside* the nested compositor started by `capture-desktop.sh`. Not useful on its own.
#
# Reads a storyboard line by line and executes it against the running app:
#
#   shot <name>     take a screenshot into $CAIRN_SHOTS/<name>.png
#   sleep <n>       wait n seconds
#   restart         kill the app and start it again, on the same collection
#   sh <command>    run a shell command (used for `xdotool type`, which needs its own quoting)
#   fixture <name>  read by capture-desktop.sh before this script runs; ignored here
#   <anything else> passed to xdotool verbatim, e.g. `mousemove %CX% 131 click 1`
#   # <comment>     ignored
#
# `%CX%` and `%CY%` expand to the centre of the output. Almost every control the app draws is either
# full-width or in the nav row, so a storyboard written with `%CX%` runs unchanged at any width —
# whereas a literal `640` silently misses at 560 and shoots the *previous* screen under the new
# screen's name, which is worse than failing.
set -uo pipefail

echo "session: DISPLAY=${DISPLAY:-unset} WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-unset}"

# XWayland is started lazily, so the first X connection can lose a race with it. Wait for the
# display rather than sleeping a guessed amount and hoping.
for _ in $(seq 1 40); do
  xdotool getdisplaygeometry >/dev/null 2>&1 && break
  sleep 0.25
done
echo "session: display geometry $(xdotool getdisplaygeometry 2>&1)"

app=

start_app() {
  # Only the *app* drops to X11. `spectacle` keeps WAYLAND_DISPLAY, because the image comes from
  # KWin's ScreenShot2 interface and an X11 spectacle silently produces no file at all.
  env -u WAYLAND_DISPLAY WINIT_UNIX_BACKEND=x11 "$CAIRN_BIN" &
  app=$!
  sleep "${CAIRN_SETTLE:-6}"
  if ! kill -0 "$app" 2>/dev/null; then
    echo "session: app exited before it could be driven" >&2
    exit 1
  fi
  echo "session: window $(xdotool search --name Cairn 2>&1 | tr '\n' ' ')"
}

stop_app() {
  [ -n "$app" ] || return 0
  kill "$app" 2>/dev/null
  wait "$app" 2>/dev/null
  app=
}

start_app

cx=$(( ${CAIRN_WIDTH:-1280} / 2 ))
cy=$(( ${CAIRN_HEIGHT:-800} / 2 ))

# `%LX%` — the left edge of the page frame's column, and `%LX+n%` for a control `n` px inside it.
#
# Before the frame existed, content started at the window edge and a literal `86` for the *Notes*
# button was correct at every width. It no longer is: the column is centred, so its left edge is 320
# at 1280 and 28 at 560, and a storyboard written with literals silently clicks empty page at one
# width and the wrong control at the other — the failure `%CX%` was introduced to kill, arriving
# from the other side.
#
# The two numbers are `cairn_app::frame`'s and are duplicated here on purpose rather than read from
# the binary: a capture harness that imports the app cannot photograph a *broken* app, which is most
# of what it is for. They are overridable so a run against a build that moved them stays honest.
margin=${CAIRN_PAGE_MARGIN:-28}
measure=${CAIRN_MEASURE:-640}
inner=$(( ${CAIRN_WIDTH:-1280} - margin * 2 ))
[ "$inner" -gt "$measure" ] && inner=$measure
lx=$(( (${CAIRN_WIDTH:-1280} - inner) / 2 ))
echo "session: page frame — margin $margin, measure $measure, column left edge $lx"

# `%EX%` — the left edge of the **editor's** frame, and `%EX+n%` for a control `n` px inside it.
#
# The editor is the one screen that does not use the page measure. `frame::cap_for` gives it 1120
# once the window can hold two panes, so its column starts at x=80 at 1280 where every other screen
# starts at 320, and below the threshold it falls back to the measure and the two edges coincide.
# `%LX+n%` is therefore right for the nav row and wrong for anything inside the editor, and `%CX%`
# — 640 — reaches a field at 560 and lands fourteen pixels past its right edge at 1280.
#
# That gap is why `persian.txt` had been aiming at empty page since #131 and why `notes-persian.txt`
# was pinned to one width instead: #122's silent miss arriving from a fifth side, and the first one
# caused by a **frame** rather than by a coordinate. A pin is not a fix — it leaves the storyboard
# correct at exactly one window and silently wrong at every other, which is the property the tokens
# exist to remove — so #163 spends the token rather than the caveat.
two_column_min=${CAIRN_TWO_COLUMN_MIN_WIDTH:-900}
two_column_measure=${CAIRN_TWO_COLUMN_MEASURE:-1120}
editor_cap=$measure
[ "${CAIRN_WIDTH:-1280}" -ge "$two_column_min" ] && editor_cap=$two_column_measure
editor_inner=$(( ${CAIRN_WIDTH:-1280} - margin * 2 ))
[ "$editor_inner" -gt "$editor_cap" ] && editor_inner=$editor_cap
ex=$(( (${CAIRN_WIDTH:-1280} - editor_inner) / 2 ))
echo "session: editor frame — cap $editor_cap, column left edge $ex"

while IFS= read -r line || [ -n "$line" ]; do
  [ -z "$line" ] && continue
  line="${line//%CX%/$cx}"
  line="${line//%CY%/$cy}"
  # **Rebuilt from the capture groups, never `${line/${BASH_REMATCH[0]}/…}`.** That form looks
  # obvious and does not work: the replacement *pattern* is a glob, and an unquoted `+` in it is a
  # quantifier rather than a literal, so `%LX+39%` never matches itself and the loop spins forever
  # with the token still in place. Measured on bash 5.3: the same substring replaces fine when the
  # pattern is written as a quoted literal and not at all when it arrives through a variable.
  while [[ "$line" =~ (.*)%LX\+([0-9]+)%(.*) ]]; do
    pre="${BASH_REMATCH[1]}"
    off="${BASH_REMATCH[2]}"
    post="${BASH_REMATCH[3]}"
    line="$pre$(( lx + off ))$post"
  done
  line="${line//%LX%/$lx}"
  # `%EX+n%` and `%EX%` — the editor's frame, expanded the same way and for the same reason. Written
  # as its own loop rather than folded into the one above because the two edges differ only above the
  # two-column threshold, so a single shared expansion would test identical at 560 and diverge
  # silently at 1280 — which is the failure, not a saving.
  while [[ "$line" =~ (.*)%EX\+([0-9]+)%(.*) ]]; do
    pre="${BASH_REMATCH[1]}"
    off="${BASH_REMATCH[2]}"
    post="${BASH_REMATCH[3]}"
    line="$pre$(( ex + off ))$post"
  done
  line="${line//%EX%/$ex}"
  # `%BY-n%` — `n` px **above the bottom of the output**, the vertical twin of `%LX+n%`.
  #
  # Since ADR-0035 the last control cluster on a screen is anchored to a reach line measured up from
  # the bottom of the page, not down from the content above it. So a grade button's y is a function
  # of the window *height*, and a literal measured at 800 lands in empty page at 860 — the same
  # silent miss `%CX%` and `%LX+n%` exist to kill, arriving from the third axis. Rebuilt from the
  # capture groups for the reason spelled out above: an unquoted `-` is harmless in a glob but the
  # `+` next door was not, and writing the two forms differently is how one of them rots.
  while [[ "$line" =~ (.*)%BY-([0-9]+)%(.*) ]]; do
    pre="${BASH_REMATCH[1]}"
    off="${BASH_REMATCH[2]}"
    post="${BASH_REMATCH[3]}"
    line="$pre$(( ${CAIRN_HEIGHT:-800} - off ))$post"
  done
  case "$line" in
    \#*) continue ;;
    # Read by `capture-desktop.sh` before this script exists — the pre-made collection has to be in
    # place before the app opens it, and by now the app holds the database. Skipped rather than
    # passed to xdotool, which would otherwise report an unknown command and carry on.
    fixture\ *) continue ;;
    shot\ *)
      name="${line#shot }"
      sleep 0.8
      spectacle -b -n -f -o "$CAIRN_SHOTS/$name.png" -d 300
      echo "session: shot $name -> $?"
      ;;
    sleep\ *)
      # shellcheck disable=SC2086
      sleep ${line#sleep }
      ;;
    restart)
      stop_app
      start_app
      ;;
    sh\ *)
      echo "session: sh ${line#sh }"
      # **Not piped.** A pipeline runs its left-hand side in a subshell, so `sh export FOO=bar`
      # would set the variable somewhere that dies at the end of the line — and the next `restart`
      # would relaunch the app with the *old* environment. That failure is silent in the worst way
      # the harness has: every subsequent `shot` succeeds and photographs the previous screen under
      # the new screen's name, which is exactly the class of fault `%CX%` was introduced to kill.
      # The `sed` prefix on the output is not worth paying for it.
      eval "${line#sh }" 2>&1
      ;;
    *)
      echo "session: xdotool $line"
      # shellcheck disable=SC2086
      xdotool $line
      sleep 0.4
      ;;
  esac
done < "$CAIRN_STORYBOARD"

stop_app
echo "session: done"
