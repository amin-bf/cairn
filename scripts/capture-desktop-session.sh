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
  case "$line" in
    \#*) continue ;;
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
