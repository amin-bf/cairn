//! The few sentences the egui screens say that the logic in `cairn-ops` only supplies the facts for
//! (ADR-0046 §4): *facts cross, words do not*. `cairn-ops` decides **which** statement applies — the
//! nudge is `Standard` or `Fitted`, a dormant card has a slot and kept reviews — and the renderer says
//! it. Kotlin says them from its string resources; the ADRs that fix a sentence are the specification
//! of it for both.

use cairn_core::replay::OptimisationNudge;
use cairn_ops::cards::{self, DormantEntry, DormantName};

/// The nudge sentence for the settings screen (ADR-0014 §2), from the fact `replay` derived. Carries
/// only the counts — no threshold, no badge, no verb — so the user infers "that sounds like a lot"
/// without the application making a claim it cannot support.
pub fn nudge_text(nudge: &OptimisationNudge) -> String {
    match nudge {
        OptimisationNudge::Standard { reviews_total } => format!(
            "Using the standard parameters. You've reviewed {} {}.",
            grouped(*reviews_total),
            times(*reviews_total),
        ),
        OptimisationNudge::Fitted {
            fitted_over,
            reviews_since,
        } => format!(
            "Fitted over {} {}. You've reviewed {} {} since.",
            grouped(*fitted_over),
            reviews(*fitted_over),
            grouped(*reviews_since),
            times(*reviews_since),
        ),
    }
}

/// `"review"` or `"reviews"` for a count — the nudge reads as a sentence, so a one-review fit is not
/// "1 reviews".
fn reviews(n: u64) -> &'static str {
    if n == 1 { "review" } else { "reviews" }
}

/// `"time"` or `"times"`, the same courtesy for the review tally.
fn times(n: u64) -> &'static str {
    if n == 1 { "time" } else { "times" }
}

/// A count with thousands separators — the nudge's numbers are large enough that "3,120" reads where
/// "3120" does not (ADR-0014 §2's own examples group this way).
fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let len = digits.len();
    let mut out = String::with_capacity(len + len / 3);
    for (i, ch) in digits.char_indices() {
        if i > 0 && (len - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Name a dormant entry (ADR-0018 §3), in the section's order of precedence:
///
/// 1. **A slot declared in a held definition** → its **field roles**, *"Term → Meaning"* — roles, not
///    content, because the content is exactly what may be gone. The lookup spans every shipped kind
///    (and, when the store grows them, acquired ones), which is well-formed only because a slot means
///    one question collection-wide (ADR-0017 §1).
/// 2. **The high bit set** → a cloze blank, in no definition at all → the **masked blank number**,
///    *"blank 3"*.
/// 3. **Neither** → the **bare slot**, *"card 7"* — **shown, never hidden** (ADR-0018 §3): an
///    unnameable dormant card is still history attached to this note, and omitting it is the
///    header-counter failure taken to its limit.
pub fn dormant_name(slot: u16) -> String {
    match cards::dormant_name(slot) {
        DormantName::Blank(n) => format!("blank {n}"),
        DormantName::Roles { prompt, answer } => {
            format!("{} → {}", prompt.join(", "), answer.join(", "))
        }
        DormantName::Slot(slot) => format!("card {slot}"),
    }
}

/// The one-line history of a dormant entry, worded *kept* (ADR-0018 §3, ADR-0012 §5): the reviews
/// stay in the log and reattach by themselves if the content returns, so nothing is lost.
pub fn dormant_history(entry: &DormantEntry) -> String {
    format!(
        "{} · dormant · {} reviews kept",
        dormant_name(entry.slot),
        entry.reviews
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use cairn_core::content::cloze_slot;

    #[test]
    fn the_standard_nudge_states_the_total_and_no_verb() {
        // ADR-0014 §2: "Using the standard parameters. You've reviewed 4,200 times." — a fact, no
        // threshold, no verb, no claim.
        let text = nudge_text(&OptimisationNudge::Standard {
            reviews_total: 4_200,
        });
        assert_eq!(
            text,
            "Using the standard parameters. You've reviewed 4,200 times."
        );
    }

    #[test]
    fn the_fitted_nudge_states_both_counts() {
        // ADR-0014 §2: "Fitted over 3,120 reviews. You've reviewed 1,400 times since."
        let text = nudge_text(&OptimisationNudge::Fitted {
            fitted_over: 3_120,
            reviews_since: 1_400,
        });
        assert_eq!(
            text,
            "Fitted over 3,120 reviews. You've reviewed 1,400 times since."
        );
    }

    #[test]
    fn the_nudge_carries_no_threshold_or_quality_claim() {
        // ADR-0014 §2: no badge, no colour, no verb — and nothing that reads as a judgement.
        for nudge in [
            OptimisationNudge::Standard { reviews_total: 900 },
            OptimisationNudge::Fitted {
                fitted_over: 900,
                reviews_since: 30,
            },
        ] {
            let lower = nudge_text(&nudge).to_lowercase();
            for banned in [
                "should",
                "recommend",
                "accurate",
                "better",
                "improve",
                "optimal",
                "stale",
                "due",
            ] {
                assert!(
                    !lower.contains(banned),
                    "the nudge made a claim ({banned:?}): {lower}"
                );
            }
        }
    }

    #[test]
    fn a_one_review_nudge_reads_singular() {
        assert_eq!(
            nudge_text(&OptimisationNudge::Fitted {
                fitted_over: 1,
                reviews_since: 1,
            }),
            "Fitted over 1 review. You've reviewed 1 time since."
        );
    }

    #[test]
    fn a_dormant_entry_is_named_three_ways_and_reads_kept() {
        // ADR-0018 §3: field roles for a held slot, masked number for a cloze blank, bare slot when
        // neither resolves — and the history reads *kept*, never *lost*.
        assert_eq!(dormant_name(2), "Term → Meaning", "a held fixed-arity slot");
        assert_eq!(dormant_name(0), "Front → Back");
        assert_eq!(dormant_name(cloze_slot(3)), "blank 3", "a cloze blank");
        assert_eq!(
            dormant_name(7),
            "card 7",
            "an unnameable slot is shown by bare number, never hidden"
        );

        let history = dormant_history(&DormantEntry {
            slot: 2,
            reviews: 23,
        });
        assert!(history.contains("kept"), "history reads kept");
        assert!(!history.contains("lost"), "never lost");
        assert!(history.contains("23"));
    }

    /// **The three ways ADR-0018 §3 names a dormant entry, all reached by one fixture.** The fixture
    /// (in `cairn-ops`) reaches the slots; only this can say what they are called.
    #[test]
    fn the_dormant_fixture_is_named_in_all_three_of_adr_0018_s_cases() {
        let data = tempfile::TempDir::new().unwrap();
        let state = tempfile::TempDir::new().unwrap();
        let mut coll = cairn_store::Collection::open(data.path(), state.path()).unwrap();
        let now_ms = 20_514 * 86_400_000 + 4 * 3_600_000;
        cairn_ops::fixtures::Fixture::Dormant
            .install(&mut coll, now_ms)
            .unwrap();

        let mut names: Vec<String> = Vec::new();
        for row in cairn_ops::notes::list(&coll, &cairn_ops::notes::Filter::default()).unwrap() {
            let pane = cairn_ops::cards::card_pane(&coll, row.id).unwrap();
            if let Some(warning) = &pane.warning {
                names.extend(warning.dormant.iter().map(|d| dormant_name(d.slot)));
            }
        }
        names.sort();

        assert_eq!(
            names,
            vec![
                // Case 1 — the field **roles** of a slot a held definition declares, twice, because
                // `vocab` asks in both directions. Roles and not content: the content is what is gone.
                "Meaning → Term".to_owned(),
                "Term → Meaning".to_owned(),
                // Case 2 — the high bit set, so a cloze blank in no definition at all.
                "blank 2".to_owned(),
                // Case 3 — neither, so the bare slot. Shown, never hidden (ADR-0018 §3).
                "card 7".to_owned(),
            ],
        );
    }
}
