//! The file bench through the real desktop user-files seam, into a temporary documents directory.
//!
//! One test in its own binary, because it sets `XDG_DOCUMENTS_DIR` — process-global state that
//! threaded tests would race on, as `cairn-export`'s own `desktop_seam` test says.
//!
//! What only this can show is **idempotence against the seam**: the set installed twice writes
//! nothing the second time, and a stranger's file under one of its names is refused rather than
//! deduped beside it. On a handset that is the whole difference between a bench that can be pressed
//! again and one that fills `Downloads` with `(1)` copies nothing can delete.

#![cfg(not(target_os = "android"))]

use cairn_app::file_bench::FileSet;
use cairn_app::fixtures::Fixture;
use cairn_export::platform;
use cairn_store::Collection;

#[test]
fn the_set_installs_once_reinstalls_as_nothing_and_refuses_a_stranger() {
    let docs = tempfile::tempdir().unwrap();
    // SAFETY: the only test in this binary, so nothing reads the environment concurrently.
    unsafe {
        std::env::set_var("XDG_DOCUMENTS_DIR", docs.path());
    }

    let data = tempfile::tempdir().unwrap();
    let state = tempfile::tempdir().unwrap();
    let mut coll = Collection::open(data.path(), state.path()).unwrap();
    Fixture::Decks
        .install(&mut coll, 20_514 * 86_400_000 + 4 * 3_600_000)
        .unwrap();

    let first = FileSet::Imports.install(&coll).unwrap();
    assert_eq!((first.files, first.written), (4, 4));

    let second = FileSet::Imports.install(&coll).unwrap();
    assert_eq!(
        (second.files, second.written),
        (4, 0),
        "a second install recognises its own files"
    );
    assert_eq!(platform::list().unwrap().len(), 4, "and adds no copies");

    let name = FileSet::Imports.files().unwrap().remove(0).name;
    std::fs::write(docs.path().join(&name), b"someone else's deck").unwrap();
    assert!(
        FileSet::Imports.install(&coll).is_err(),
        "a different file under a bench name is refused, not deduped beside"
    );
    assert_eq!(platform::list().unwrap().len(), 4);
}
