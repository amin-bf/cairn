//! The application's operations and the state they need (ADR-0046 §2) — everything both renderers
//! share, and nothing that draws.
//!
//! This crate depends on `cairn-core`, `cairn-store` and `cairn-export`, and on **no renderer**
//! (`egui`, `eframe`, `winit`, `android-activity`) and no binding. That is the point of it being a
//! crate: the compiler states what this code cannot reach. The egui app calls it, and so will the
//! Kotlin binding, so the desktop and Android run one implementation of every operation.
//!
//! **Time is a value** (ADR-0046 §3). Where an operation needs *now*, the caller passes it; nothing in
//! this crate's sitting or editor reads a clock.

pub mod appearance;
pub mod cards;
pub mod deck;
pub mod editor;
pub mod file_bench;
pub mod fixtures;
pub mod inbound;
pub mod listing;
pub mod markdown;
pub mod notes;
pub mod optimise;
pub mod review;
pub mod session;
pub mod sitting;

use std::path::Path;

use cairn_core::log::DEFAULT_NEW_CARD_RATE;
use cairn_store::Collection;

/// Open the collection under the two directories the caller passes (ADR-0007 §6, ADR-0046 §5) and, on
/// a first launch, seed a few `basic` notes so the walking skeleton has cards to review — issue #94's
/// opening line.
///
/// **The caller owns the directories**: the egui app passes the platform seam's, and the Kotlin app
/// passes `filesDir` and `noBackupFilesDir`. Both then seed alike, because this is the one place the
/// seed is written.
///
/// **More than one note, deliberately.** A single card cannot exercise the session at all: the count
/// picker collapses to one `All 1` button with none of ADR-0006 §1's choices reachable, and a sitting
/// ends on the first grading — so §2's *position is derived, never stored* cannot be demonstrated,
/// because there is no mid-session to be force-quit out of (issue #96). One note past
/// [`DEFAULT_NEW_CARD_RATE`] keeps the rate itself visible as the binding limit rather than the
/// seed's size.
pub fn open_collection(data: &Path, state: &Path) -> Result<Collection, String> {
    const SEED: [(&str, &str); DEFAULT_NEW_CARD_RATE as usize + 1] = [
        ("chien", "dog"),
        ("chat", "cat"),
        ("livre", "book"),
        ("eau", "water"),
        ("pain", "bread"),
        ("maison", "house"),
    ];

    let mut coll = Collection::open(data, state).map_err(|e| e.to_string())?;
    if coll.is_empty().map_err(|e| e.to_string())? {
        for (front, back) in SEED {
            coll.create_note("basic", &[("Front", front), ("Back", back)])
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(coll)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn a_first_open_seeds_the_starter_notes_and_a_second_does_not_seed_again() {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();

        let first = open_collection(data.path(), state.path()).unwrap();
        let seeded = notes::list(&first, &notes::Filter::default())
            .unwrap()
            .len();
        assert_eq!(seeded, DEFAULT_NEW_CARD_RATE as usize + 1);
        drop(first);

        let again = open_collection(data.path(), state.path()).unwrap();
        let after = notes::list(&again, &notes::Filter::default())
            .unwrap()
            .len();
        assert_eq!(
            after, seeded,
            "reopening a collection with notes seeds nothing"
        );
    }

    #[test]
    fn the_collection_lands_in_the_directories_the_caller_passed() {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        open_collection(data.path(), state.path()).unwrap();
        assert!(data.path().join("collection.db").exists());
        assert!(state.path().join("derived.db").exists());
    }
}
