#!/usr/bin/env python3
"""**Throwaway.** Build `crates/app/assets/Proto162Icons-Regular.ttf` — the mark plus the two
pictures a note-list row would repeat twenty-five times.

    python3 -m venv /tmp/faceenv && /tmp/faceenv/bin/pip install 'fonttools==4.64.0'
    /tmp/faceenv/bin/python scripts/proto-162-icon-face.py

This exists for [#162](https://github.com/amin-bf/cairn/issues/162)'s sitting and is preserved as
`prototypes/issue-162`, never merged. `scripts/build-icon-face.py` — the shipped one — is **not
touched**, so `--check`'s claim that the shipped face is the launcher's four stones stays exactly as
true as it was.

# Why a second script rather than an argument on the first

The shipped face carries **one** glyph generated from an Android vector drawable, whose paths are
already **filled**. The sixteen icons in the design project are a different kind of source: a 24px
grid of `stroke-width: 1.5` **strokes** with round caps and joins, and `fill: none`. ADR-0038 §1's
route needs filled outlines, which is why the design project's own note calls the SVGs *the drawing
source, not the delivery mechanism* — each icon is **redrawn** for the face rather than exported to
it. That redrawing is what this script does, and it is a different program from the one that reads a
drawable.

# How a stroke becomes a fill, and why it is one contour per segment

Each segment of a path is emitted as its own closed **stadium** — a rectangle of the stroke's width
with a semicircular cap at each end — and every one of them is wound the same way. Under non-zero
winding, same-wound overlapping contours **merge**, so the caps that overlap at a joint produce a
round join for free and the whole path comes out as one filled shape. That is not a trick invented
here: the shipped face rests on the identical property, and its own comment records what happens
when it is violated — two stones wound in opposite directions cancel where they overlap and punch a
hole through the middle of the mark, from a file that is otherwise perfectly valid.

# The metric that had to change, and it is a finding rather than a convenience

ADR-0038 §1 gives the mark **advance width = ink width, left side bearing = zero**, so that a
centred label centres the stones rather than a box with unequal air in it. That is right for **one**
picture drawn on its own.

It does not survive a **set**. Two icons with different ink widths get different advances, so two
icon-only buttons in a row get different widths — and a right-aligned action column drawn from them
is **ragged in exactly the way the words were**, which is the defect the column exists to fix. So
every glyph here is given a **square advance** of one cap height with its ink centred in it, and the
divergence is written onto the ticket rather than absorbed.
"""

import math
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

from fontTools.fontBuilder import FontBuilder
from fontTools.pens.recordingPen import RecordingPen
from fontTools.pens.reverseContourPen import ReverseContourPen
from fontTools.pens.ttGlyphPen import TTGlyphPen

ROOT = Path(__file__).resolve().parent.parent
DRAWABLE = ROOT / "crates/app/res/drawable/ic_launcher_monochrome.xml"
FACE = ROOT / "crates/app/assets/Proto162Icons-Regular.ttf"

ANDROID = "{http://schemas.android.com/apk/res/android}"
NUMBER = re.compile(r"-?\d*\.?\d+")

UPM = 1000
CAP = 720

#: The mark, unchanged — the shipped code point, so the prototype face is a **superset** of the
#: shipped one and nothing that draws the mark today notices the swap.
MARK = 0xE000
#: The two pictures a row repeats. Private use, like the mark, for the same reason.
MOVE = 0xE001
DELETE = 0xE002

#: The design project's grid, and the stroke every one of its sixteen icons is drawn with.
GRID = 24.0
STROKE = 1.5

#: **`move` is not one of the sixteen**, so this is drawn rather than redrawn — the set was authored
#: before the screen that needed it, and the one control this row repeats has no picture in it. A
#: vertical double-headed arrow, in the set's own language: 24px grid, 1.5 stroke, round caps.
MOVE_PATH = "M12 4.75v14.5M12 4.75l-3.5 3.5M12 4.75l3.5 3.5M12 19.25l-3.5-3.5M12 19.25l3.5-3.5"

#: `assets/icons/delete.svg` from the design project, verbatim — a lid rule, a handle, and a bin.
DELETE_PATH = "M5 7h14M9.5 7V4.5h5V7M7.5 7l.9 12.5h7.2L16.5 7"


def polylines(data):
    """The subpaths of an SVG path, as point lists. `M`, `L`, `h`, `v` and `l` — the subset the
    design project's icons use, all of them straight."""
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
                cursor = (
                    (n[i], n[i + 1])
                    if command == "L"
                    else (cursor[0] + n[i], cursor[1] + n[i + 1])
                )
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
    """One segment as a closed contour: two parallel edges and a semicircular cap at each end.

    Wound counter-clockwise in **font** space, which is what lets overlapping segments merge under
    non-zero winding instead of cancelling."""
    (x0, y0), (x1, y1) = a, b
    dx, dy = x1 - x0, y1 - y0
    length = math.hypot(dx, dy)
    if length == 0:
        return []
    ux, uy = dx / length, dy / length
    base = math.atan2(uy, ux)
    points = []
    # The cap at `b`, swept from the left edge round to the right edge.
    for i in range(steps + 1):
        angle = base - math.pi / 2 + math.pi * i / steps
        points.append((x1 + radius * math.cos(angle), y1 + radius * math.sin(angle)))
    # The cap at `a`, the same sweep half a turn round.
    for i in range(steps + 1):
        angle = base + math.pi / 2 + math.pi * i / steps
        points.append((x0 + radius * math.cos(angle), y0 + radius * math.sin(angle)))
    return points


def stroked(data):
    """A stroked 24px path as filled contours on the design project's grid, still y-down."""
    out = []
    for line in polylines(data):
        for a, b in zip(line, line[1:]):
            contour = stadium(a, b, STROKE / 2)
            if contour:
                out.append(contour)
    return out


def drawable_contours(path):
    """The launcher drawable's filled paths — absolute `M`, `C`, `Z`, flattened to polylines so the
    mark goes through the same placement code as the other two."""
    root = ET.parse(path).getroot()
    for element in root.iter("path"):
        contour, cursor = [], (0.0, 0.0)
        for command, args in re.findall(
            r"([MCZmcz])([^MCZmcz]*)", element.get(ANDROID + "pathData")
        ):
            n = [float(v) for v in NUMBER.findall(args)]
            if command == "M":
                cursor = (n[0], n[1])
                contour.append(cursor)
            elif command == "C":
                for i in range(0, len(n), 6):
                    p0, p1, p2, p3 = cursor, n[i : i + 2], n[i + 2 : i + 4], n[i + 4 : i + 6]
                    for step in range(1, 13):
                        t = step / 12
                        s = 1 - t
                        contour.append(
                            (
                                s**3 * p0[0]
                                + 3 * s * s * t * p1[0]
                                + 3 * s * t * t * p2[0]
                                + t**3 * p3[0],
                                s**3 * p0[1]
                                + 3 * s * s * t * p1[1]
                                + 3 * s * t * t * p2[1]
                                + t**3 * p3[1],
                            )
                        )
                    cursor = (p3[0], p3[1])
            elif command == "Z":
                pass
            else:
                raise SystemExit(f"{path}: unsupported path command {command!r}")
        if len(contour) > 2:
            yield contour


def signed_area(points):
    return (
        sum(
            x0 * y1 - x1 * y0
            for (x0, y0), (x1, y1) in zip(points, points[1:] + points[:1])
        )
        / 2
    )


def glyph(contours, box):
    """Scale a set of y-down contours to `box` tall, centre them in a square advance, and draw.

    The **square advance** is the divergence from ADR-0038 §1 this script's docstring argues for:
    without it two icon-only buttons are two different widths and the action column is ragged again.
    """
    xs = [x for c in contours for x, _ in c]
    ys = [y for c in contours for _, y in c]
    left, right, top, bottom = min(xs), max(xs), min(ys), max(ys)
    scale = box / max(right - left, bottom - top)
    advance = round(box)
    # Centre the ink in the square, and flip y — the sources are y-down and a font is y-up.
    ox = (advance - (right - left) * scale) / 2 - left * scale
    baseline = bottom * scale - (box - (bottom - top) * scale) / 2

    pen = TTGlyphPen(None)
    for contour in contours:
        placed = [(x * scale + ox, baseline - y * scale) for x, y in contour]
        recording = RecordingPen()
        recording.moveTo(placed[0])
        for point in placed[1:]:
            recording.lineTo(point)
        recording.closePath()
        recording.replay(pen if signed_area(placed) > 0 else ReverseContourPen(pen))
    return pen.glyph(), advance, (right - left) * scale, (bottom - top) * scale


def main():
    sources = {
        "mark": list(drawable_contours(DRAWABLE)),
        "move": stroked(MOVE_PATH),
        "delete": stroked(DELETE_PATH),
    }
    glyphs, metrics = {".notdef": TTGlyphPen(None).glyph()}, {".notdef": (round(CAP), 0)}
    for name, contours in sources.items():
        # The mark is measured to a full cap height because that is what ADR-0038 §1 pins and the
        # shipped face already draws. The two row icons are measured on the design project's **24px
        # grid** instead, so the 1.5 stroke stays 1.5 relative to the picture — an icon scaled to
        # its own ink would draw a heavier line the smaller its drawing happened to be.
        box = CAP if name == "mark" else CAP * (max(
            max(x for c in contours for x, _ in c) - min(x for c in contours for x, _ in c),
            max(y for c in contours for _, y in c) - min(y for c in contours for _, y in c),
        ) / GRID)
        shape, advance, ink_w, ink_h = glyph(contours, box)
        glyphs[name] = shape
        metrics[name] = (round(CAP), 0)
        print(f"{name:7s} ink {ink_w:6.1f}×{ink_h:6.1f}  advance {round(CAP)}  of {UPM}/em")

    font = FontBuilder(UPM, isTTF=True)
    font.setupGlyphOrder([".notdef", "mark", "move", "delete"])
    font.setupCharacterMap({MARK: "mark", MOVE: "move", DELETE: "delete"})
    font.setupGlyf(glyphs)
    font.setupHorizontalMetrics(metrics)
    font.setupHorizontalHeader(ascent=CAP, descent=-(UPM - CAP))
    font.setupNameTable(
        {
            "familyName": "Proto 162 Icons",
            "styleName": "Regular",
            "psName": "Proto162Icons-Regular",
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
    font.font["head"].created = font.font["head"].modified = 0
    font.save(FACE)
    print(f"wrote {FACE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
