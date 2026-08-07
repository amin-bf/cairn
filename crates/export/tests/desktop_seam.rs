//! The desktop user-files seam, exercised against a real temporary documents directory.
//!
//! There is no fake filesystem here for the same reason `cairn-store` has no fake store: the seam
//! *is* file I/O, so the test points `XDG_DOCUMENTS_DIR` at a temp dir and drives put/get/list end to
//! end. A single test in its own binary, because it mutates a process-global environment variable —
//! parallel tests would race on it.
//!
//! `hand_off` is deliberately absent: it opens a file manager or a share sheet, which no headless
//! environment can assert against — that is [#98](https://github.com/amin-bf/cairn/issues/98)'s.

#![cfg(not(target_os = "android"))]

use cairn_export::platform;

#[test]
fn put_reads_back_the_deduped_name_and_list_finds_it() {
    let dir = tempfile::tempdir().unwrap();
    // SAFETY: this is the only test in this binary, so nothing else reads the environment
    // concurrently. `XDG_DOCUMENTS_DIR` is what the desktop arm resolves the files directory from.
    unsafe {
        std::env::set_var("XDG_DOCUMENTS_DIR", dir.path());
    }

    // A first write takes the requested name verbatim and round-trips its bytes.
    let first = platform::put("French A1.cdeck", b"deck-bytes").unwrap();
    assert_eq!(first.name, "French A1.cdeck");
    assert_eq!(platform::get("French A1.cdeck").unwrap(), b"deck-bytes");

    // A colliding write dedupes before the extension, and the report reads back the written name —
    // never the requested one (ADR-0022 §10).
    let second = platform::put("French A1.cdeck", b"other-bytes").unwrap();
    assert_eq!(second.name, "French A1 (1).cdeck");
    assert_eq!(
        platform::get("French A1 (1).cdeck").unwrap(),
        b"other-bytes"
    );

    // A file we do not recognise is invisible to the list; both `.cdeck` files are present, sorted.
    std::fs::write(dir.path().join("notes.txt"), b"not ours").unwrap();
    let listed = platform::list().unwrap();
    assert_eq!(listed, vec!["French A1 (1).cdeck", "French A1.cdeck"]);
}
