//! The `position` order key — a fractional index (ADR-0021 §3, `content` `CONTEXT.md`).
//!
//! A note's `position` is not an integer and not a counter: its defining property is **infill**,
//! that there is always a value strictly between any two neighbours, so reordering a note writes
//! **exactly one value, forever** (ADR-0021 §3). That is what makes two devices each moving a note
//! both survive — each wrote one independent value — where a renumber would be N writes and *"order
//! is a gestalt, so one lost value scrambles the whole list"*.
//!
//! The representation is *"an implementation choice so long as it has that property and one total
//! order every device computes identically"* (ADR-0021 §3): here a lexicographically-ordered string
//! over the fixed alphabet `a`–`z`, read as a base-26 fraction (`a` = 0, `z` = 25). Byte order over
//! lowercase ASCII **is** that total order, so a plain `MAX(value)`/`<` in SQLite computes the same
//! ordering the store leans on to place a new note after the current last.
//!
//! The key is **never shown** (ADR-0021 §4) — the list's own sequence is the rendering of order — so
//! this is machinery, not a user-facing value.

/// The base of the fraction: the 26 lowercase ASCII letters, `a`–`z`.
const BASE: i32 = 26;

fn digit(b: u8) -> i32 {
    i32::from(b - b'a')
}

fn undigit(d: i32) -> u8 {
    b'a' + d as u8
}

/// A key strictly between `low` and `high`, admitting the infill ADR-0021 §3 requires.
///
/// `low = None` means *before everything*, `high = None` means *after everything*, so:
/// - `between(None, None)` is the first key in an empty collection,
/// - `between(Some(last), None)` is the *"key after the current last"* creation assigns (ADR-0021 §3),
/// - `between(Some(a), Some(b))` places a note between two visible neighbours (ADR-0021 §4).
///
/// The result always sorts strictly after `low` and strictly before `high` under plain byte order,
/// and the two bounds must satisfy `low < high` when both are given — the one precondition (a caller
/// that has two neighbours from a sorted list always does).
pub fn between(low: Option<&str>, high: Option<&str>) -> String {
    debug_assert!(
        match (low, high) {
            (Some(l), Some(h)) => l < h,
            _ => true,
        },
        "between requires low < high"
    );

    let low = low.unwrap_or("").as_bytes();
    let high = high.map(str::as_bytes);
    // `high` stops constraining the moment we place a digit strictly below it: everything after that
    // digit is already less than `high`, so the upper bound opens up to the top of the base.
    let mut high_active = high.is_some();

    let mut out = Vec::new();
    let mut i = 0;
    loop {
        let l = low.get(i).map_or(0, |&b| digit(b));
        let h = if high_active {
            high.expect("active implies Some")
                .get(i)
                .map_or(0, |&b| digit(b))
        } else {
            BASE
        };
        if h - l > 1 {
            // Room for a digit strictly between: take the midpoint and stop.
            out.push(undigit((l + h) / 2));
            break;
        }
        // No room at this place — fix it to `low`'s digit and descend one place, where the gap
        // reopens because a shorter `low` reads as trailing zeros and `high` (if we dropped below it)
        // no longer binds.
        out.push(undigit(l));
        if high_active && l < h {
            high_active = false;
        }
        i += 1;
    }
    String::from_utf8(out).expect("alphabet is ASCII")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_in_an_empty_collection_is_a_midpoint() {
        // between(None, None) is the very first note's key: a value with room on both sides.
        let first = between(None, None);
        assert!(!first.is_empty());
        // Room remains before it and after it — the property the whole type exists for.
        assert!(between(None, Some(&first)) < first);
        assert!(between(Some(&first), None) > first);
    }

    #[test]
    fn appending_after_the_last_always_sorts_after_it() {
        // The creation rule (ADR-0021 §3): a key after the current last. Append many times and every
        // key must be strictly greater than the one before — a monotone run under plain `<`.
        let mut last = between(None, None);
        for _ in 0..200 {
            let next = between(Some(&last), None);
            assert!(next > last, "append produced {next:?} not after {last:?}");
            last = next;
        }
    }

    #[test]
    fn a_key_fits_strictly_between_two_neighbours() {
        // The reorder rule (ADR-0021 §4): place a note between two visible neighbours. Repeatedly
        // inserting at the same spot keeps admitting a value — infill never runs out.
        let (mut lo, hi) = (
            between(None, None),
            between(Some(&between(None, None)), None),
        );
        assert!(lo < hi);
        for _ in 0..200 {
            let mid = between(Some(&lo), Some(&hi));
            assert!(
                lo < mid && mid < hi,
                "{mid:?} is not between {lo:?} and {hi:?}"
            );
            lo = mid; // insert again just after the low neighbour — the tightest case
        }
    }

    #[test]
    fn inserting_at_the_front_sorts_before_the_first() {
        let first = between(Some(&between(None, None)), None);
        let front = between(None, Some(&first));
        assert!(front < first);
        // And still leaves room before itself.
        assert!(between(None, Some(&front)) < front);
    }

    #[test]
    fn byte_order_is_the_total_order_the_store_relies_on() {
        // The store places a new note after `MAX(position)`, which SQLite computes as a BINARY (byte)
        // comparison. Over the lowercase-ASCII alphabet that is exactly lexicographic order, so the
        // largest string byte-wise is the last note — assert the two agree on a hand-picked run.
        let mut keys = vec![
            between(None, None),
            String::from("z"),
            String::from("zn"),
            String::from("a"),
        ];
        let mut by_bytes = keys.clone();
        by_bytes.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
        keys.sort();
        assert_eq!(keys, by_bytes);
    }
}
