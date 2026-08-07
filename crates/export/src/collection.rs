//! The **collection profile**: the `.ccoll` archive a user keeps for themselves, and the read that
//! previews a restore before it merges.
//!
//! This is the third profile in the *same* container as [`crate::deck`]
//! ([ADR-0016 §2](../../../docs/adr/0016-backup-and-restore.md)), and its selection rule is the
//! opposite one: *the log verbatim, plus everything that settles, minus device identity and
//! credentials.* Where a deck file drops most of a restore's needs and restamps on import, the
//! collection profile **carries stamps byte for byte** because a restore does not cross a collection
//! boundary — the profile is what selects that rule, which is why it is a profile and not a flag
//! (ADR-0016 §2, ADR-0008 §3).
//!
//! Two things travel that a deck file forbids, and two never travel that nothing may leak:
//!
//! - **Carried**: the review log exactly as received (never re-encoded, ADR-0004 §11); the whole
//!   mutable surface with its stamps — unfiled notes, per-deck revisions, suspensions, the new-card
//!   rate, device labels; and the [`CollectionId`] that tells this collection from a stranger's.
//! - **Never**: a `writer_id` or `seq_highwater` — a restored device mints a **fresh** writer id
//!   (ADR-0007 §6), or two devices become one writer and the union drops reviews — and the sync
//!   credential (ADR-0016 §2). The build API cannot carry them: it is handed the log, the mutable
//!   surface and the collection id, and nothing else.
//!
//! **Determinism is not inherited** ([ADR-0016 §11](../../../docs/adr/0016-backup-and-restore.md)):
//! the manifest carries a **creation date**, so two archives of the same collection differ and a user
//! can tell them apart. The disclosure half of [ADR-0008 §12](../../../docs/adr/0008-the-deck-export-format.md)
//! still binds — no author name, no device label, no ambient identity is ever auto-populated, which
//! is why this manifest carries none of the deck profile's authoring fields.
//!
//! **Restore is a merge that only ever adds** (ADR-0016 §4), so a restore preview has no destructive
//! effect to enumerate and stays **one line** describing the file (ADR-0022 §12). The one refusal it
//! needs is [ADR-0016 §10](../../../docs/adr/0016-backup-and-restore.md)'s identity gate, run through
//! the same [`cairn_core::identity::adopt_or_refuse`] the enrolment seam runs.

use crate::container::{
    self, COLLECTION_MEDIA_TYPE, FORMAT, LOG_MEMBER, MANIFEST_MEMBER, MUTABLE_MEMBER, Member,
};
use crate::files::COLLECTION_EXTENSION;
use crate::import::{Profile, plain, sniff};
use cairn_core::identity::{Adoption, CollectionId, adopt_or_refuse};
use cairn_core::log::Json;
use std::io::Cursor;

/// The longest a date string read from an archive is shown — the one manifest field that is free
/// text rather than a number or an id, so it is bounded plain text like every other string a file
/// carries (ADR-0022 §7).
const MAX_CREATED_CHARS: usize = 40;

/// The line the interface must state before a restore, because "restore" universally implies a
/// replacement this design cannot implement (ADR-0016 §4, §12). Settled here; its placement on the
/// preview screen is the visual pass's.
pub const RESTORE_IS_A_MERGE: &str =
    "Restore adds to this collection. It never removes anything you already hold.";

/// The way out of an identity refusal, stated so a user is not left holding a device that will not
/// take their archive (ADR-0016 §10). Always available, and the refusal must name it.
pub const RESTORE_MISMATCH_WAY_OUT: &str = "This archive is from a different collection. \
     To use it on this device: back up what is here, clear the app's data, restore, then enrol.";

/// Everything a `.ccoll` archive carries, handed in by the caller (the store, which owns the log,
/// the mutable surface and the minted [`CollectionId`]). Both payloads are **verbatim lines** — this
/// crate assembles the container and never parses their schema, which is what keeps stamps byte for
/// byte and the log un-re-encoded.
pub struct CollectionArchive<'a> {
    /// The collection's identity, carried so a restore can tell this collection from a stranger's
    /// (ADR-0016 §3, §10).
    pub collection_id: &'a CollectionId,
    /// The creation date, an ISO-8601 UTC instant supplied by the caller (this crate reads no clock).
    /// The one piece of metadata determinism forbids and a personal archive needs (ADR-0016 §11).
    pub created: &'a str,
    /// The count of live notes, and of reviews ever recorded — the two numbers the one-line preview
    /// states (ADR-0016 §11). Supplied by the store, which computes them for the backup nudge too, so
    /// the manifest and the nudge cannot disagree.
    pub notes: usize,
    pub reviews: usize,
    /// The review log, one interchange line per entry, **as received** (ADR-0004 §11).
    pub log: &'a [String],
    /// The mutable surface, one serialised row per entry, its stamps intact (ADR-0016 §2).
    pub mutable: &'a [String],
}

/// The manifest gating and describing a `.ccoll` file. Keys sorted for a tidy, diffable document —
/// **not** for determinism, which this profile does not inherit (ADR-0016 §11). No author, no
/// device label, no ambient identity: minimal disclosure binds this profile too.
fn manifest(a: &CollectionArchive) -> String {
    crate::json::Object::new()
        .string("collection", &a.collection_id.to_canonical())
        .string("created", a.created)
        .raw("format", &FORMAT.to_string())
        .raw("notes", &a.notes.to_string())
        .string("profile", "collection")
        .raw("reviews", &a.reviews.to_string())
        .finish()
}

/// One `\n`-joined member body from verbatim lines, with a trailing newline when non-empty — the
/// JSON-lines shape the log and the mutable surface both take.
fn jsonl(lines: &[String]) -> Vec<u8> {
    if lines.is_empty() {
        return Vec::new();
    }
    let mut out = lines.join("\n");
    out.push('\n');
    out.into_bytes()
}

/// Assemble the whole `.ccoll` archive. Members in fixed order: `mimetype` (stored, first, so the
/// profile sits at a fixed byte offset — ADR-0016 §9), `manifest.json`, `log.jsonl`, `mutable.jsonl`.
/// The order and the container are fixed; the bytes are **not** byte-for-byte reproducible across
/// runs, because the creation date differs (ADR-0016 §11).
pub fn build_collection(a: &CollectionArchive) -> Vec<u8> {
    container::build(&[
        Member::stored(container::MIMETYPE_MEMBER, COLLECTION_MEDIA_TYPE),
        Member::deflated(MANIFEST_MEMBER, manifest(a).into_bytes()),
        Member::deflated(LOG_MEMBER, jsonl(a.log)),
        Member::deflated(MUTABLE_MEMBER, jsonl(a.mutable)),
    ])
}

/// The `.ccoll` filename a write requests — a plain `collection.ccoll`. The user chose neither the
/// name nor the location (there is no picker, ADR-0016 §5), and a collision **dedupes** to
/// `collection (1).ccoll` at the seam (ADR-0024 §4), so archives made on different days are told
/// apart by the manifest date the list reads, not by the filename (ADR-0022 §11).
pub fn collection_filename() -> String {
    format!("collection.{COLLECTION_EXTENSION}")
}

/// This device, as the restore identity gate needs to see it (ADR-0016 §10). `empty` is the store's
/// "authored nothing" judgement — no log rows under this device's own writer id and nothing on the
/// mutable surface — supplied because this crate cannot read the store.
pub struct RestoreTarget {
    pub empty: bool,
    pub own_id: CollectionId,
}

/// A `.ccoll` a restore must not act on, refused **in place of the preview** and without inflating
/// the log (ADR-0022 §2). Like an import refusal it carries no detail for whoever built the file,
/// with the one exception the specification requires: a mismatch **names the collection** so the
/// user can act (ADR-0016 §10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreRefusal {
    /// Not a sniffable archive, or its manifest would not parse. Nothing to preview.
    Unreadable,
    /// A `format` integer this build cannot read (ADR-0008 §7).
    UnknownFormat(u64),
    /// The file is not a `collection` archive — a deck file, or anything else, offered to restore.
    WrongProfile,
    /// An absolute path, a `..` segment, a symlink entry, or a member name that is none of the known
    /// ones or the `media/` prefix (ADR-0008 §6). One message, no invitation to repair.
    BrokenPath,
    /// This device already holds a **different** collection (ADR-0016 §10). Names the archive's
    /// collection id so the interface can state the mismatch; the way out is [`RESTORE_MISMATCH_WAY_OUT`].
    IdentityMismatch { archive: String },
}

/// A restore this device may perform, described in the one line the preview shows (ADR-0016 §11,
/// ADR-0022 §12). It states the *file*, because a merge has no destructive effect to enumerate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestorePlan {
    /// The creation date the manifest declares, bounded plain text (ADR-0022 §7).
    pub created: String,
    pub notes: usize,
    pub reviews: usize,
    /// Whether this device **adopts** the archive's identity (it has authored nothing) or agrees it
    /// is its **own** (ADR-0016 §10). A [`Adoption::Mismatch`] never reaches here — it is a refusal.
    pub adoption: Adoption,
}

impl RestorePlan {
    /// The single line the preview states (ADR-0016 §11): the file, not its effects. The numbers are
    /// plain here; grouping them (`4,200`) and formatting the date (`3 March 2026`) is the visual
    /// pass's, over the raw ISO instant this carries.
    pub fn one_line(&self) -> String {
        format!(
            "Collection archive, {}. {} {}, {} {}.",
            self.created,
            self.notes,
            noun(self.notes, "note"),
            self.reviews,
            noun(self.reviews, "review"),
        )
    }
}

/// `"note"` for one, `"notes"` for any other count — so the one-line preview never reads "1 notes".
fn noun(n: usize, singular: &str) -> String {
    if n == 1 {
        singular.to_owned()
    } else {
        format!("{singular}s")
    }
}

/// Read a received `.ccoll` and derive the one-line [`RestorePlan`], or the [`RestoreRefusal`] shown
/// in its place. Reads the `mimetype` member, the member-name list and the small `manifest.json`
/// **only** — the log and the mutable surface are never inflated here (ADR-0022 §2), because a
/// restore preview has nothing to say about a merge's effects and the identity gate fires from the
/// manifest alone.
pub fn restore_preview(
    bytes: &[u8],
    target: &RestoreTarget,
) -> Result<RestorePlan, RestoreRefusal> {
    match sniff(bytes) {
        Some(Profile::Collection) => {}
        Some(Profile::Deck | Profile::Other(_)) => return Err(RestoreRefusal::WrongProfile),
        None => return Err(RestoreRefusal::Unreadable),
    }

    let mut archive =
        zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| RestoreRefusal::Unreadable)?;

    // Path rules over the central directory only — no payload inflated (ADR-0008 §6).
    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|_| RestoreRefusal::Unreadable)?;
        if !member_is_allowed(&entry) {
            return Err(RestoreRefusal::BrokenPath);
        }
    }

    let manifest_text = container::read_member(&mut archive, MANIFEST_MEMBER)
        .map_err(|_| RestoreRefusal::Unreadable)?;
    let manifest = Json::parse(&manifest_text).ok_or(RestoreRefusal::Unreadable)?;

    let format = manifest
        .get("format")
        .and_then(Json::as_u64)
        .ok_or(RestoreRefusal::Unreadable)?;
    if format != FORMAT as u64 {
        return Err(RestoreRefusal::UnknownFormat(format));
    }

    // The manifest must agree with its own `mimetype` member; a disagreement is not a file we act on.
    if manifest.get("profile").and_then(Json::as_str) != Some("collection") {
        return Err(RestoreRefusal::WrongProfile);
    }

    let archive_id = manifest
        .get("collection")
        .and_then(Json::as_str)
        .and_then(CollectionId::parse_canonical)
        .ok_or(RestoreRefusal::Unreadable)?;

    // The identity gate, run through the exact function the enrolment seam runs (ADR-0016 §10).
    let adoption = adopt_or_refuse(target.empty, &target.own_id, &archive_id);
    if adoption == Adoption::Mismatch {
        return Err(RestoreRefusal::IdentityMismatch {
            archive: archive_id.to_canonical(),
        });
    }

    let created = manifest
        .get("created")
        .and_then(Json::as_str)
        .map(|s| plain(s, MAX_CREATED_CHARS))
        .unwrap_or_default();
    let notes = manifest.get("notes").and_then(Json::as_u64).unwrap_or(0) as usize;
    let reviews = manifest.get("reviews").and_then(Json::as_u64).unwrap_or(0) as usize;

    Ok(RestorePlan {
        created,
        notes,
        reviews,
        adoption,
    })
}

/// Whether a member is one the restore reader accepts: traversal-safe (the shared
/// [`container::member_path_is_safe`]) and either a known `collection` member name or the `media/`
/// prefix (ADR-0008 §6).
fn member_is_allowed(entry: &zip::read::ZipFile<'_, Cursor<&[u8]>>) -> bool {
    let name = entry.name();
    container::member_path_is_safe(entry)
        && (matches!(
            name,
            "mimetype" | MANIFEST_MEMBER | LOG_MEMBER | MUTABLE_MEMBER
        ) || (name.starts_with("media/") && name.len() > "media/".len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cid(b: u8) -> CollectionId {
        CollectionId([b; 16])
    }

    fn sample<'a>(
        id: &'a CollectionId,
        log: &'a [String],
        mutable: &'a [String],
    ) -> CollectionArchive<'a> {
        CollectionArchive {
            collection_id: id,
            created: "2026-03-03T09:00:00.000Z",
            notes: 812,
            reviews: 4200,
            log,
            mutable,
        }
    }

    #[test]
    fn a_built_archive_is_self_identifying_as_a_collection() {
        let id = cid(0xc0);
        let bytes = build_collection(&sample(&id, &[], &[]));
        // The media type sits at the fixed offset, readable without inflating anything (ADR-0016 §9).
        let start = 30 + container::MIMETYPE_MEMBER.len();
        let end = start + COLLECTION_MEDIA_TYPE.len();
        assert_eq!(&bytes[start..end], COLLECTION_MEDIA_TYPE.as_bytes());
        assert_eq!(sniff(&bytes), Some(Profile::Collection));
    }

    #[test]
    fn the_log_and_mutable_surface_travel_verbatim() {
        let id = cid(0xc0);
        // Stamps and writer ids inside the log rows are part of the bytes and must survive unchanged.
        let log = vec![
            r#"{"k":"rev","w":"7f3a","s":412,"n":"x","o":0,"g":3,"t":"t","d":20514,"ms":4200}"#
                .to_owned(),
            r#"{"k":"cut","w":"7f3a","s":413,"t":"t","d":20515}"#.to_owned(),
        ];
        let mutable = vec![
            r#"{"entity":"note","id":"abc","attr":"deck","value":"d","stamp":"7f3a:9"}"#.to_owned(),
            r#"{"entity":"suspension","id":"card","attr":"suspended","value":"true","stamp":"7f3a:10"}"#
                .to_owned(),
        ];
        let bytes = build_collection(&sample(&id, &log, &mutable));
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
        assert_eq!(
            container::read_member(&mut archive, LOG_MEMBER).unwrap(),
            jsonl_str(&log)
        );
        assert_eq!(
            container::read_member(&mut archive, MUTABLE_MEMBER).unwrap(),
            jsonl_str(&mutable)
        );
    }

    fn jsonl_str(lines: &[String]) -> String {
        let mut s = lines.join("\n");
        s.push('\n');
        s
    }

    #[test]
    fn the_manifest_carries_the_date_and_id_but_no_identity_or_credential() {
        let id = cid(0xab);
        let bytes = build_collection(&sample(&id, &[], &[]));
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes.as_slice())).unwrap();
        let text = container::read_member(&mut archive, MANIFEST_MEMBER).unwrap();
        let m = Json::parse(&text).unwrap();
        assert_eq!(m.get("profile").and_then(Json::as_str), Some("collection"));
        assert_eq!(
            m.get("collection").and_then(Json::as_str),
            Some(id.to_canonical().as_str())
        );
        assert_eq!(
            m.get("created").and_then(Json::as_str),
            Some("2026-03-03T09:00:00.000Z")
        );
        // Never a writer id, a sequence high-water, a credential, or an ambient identity (ADR-0016 §2, §11).
        for forbidden in [
            "writer",
            "writer_id",
            "w",
            "seq",
            "seq_highwater",
            "credential",
            "token",
            "author",
            "device",
        ] {
            assert!(
                m.get(forbidden).is_none(),
                "the collection manifest must never carry {forbidden:?}"
            );
        }
    }

    #[test]
    fn an_empty_device_adopts_and_the_preview_is_one_line() {
        let archive_id = cid(0x11);
        let bytes = build_collection(&sample(&archive_id, &[], &[]));
        // A fresh install minted its own id but has authored nothing, so it adopts (ADR-0016 §10).
        let target = RestoreTarget {
            empty: true,
            own_id: cid(0x99),
        };
        let plan = restore_preview(&bytes, &target).unwrap();
        assert_eq!(plan.adoption, Adoption::Adopt);
        assert_eq!(
            plan.one_line(),
            "Collection archive, 2026-03-03T09:00:00.000Z. 812 notes, 4200 reviews."
        );
    }

    #[test]
    fn a_non_empty_device_holding_the_same_collection_merges() {
        let id = cid(0x11);
        let bytes = build_collection(&sample(&id, &[], &[]));
        let target = RestoreTarget {
            empty: false,
            own_id: id,
        };
        assert_eq!(
            restore_preview(&bytes, &target).unwrap().adoption,
            Adoption::Same
        );
    }

    #[test]
    fn a_non_empty_device_holding_a_different_collection_is_refused_by_name() {
        let archive_id = cid(0x11);
        let bytes = build_collection(&sample(&archive_id, &[], &[]));
        let target = RestoreTarget {
            empty: false,
            own_id: cid(0x22),
        };
        assert_eq!(
            restore_preview(&bytes, &target),
            Err(RestoreRefusal::IdentityMismatch {
                archive: archive_id.to_canonical(),
            })
        );
    }

    #[test]
    fn a_deck_file_offered_to_restore_is_the_wrong_profile() {
        // A real deck archive, built by the deck path.
        let deck = crate::deck::build_deck(
            &Default::default(),
            &[crate::deck::DeckExport {
                content: crate::deck::DeckContent {
                    id: cairn_core::content::DeckId([1; 16]),
                    name: "French".to_owned(),
                    notes: vec![],
                    tombstones: vec![],
                },
                revision: crate::deck::DeckRevision {
                    revision: 1,
                    digest: "d".to_owned(),
                },
            }],
        )
        .unwrap();
        let target = RestoreTarget {
            empty: true,
            own_id: cid(0),
        };
        assert_eq!(
            restore_preview(&deck, &target),
            Err(RestoreRefusal::WrongProfile)
        );
        assert_eq!(
            restore_preview(b"not a zip", &target),
            Err(RestoreRefusal::Unreadable)
        );
    }

    #[test]
    fn an_unknown_format_is_refused_without_inflating_the_payload() {
        // Format 2 with no log.jsonl or mutable.jsonl: a clean UnknownFormat proves the gate refused
        // before touching a payload (ADR-0022 §2).
        let manifest = format!(
            r#"{{"collection":"{}","created":"t","format":2,"notes":0,"profile":"collection","reviews":0}}"#,
            cid(1).to_canonical()
        );
        let bytes = container::build(&[
            Member::stored(container::MIMETYPE_MEMBER, COLLECTION_MEDIA_TYPE),
            Member::deflated(MANIFEST_MEMBER, manifest.into_bytes()),
        ]);
        let target = RestoreTarget {
            empty: true,
            own_id: cid(0),
        };
        assert_eq!(
            restore_preview(&bytes, &target),
            Err(RestoreRefusal::UnknownFormat(2))
        );
    }

    #[test]
    fn a_traversing_or_unknown_member_is_refused() {
        let id = cid(1);
        let manifest = manifest(&sample(&id, &[], &[]));
        for evil in [
            "../escape",
            "/etc/passwd",
            "media/../x",
            "surprise.txt",
            "media/",
        ] {
            let bytes = container::build(&[
                Member::stored(container::MIMETYPE_MEMBER, COLLECTION_MEDIA_TYPE),
                Member::deflated(MANIFEST_MEMBER, manifest.clone().into_bytes()),
                Member::deflated(evil, b"x".to_vec()),
            ]);
            let target = RestoreTarget {
                empty: true,
                own_id: cid(0),
            };
            assert_eq!(
                restore_preview(&bytes, &target),
                Err(RestoreRefusal::BrokenPath),
                "member {evil:?} must be refused"
            );
        }
    }

    #[test]
    fn a_media_member_is_accepted_alongside_the_known_ones() {
        let id = cid(1);
        let bytes = container::build(&[
            Member::stored(container::MIMETYPE_MEMBER, COLLECTION_MEDIA_TYPE),
            Member::deflated(
                MANIFEST_MEMBER,
                manifest(&sample(&id, &[], &[])).into_bytes(),
            ),
            Member::deflated(LOG_MEMBER, b"".to_vec()),
            Member::deflated("media/a.mp3", b"audio".to_vec()),
        ]);
        let target = RestoreTarget {
            empty: true,
            own_id: cid(0),
        };
        assert!(restore_preview(&bytes, &target).is_ok());
    }
}
