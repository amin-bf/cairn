#!/usr/bin/env python3
"""Build `crates/app/assets/CairnIcons-Regular.ttf` from the shipped monochrome drawable.

    scripts/build-icon-face.py            # regenerate the face in place
    scripts/build-icon-face.py --check    # rebuild to a temporary file and diff against the shipped one

**The face is checked in; this script is why it is the shape it is.** ADR-0038 §1 routes icons
through the font stack, and a font is a binary — so the one thing that can rot is the claim that the
glyph *is* the launcher's four stones. This script makes that claim executable: it reads
`crates/app/res/drawable/ic_launcher_monochrome.xml`, which the Android build already ships, and
emits the glyph from its paths. Edit the drawable and the mark follows; edit neither and `--check`
says so.

**It is not part of any build.** It needs `fonttools` (built with 4.64.0) and runs when the drawable
changes, which is approximately never. Nothing in `cargo build`, `cargo test` or the capture harness
invokes it — a Rust workspace that needed a Python interpreter to compile would be a far worse trade
than a 952-byte asset with a recipe beside it.

    python3 -m venv /tmp/faceenv && /tmp/faceenv/bin/pip install 'fonttools==4.64.0'
    /tmp/faceenv/bin/python scripts/build-icon-face.py --check

# The metrics, and why they are these

**The ink is one cap height tall and sits on the baseline.** An icon is a glyph, so its size *is* a
font size (ADR-0038 §1) — and a glyph that filled the em would overshoot the line it is set in and
collide with the line above. At `CAP` the mark stands as tall as a capital letter beside the word it
illustrates, which is what the icon rule asks of it.

**The advance width is the ink width and the left side bearing is zero**, so a centred label centres
the *stones* rather than a box with unequal air in it. An icon set inline against a word wants a
space in the string, not a sidebearing baked into every use.

**The glyph is placed by what it draws, not by the viewport.** The drawable's 108×108 box has padding
around the stones because a launcher icon is masked; that padding is Android's business, and carrying
it into the face would make every stated size a lie by whatever fraction of the viewport happens to
be empty.
"""

import argparse
import filecmp
import re
import subprocess
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
FACE = ROOT / "crates/app/assets/CairnIcons-Regular.ttf"

ANDROID = "{http://schemas.android.com/apk/res/android}"

UPM = 1000
CAP = 720
#: The mark's code point. Private use, so nothing else can ever claim it and no shipped face can
#: shadow it — which is what lets the icon face be appended last in every family (`fonts.rs`).
MARK = 0xE000

NUMBER = re.compile(r"-?\d*\.?\d+")


def contours(drawable):
    """The drawable's paths, as command lists. Absolute `M`, `C` and `Z` only — the subset it uses."""
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


def winding(points):
    """The signed area of the polygon through a contour's on-curve points."""
    return sum(x0 * y1 - x1 * y0 for (x0, y0), (x1, y1) in zip(points, points[1:] + points[:1])) / 2


def replay(commands, pen, scale, left, top):
    """Draw one contour, flipping y — the drawable is y-down and a font is y-up."""
    points = []
    for kind, n in commands:
        if kind == "move":
            points.append((n[0] * scale - left, top - n[1] * scale))
            pen.moveTo(points[-1])
        elif kind == "curve":
            bezier = [(n[i] * scale - left, top - n[i + 1] * scale) for i in (0, 2, 4)]
            points.append(bezier[2])
            pen.curveTo(*bezier)
        elif kind == "close":
            pen.closePath()
    return points


def build(drawable, out):
    paths = list(contours(drawable))
    coordinates = [(n[i], n[i + 1]) for p in paths for _, n in p for i in range(0, len(n), 2)]
    left, right = min(x for x, _ in coordinates), max(x for x, _ in coordinates)
    top, bottom = min(y for _, y in coordinates), max(y for _, y in coordinates)
    scale = CAP / (bottom - top)
    width = round((right - left) * scale)

    pen = TTGlyphPen(None)
    for path in paths:
        recording = RecordingPen()
        points = replay(path, recording, scale, left * scale, bottom * scale)
        # The four stones overlap, and non-zero winding only merges them when they wind the same
        # way. Two stones drawn in opposite directions would *cancel* where they overlap — a hole
        # through the middle of the mark, from a file that is otherwise perfectly valid.
        outward = pen if winding(points) > 0 else ReverseContourPen(pen)
        # TrueType has no cubics. 0.3 units of an em at 1000/em is well under a pixel at any size a
        # screen draws this at.
        recording.replay(Cu2QuPen(outward, 0.3))

    font = FontBuilder(UPM, isTTF=True)
    font.setupGlyphOrder([".notdef", "mark"])
    font.setupCharacterMap({MARK: "mark"})
    font.setupGlyf({".notdef": TTGlyphPen(None).glyph(), "mark": pen.glyph()})
    font.setupHorizontalMetrics({".notdef": (width, 0), "mark": (width, 0)})
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
    return f"ink x[{left},{right}] y[{top},{bottom}] → {width}×{CAP} of {UPM}/em"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="verify the shipped face is current")
    args = parser.parse_args()

    if not args.check:
        print(build(DRAWABLE, FACE))
        print(f"wrote {FACE.relative_to(ROOT)}")
        return 0

    with tempfile.NamedTemporaryFile(suffix=".ttf") as fresh:
        print(build(DRAWABLE, fresh.name))
        if filecmp.cmp(fresh.name, FACE, shallow=False):
            print(f"{FACE.relative_to(ROOT)} is the drawable's four stones")
            return 0
        print(
            f"{FACE.relative_to(ROOT)} does not match {DRAWABLE.relative_to(ROOT)} — "
            f"run this script with no arguments",
            file=sys.stderr,
        )
        return 1


if __name__ == "__main__":
    sys.exit(main())
