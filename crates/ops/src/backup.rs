//! A backup (ADR-0016 §2 §6, ADR-0053 §4): the whole collection as one `.ccoll` archive, written
//! where the person chooses, and the nudge that says how long ago the last one was and how large
//! the collection is against Android's automatic-backup quota (ADR-0016 §7).
//!
//! **Building is not backing up.** [`build`] hands back the archive's bytes, the suggested name and
//! the counts, and records nothing; the caller writes the bytes through the platform and calls
//! [`record`] only once the write succeeded, so a cancelled or failed save leaves the nudge as it was.
//! "Last backup" lives in the store's `local` table and never travels or settles.
//!
//! **One instant serves the name and the manifest** (ADR-0016 §11). The caller takes it once and
//! passes it with the device's offset from UTC at that instant: the manifest's `created` is the
//! instant in UTC, and the name is dated by the local day, which this crate cannot read.

use cairn_core::identity::CollectionId;
use cairn_export::{CollectionArchive, build_collection, collection_filename};
use cairn_store::{Collection, StoreError, iso8601_millis};

/// A built archive, not yet written anywhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backup {
    /// The whole `.ccoll` file.
    pub bytes: Vec<u8>,
    /// The name the save dialog suggests — [`filename`] at the creation instant.
    pub name: String,
    /// The live notes and every review, as the manifest states them and the report repeats them.
    pub notes: u64,
    pub reviews: u64,
}

/// What the backup nudge states (ADR-0016 §6): when the last backup was, or that there is none, and
/// the counts that say what is at stake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nudge {
    /// The creation instant of the last backup written, in epoch milliseconds; `None` before the first.
    pub last_backup_ms: Option<i64>,
    /// The reviews recorded after the last backup — every review when there is none.
    pub reviews_since: u64,
    pub reviews: u64,
    pub notes: u64,
    /// The collection's size against Android's automatic-backup quota (ADR-0016 §7).
    pub size: Size,
}

/// Android's automatic-backup quota: 25 MiB, which ADR-0016 §7's sentence states as *"25 MB"*. The
/// platform documents *"25 MB"*; the binary reading is the stricter of the two, so the line appears
/// no later than the cutoff could.
pub const QUOTA_BYTES: u64 = 25 * MIB;

const MIB: u64 = 1_048_576;

/// The collection's size as the backup row states it (ADR-0016 §7): whole MB, rounded up so the
/// size is never understated, in the same MB the quota is stated in — so a collection over the quota
/// never reads as *"25 MB"* beside *"stops above 25 MB"*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    pub mb: u64,
    /// Past the quota, where the platform's automatic backup silently stops.
    pub over_quota: bool,
}

/// `bytes`, in the tar unit the platform measures, stated against the quota.
pub fn size(bytes: u64) -> Size {
    Size {
        mb: bytes.div_ceil(MIB),
        over_quota: bytes > QUOTA_BYTES,
    }
}

/// The name a backup made at `created_ms` suggests: `Cairn backup YYYY-MM-DD.ccoll`, dated by the
/// day in the device's local time, `offset_minutes` east of UTC at that instant.
pub fn filename(created_ms: i64, offset_minutes: i32) -> String {
    let local = created_ms + i64::from(offset_minutes) * 60_000;
    collection_filename(&iso8601_millis(local)[..10])
}

/// The collection as an archive: the log and the mutable surface as the store holds them, and the
/// manifest's identity, instant and counts. Reads only; nothing is recorded.
pub fn build(
    coll: &Collection,
    created_ms: i64,
    offset_minutes: i32,
) -> Result<Backup, StoreError> {
    let log = coll.log_lines()?;
    let mutable = coll.mutable_lines()?;
    let notes = live_notes(coll)?;
    let reviews = coll.review_count()?;
    // The store mints the id and keeps it in canonical text, so it always reads back.
    let collection_id = CollectionId::parse_canonical(coll.collection_id())
        .expect("the store keeps its collection id in canonical text");
    let bytes = build_collection(&CollectionArchive {
        collection_id: &collection_id,
        created: &iso8601_millis(created_ms),
        notes: notes as usize,
        reviews: reviews as usize,
        log: &log,
        mutable: &mutable,
    });
    Ok(Backup {
        bytes,
        name: filename(created_ms, offset_minutes),
        notes,
        reviews,
    })
}

/// Record that the archive made at `created_ms` was written. Call it only after the write succeeded.
pub fn record(coll: &mut Collection, created_ms: i64) -> Result<(), StoreError> {
    coll.record_backup(created_ms)
}

/// The nudge's facts as the collection stands.
pub fn nudge(coll: &Collection) -> Result<Nudge, StoreError> {
    let last_backup_ms = coll.last_backup()?;
    let reviews = coll.review_count()?;
    let reviews_since = match last_backup_ms {
        Some(at) => coll.reviews_after(at)?,
        None => reviews,
    };
    Ok(Nudge {
        last_backup_ms,
        reviews_since,
        reviews,
        notes: live_notes(coll)?,
        size: size(coll.backup_size()?),
    })
}

/// The live notes, counted from the note list so the nudge, the manifest and the list never disagree
/// on what a deleted deck takes with it (ADR-0005 §7).
fn live_notes(coll: &Collection) -> Result<u64, StoreError> {
    Ok(crate::notes::list(coll, &crate::notes::Filter::default())?.len() as u64)
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Read};

    use super::*;
    use crate::fixtures::Fixture;
    use crate::notes::{self, Filter};
    use cairn_core::log::Json;
    use cairn_export::{Profile, sniff};
    use tempfile::TempDir;

    /// 2026-03-02T23:30:00Z: half an hour before midnight in UTC, and already the 3rd east of it.
    const LATE_MS: i64 = 1_772_494_200_000;

    fn open() -> (Collection, TempDir, TempDir) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let coll = Collection::open(data.path(), state.path()).unwrap();
        (coll, data, state)
    }

    fn member(bytes: &[u8], name: &str) -> String {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
        let mut text = String::new();
        zip.by_name(name)
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        text
    }

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_owned).collect()
    }

    #[test]
    fn the_archive_carries_the_log_the_mutable_surface_and_the_manifest_and_nothing_local() {
        let (mut coll, _d, _s) = open();
        Fixture::Decks.install(&mut coll, LATE_MS).unwrap();
        coll.set_theme_preference("dark-theme-choice").unwrap();
        coll.record_backup(LATE_MS - 1).unwrap();

        let backup = build(&coll, LATE_MS, 0).unwrap();
        assert_eq!(sniff(&backup.bytes), Some(Profile::Collection));

        // The two payloads are the store's lines, verbatim.
        assert_eq!(
            lines(&member(&backup.bytes, "log.jsonl")),
            coll.log_lines().unwrap()
        );
        assert_eq!(
            lines(&member(&backup.bytes, "mutable.jsonl")),
            coll.mutable_lines().unwrap()
        );

        // The manifest: the identity, the instant, and the counts the nudge states.
        let manifest = Json::parse(&member(&backup.bytes, "manifest.json")).unwrap();
        let live = notes::list(&coll, &Filter::default()).unwrap().len() as u64;
        assert!(live > 0);
        assert_eq!(backup.notes, live);
        assert_eq!(backup.reviews, coll.review_count().unwrap());
        assert_eq!(manifest.get("notes").and_then(Json::as_u64), Some(live));
        assert_eq!(
            manifest.get("reviews").and_then(Json::as_u64),
            Some(backup.reviews)
        );
        assert_eq!(
            manifest.get("collection").and_then(Json::as_str),
            Some(coll.collection_id())
        );
        assert_eq!(
            manifest.get("created").and_then(Json::as_str),
            Some("2026-03-02T23:30:00.000Z")
        );

        // Nothing ADR-0016 §2 excludes: the device-local values set above appear nowhere in the
        // archive. Which lines the payloads select is the store's, proved in its own backup tests.
        let everything = ["manifest.json", "log.jsonl", "mutable.jsonl"]
            .map(|m| member(&backup.bytes, m))
            .join("\n");
        for local in [
            "dark-theme-choice".to_string(),
            (LATE_MS - 1).to_string(),
            iso8601_millis(LATE_MS - 1),
        ] {
            assert!(!everything.contains(&local), "{local} must never travel");
        }
        // Building is not backing up: nothing is recorded until the write succeeds.
        assert_eq!(coll.last_backup().unwrap(), Some(LATE_MS - 1));
    }

    #[test]
    fn the_name_is_dated_in_local_time_across_a_midnight() {
        let (coll, _d, _s) = open();
        assert_eq!(
            build(&coll, LATE_MS, 0).unwrap().name,
            "Cairn backup 2026-03-02.ccoll"
        );
        assert_eq!(filename(LATE_MS, 0), "Cairn backup 2026-03-02.ccoll");
        // An hour east of UTC it is already the 3rd; seven hours west it is still the 2nd.
        assert_eq!(filename(LATE_MS, 60), "Cairn backup 2026-03-03.ccoll");
        assert_eq!(filename(LATE_MS, -7 * 60), "Cairn backup 2026-03-02.ccoll");
        assert_eq!(
            build(&coll, LATE_MS, 60).unwrap().name,
            filename(LATE_MS, 60)
        );
        // The manifest keeps the instant in UTC whatever the zone.
        let manifest = member(&build(&coll, LATE_MS, 60).unwrap().bytes, "manifest.json");
        assert!(manifest.contains("2026-03-02T23:30:00.000Z"), "{manifest}");
    }

    #[test]
    fn the_nudge_counts_everything_before_the_first_backup_and_what_followed_after() {
        let (mut coll, _d, _s) = open();
        Fixture::History.install(&mut coll, LATE_MS).unwrap();
        let reviews = coll.review_count().unwrap();
        let notes = notes::list(&coll, &Filter::default()).unwrap().len() as u64;
        assert!(reviews > 0);

        let before = nudge(&coll).unwrap();
        assert_eq!(
            before,
            Nudge {
                last_backup_ms: None,
                reviews_since: reviews,
                reviews,
                notes,
                size: before.size,
            }
        );

        record(&mut coll, LATE_MS).unwrap();
        let after = nudge(&coll).unwrap();
        assert_eq!(after.last_backup_ms, Some(LATE_MS));
        assert_eq!(after.reviews_since, 0);
        assert_eq!(after.reviews, reviews);
        assert_eq!(after.notes, notes);
    }

    #[test]
    fn the_size_is_over_the_quota_only_past_25_mib() {
        // 25 MiB is 26,214,400 bytes: at it the backup still runs, a byte past it does not.
        assert_eq!(
            size(26_214_399),
            Size {
                mb: 25,
                over_quota: false
            }
        );
        assert_eq!(
            size(26_214_400),
            Size {
                mb: 25,
                over_quota: false
            }
        );
        assert_eq!(
            size(26_214_401),
            Size {
                mb: 26,
                over_quota: true
            }
        );
    }

    #[test]
    fn the_size_is_stated_in_whole_mb_rounded_up() {
        assert_eq!(size(0).mb, 0);
        assert_eq!(size(1).mb, 1);
        assert_eq!(size(1_048_576).mb, 1);
        assert_eq!(size(1_048_577).mb, 2);
        // ADR-0016 §7's example: a collection of 31 MB is a little over 30 MiB.
        assert_eq!(
            size(31_457_281),
            Size {
                mb: 31,
                over_quota: true
            }
        );
    }

    #[test]
    fn the_nudge_carries_the_collections_size() {
        let (mut coll, _d, _s) = open();
        Fixture::History.install(&mut coll, LATE_MS).unwrap();
        let bytes = coll.backup_size().unwrap();
        assert!(bytes > 0);
        assert_eq!(nudge(&coll).unwrap().size, size(bytes));
        assert!(!nudge(&coll).unwrap().size.over_quota);
    }
}
