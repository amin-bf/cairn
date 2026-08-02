#!/usr/bin/env bash
#
# Verify `vendor/egui-winit` is the published crate plus exactly one change (ADR-0026 §3, §6).
#
# Two checks, and they answer different questions:
#
#   1. **Recursive diff against a pristine copy.** Says *nothing else moved* — the vendored tree is
#      third-party source we carry, and the delta is invisible by inspection, so this is what makes
#      it recoverable rather than merely tidy.
#   2. **The block-shape check.** Says the guard is still on *the block it was justified against*.
#      A guard mechanically re-applied to a block that no longer means the same thing looks
#      perfectly healthy in a diff, which is the silent failure the shape check exists to catch.
#
# So a bump that passes both is a routine bump. A bump that fails either is a **re-judgement**, and
# it needs the handset measurement in `AGENTS.md` client-stack rule 9 — not a re-application.
#
# Usage:  scripts/verify-vendor.sh
#
# The pristine copy comes from the local registry cache when it is there, and from the registry
# otherwise. Both are the same bytes: the `.crate` tarball cargo itself verified the checksum of.

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
vendored="$repo/vendor/egui-winit"

# The version is read from the vendored copy rather than pinned here, so this script cannot drift
# from the tree it checks.
version="$(sed -n 's/^version = "\(.*\)"$/\1/p' "$vendored/Cargo.toml" | head -1)"
crate="egui-winit-$version.crate"
echo "verifying vendor/egui-winit against published egui-winit $version"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

cached="$(find "${CARGO_HOME:-$HOME/.cargo}/registry/cache" -name "$crate" -print -quit 2>/dev/null || true)"
if [ -n "$cached" ]; then
  echo "  pristine source: $cached"
  tar xzf "$cached" -C "$work"
else
  url="https://static.crates.io/crates/egui-winit/$crate"
  echo "  pristine source: $url"
  curl -fsSL "$url" -o "$work/$crate"
  tar xzf "$work/$crate" -C "$work"
fi
pristine="$work/egui-winit-$version"

# ---- 1. Recursive diff -------------------------------------------------------------------------
#
# `.cargo-ok` is cargo's own extraction marker, written into the registry's *unpacked* directory and
# absent from the tarball. Nothing else is excluded: every file the crate publishes is compared.
echo
echo "== recursive diff =="
status=0
diff -ru --exclude=.cargo-ok "$pristine" "$vendored" > "$work/delta.diff" || status=$?
if [ "$status" -gt 1 ]; then
  echo "FAIL: could not diff the two trees"
  exit 1
fi

changed_files="$(grep -c '^diff -ru' "$work/delta.diff" || true)"
if [ "$changed_files" != "1" ] || ! grep -q '^diff -ru.*src/lib\.rs$' "$work/delta.diff"; then
  echo "FAIL: expected exactly one changed file (src/lib.rs), found $changed_files:"
  grep '^diff -ru' "$work/delta.diff" || echo "  (none)"
  exit 1
fi

# One hunk, and its only removed/added *code* line is the `#[cfg]` attribute. Comment lines are
# allowed — the marker at the patch site is what tells a reader landing there that the block is
# guarded and why — so the test is on what the compiler sees.
added_code="$(grep '^+' "$work/delta.diff" | grep -v '^+++' | sed 's/^+[[:space:]]*//' \
  | grep -v '^//' | grep -v '^$' || true)"
removed_code="$(grep '^-' "$work/delta.diff" | grep -v '^---' | sed 's/^-[[:space:]]*//' \
  | grep -v '^//' | grep -v '^$' || true)"
if [ "$added_code" != '#[cfg(not(target_os = "android"))]' ] || [ -n "$removed_code" ]; then
  echo "FAIL: the delta is not the one guard attribute."
  echo "  added code:   ${added_code:-(none)}"
  echo "  removed code: ${removed_code:-(none)}"
  exit 1
fi
echo "OK: src/lib.rs differs by one added attribute and nothing else"

# ---- 2. The block shape ------------------------------------------------------------------------
#
# ADR-0026 §6 writes the shape out rather than a line number: the guard must sit on the block that
# hides and re-shows the IME to interrupt a composition. If a release restructures it — renames the
# condition, moves the calls, splits the branch — the instruction is **re-judge, not re-apply**.
echo
echo "== block shape =="
guarded="$(awk '
  /^[[:space:]]*#\[cfg\(not\(target_os = "android"\)\)\]$/ { armed = 1; next }
  armed && /^[[:space:]]*if !is_toggling_ime && ime\.should_interrupt_composition \{$/ { found = 1 }
  { if ($0 !~ /^[[:space:]]*\/\//) armed = 0 }
  END { print found + 0 }
' "$vendored/src/lib.rs")"
if [ "$guarded" != "1" ]; then
  echo "FAIL: the guard is not on \`if !is_toggling_ime && ime.should_interrupt_composition\`."
  echo "      Re-judge the block against ADR-0026 §2 and §6 — do not re-apply the attribute."
  exit 1
fi

for line in 'window.set_ime_allowed(false);' 'window.set_ime_allowed(true);'; do
  if ! grep -qF "$line" "$vendored/src/lib.rs"; then
    echo "FAIL: the guarded block no longer contains \`$line\` — re-judge, do not re-apply."
    exit 1
  fi
done
echo "OK: the guard sits on the hide-then-show interrupt block"

echo
echo "verbatim plus exactly one change. Routine bump."
