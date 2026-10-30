//! **The new-card rate** (ADR-0011 §3, §5) — read and set as operations, so the egui Settings screen
//! and the Kotlin one write it through one implementation (ADR-0046 §1).

use cairn_store::{Collection, StoreError};

/// The stored rate, or [`cairn_core::log::DEFAULT_NEW_CARD_RATE`] when unset.
pub fn rate(coll: &Collection) -> Result<u32, StoreError> {
    coll.new_card_rate()
}

/// Set the rate and return **the value stored after the store's clamp**, so a field can show what
/// the app did with an out-of-range entry. Zero is kept: it is the backlog answer (ADR-0011 §5).
/// A refusal is the store's own typed error.
pub fn set_rate(coll: &mut Collection, rate: u32) -> Result<u32, StoreError> {
    coll.set_new_card_rate(rate)?;
    coll.new_card_rate()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cairn_core::log::{DEFAULT_NEW_CARD_RATE, MAX_NEW_CARD_RATE};
    use tempfile::TempDir;

    fn open() -> (Collection, TempDir, TempDir) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let coll = Collection::open(data.path(), state.path()).unwrap();
        (coll, data, state)
    }

    #[test]
    fn an_unset_rate_reads_as_the_default() {
        let (coll, _d, _s) = open();
        assert_eq!(rate(&coll).unwrap(), DEFAULT_NEW_CARD_RATE);
    }

    #[test]
    fn a_set_rate_round_trips_and_returns_what_was_stored() {
        let (mut coll, _d, _s) = open();
        assert_eq!(set_rate(&mut coll, 37).unwrap(), 37);
        assert_eq!(rate(&coll).unwrap(), 37);
    }

    #[test]
    fn a_rate_above_the_maximum_is_clamped_and_the_clamp_is_returned() {
        let (mut coll, _d, _s) = open();
        assert_eq!(set_rate(&mut coll, 1_000_000).unwrap(), MAX_NEW_CARD_RATE);
        assert_eq!(MAX_NEW_CARD_RATE, 9_999);
        assert_eq!(rate(&coll).unwrap(), 9_999);
    }

    #[test]
    fn zero_is_kept() {
        let (mut coll, _d, _s) = open();
        assert_eq!(set_rate(&mut coll, 0).unwrap(), 0);
        assert_eq!(rate(&coll).unwrap(), 0);
    }
}
