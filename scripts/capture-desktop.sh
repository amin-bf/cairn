#!/usr/bin/env bash
#
# Capture the desktop app's screens as PNGs, for design work that is judged by looking.
#
#   scripts/capture-desktop.sh <storyboard> [width] [height] [settle-seconds]
#
# **Nothing appears on the operator's screen and nothing touches their collection.** The app runs
# inside a *nested* compositor rendering to a virtual framebuffer (`kwin_wayland --virtual`), on a
# throwaway XDG profile, so a capture run can be fired mid-conversation without taking the display
# and without a single row landing in `~/.local/share/cairn`. That is the property the whole design
# pass rests on: a screen that costs the operator their focus to look at gets looked at less.
#
# **The output is exactly `width`×`height` of application pixels** — no title bar, no shadow, no
# desktop behind it. A window rule forced into the scratch config fullscreens whatever opens, so the
# client area *is* the output, and two runs at the same size produce images that can be diffed. The
# earlier alternative, shooting the window and cropping its drop shadow away, is not reproducible:
# the shadow's alpha falls off gradually, so `-trim` lands on a different rectangle run to run.
#
# **The app is an X11 client on the nested XWayland; only the app.** `spectacle` keeps the Wayland
# socket, because the image is produced by KWin's ScreenShot2 interface and nothing else in the
# session can produce it. The app drops to X11 purely so `xdotool` can drive it — Wayland has no
# equivalent an unprivileged script can reach, and driving it is what makes any screen past the
# first one reachable at all.
#
# See `docs/environment/desktop-capture.md` for what a storyboard may contain and how the coordinates
# are chosen.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/.." && pwd)"

storyboard="${1:?usage: capture-desktop.sh <storyboard> [width] [height] [settle-seconds]}"
width="${2:-1280}"
height="${3:-800}"

export CAIRN_SETTLE="${4:-6}"
export CAIRN_WIDTH="$width"
export CAIRN_HEIGHT="$height"
# Overridable so the same harness can photograph something *other* than the shipped app — the
# throwaway prototypes a design ticket builds to be reacted to. The harness's value is that a
# capture costs the operator no window and no focus, and that is worth exactly as much to a
# prototype as to the app.
export CAIRN_BIN="${CAIRN_BIN:-$root/target/debug/cairn}"
export CAIRN_STORYBOARD="$(cd "$(dirname "$storyboard")" && pwd)/$(basename "$storyboard")"
export CAIRN_SHOTS="${CAIRN_SHOTS:-$root/target/capture/${width}x${height}}"
fixture_bin="${CAIRN_FIXTURE_BIN:-$root/target/debug/cairn-fixture}"

if [ ! -x "$CAIRN_BIN" ]; then
  echo "capture: no binary at $CAIRN_BIN — run 'cargo build -p cairn-desktop' first" >&2
  exit 1
fi

# **The storyboard names its own fixture** (issue #153), on a `fixture <name>` line read here rather
# than by the session script — the collection has to exist before the app opens it, and by the time
# the session is running the app holds the database open.
#
# Naming it *in the storyboard* rather than passing it on the command line is the whole point. A
# storyboard that needs a pre-made collection and is run without one produces a full set of
# perfectly valid captures of the shipping seed, under the fixture's names — the silent-miss failure
# `%CX%` exists to kill, arriving from a third side. Tying the two together makes that unreachable.
# `CAIRN_FIXTURE` still overrides, for photographing one storyboard against another's state.
fixture="${CAIRN_FIXTURE:-$(sed -n 's/^[[:space:]]*fixture[[:space:]]\+\([^[:space:]]*\).*/\1/p' "$CAIRN_STORYBOARD" | head -1)}"

mkdir -p "$CAIRN_SHOTS"

# The scratch profile. Every XDG base the app or the nested session might write to is redirected
# here and wiped first, so a run always starts from a *first launch* — which is what makes the seed
# in `CairnApp::open_store` the fixture, and what stops one run's grades from colouring the next.
profile="$(mktemp -d -t cairn-capture-XXXXXX)"
trap 'rm -rf "$profile"' EXIT
export XDG_DATA_HOME="$profile/data"
export XDG_STATE_HOME="$profile/state"
export XDG_CONFIG_HOME="$profile/config"
export XDG_CACHE_HOME="$profile/cache"
mkdir -p "$XDG_DATA_HOME" "$XDG_STATE_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME"

# The fixture goes in *before* the compositor starts, into the scratch profile above — so the app's
# first launch opens a collection that is already in the wanted state and the seed never fires
# (`crates/app/src/fixtures.rs`). `cairn-fixture` verifies what it installed and exits non-zero if it
# did not land, and the run is abandoned here rather than producing a plausible picture of the wrong
# screen.
if [ -n "$fixture" ]; then
  if [ ! -x "$fixture_bin" ]; then
    echo "capture: '$CAIRN_STORYBOARD' asks for fixture '$fixture' but there is no binary at $fixture_bin — run 'cargo build -p cairn-desktop'" >&2
    exit 1
  fi
  if ! "$fixture_bin" "$fixture"; then
    echo "capture: fixture '$fixture' did not install — abandoning the run" >&2
    exit 1
  fi
fi

# Force every window in the nested session fullscreen and undecorated. `wmclassmatch=0` is
# *unimportant*, i.e. match all windows — safe precisely because this config is only ever read by
# the nested compositor, which has exactly one client.
cat > "$XDG_CONFIG_HOME/kwinrulesrc" <<'RULES'
[$Version]
update_info=

[1]
Description=capture harness: every window fills the virtual output
fullscreen=true
fullscreenrule=2
noborder=true
noborderrule=2
wmclassmatch=0

[General]
count=1
rules=1
RULES

# KWin asks the user before letting an X11 client inject input ("xdotool is asking to control input
# devices"), and an unattended run has nobody to answer it — the prompt simply sits there and every
# shot after the first one is a picture of the prompt. `XwaylandEisNoPrompt` is the switch that
# question is asked from. It is set **in the scratch config**, so the operator's own session keeps
# its prompt: this grant covers one throwaway compositor with one client in it.
cat > "$XDG_CONFIG_HOME/kwinrc" <<'KWINRC'
[Xwayland]
XwaylandEisNoPrompt=true
KWINRC

# Unset so the nested compositor cannot attach to the real session and put a window on the screen.
unset WAYLAND_DISPLAY DISPLAY

timeout "${CAIRN_TIMEOUT:-240}" dbus-run-session -- \
  kwin_wayland_wrapper --virtual --width "$width" --height "$height" \
    --no-lockscreen --no-global-shortcuts --no-kactivities \
    --socket cairn-capture \
    --xwayland \
    --exit-with-session="$here/capture-desktop-session.sh"
status=$?

echo "capture: ${width}x${height} -> $CAIRN_SHOTS (kwin exit $status)"
exit "$status"
