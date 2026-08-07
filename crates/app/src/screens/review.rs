//! The **Review** destination: the count picker, the running sitting, the leech screen it hangs off
//! (ADR-0010 §6), and the wording helpers each of those needs.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use leitner_core::content::{CardRef, NoteId};
use leitner_core::log::{DEFAULT_NEW_CARD_RATE, DayScale};
use leitner_core::replay::{Leech, Replayed, leeches, replay};
use leitner_core::scheduling::Grade;
use leitner_store::Collection;

use crate::session::{self, Offered, ReviewState};
use crate::{
    Sitting, badge, body, box_badge_wording, card_face, deck, full_width_button, heading, text,
};

/// Draw the whole review destination for this frame: the count picker when no sitting is running,
/// otherwise the current card. Returns the note the user asked to **edit**, if any — the review
/// screen is one of the editor's four entrances (ADR-0021 §5), and opening it counts as a reveal
/// (ADR-0021 §6), which is why the card is flipped here before the request leaves.
pub(crate) fn review(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    sitting: &mut Option<Sitting>,
    showing_leeches: &mut bool,
    session_pointer: &mut Option<usize>,
    now_ms: i64,
    today: i64,
) -> Option<NoteId> {
    // Everything on screen is derived from the log this frame — there is no cached session state to
    // fall out of step with it. The new-card rate and the notes' authored positions ride the mutable
    // surface (ADR-0011 §5, §7), read fresh alongside the log; suspension is the per-card flag beside
    // them (ADR-0010 §8), so the queue now refuses a suspended card wherever it would appear.
    let current = deck::current_cards(coll).unwrap_or_default();
    let positions = deck::note_positions(coll).unwrap_or_default();
    let rate = coll.new_card_rate().unwrap_or(DEFAULT_NEW_CARD_RATE) as usize;
    let suspended = coll.suspended().unwrap_or_default();
    let lines = coll.log_lines().unwrap_or_default();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let replayed = replay(&current, &refs);
    let queue = session::compose(&current, &positions, &replayed, today, rate, &suspended);
    // The leech list is a query over replayed history (ADR-0010 §1, §4), derived fresh every frame —
    // never stored, always current, self-clearing. Suspension does not touch detection: a suspended
    // card is still a leech if its record says so, and has its permanent home below.
    let ranked = leeches(&replayed, today);
    let total = current.len();

    // The leech screen and the end-of-session pointer both hang off Review (ADR-0010 §6), and only
    // when no sitting is running: mid-sitting the card is the sole speaker (ADR-0006). The picker sits
    // last, so a durable entry to the leech screen can precede it.
    if sitting.is_none() {
        if *showing_leeches {
            heading(ui, "Leeches");
            ui.add_space(8.0);
            let edit = leech_screen(ui, coll, &ranked, &suspended, &replayed);
            ui.add_space(8.0);
            if full_width_button(ui, "Back to review").clicked() {
                *showing_leeches = false;
            }
            return edit;
        }

        heading(ui, "Review");
        ui.add_space(8.0);

        // The end-of-session pointer: a plain statement that N cards are costing a lot, with a way
        // through to the list where the decision is made — never a decision point itself (ADR-0010
        // §6). Dismissing it or tapping through clears it; it is shown once and never nags.
        if let Some(count) = *session_pointer {
            body(ui, &pointer_wording(count));
            ui.add_space(8.0);
            if full_width_button(ui, "Show me").clicked() {
                *showing_leeches = true;
                *session_pointer = None;
            }
            if full_width_button(ui, "Not now").clicked() {
                *session_pointer = None;
            }
            return None;
        }

        // Whether the collection has *any* history — the one fact the queue cannot carry, and what
        // separates a first look from a finished day (`ReviewState::of`). A card appears in
        // `replayed.cards` exactly when it has been reviewed.
        let reviewed_ever = !replayed.cards.is_empty();
        if let Some(count) = picker(ui, &queue, total, reviewed_ever) {
            // Snapshot the cards that are already leeches, so the pointer at this sitting's end covers
            // only what crosses during it (ADR-0010 §6).
            let before = ranked.iter().map(|l| l.card).collect();
            *sitting = Some(Sitting::new(count, before));
        }
        // The durable way back to the leech screen (ADR-0010 §6): the notice is the discovery path,
        // this is the place to return to. Offered whenever any card is a leech or is suspended (whose
        // permanent home this is, ADR-0010 §8), below the picker so it never competes with it.
        if !ranked.is_empty() || !suspended.is_empty() {
            ui.add_space(12.0);
            if full_width_button(ui, &leech_entry_wording(ranked.len(), suspended.len())).clicked()
            {
                *showing_leeches = true;
            }
        }
        return None;
    }

    heading(ui, "Review");
    ui.add_space(8.0);

    // A running sitting: keep the frame ticking so the 10-minute checkpoint can surface without an
    // input event (immediate mode has nowhere to wait — client-stack rule 4).
    ui.ctx().request_repaint_after(Duration::from_secs(1));

    // The next card is the head of the **live** queue that still renders — re-derived this frame, so
    // a card failed a moment ago is back in the running (ADR-0011 §9) and a card just made dormant is
    // gone. `find` stops at the first that renders, which is virtually always the head; the scan only
    // matters for the mid-edit race where the head went dormant this very frame.
    let next = queue.sitting(usize::MAX).into_iter().find_map(|offered| {
        deck::render(coll, offered.card)
            .ok()
            .flatten()
            .map(|rendered| (offered, rendered))
    });

    let mut end_sitting = false;
    let mut edit_request: Option<NoteId> = None;
    {
        let s = sitting.as_mut().expect("just checked it is Some");

        if s.graded >= s.chosen {
            // Reaching the chosen count of **gradings** ends the sitting (ADR-0011 §9, issue #94).
            end_sitting = true;
        } else if s.checkpoint_due() {
            body(ui, "You've been reviewing for 10 minutes.");
            ui.add_space(8.0);
            if full_width_button(ui, "Finish here").clicked() {
                end_sitting = true;
            }
            if full_width_button(ui, "Keep going").clicked() {
                s.checkpoint_dismissed = true;
            }
        } else if let Some((offered, rendered)) = next {
            // A new card on screen resets the reveal and the answer-timer; the same card across
            // frames keeps them, so a reveal survives a repaint.
            if s.shown != Some(offered.card) {
                s.shown = Some(offered.card);
                s.revealed = false;
                s.card_shown = Instant::now();
            }

            // Progress counts gradings against the chosen count (ADR-0011 §9), so the bar moves on
            // every grade press — a lapse re-show included — never freezing when the user struggles.
            body(ui, &format!("{} of {}", s.graded, s.chosen));
            ui.add_space(8.0);

            // Reveal is tap-the-card: the prompt is one wide button, and clicking it shows the back.
            // Identical by touch and by mouse — egui does not distinguish them.
            if card_face(ui, &rendered.prompt).clicked() {
                s.revealed = true;
            }

            // Edit this note, at any point in the card's life (ADR-0021 §6): the honest diagnosis of
            // most leeches is a defective card, and the moment to fix it is when it is in front of
            // you, not twenty cards later. Opening the editor **counts as a reveal** — the editor
            // shows the back, so without flipping the card here ADR-0006 §4's "no grading before the
            // answer is seen" would be quietly false. An edit that makes the card dormant needs no
            // mechanism: the next frame re-derives the queue and simply does not offer it.
            ui.add_space(4.0);
            if full_width_button(ui, "Edit note").clicked() {
                s.revealed = true;
                edit_request = Some(offered.card.note);
            }

            if s.revealed {
                ui.add_space(4.0);
                card_face(ui, &rendered.answer);

                // The box badge appears only after reveal, is non-interactive, and reports
                // durability — never a queue (scheduling `CONTEXT.md`). A card with no review history
                // reads `new`, never `Box 1`, which would state a durability nothing has measured
                // (ADR-0006 §6).
                ui.add_space(4.0);
                badge(ui, &box_badge_wording(!offered.is_new, offered.box_));

                ui.add_space(12.0);
                if let Some(grade) = grade_buttons(ui, &offered, today) {
                    let duration_ms = s.card_shown.elapsed().as_millis() as u64;
                    // A failed append drops this one review rather than wedging the session: the next
                    // frame re-derives the queue from whatever did commit. Surfacing write errors is
                    // a later ticket.
                    let _ = coll.append_review(
                        offered.card,
                        grade,
                        now_ms,
                        DayScale::default(),
                        duration_ms,
                    );
                    // A grading advances the count whatever the grade — the re-derived queue decides
                    // whether the card returns (a lapse) or leaves (a pass), not this counter.
                    s.graded += 1;
                    s.revealed = false;
                    s.shown = None;
                }
            }
        } else {
            // The queue emptied before the chosen count — every due card passed and the day's new
            // cards are used up. Nothing is stored; the sitting simply ends (ADR-0006 §8).
            end_sitting = true;
        }
    }

    if end_sitting {
        // The end-of-session pointer covers only leeches that crossed **this** sitting (ADR-0010 §6):
        // the leeches now, minus those already crossed when it began. The `before` snapshot lived in
        // the sitting, which is in-memory, so this needs no stored dismissal or last-seen marker.
        if let Some(s) = sitting.as_ref() {
            let crossed = session::crossed_this_session(&s.leeches_at_start, &ranked);
            *session_pointer = (!crossed.is_empty()).then_some(crossed.len());
        }
        *sitting = None;
    }
    edit_request
}

/// The end-of-session pointer's wording (ADR-0010 §6): a plain statement of cost and an offer to look,
/// never a decision made in the moment. Kept pure so the sentence is testable without a window.
fn pointer_wording(count: usize) -> String {
    if count == 1 {
        "1 card is costing you a lot. Take a look?".to_owned()
    } else {
        format!("{count} cards are costing you a lot. Take a look?")
    }
}

/// The durable leech-screen entry's label (ADR-0010 §6, §8): names how many cards are leeches and how
/// many are suspended, so the button that leads to their permanent home says what it holds.
fn leech_entry_wording(leeches: usize, suspended: usize) -> String {
    match (leeches, suspended) {
        (l, 0) => format!("Leeches ({l})"),
        (0, s) => format!("Suspended ({s})"),
        (l, s) => format!("Leeches ({l}) · suspended ({s})"),
    }
}

/// The leech screen (ADR-0010 §4, §6, §7, §8): the ranked list of cards costing the user, worst first,
/// each offering **edit** (primary), **suspend** and **delete** — and never a tag, which would publish
/// a private struggle into a deck (ADR-0010 §7); plus the **permanent** section of suspended cards,
/// each with **unsuspend** (ADR-0010 §8). Returns the note to edit, if any — the shared editor
/// entrance (ADR-0021 §5). The exact visual design is the pass's; what is fixed is the ranking, the
/// three actions, the never-a-tag, and the suspended section's permanence.
///
/// A suspended card is not listed among the active leeches even when its history still qualifies — it
/// lives in the suspended section instead, so it is named once, not twice. Actions are collected while
/// drawing and applied after, so the immutable reads that render each card's preview do not fight the
/// mutable writes.
fn leech_screen(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    ranked: &[Leech],
    suspended: &HashSet<CardRef>,
    replayed: &Replayed,
) -> Option<NoteId> {
    // Render each card's preview text up front, while the collection is only read; the owned strings
    // then outlive the immutable borrow so the action writes below are free to take `&mut coll`.
    let active: Vec<(CardRef, String, u32, u32)> = ranked
        .iter()
        .filter(|leech| !suspended.contains(&leech.card))
        .map(|leech| {
            let reviews = replayed
                .cards
                .get(&leech.card)
                .map_or(0, |state| state.review_count);
            (
                leech.card,
                card_preview(coll, leech.card),
                leech.failure_days,
                reviews,
            )
        })
        .collect();
    // The suspended section is ordered by card identity so two devices show it the same way.
    let mut suspended_sorted: Vec<CardRef> = suspended.iter().copied().collect();
    suspended_sorted.sort_by_key(|card| card.encode());
    let suspended_rows: Vec<(CardRef, String)> = suspended_sorted
        .iter()
        .map(|&card| (card, card_preview(coll, card)))
        .collect();

    let mut edit: Option<NoteId> = None;
    let mut suspend: Option<CardRef> = None;
    let mut unsuspend: Option<CardRef> = None;
    let mut delete: Option<NoteId> = None;

    // The floor is what lets the empty state speak plainly (ADR-0010 §4): nothing is hurting.
    if active.is_empty() && suspended_rows.is_empty() {
        body(ui, "Nothing is costing you a lot right now.");
        return None;
    }

    if !active.is_empty() {
        body(ui, "These keep catching you out — worst first.");
        ui.add_space(4.0);
        for (card, preview, days, reviews) in &active {
            // The cost, made concrete (ADR-0010 §6): failure days and how many reviews they took.
            badge(ui, &format!("{days} bad days · {reviews} reviews"));
            ui.horizontal(|ui| {
                if ui.button(text(ui, preview)).clicked() {
                    edit = Some(card.note); // edit is the primary action (ADR-0010 §7)
                }
                if ui.button(text(ui, "Suspend")).clicked() {
                    suspend = Some(*card);
                }
                if ui.button(text(ui, "Delete")).clicked() {
                    delete = Some(card.note);
                }
            });
        }
    }

    if !suspended_rows.is_empty() {
        ui.add_space(12.0);
        // Suspended cards have a permanent home here (ADR-0010 §8) — their own section, always, with
        // unsuspend available. Never a one-way door.
        body(ui, "Suspended — not shown in review.");
        ui.add_space(4.0);
        for (card, preview) in &suspended_rows {
            ui.horizontal(|ui| {
                if ui.button(text(ui, preview)).clicked() {
                    edit = Some(card.note);
                }
                if ui.button(text(ui, "Unsuspend")).clicked() {
                    unsuspend = Some(*card);
                }
            });
        }
    }

    // Apply the one action taken this frame. Each is a mutable-surface write (suspend/unsuspend) or the
    // note delete (ADR-0004 §7); a failed write is dropped and the next frame re-derives, as elsewhere.
    if let Some(card) = suspend {
        let _ = coll.suspend(card);
    }
    if let Some(card) = unsuspend {
        let _ = coll.unsuspend(card);
    }
    if let Some(note) = delete {
        let _ = coll.mutable_set("note", &note.0, "deleted", Some("true"));
    }
    edit
}

/// One card's prompt, for a leech row — the card the user will recognise. A dormant card (its content
/// no longer generated, ADR-0002 §7) renders nothing, so it falls back to a plain marker rather than a
/// blank row: a suspended card whose note was edited away still needs a line to carry its unsuspend.
fn card_preview(coll: &Collection, card: CardRef) -> String {
    deck::render(coll, card)
        .ok()
        .flatten()
        .map(|rendered| rendered.prompt)
        .filter(|prompt| !prompt.trim().is_empty())
        .unwrap_or_else(|| "(card no longer generated)".to_owned())
}

/// The count picker and the explicit worded states (issue #94). Returns the chosen sitting size when
/// the user starts one.
fn picker(
    ui: &mut egui::Ui,
    queue: &session::Queue,
    total: usize,
    reviewed_ever: bool,
) -> Option<usize> {
    let available = queue.available();
    match ReviewState::of(queue, total, reviewed_ever) {
        ReviewState::Empty => {
            body(ui, "No cards yet. Add a note to start reviewing.");
            None
        }
        ReviewState::CaughtUp => {
            body(ui, "All caught up — nothing is due right now.");
            None
        }
        ReviewState::NewDeck { new } => {
            body(
                ui,
                "A fresh deck. These cards are new — start whenever you like.",
            );
            count_buttons(ui, new)
        }
        // Nothing due in a collection that *has* been reviewed: the day's repeats are done and the
        // new-card rate still has room. Never "a fresh deck", which is a false statement about a
        // collection with history behind it (ADR-0006 §8, whose two states predate ADR-0011's rate).
        ReviewState::NewOnly { new } => {
            body(ui, &new_only_wording(new));
            count_buttons(ui, new)
        }
        ReviewState::Due { due, new, backlog } => {
            if backlog {
                // Backlog is framed, never a bare number (issue #94, ADR-0001 §3).
                body(
                    ui,
                    "Plenty due — pick a comfortable size, the rest will keep.",
                );
            } else if new > 0 {
                body(ui, &format!("{due} due, plus {new} new. Pick a size."));
            } else {
                body(ui, &format!("{due} due. Pick a size."));
            }
            count_buttons(ui, available)
        }
    }
}

/// A row of sitting-size choices, each capped by what is actually available so the picker never
/// offers more work than exists.
fn count_buttons(ui: &mut egui::Ui, available: usize) -> Option<usize> {
    let mut chosen = None;
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        for option in [5usize, 10, 20] {
            if option <= available && ui.button(text(ui, &option.to_string())).clicked() {
                chosen = Some(option);
            }
        }
        // "All" is always meaningful when anything is available.
        if available > 0 && ui.button(text(ui, &format!("All {available}"))).clicked() {
            chosen = Some(available);
        }
    });
    chosen
}

/// The four grade buttons: full-width, stacked, with a visual break between 1 and 2 and an
/// illustrative interval preview on each (issue #94). Returns the grade pressed, if any.
fn grade_buttons(ui: &mut egui::Ui, offered: &Offered, today: i64) -> Option<Grade> {
    let mut pressed = None;
    let mut button = |ui: &mut egui::Ui, grade: Grade, label: &str| {
        let days = session::interval_preview(offered, grade, today);
        if full_width_button(ui, &format!("{label}   ·   {days}d")).clicked() {
            pressed = Some(grade);
        }
    };
    button(ui, Grade::Forgot, "Forgot");
    // The visual break between the failure grade and the passes.
    ui.add_space(12.0);
    button(ui, Grade::Barely, "Barely");
    button(ui, Grade::Good, "Good");
    button(ui, Grade::Easy, "Easy");
    pressed
}

/// The picker's statement when nothing is due but cards have never been seen: the fact, then the
/// invitation, and **no claim that the deck is fresh** — the collection has history behind it.
///
/// It states nothing about being behind, because the reviewer is not: reaching this state means the
/// day's repeats are finished and only ADR-0011 §2's rate stands between them and the rest.
fn new_only_wording(new: usize) -> String {
    if new == 1 {
        "Nothing due right now. One new card, whenever you like.".to_owned()
    } else {
        format!("Nothing due right now. {new} new cards, whenever you like.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pointer_states_a_cost_and_is_singular_or_plural() {
        // ADR-0010 §6: the pointer is a plain statement of cost, not a bare count and not a decision.
        assert_eq!(
            pointer_wording(1),
            "1 card is costing you a lot. Take a look?"
        );
        assert_eq!(
            pointer_wording(3),
            "3 cards are costing you a lot. Take a look?"
        );
    }

    #[test]
    fn the_leech_entry_names_leeches_suspended_or_both() {
        // ADR-0010 §6, §8: the durable entry to the leech screen says what it holds — the active
        // leeches, the suspended cards whose permanent home it is, or both.
        assert_eq!(leech_entry_wording(2, 0), "Leeches (2)");
        assert_eq!(leech_entry_wording(0, 3), "Suspended (3)");
        assert_eq!(leech_entry_wording(2, 3), "Leeches (2) · suspended (3)");
    }

    #[test]
    fn nothing_due_with_new_cards_left_never_calls_the_deck_fresh() {
        // The sentence a reviewer with history gets when the day's repeats are done. It states the
        // fact and invites, and it must not claim freshness (ADR-0006 §8) or imply falling behind.
        assert_eq!(
            new_only_wording(1),
            "Nothing due right now. One new card, whenever you like."
        );
        assert_eq!(
            new_only_wording(4),
            "Nothing due right now. 4 new cards, whenever you like."
        );
    }
}
