//! The file list — the files [`cairn_export::platform::list`] can see, each row **described from
//! its own bytes** rather than its filename, and worded so it never claims to be a view of a folder
//! it is not (issue #108, ADR-0022 §11, ADR-0024 §1 §3).
//!
//! **What the list is, and what it is not.** Enumeration is by extension — `list` runs a `LIKE`
//! clause over `.cdeck` and `.ccoll` on Android and the same recognition on the desktop — but the
//! extension is where its authority ends (deck-export rule 13). What each row *says it is* comes from
//! sniffing the bytes' `mimetype` member, never the name: on Android both profiles store as
//! `application/octet-stream` and a `.cdeck` may in fact carry a collection archive, so the sniff is
//! the only thing that can tell them apart ([`describe`], ADR-0024 §1). A file this application wrote
//! but can no longer parse sniffs to `None` and is **still listed, marked unreadable** — hiding it
//! would send a user after a permissions problem that does not exist (ADR-0022 §11).
//!
//! **What the list cannot show, and must not imply it can.** Scoped storage grants this application
//! its own `MediaStore` rows and nothing else, so a `.cdeck` another application dropped in
//! `Downloads` is **invisible to the query, not merely unreadable** (ADR-0024 §3). The list is
//! therefore *"the files this application wrote"*, never *"the downloads folder"* — the wording must
//! not invite a user to put a file there and expect it to appear, because it never can.
//!
//! **One mechanism, not two.** Selecting a listed row does not open a second identification path: it
//! re-reads the bytes and hands them to [`select`], which produces an [`Inbound`] exactly as an
//! arriving file does, so it reaches the same [`crate::inbound::read`] gate-and-plan (acceptance of
//! #108, ADR-0022 §5). The row description here is the cheap sniff, deliberately: describing a whole
//! folder must inflate **zero payloads** (ADR-0022 §11), and the plan — which does inflate — is
//! derived only for the one file the user selects.

use cairn_export::Profile;

use crate::inbound::{Arrival, Inbound};

/// One row of the file list: a file this application wrote, named by the platform and described from
/// its own bytes. The name is the platform's own (a collision may have deduped it, ADR-0022 §10),
/// carried for display and for the re-read on selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub name: String,
    /// What the bytes say the file is, sniffed from the `mimetype` member (ADR-0024 §1). `None` when
    /// a file we wrote no longer parses as a container — **listed and marked unreadable**, never
    /// hidden (ADR-0022 §11).
    pub sniffed: Option<Profile>,
}

/// Describe one listed file **from its bytes**, never its extension. The profile is the sniff's, so a
/// `.cdeck` whose bytes are in fact a collection archive is described as a collection, and a file we
/// wrote that no longer parses is described as unreadable rather than dropped from the list
/// (ADR-0024 §1, ADR-0022 §11).
pub fn describe(name: &str, bytes: &[u8]) -> Listed {
    Listed {
        name: name.to_owned(),
        sniffed: cairn_export::sniff(bytes),
    }
}

/// Turn a selected listed file into an [`Inbound`], so it takes the **same** identification and plan
/// path an arriving file takes — one mechanism, not two (acceptance of #108). The bytes are the
/// platform's re-read of the file at selection time, and the name is the one the list showed; from
/// here on a selected file is indistinguishable from one opened, shared or dropped (ADR-0022 §5).
pub fn select(name: &str, bytes: Vec<u8>) -> Inbound {
    Inbound {
        arrival: Arrival::Listed,
        name: Some(name.to_owned()),
        bytes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inbound::{self, Arrival};
    use cairn_core::content::DeckId;
    use cairn_core::identity::CollectionId;
    use cairn_export::{
        CollectionArchive, DeckContent, DeckExport, Metadata, build_collection, build_deck,
        deck_digest, next_revision,
    };
    use cairn_store::Collection;
    use tempfile::TempDir;

    fn deck_bytes(id: DeckId, name: &str) -> Vec<u8> {
        let content = DeckContent {
            id,
            name: name.to_owned(),
            notes: Vec::new(),
            tombstones: Vec::new(),
        };
        let digest = deck_digest(&content).unwrap();
        let revision = next_revision(None, &digest);
        build_deck(&Metadata::default(), &[DeckExport { content, revision }]).unwrap()
    }

    fn collection_bytes() -> Vec<u8> {
        build_collection(&CollectionArchive {
            collection_id: &CollectionId([0x11; 16]),
            created: "2026-03-03T00:00:00Z",
            notes: 812,
            reviews: 4200,
            log: &[],
            mutable: &[],
        })
    }

    /// A file we wrote but can no longer parse is **listed and marked unreadable**, never hidden — a
    /// user who put it there must see it, not an empty list that reads as a permissions failure
    /// (ADR-0022 §11).
    #[test]
    fn an_unparseable_file_we_wrote_is_still_listed_marked_unreadable() {
        let listed = describe("French A1.cdeck", b"was a deck once, now truncated");
        assert_eq!(listed.name, "French A1.cdeck");
        assert_eq!(listed.sniffed, None);
    }

    /// Identity is the sniff, never the extension: a `.cdeck` whose **bytes** are a collection archive
    /// is described as a collection, because on Android the two are indistinguishable by name and type
    /// and only the `mimetype` member tells them apart (ADR-0024 §1).
    #[test]
    fn the_profile_is_the_sniff_not_the_extension() {
        // A collection archive that a collision or a rename left carrying a deck extension.
        let listed = describe("backup.cdeck", &collection_bytes());
        assert_eq!(listed.sniffed, Some(Profile::Collection));

        let listed = describe("shared.ccoll", &deck_bytes(DeckId([0xaa; 16]), "Deck"));
        assert_eq!(listed.sniffed, Some(Profile::Deck));
    }

    /// Selecting a listed file reaches the **same** identification and plan path an arriving file
    /// takes — [`select`] builds an [`Inbound`], and [`inbound::read`] plans it against the live
    /// collection exactly as a drop or a launch intent would (acceptance of #108).
    #[test]
    fn selecting_a_listed_file_takes_the_arriving_path() {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let coll = Collection::open(data.path(), state.path()).unwrap();

        let bytes = deck_bytes(DeckId([0xbb; 16]), "French A1");
        let inbound = select("French A1.cdeck", bytes);
        assert_eq!(inbound.arrival, Arrival::Listed);
        assert_eq!(inbound.name.as_deref(), Some("French A1.cdeck"));

        let report = inbound::read(&inbound, &coll).unwrap();
        assert_eq!(report.sniffed, Some(Profile::Deck));
        let plan = report.outcome.expect("a deck previews rather than refuses");
        assert_eq!(plan.decks[0].name, "French A1");
        assert_eq!(plan.decks[0].path, cairn_export::Path::Create);
    }
}
