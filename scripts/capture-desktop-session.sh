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
#   <anything else> passed to xdotool verbatim, e.g. `mousemove 640 131 click 1`
#   # <comment>     ignored
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

while IFS= read -r line || [ -n "$line" ]; do
  [ -z "$line" ] && continue
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
      eval "${line#sh }" 2>&1 | sed 's/^/session:   /'
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
