#!/usr/bin/env python3
"""Build `crates/app/assets/CairnIcons-Regular.ttf` from the sources the product keeps.

    scripts/build-icon-face.py            # regenerate the face in place
    scripts/build-icon-face.py --check    # rebuild to a temporary file and diff against the shipped one

**The face is checked in; this script is why it is the shape it is.** ADR-0038 §1 routes icons
through the font stack, and a font is a binary — so the thing that can rot is the claim that a glyph
*is* the drawing it came from. This script makes that claim executable, and every source it reads
lives in this repository:

- **the mark** — `crates/app/res/drawable/ic_launcher_monochrome.xml`, which the Android build
  already ships, so the four stones in the face are the launcher's own;
- **`move`** and **`delete`** — `crates/app/res/icons/*.svg`, the note-list row's two controls
  (#162). `delete.svg` is the design project's file verbatim; `move.svg` is drawn here, because the
  design project's sixteen icons have no `move` — the set was authored before the screen that
  needed one.

Edit a source and the face follows; edit neither and `--check` says so.

**It is not part of any build.** It needs `fonttools` (built with 4.64.0) and runs when a source
changes, which is approximately never. Nothing in `cargo build`, `cargo test` or the capture harness
invokes it — a Rust workspace that needed a Python interpreter to compile would be a far worse trade
than a two-kilobyte asset with a recipe beside it.

    python3 -m venv /tmp/faceenv && /tmp/faceenv/bin/pip install 'fonttools==4.64.0'
    /tmp/faceenv/bin/python scripts/build-icon-face.py --check

# Two kinds of source, and why the SVGs are redrawn rather than exported

The drawable's paths are **filled** already. The design project's icons are a 24px grid of
`stroke-width: 1.5` **strokes** with `fill: none`, and a glyph has no strokes — so each one is
converted, which is what the design project means when it calls the SVGs *the drawing source, not
the delivery mechanism*.

The conversion is mechanical and rests on the property the mark already rests on. Every segment
becomes its own closed **stadium** — a rectangle of the stroke's width with a semicircular cap at
each end — and every one of them is wound the same way. Under non-zero winding, same-wound
overlapping contours merge, so the caps that overlap at a joint give a round join for free and the
path comes out as one filled shape. Wind two of them against each other and they *cancel* where they
overlap, punching a hole through a file that is otherwise perfectly valid — which is the failure the
mark's four overlapping stones were already exposed to.

# The metrics, and why they are these

**The ink is one cap height tall and sits on the baseline.** An icon is a glyph, so its size *is* a
font size (ADR-0038 §1) — and a glyph that filled the em would overshoot the line it is set in and
collide with the line above.

**A glyph in a set is scaled on its own drawing grid, not to its own ink.** The mark is measured to
a full cap height because it is one picture with nothing to agree with. The row icons are measured
against the design project's **24px grid**, so a 16-unit-tall arrow and a 16.5-unit-tall bin draw at
the sizes they were drawn at relative to each other — and, more importantly, the 1.5 stroke stays
1.5 across the set. An icon normalised to its own ink would draw a heavier line the smaller its
drawing happened to be.

**The advance width depends on whether the glyph stands alone.** ADR-0038 §1 as written gives the
mark *advance = ink width, left side bearing zero*, so a centred label centres the stones rather
than a box with unequal air in it. That is right for one picture standing on its own and it does not
survive a set: two icons of different ink widths get two different advances, so two icon-only
buttons get two different widths, and a right-aligned action column drawn from them comes out ragged
in exactly the way the words it replaced were. So a glyph in a set takes a **square advance of one
cap height** with its ink centred in it, and §1 gains that clause (#162).

**The glyph is placed by what it draws, not by the viewport.** The drawable's 108×108 box has padding
around the stones because a launcher icon is masked; that padding is Android's business, and carrying
it into the face would make every stated size a lie by whatever fraction of the viewport happens to
be empty. The SVGs' 24×24 box is the grid they are drawn on, which is a different thing and is why
it *is* read — see the metric note above.
"""

import argparse
import filecmp
import math
import re
import sys
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.recordingPen import RecordingPen
from fontTools.pens.reverseContourPen import ReverseContourPen
from fontTools.pens.ttGlyphPen import TTGlyphPen

ROOT = Path(__file__).resolve().parent.parent
DRAWABLE = ROOT / "crates/app/res/drawable/ic_launcher_monochrome.xml"
ICONS = ROOT / "crates/app/res/icons"
FACE = ROOT / "crates/app/assets/CairnIcons-Regular.ttf"

ANDROID = "{http://schemas.android.com/apk/res/android}"
NUMBER = re.compile(r"-?\d*\.?\d+")

UPM = 1000
CAP = 720

#: The design project's grid, and the stroke every one of its icons is drawn with.
GRID = 24.0
STROKE = 1.5

#: The code points, all private use — which is what makes the face safe to append **last** in every
#: family: nothing else can claim them, so the face shadows nothing and nothing shadows it. Kept in
#: step with `crates/app/src/fonts.rs`.
GLYPHS = [
    # name      code point  source                    advance
    ("mark", 0xE000, DRAWABLE, "ink"),
    ("move", 0xE001, ICONS / "move.svg", "square"),
    ("delete", 0xE002, ICONS / "delete.svg", "square"),
]


# --- reading the two kinds of source -----------------------------------------------------------


def drawable_contours(drawable):
    """The drawable's paths, as command lists. Absolute `M`, `C` and `Z` only — the subset it uses.

    **The mark keeps its curves.** This is deliberately not the polyline route the SVGs take: the
    mark is a shipped, judged glyph (ADR-0038 §3), so adding icons beside it must change it by
    exactly nothing, and flattening its cubics would quietly re-cut every stone.
    `the_face_still_carries_the_launchers_stones` in `fonts.rs` holds that."""
    root = ET.parse(drawable).getroot()
    for path in root.iter("path"):
        out = []
        for command, args in re.findall(r"([MCZmcz])([^MCZmcz]*)", path.get(ANDROID + "pathData")):
            numbers = [float(n) for n in NUMBER.findall(args)]
            if command == "M":
                out.append(("move", (numbers[0], numbers[1])))
            elif command == "C":
                for i in range(0, len(numbers), 6):
                    out.append(("curve", tuple(numbers[i : i + 6])))
            elif command == "Z":
                out.append(("close", ()))
            else:
                raise SystemExit(f"{drawable}: unsupported path command {command!r}")
        yield out


def replay_drawable(commands, pen, scale, ox, baseline):
    """Draw one of the drawable's contours, flipping y — the drawable is y-down and a font is y-up."""
    points = []
    for kind, n in commands:
        if kind == "move":
            points.append((n[0] * scale + ox, baseline - n[1] * scale))
            pen.moveTo(points[-1])
        elif kind == "curve":
            bezier = [(n[i] * scale + ox, baseline - n[i + 1] * scale) for i in (0, 2, 4)]
            points.append(bezier[2])
            pen.curveTo(*bezier)
        elif kind == "close":
            pen.closePath()
    return points


def polylines(data):
    """An SVG path's subpaths, as point lists. `M`, `L`, `H`, `V` and their relative forms — the
    subset the design project's icons use, all of them straight."""
    out, points, cursor = [], [], (0.0, 0.0)
    for command, args in re.findall(r"([MLHVmlhvZz])([^MLHVmlhvZz]*)", data):
        n = [float(v) for v in NUMBER.findall(args)]
        if command in "Mm":
            if len(points) > 1:
                out.append(points)
            cursor = (n[0], n[1]) if command == "M" else (cursor[0] + n[0], cursor[1] + n[1])
            points = [cursor]
            rest = n[2:]
            for i in range(0, len(rest), 2):
                cursor = (
                    (rest[i], rest[i + 1])
                    if command == "M"
                    else (cursor[0] + rest[i], cursor[1] + rest[i + 1])
                )
                points.append(cursor)
        elif command in "Ll":
            for i in range(0, len(n), 2):
                cursor = (n[i], n[i + 1]) if command == "L" else (cursor[0] + n[i], cursor[1] + n[i + 1])
                points.append(cursor)
        elif command in "Hh":
            for v in n:
                cursor = (v, cursor[1]) if command == "H" else (cursor[0] + v, cursor[1])
                points.append(cursor)
        elif command in "Vv":
            for v in n:
                cursor = (cursor[0], v) if command == "V" else (cursor[0], cursor[1] + v)
                points.append(cursor)
        else:
            raise SystemExit(f"unsupported path command {command!r}")
    if len(points) > 1:
        out.append(points)
    return out


def stadium(a, b, radius, steps=8):
    """One stroke segment as a closed contour: two parallel edges and a semicircular cap at each
    end, wound consistently so overlapping segments merge rather than cancel."""
    (x0, y0), (x1, y1) = a, b
    dx, dy = x1 - x0, y1 - y0
    length = math.hypot(dx, dy)
    if length == 0:
        return []
    base = math.atan2(dy / length, dx / length)
    points = []
    for i in range(steps + 1):  # the cap at `b`
        angle = base - math.pi / 2 + math.pi * i / steps
        points.append((x1 + radius * math.cos(angle), y1 + radius * math.sin(angle)))
    for i in range(steps + 1):  # the cap at `a`
        angle = base + math.pi / 2 + math.pi * i / steps
        points.append((x0 + radius * math.cos(angle), y0 + radius * math.sin(angle)))
    return points


def stroked_contours(path):
    """A stroked SVG icon as filled contours, still on its 24px grid and still y-down."""
    data = re.findall(r'\sd="([^"]+)"', path.read_text())
    if not data:
        raise SystemExit(f"{path}: no path data")
    for one in data:
        for line in polylines(one):
            for a, b in zip(line, line[1:]):
                contour = stadium(a, b, STROKE / 2)
                if contour:
                    yield contour


# --- placing them ------------------------------------------------------------------------------


def signed_area(points):
    return (
        sum(x0 * y1 - x1 * y0 for (x0, y0), (x1, y1) in zip(points, points[1:] + points[:1])) / 2
    )


def place(left, right, top, bottom, box, advance_mode):
    """The scale and offsets that put a source's ink where the metrics say it goes.

    See the module docstring: a glyph standing alone is centred by having no bearings at all; a
    glyph in a set is centred inside a square, so a column of them lines up."""
    scale = box / max(right - left, bottom - top)
    ink_w, ink_h = (right - left) * scale, (bottom - top) * scale
    if advance_mode == "ink":
        # ADR-0038 §1 as written: advance = ink width, **left side bearing zero**. Not centred —
        # centring the ink inside its own *rounded* advance moves it by half the rounding error,
        # which is a fifth of a unit on the mark and is still the mark moving.
        advance, ox = round(ink_w), -left * scale
    else:
        advance = round(CAP)
        ox = (advance - ink_w) / 2 - left * scale
    # y-up, sitting on the baseline. The mark's ink fills the cap height so this is exactly the
    # baseline; a shorter set glyph is centred on the same optical middle rather than dropped to it.
    baseline = bottom * scale - (CAP - ink_h) / 2
    return scale, ox, baseline, advance, ink_w, ink_h


def draw_drawable(paths, box, advance_mode):
    """The mark: cubics, converted to quadratics rather than flattened."""
    coordinates = [(n[i], n[i + 1]) for p in paths for _, n in p for i in range(0, len(n), 2)]
    left, right = min(x for x, _ in coordinates), max(x for x, _ in coordinates)
    top, bottom = min(y for _, y in coordinates), max(y for _, y in coordinates)
    scale, ox, baseline, advance, ink_w, ink_h = place(
        left, right, top, bottom, box, advance_mode
    )

    pen = TTGlyphPen(None)
    for path in paths:
        recording = RecordingPen()
        points = replay_drawable(path, recording, scale, ox, baseline)
        # The four stones overlap, and non-zero winding only merges them when they wind the same
        # way. Two stones drawn in opposite directions would *cancel* where they overlap — a hole
        # through the middle of the mark, from a file that is otherwise perfectly valid.
        outward = pen if signed_area(points) > 0 else ReverseContourPen(pen)
        # TrueType has no cubics. 0.3 units of an em at 1000/em is well under a pixel at any size a
        # screen draws this at.
        recording.replay(Cu2QuPen(outward, 0.3))
    return pen.glyph(), advance, ink_w, ink_h


def draw_stroked(contours, box, advance_mode):
    """A set glyph: stroke outlines, which are straight by construction."""
    xs = [x for c in contours for x, _ in c]
    ys = [y for c in contours for _, y in c]
    scale, ox, baseline, advance, ink_w, ink_h = place(
        min(xs), max(xs), min(ys), max(ys), box, advance_mode
    )

    pen = TTGlyphPen(None)
    for contour in contours:
        placed = [(x * scale + ox, baseline - y * scale) for x, y in contour]
        recording = RecordingPen()
        recording.moveTo(placed[0])
        for point in placed[1:]:
            recording.lineTo(point)
        recording.closePath()
        # Same winding rule as the mark's, for the same reason — here it is what turns overlapping
        # segment caps into a round join instead of a bite out of the corner.
        recording.replay(pen if signed_area(placed) > 0 else ReverseContourPen(pen))
    return pen.glyph(), advance, ink_w, ink_h


def build(out):
    glyphs = {".notdef": TTGlyphPen(None).glyph()}
    metrics = {".notdef": (round(CAP), 0)}
    order = [".notdef"]
    cmap = {}
    report = []

    for name, code_point, source, advance_mode in GLYPHS:
        if source == DRAWABLE:
            glyph, advance, ink_w, ink_h = draw_drawable(
                list(drawable_contours(source)), CAP, advance_mode
            )
        else:
            contours = list(stroked_contours(source))
            # Measured on the drawing grid, not on its own ink — see the module docstring.
            extent = max(
                max(x for c in contours for x, _ in c) - min(x for c in contours for x, _ in c),
                max(y for c in contours for _, y in c) - min(y for c in contours for _, y in c),
            )
            glyph, advance, ink_w, ink_h = draw_stroked(
                contours, CAP * extent / GRID, advance_mode
            )
        glyphs[name] = glyph
        metrics[name] = (advance, 0)
        order.append(name)
        cmap[code_point] = name
        report.append(
            f"{name:7s} U+{code_point:04X}  ink {ink_w:5.0f}×{ink_h:5.0f}  "
            f"advance {advance:4d} ({advance_mode})  {source.relative_to(ROOT)}"
        )

    font = FontBuilder(UPM, isTTF=True)
    font.setupGlyphOrder(order)
    font.setupCharacterMap(cmap)
    font.setupGlyf(glyphs)
    font.setupHorizontalMetrics(metrics)
    font.setupHorizontalHeader(ascent=CAP, descent=-(UPM - CAP))
    font.setupNameTable(
        {
            "familyName": "Cairn Icons",
            "styleName": "Regular",
            "psName": "CairnIcons-Regular",
            "version": "1.0",
        }
    )
    font.setupOS2(
        sTypoAscender=CAP,
        sTypoDescender=-(UPM - CAP),
        usWinAscent=CAP,
        usWinDescent=UPM - CAP,
        sCapHeight=CAP,
    )
    font.setupPost()
    # Zeroed so two runs of this script produce byte-identical files and `--check` means something.
    font.font["head"].created = font.font["head"].modified = 0
    font.save(out)
    return "\n".join(report)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify the shipped face is current")
    args = parser.parse_args()

    if not args.check:
        print(build(FACE))
        print(f"wrote {FACE.relative_to(ROOT)}")
        return 0

    with tempfile.NamedTemporaryFile(suffix=".ttf") as fresh:
        print(build(fresh.name))
        if filecmp.cmp(fresh.name, FACE, shallow=False):
            print(f"{FACE.relative_to(ROOT)} is those three sources")
            return 0
        print(
            f"{FACE.relative_to(ROOT)} does not match its sources — "
            f"run this script with no arguments",
            file=sys.stderr,
        )
        return 1


if __name__ == "__main__":
    sys.exit(main())
