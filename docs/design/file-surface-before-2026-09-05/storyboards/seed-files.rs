//! Throwaway: write a set of files into a scratch documents directory so the file-list specimen and
//! the inbound specimen have something to draw. Not part of the repo.

use cairn_core::content::{DeckId, NoteId};
use cairn_core::identity::CollectionId;
use cairn_export::{
    CollectionArchive, DeckContent, DeckExport, Metadata, NoteContent, Tombstone, build_collection,
    build_deck, deck_digest, next_revision,
};

fn note(seed: u8, front: &str, back: &str, pos: &str) -> NoteContent {
    NoteContent {
        id: NoteId([seed; 16]),
        position: pos.to_owned(),
        kind: "basic".to_owned(),
        fields: vec![
            ("Back".to_owned(), back.to_owned()),
            ("Front".to_owned(), front.to_owned()),
        ],
    }
}

fn deck(seed: u8, name: &str, n: usize, tombs: usize) -> DeckContent {
    let notes = (0..n)
        .map(|i| {
            note(
                seed.wrapping_add(i as u8).wrapping_add(1),
                &format!("mot {i}"),
                &format!("word {i}"),
                &format!("a{i:03}"),
            )
        })
        .collect();
    let tombstones = (0..tombs)
        .map(|i| Tombstone {
            id: NoteId([seed.wrapping_add(200).wrapping_add(i as u8); 16]),
        })
        .collect();
    DeckContent {
        id: DeckId([seed; 16]),
        name: name.to_owned(),
        notes,
        tombstones,
    }
}

fn export(content: DeckContent) -> DeckExport {
    let digest = deck_digest(&content).unwrap();
    let revision = next_revision(None, &digest);
    DeckExport { content, revision }
}

fn main() {
    let dir = std::env::args().nth(1).expect("usage: seedfiles <dir>");
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();

    let meta = Metadata {
        author: "Marjan Rahimi".to_owned(),
        description: "A1 vocabulary for the first ten chapters.".to_owned(),
        licence: "CC BY-SA 4.0".to_owned(),
    };

    // One deck, with tombstones — the ordinary case.
    let one = build_deck(&meta, &[export(deck(0x21, "French A1", 38, 3))]).unwrap();
    std::fs::write(dir.join("French A1.cdeck"), one).unwrap();

    // Three decks in one file — ADR-0008 §8's upstream split.
    let three = build_deck(
        &meta,
        &[
            export(deck(0x31, "French A1", 24, 0)),
            export(deck(0x41, "German", 204, 0)),
            export(deck(0x51, "Dutch", 61, 2)),
        ],
    )
    .unwrap();
    std::fs::write(dir.join("French A1 and 2 more.cdeck"), three).unwrap();

    // A collection archive — the third profile, in the same container.
    let coll = build_collection(&CollectionArchive {
        collection_id: &CollectionId([0x11; 16]),
        created: "2026-03-03T00:00:00Z",
        notes: 812,
        reviews: 4200,
        log: &[],
        mutable: &[],
    });
    std::fs::write(dir.join("backup.ccoll"), coll).unwrap();

    // A file we wrote and can no longer parse (ADR-0022 §11) — listed, marked unreadable.
    std::fs::write(
        dir.join("Chapters 1-4.cdeck"),
        b"was a deck once, now truncated",
    )
    .unwrap();

    println!("wrote 4 files into {}", dir.display());
}
