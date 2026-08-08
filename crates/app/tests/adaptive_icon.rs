//! The launcher mark ships as an Android adaptive icon (issue #119).
//!
//! `cargo-apk` assembles the APK from Cargo metadata and a `resources` folder, and it has dropped
//! attributes before — the intent filters carry the same warning (see `Cargo.toml`). The real
//! verification is against the *emitted* `AndroidManifest.xml` and the built APK on the handset,
//! which this sandbox cannot package or run. What it *can* pin is the half that fails in silence:
//! a layer removed, a reference left dangling, or the `resources`/`icon` wiring deleted so the mark
//! never enters the build. None of those fail any other test, and the launcher shows a blank
//! platform default rather than an error — so the drift is invisible without this guard.
//!
//! These are string checks on purpose: parsing would need a TOML/XML dependency, and the acceptance
//! criterion for this ticket is *no new crate dependency*.

use std::fs;
use std::path::{Path, PathBuf};

fn res_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("res")
}

fn read(rel: &str) -> String {
    let path = res_dir().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("missing icon resource {rel}: {e}"))
}

/// The adaptive icon (API 26+) must reference all three layers ADR-0003's note names: a full-bleed
/// background, a foreground sized into the safe zone, and a monochrome silhouette for themed icons.
#[test]
fn adaptive_icon_references_all_three_layers() {
    let xml = read("mipmap-anydpi-v26/ic_launcher.xml");
    assert!(xml.contains("<adaptive-icon"), "not an <adaptive-icon>");
    assert!(
        xml.contains("<background") && xml.contains("@drawable/ic_launcher_background"),
        "adaptive icon drops its background layer"
    );
    assert!(
        xml.contains("<foreground") && xml.contains("@drawable/ic_launcher_foreground"),
        "adaptive icon drops its foreground layer"
    );
    assert!(
        xml.contains("<monochrome") && xml.contains("@drawable/ic_launcher_monochrome"),
        "adaptive icon drops its monochrome layer — themed icons fall back to a default"
    );
}

/// Every referenced layer is a vector drawable, so the framework renders it at whatever density the
/// launcher requests — "every density the build needs, from the vector sources" with no raster
/// buckets to keep in step.
#[test]
fn every_layer_is_a_vector_source() {
    for layer in [
        "ic_launcher_background",
        "ic_launcher_foreground",
        "ic_launcher_monochrome",
    ] {
        let xml = read(&format!("drawable/{layer}.xml"));
        assert!(xml.contains("<vector"), "{layer} is not a <vector> source");
    }
}

/// `min_sdk_version = 24` predates adaptive icons (API 26). Without a fallback at the unqualified
/// `mipmap/` the launcher on 24–25 shows the platform default, not the mark — so the legacy entry
/// must composite the same background and foreground layers rather than being absent.
#[test]
fn legacy_fallback_carries_the_mark() {
    let xml = read("mipmap/ic_launcher.xml");
    assert!(
        xml.contains("@drawable/ic_launcher_background")
            && xml.contains("@drawable/ic_launcher_foreground"),
        "the API 24-25 fallback does not draw the mark"
    );
}

/// The wiring that actually puts the mark in the APK: `resources` points cargo-apk at the folder,
/// and the application `icon` names the launcher resource. Delete either and every file above is
/// still present and correct while the built APK carries no icon at all.
#[test]
fn cargo_metadata_ships_the_resources() {
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("read app Cargo.toml");
    assert!(
        manifest.contains("resources = \"res\""),
        "cargo-apk is not told to bundle the res folder"
    );
    assert!(
        manifest.contains("icon = \"@mipmap/ic_launcher\""),
        "the application icon does not name the launcher resource"
    );
}
