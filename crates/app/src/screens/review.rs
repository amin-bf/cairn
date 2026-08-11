//! The **Review** destination: the count picker, the running sitting, the leech screen it hangs off
//! (ADR-0010 §6), and the wording helpers each of those needs.

use std::collections::HashSet;
use std::time::{Duration, Instant};

use cairn_core::content::{CardRef, NoteId};
use cairn_core::log::{DEFAULT_NEW_CARD_RATE, DayScale};
use cairn_core::replay::{Leech, Replayed, leeches, replay};
use cairn_core::scheduling::Grade;
use cairn_store::Collection;

use crate::session::{self, Offered, ReviewState};
use crate::spacing;
use crate::{
    Sitting, badge, body, box_badge_wording, deck, full_width_button, heading, surface, text,
};

/// Draw the whole review destination for this frame: the count picker when no sitting is running,
/// otherwise the current card. Returns the note the user asked to **edit**, if any — the review
/// screen is one of the editor's four entrances (ADR-0021 §5), and it offers that entrance **only on
/// a revealed card** (ADR-0029 §1). Nothing is flipped on the way out: ADR-0021 §6's "counts as a
/// reveal" is retired with the pre-reveal control that needed it, so ADR-0006 §4's guarantee holds by
/// there being no route rather than by a side-effect.
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
            ui.add_space(spacing::gap(2));
            let edit = leech_screen(ui, coll, &ranked, &suspended, &replayed);
            ui.add_space(spacing::gap(2));
            if full_width_button(ui, "Back to review").clicked() {
                *showing_leeches = false;
            }
            return edit;
        }

        heading(ui, "Review");
        ui.add_space(spacing::gap(2));

        // The end-of-session pointer: a plain statement that N cards are costing a lot, with a way
        // through to the list where the decision is made — never a decision point itself (ADR-0010
        // §6). Dismissing it or tapping through clears it; it is shown once and never nags.
        if let Some(count) = *session_pointer {
            body(ui, &pointer_wording(count));
            ui.add_space(spacing::gap(2));
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
            ui.add_space(spacing::gap(3));
            if full_width_button(ui, &leech_entry_wording(ranked.len(), suspended.len())).clicked()
            {
                *showing_leeches = true;
            }
        }
        return None;
    }

    heading(ui, "Review");
    ui.add_space(spacing::gap(2));

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
            ui.add_space(spacing::gap(2));
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
            ui.add_space(spacing::gap(2));

            // **One card with two faces, not two cards** (ADR-0033 §1). Reveal is tap-the-card:
            // the whole surface is the target, and clicking it shows the back. Identical by touch
            // and by mouse — egui does not distinguish them.
            //
            // The box badge rides **inside** the card, because it reports the durability of *this
            // card* and a footnote on the page below could as easily belong to the screen. It
            // appears only after the reveal and is non-interactive, reporting durability and never
            // a queue (scheduling `CONTEXT.md`); a card with no history reads `new`, never `box 1`,
            // which would state a durability nothing has measured (ADR-0006 §6). Which corner it
            // takes follows the prompt's script — see `surface`.
            let badge_text = s
                .revealed
                .then(|| box_badge_wording(!offered.is_new, offered.box_));
            let answer = s.revealed.then_some(rendered.answer.as_str());
            if surface::card(
                ui,
                &rendered.prompt,
                answer,
                badge_text.as_deref(),
                surface::REVIEW_HEIGHT,
            )
            .clicked()
            {
                s.revealed = true;
            }

            if s.revealed {
                ui.add_space(spacing::gap(3));
                let pressed = grade_buttons(ui, &offered, today);

                // Edit this note — offered **only now the card is revealed** (ADR-0029 §1). The
                // honest diagnosis of most leeches is a defective card (ADR-0010 §7), and its three
                // named forms — ambiguous, too large, testing two facts at once — are all judgements
                // about the *pair*, so all three are post-reveal findings already.
                //
                // **Nothing flips the card here, and that absence is the decision.** ADR-0021 §6's
                // "entering the editor counts as a reveal" is retired with the pre-reveal control it
                // existed for: a full-width button under the card, which is itself the reveal target,
                // made a mis-tap spend the reveal on a card nobody chose to look at. So ADR-0006 §4's
                // "no grading before the answer is seen" now holds because there is **no route** into
                // the editor before the reveal — put this control back outside the `revealed` branch
                // and the guarantee is silently false again, with no rule left to catch it.
                //
                // An edit that makes the card dormant still needs no mechanism: the next frame
                // re-derives the queue and simply does not offer it (ADR-0006 §2).
                ui.add_space(spacing::gap(3));
                if full_width_button(ui, "Edit note").clicked() {
                    edit_request = Some(offered.card.note);
                }

                if let Some(grade) = pressed {
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
        ui.add_space(spacing::gap(1));
        for (i, (card, preview, days, reviews)) in active.iter().enumerate() {
            // Two units between leeches, one between a leech's badge and its own controls — so the
            // entry binds to its cost rather than to the entry below it. Both were the ambient 3px
            // until ADR-0032, which is to say neither was chosen and the pair read as one list of
            // alternating strips.
            if i > 0 {
                ui.add_space(spacing::gap(2));
            }
            // The cost, made concrete (ADR-0010 §6): failure days and how many reviews they took.
            badge(ui, &format!("{days} bad days · {reviews} reviews"));
            ui.add_space(spacing::gap(1));
            spacing::row(ui, 1, |ui| {
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
        ui.add_space(spacing::gap(3));
        // Suspended cards have a permanent home here (ADR-0010 §8) — their own section, always, with
        // unsuspend available. Never a one-way door.
        body(ui, "Suspended — not shown in review.");
        ui.add_space(spacing::gap(1));
        for (i, (card, preview)) in suspended_rows.iter().enumerate() {
            if i > 0 {
                ui.add_space(spacing::gap(1));
            }
            spacing::row(ui, 1, |ui| {
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
    ui.add_space(spacing::gap(2));
    spacing::row_wrapped(ui, 1, |ui| {
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
    ui.add_space(spacing::gap(3));
    button(ui, Grade::Barely, "Barely");
    // **One unit, stated.** These three were never separated by anything anyone chose: they leaned
    // on egui's ambient 3px, and zeroing it (ADR-0032 §2) fused them into a single slab. Three units
    // hold the passes apart from *Forgot*; one holds them apart from each other, which is the
    // grouping the old 3-against-15 accidentally expressed and the only thing being preserved here.
    // Whether the three become a segmented row instead is #134's, not this ticket's.
    ui.add_space(spacing::gap(1));
    button(ui, Grade::Good, "Good");
    ui.add_space(spacing::gap(1));
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
    use tempfile::TempDir;

    /// Every string a frame actually drew, so a control's presence can be **asserted** rather than
    /// assumed from reading the branch it sits in.
    ///
    /// Text reaches the frame as laid-out galleys inside its shapes, and shapes nest — a widget's
    /// contents arrive as a `Shape::Vec` — so this recurses. Reading the galley rather than the
    /// source string is what makes the assertion about what the user can see.
    fn drawn_text(out: &egui::FullOutput) -> String {
        fn walk(shape: &egui::Shape, into: &mut String) {
            match shape {
                egui::Shape::Text(t) => {
                    into.push_str(t.galley.text());
                    into.push('\n');
                }
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, into)),
                _ => {}
            }
        }
        let mut text = String::new();
        for clipped in &out.shapes {
            walk(&clipped.shape, &mut text);
        }
        text
    }

    /// **ADR-0029 §1, and the only reason it is enforceable.** The edit entrance appears on a
    /// revealed card and never before one.
    ///
    /// Nothing fails when this drifts. Moving the control out of the `revealed` branch compiles,
    /// renders and passes every other test — and silently restores the hazard ADR-0029 removed: the
    /// button sits under the card, the card *is* the reveal target (ADR-0006 §3), and a mis-tap would
    /// open the editor on an unrevealed card. That used to be guarded by ADR-0021 §6's "entering the
    /// editor counts as a reveal", which ADR-0029 **retired** along with the state that needed it —
    /// so ADR-0006 §4's "no self-grading before the answer is seen" now rests on there being *no
    /// route*, with no rule left underneath to catch a regression.
    #[test]
    fn the_edit_entrance_appears_only_once_the_card_is_revealed() {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let mut coll = Collection::open(data.path(), state.path()).unwrap();
        coll.create_note("basic", &[("Front", "der Hund"), ("Back", "the dog")])
            .unwrap();

        const TODAY: i64 = 20_514;
        let now_ms = TODAY * 86_400_000 + 4 * 3_600_000;

        let mut sitting = Some(Sitting::new(10, HashSet::new()));
        let mut showing_leeches = false;
        let mut pointer = None;
        let ctx = egui::Context::default();
        // The card face draws at the display tier, and **an unregistered `TextStyle::Name` panics
        // rather than falling back** to a default size — so a context that draws a card must carry
        // the scale exactly as `CairnApp::new` installs it. That is the loud failure, and it is the
        // one this crate wants: the alternative, resolving defensively at the call site, would draw
        // a 40px face at 13px on any path that forgot, with nothing failing.
        crate::typography::install(&ctx);
        crate::spacing::install(&ctx);

        let mut frame = |sitting: &mut Option<Sitting>, coll: &mut Collection| {
            ctx.run_ui(Default::default(), |ui| {
                review(
                    ui,
                    coll,
                    sitting,
                    &mut showing_leeches,
                    &mut pointer,
                    now_ms,
                    TODAY,
                );
            })
        };

        // The first frame is what binds the card to the sitting — `shown` is `None` until a card is
        // offered, and the reveal resets whenever it changes. So it must run before `revealed` is set,
        // or the next frame would clear it again.
        let unrevealed = frame(&mut sitting, &mut coll);
        let text = drawn_text(&unrevealed);
        assert!(
            text.contains("der Hund"),
            "the prompt should be on screen, so the absence below is about the control and not \
             about an empty queue — drew: {text}"
        );
        assert!(
            !text.contains("Edit note"),
            "ADR-0029 §1: no route into the editor exists before the reveal — drew: {text}"
        );

        sitting.as_mut().unwrap().revealed = true;
        let revealed = drawn_text(&frame(&mut sitting, &mut coll));
        assert!(
            revealed.contains("the dog"),
            "the answer should be on screen once revealed — drew: {revealed}"
        );
        assert!(
            revealed.contains("Edit note"),
            "ADR-0021 §5: the review screen is one of the editor's four entrances, and ADR-0029 \
             narrows *when* it is offered rather than removing it — drew: {revealed}"
        );
    }

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
