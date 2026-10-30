//! The Kotlin binding over `cairn-ops` (ADR-0046 §2) — thin on purpose.
//!
//! What crosses is an operation and the facts it returns. This crate exports **no platform function
//! and calls none** (ADR-0046 §5): Kotlin passes `filesDir` and `noBackupFilesDir` to [`open`](CollectionHandle::open),
//! and never reaches `cairn_store::platform`, whose Android arm panics in a library no
//! `android-activity` loaded. Time is a value here as everywhere: where an operation needs *now*, the
//! caller passes it.
//!
//! **A panic reaches Kotlin as an exception only while the release profile unwinds** (ADR-0046 §7), so
//! `panic = "abort"` must never be added to the workspace's `[profile.release]`.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};

use cairn_store::Collection;

uniffi::setup_scaffolding!();

/// The one error that crosses, as Kotlin's `CairnException`. The field is `reason` and never
/// `message`: a field called `message` generates Kotlin that clashes with `Throwable.message`.
#[derive(Debug, uniffi::Error)]
pub enum CairnError {
    /// An operation failed; `reason` is the operation's own account of why, for a log, not for the
    /// screen (facts cross, words do not — ADR-0046 §4).
    Failed { reason: String },
}

impl std::fmt::Display for CairnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let CairnError::Failed { reason } = self;
        write!(f, "{reason}")
    }
}

impl std::error::Error for CairnError {}

impl From<String> for CairnError {
    fn from(reason: String) -> Self {
        CairnError::Failed { reason }
    }
}

/// The opened collection. Kotlin holds exactly one per process (ADR-0046 §7). Objects are shared
/// through `Arc` so every method takes `&self`, and `Collection` is `Send` but not `Sync`, hence the
/// `Mutex`: calls run one at a time.
#[derive(uniffi::Object)]
pub struct CollectionHandle(Mutex<Collection>);

impl CollectionHandle {
    /// A panic inside one call poisons the lock; the collection itself is not left half-written by
    /// that (every write is a transaction), so later calls carry on rather than all panicking too.
    fn collection(&self) -> MutexGuard<'_, Collection> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[uniffi::export]
impl CollectionHandle {
    /// Open the collection under the two directories the caller passes and, on a first launch, seed
    /// the starter notes — `cairn_ops::open_collection`, the one place the seed is written, so both
    /// renderers seed alike.
    #[uniffi::constructor]
    pub fn open(data_dir: String, state_dir: String) -> Result<Self, CairnError> {
        let coll = cairn_ops::open_collection(Path::new(&data_dir), Path::new(&state_dir))?;
        Ok(Self(Mutex::new(coll)))
    }

    /// How many cards are due at `now_ms` (wall-clock milliseconds, handed over by the caller).
    pub fn due_count(&self, now_ms: i64) -> Result<u64, CairnError> {
        Ok(cairn_ops::session::due_count(&self.collection(), now_ms)? as u64)
    }

    /// Panic on purpose, holding the lock, so a test can see the panic arrive as an exception and the
    /// handle survive it.
    #[cfg(feature = "panic-probe")]
    pub fn panic_probe(&self) {
        let _held = self.collection();
        panic!("panic probe");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn dirs() -> (TempDir, TempDir) {
        (TempDir::new().unwrap(), TempDir::new().unwrap())
    }

    fn open(data: &TempDir, state: &TempDir) -> CollectionHandle {
        CollectionHandle::open(
            data.path().display().to_string(),
            state.path().display().to_string(),
        )
        .unwrap()
    }

    #[test]
    fn opening_lands_the_files_in_the_two_directories_and_seeds_a_first_launch() {
        let (data, state) = dirs();
        let handle = open(&data, &state);
        assert!(data.path().join("collection.db").exists());
        assert!(state.path().join("derived.db").exists());
        assert!(state.path().join("writer.id").exists(), "the writer marker");
        // The seed is all new cards, none yet due.
        assert_eq!(handle.due_count(1_700_000_000_000).unwrap(), 0);
    }

    #[test]
    fn a_failed_open_crosses_as_the_one_error_with_a_reason() {
        let data = TempDir::new().unwrap();
        let file = data.path().join("not-a-directory");
        std::fs::write(&file, b"x").unwrap();
        let err = CollectionHandle::open(
            file.join("data").display().to_string(),
            file.join("state").display().to_string(),
        )
        .err()
        .expect("opening under a file must fail");
        let CairnError::Failed { reason } = err;
        assert!(!reason.is_empty());
    }

    #[cfg(feature = "panic-probe")]
    #[test]
    fn a_panic_unwinds_and_the_handle_keeps_working() {
        let (data, state) = dirs();
        let handle = open(&data, &state);
        let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            handle.panic_probe();
        }));
        assert!(caught.is_err());
        assert_eq!(handle.due_count(1_700_000_000_000).unwrap(), 0);
    }
}
