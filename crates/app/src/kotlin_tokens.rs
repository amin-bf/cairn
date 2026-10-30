//! **The guard between the desktop's design tokens and the Android client's** (ADR-0045).
//!
//! The native client's design module (`android/design`) restates these modules' numbers in Kotlin,
//! because Kotlin cannot call a Rust `const`. A second copy drifts the way every second copy does —
//! one side edited, nothing failing — so this test reads the Kotlin source and fails on **any
//! difference, any token missing from Kotlin, and any token Kotlin has that Rust does not**. It is
//! the Rust side's, and not the Gradle build's, because Rust holds the decision: Kotlin follows.
//!
//! Colours are not compared constant by constant. Each Kotlin role is compared with the value the
//! desktop *draws* — the egui `Visuals` the theme builds, and `card_fill`, `card_divider` and
//! `primary_fill` — so a role that Rust re-derives is caught too.
//!
//! The faces are compared by bytes, in order, family by family: the order decides which face a
//! glyph reaches (client-stack rule 7), and a file that merely shares a name proves nothing.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use egui::Color32;

use crate::{controls, fonts, frame, motion, spacing, surface, theme, typography};

/// A Kotlin literal, or the Rust value it must equal.
#[derive(Debug, Clone, PartialEq)]
enum Value {
    Int(i64),
    Float(f32),
    Text(String),
}

fn design_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../android/design/src/main")
}

fn read_kotlin(file: &str) -> String {
    let path = design_root()
        .join("kotlin/dev/cairn/design/tokens")
        .join(file);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn literal(raw: &str) -> Value {
    let raw = raw.trim().replace('_', "");
    if let Some(text) = raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        return Value::Text(text.to_owned());
    }
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        let hex = hex.trim_end_matches('L');
        return Value::Int(i64::from_str_radix(hex, 16).unwrap_or_else(|_| panic!("hex {raw}")));
    }
    if raw.contains('.') || raw.ends_with('f') || raw.ends_with('F') {
        let number = raw.trim_end_matches(['f', 'F']);
        return Value::Float(number.parse().unwrap_or_else(|_| panic!("float {raw}")));
    }
    Value::Int(raw.parse().unwrap_or_else(|_| panic!("int {raw}")))
}

/// Every `const val` of every `object` in a Kotlin source, keyed `Object.NAME`. A string literal
/// that carries a `//` stays whole; a trailing comment is cut.
fn tokens(source: &str) -> BTreeMap<String, Value> {
    let mut found = BTreeMap::new();
    let mut object = String::new();
    let mut lines = source.lines().peekable();
    while let Some(line) = lines.next() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("object ") {
            object = rest
                .split(|c: char| !c.is_alphanumeric())
                .next()
                .unwrap_or("")
                .to_owned();
        } else if let Some(rest) = line.strip_prefix("const val ") {
            let (name, tail) = rest.split_once('=').unwrap_or_else(|| {
                // A declaration wrapped onto the next line: `const val NAME =\n    "..."`.
                panic!("`{line}` has no `=` on its line")
            });
            let name = name.split(':').next().unwrap().trim().to_owned();
            let mut value = tail.trim().to_owned();
            if value.is_empty() {
                value = lines.next().expect("a value after `=`").trim().to_owned();
            }
            if !value.starts_with('"') {
                value = value.split("//").next().unwrap().trim().to_owned();
            }
            found.insert(format!("{object}.{name}"), literal(&value));
        }
    }
    found
}

/// What differs between the Kotlin `object` and what Rust says it must hold; empty when they mirror.
fn mismatches(source: &str, object: &str, expected: &[(&str, Value)]) -> Vec<String> {
    let kotlin = tokens(source);
    let mut problems = Vec::new();
    for (name, want) in expected {
        let key = format!("{object}.{name}");
        match kotlin.get(&key) {
            None => problems.push(format!("{key}: missing in Kotlin (Rust has {want:?})")),
            Some(got) if got != want => {
                problems.push(format!("{key}: Kotlin has {got:?}, Rust has {want:?}"));
            }
            Some(_) => {}
        }
    }
    let prefix = format!("{object}.");
    for key in kotlin.keys().filter(|k| k.starts_with(&prefix)) {
        if !expected
            .iter()
            .any(|(name, _)| format!("{prefix}{name}") == *key)
        {
            problems.push(format!("{key}: in Kotlin, with no counterpart in Rust"));
        }
    }
    problems
}

fn assert_mirrors(file: &str, object: &str, expected: &[(&str, Value)]) {
    let problems = mismatches(&read_kotlin(file), object, expected);
    assert!(
        problems.is_empty(),
        "android/design/.../tokens/{file} has drifted from the Rust tokens:\n  {}",
        problems.join("\n  ")
    );
}

fn f(x: f32) -> Value {
    Value::Float(x)
}

fn argb(c: Color32) -> Value {
    assert_eq!(c.a(), 255, "a palette role is opaque");
    Value::Int(0xFF00_0000 | (i64::from(c.r()) << 16) | (i64::from(c.g()) << 8) | i64::from(c.b()))
}

#[test]
fn the_type_scale_is_mirrored() {
    assert_mirrors(
        "Typography.kt",
        "Typography",
        &[
            ("DISPLAY", f(typography::DISPLAY)),
            ("HEADING", f(typography::HEADING)),
            ("BODY", f(typography::BODY)),
            ("SMALL", f(typography::SMALL)),
            ("MARK", f(typography::MARK)),
        ],
    );
}

#[test]
fn the_rhythm_is_mirrored() {
    assert_mirrors("Spacing.kt", "Spacing", &[("UNIT", f(spacing::UNIT))]);
}

#[test]
fn the_page_geometry_is_mirrored() {
    assert_mirrors(
        "Frame.kt",
        "Frame",
        &[
            ("PAGE_MARGIN", f(frame::PAGE_MARGIN)),
            ("MEASURE", f(frame::MEASURE)),
            ("REACH_LINE", f(frame::REACH_LINE)),
            ("TWO_COLUMN_MEASURE", f(frame::TWO_COLUMN_MEASURE)),
            ("PANE_GUTTER", f(frame::PANE_GUTTER)),
        ],
    );
}

#[test]
fn the_controls_are_mirrored() {
    let radius = theme::cairn_dark().widgets.inactive.corner_radius;
    assert_eq!(radius, theme::cairn_light().widgets.inactive.corner_radius);
    assert_mirrors(
        "Controls.kt",
        "Controls",
        &[
            ("HEIGHT", f(controls::HEIGHT)),
            ("TARGET_INK", Value::Int(i64::from(controls::TARGET_INK))),
            ("WIDGET_RADIUS", f(f32::from(radius.nw))),
        ],
    );
}

#[test]
fn the_card_surface_is_mirrored() {
    assert_mirrors(
        "Surface.kt",
        "Surface",
        &[
            ("RADIUS", f(f32::from(surface::RADIUS))),
            ("REVIEW_HEIGHT", f(surface::REVIEW_HEIGHT)),
            ("FIT", f(surface::FIT)),
        ],
    );
}

#[test]
fn the_motion_is_mirrored() {
    assert_mirrors(
        "Motion.kt",
        "Motion",
        &[
            ("DURATION", f(motion::DURATION)),
            ("EASING_AT_HALF", f((motion::EASING)(0.5))),
        ],
    );
}

/// The effective weak text colour: body text at the visuals' reduced alpha, composited over the
/// page, because that is what the eye reads (dark derives it, light names it).
fn weak_over_page(v: &egui::Visuals) -> Color32 {
    let (fg, bg) = (v.weak_text_color(), v.panel_fill);
    let a = f64::from(fg.a()) / 255.0;
    let blend = |f: u8, b: u8| (f64::from(f) + f64::from(b) * (1.0 - a)).round() as u8;
    Color32::from_rgb(
        blend(fg.r(), bg.r()),
        blend(fg.g(), bg.g()),
        blend(fg.b(), bg.b()),
    )
}

fn roles(v: &egui::Visuals) -> Vec<(&'static str, Value)> {
    vec![
        ("PAGE", argb(v.panel_fill)),
        ("POPUP", argb(v.window_fill)),
        ("POPUP_STROKE", argb(v.window_stroke.color)),
        ("FIELD", argb(v.extreme_bg_color)),
        ("CONTROL", argb(theme::control_fill(v))),
        ("CARD", argb(theme::card_fill(v))),
        ("CARD_DIVIDER", argb(theme::card_divider(v))),
        ("SEPARATOR", argb(theme::control_stroke(v).color)),
        ("BODY", argb(v.text_color())),
        ("WEAK", argb(weak_over_page(v))),
        ("ON_CONTROL", argb(v.widgets.inactive.fg_stroke.color)),
        ("PRIMARY", argb(theme::primary_fill(v))),
        ("PRIMARY_STROKE", argb(theme::primary_stroke(v).color)),
        ("HOVER", argb(v.widgets.hovered.bg_fill)),
        ("HOVER_STROKE", argb(v.widgets.hovered.bg_stroke.color)),
        ("PRESSED", argb(v.widgets.active.bg_fill)),
        ("SELECTION", argb(v.selection.bg_fill)),
        ("SELECTION_STROKE", argb(v.selection.stroke.color)),
        ("LINK", argb(theme::link(v))),
        ("WARN", argb(v.warn_fg_color)),
        ("ERROR", argb(v.error_fg_color)),
    ]
}

#[test]
fn both_palettes_are_mirrored_as_drawn() {
    assert_mirrors("Palette.kt", "DarkPalette", &roles(&theme::cairn_dark()));
    assert_mirrors("Palette.kt", "LightPalette", &roles(&theme::cairn_light()));
}

/// The window theme cannot read Kotlin, so the page colour the window shows before the first frame
/// is written once more in a resource per mode; it must be the page of that mode's palette.
#[test]
fn the_window_background_before_the_first_frame_is_the_page_of_each_palette() {
    let page = |v: egui::Visuals| match argb(v.panel_fill) {
        Value::Int(i) => i,
        other => panic!("{other:?}"),
    };
    for (dir, expected) in [
        ("values", page(theme::cairn_light())),
        ("values-night", page(theme::cairn_dark())),
    ] {
        let path = design_root().join(format!("res/{dir}/cairn_window.xml"));
        let xml =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let hex = xml
            .split("name=\"cairn_page\">#")
            .nth(1)
            .and_then(|r| r.split('<').next())
            .unwrap_or_else(|| panic!("{dir}: no cairn_page colour"));
        assert_eq!(
            i64::from_str_radix(hex, 16).unwrap(),
            expected,
            "{dir}/cairn_window.xml"
        );
    }
}

/// The names of the faces egui's own defaults add and the native client leaves to the platform's
/// emoji fallback.
const EMOJI: [&str; 2] = ["NotoEmoji-Regular", "emoji-icon-font"];

fn bytes_of_family(defs: &egui::FontDefinitions, family: &egui::FontFamily) -> Vec<Vec<u8>> {
    defs.families[family]
        .iter()
        .filter(|name| !EMOJI.contains(&name.as_str()))
        .map(|name| defs.font_data[name].font.to_vec())
        .collect()
}

fn kotlin_family(faces: &BTreeMap<String, Value>, name: &str) -> Vec<Vec<u8>> {
    let Some(Value::Text(list)) = faces.get(&format!("Faces.{name}")) else {
        panic!("Faces.{name} is missing from Faces.kt, or is not a string");
    };
    list.split(',')
        .map(|file| {
            let path = design_root().join("assets/fonts").join(file);
            std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        })
        .collect()
}

#[test]
fn the_faces_are_bundled_as_they_ship_in_the_order_each_family_reaches_them() {
    let defs = fonts::definitions();
    let source = read_kotlin("Faces.kt");
    let kotlin = tokens(&source);
    for (name, family) in [
        ("PROPORTIONAL", egui::FontFamily::Proportional),
        ("MONOSPACE", egui::FontFamily::Monospace),
        ("BOLD", fonts::bold_family()),
    ] {
        let rust = bytes_of_family(&defs, &family);
        let got = kotlin_family(&kotlin, name);
        assert_eq!(
            rust.len(),
            got.len(),
            "{name}: Kotlin lists {} faces, Rust registers {}",
            got.len(),
            rust.len()
        );
        for (place, (a, b)) in rust.iter().zip(&got).enumerate() {
            assert!(
                a == b,
                "{name}: the face at place {place} is a different file"
            );
        }
        assert!(
            got.last() == Some(&defs.font_data["icons"].font.to_vec()),
            "{name}: the icon face goes last"
        );
    }
    assert_eq!(
        kotlin.get("Faces.MARK"),
        Some(&Value::Int(i64::from(u32::from(fonts::MARK)))),
        "the mark is a different code point"
    );
}

#[test]
fn every_bundled_face_has_its_licence_and_nothing_unused_ships() {
    let root = design_root().join("assets");
    let used: std::collections::BTreeSet<String> = {
        let kotlin = tokens(&read_kotlin("Faces.kt"));
        ["PROPORTIONAL", "MONOSPACE", "BOLD"]
            .iter()
            .flat_map(|n| match &kotlin[&format!("Faces.{n}")] {
                Value::Text(list) => list.split(',').map(str::to_owned).collect::<Vec<_>>(),
                other => panic!("{other:?}"),
            })
            .collect()
    };
    let shipped: std::collections::BTreeSet<String> = std::fs::read_dir(root.join("fonts"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        used, shipped,
        "the files under assets/fonts are exactly the faces Faces.kt lists"
    );

    // One licence per foundry; the file name says whose it is.
    for licence in [
        "Ubuntu-Light-UFL.txt",
        "Hack-MIT-and-Bitstream-Vera.txt",
        "NotoSansArabic-OFL.txt",
        "DejaVu-Sans.txt",
        "CairnIcons.txt",
    ] {
        let path = root.join("licenses").join(licence);
        let text =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(!text.trim().is_empty(), "{licence} is empty");
    }
}

#[test]
fn a_difference_a_missing_token_and_an_extra_token_each_fail_the_guard() {
    let source = "object Spacing {\n    const val UNIT = 8f\n    const val EXTRA = 3f\n}\n";
    let expected = [("UNIT", f(9.0)), ("GONE", f(1.0))];
    let problems = mismatches(source, "Spacing", &expected);
    assert_eq!(problems.len(), 3, "{problems:?}");
    assert!(problems.iter().any(|p| p.contains("UNIT: Kotlin has")));
    assert!(problems.iter().any(|p| p.contains("GONE: missing")));
    assert!(problems.iter().any(|p| p.contains("EXTRA: in Kotlin")));
    assert!(
        mismatches(
            "object Spacing {\n const val UNIT = 8f\n}",
            "Spacing",
            &[("UNIT", f(8.0))]
        )
        .is_empty()
    );
}
