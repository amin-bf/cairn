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
use crate::{
    Sitting, badge, body, box_badge_wording, deck, field_label, full_width_button, heading,
    surface, text,
};
use crate::{bidi, controls, fonts, frame, spacing, typography};

/// Draw the whole review destination for this frame: the count picker when no sitting is running,
/// otherwise the current card. Returns the note the user asked to **edit**, if any — the review
/// screen is one of the editor's four entrances (ADR-0021 §5), and it offers that entrance **only on
/// a revealed card** (ADR-0029 §1). Nothing is flipped on the way out: ADR-0021 §6's "counts as a
/// reveal" is retired with the pre-reveal control that needed it, so ADR-0006 §4's guarantee holds by
/// there being no route rather than by a side-effect.
// Each screen threads its own `&mut` slice of `CairnApp` state plus the frame's facts — `now_ms`,
// `today`, and now whether a thumb is driving (ADR-0035 §3). Grouping them behind a struct would
// relocate the same fields, not reduce them; the same trade `settings_screen` makes.
#[allow(clippy::too_many_arguments)]
pub(crate) fn review(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    sitting: &mut Option<Sitting>,
    showing_leeches: &mut bool,
    session_pointer: &mut Option<usize>,
    now_ms: i64,
    today: i64,
    touch: bool,
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
        //
        // **The pair keeps the primary weight** (ADR-0034 §2). This screen has no card on it, so
        // ADR-0033 §3's comparison is not about anything here, and the pointer is the whole of what
        // the screen is for.
        if let Some(count) = *session_pointer {
            body(ui, &pointer_wording(count));
            ui.add_space(spacing::gap(2));
            if controls::wide_primary(ui, "Show me").clicked() {
                *showing_leeches = true;
                *session_pointer = None;
            }
            ui.add_space(spacing::gap(1));
            if controls::wide_primary(ui, "Not now").clicked() {
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
        //
        // **It takes the primary weight** (ADR-0034 §2), and the caught-up screen is why. On a
        // Review with nothing due this is the *only control on the screen* — and three of #124's
        // five variants dropped it entirely with nothing failing, which is how little there is to
        // notice its absence. Drawn at the ordinary control weight it is a faint rectangle on an
        // otherwise empty page, which is the same defect wearing a fill.
        //
        // **And it sits on the reach line** — [ADR-0035 §1]'s second call site, and the first
        // anywhere but the grade cluster (ADR-0038 §5). §1 was argued from a thumb on Review and
        // written about *a screen*, but `frame::slack_above` had one caller, so until #155 nothing
        // had had cause to apply it or ignore it elsewhere. This screen is the only state in the
        // application where Review carries a control with an empty page under it, and it was
        // drawing the entrance directly under the statement — §1 standing as written while the app
        // did otherwise. #155 made it a page rule rather than narrowing it to Review.
        //
        // The fallback arm needs no branch here: `slack_above` returns the stated gap on a page
        // with no room, so the 560×860 window and the handset reach the two arms by arithmetic.
        //
        // [ADR-0035 §1]: ../../../../docs/adr/0035-the-vertical-anchor.md
        if !ranked.is_empty() || !suspended.is_empty() {
            ui.add_space(frame::slack_above(
                frame::page_room(ui),
                controls::HEIGHT,
                spacing::gap(3),
            ));
            if controls::wide_primary(ui, &leech_entry_wording(ranked.len(), suspended.len()))
                .clicked()
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
        } else if let Some((offered, rendered)) = next {
            // **The 10-minute checkpoint sits above the card and never replaces it**
            // (ADR-0006 §1, discharged by ADR-0034 §4). This was an `else if` branch until #134 —
            // the card was not drawn at all while the checkpoint was up, and §1 says in as many
            // words that it surfaces *"without hiding the card underneath — the reviewer can still
            // grade what they're looking at while deciding"*. The application had contradicted its
            // own accepted ADR since the checkpoint was written, nothing failed, and no test could
            // have noticed: `checkpoint_due` only becomes true after ten real minutes, which no
            // capture run and no test suite ever waits for.
            //
            // It is drawn **compact**: the sentence and two controls on one line. §1 also calls the
            // timer *"a courtesy check-in, not an enforcement mechanism"*, and a stack of
            // full-width controls above the card is how an application draws an enforcement — it
            // pushes the card 140px down the page to ask a question the reviewer did not raise.
            if s.checkpoint_due() {
                match checkpoint(ui) {
                    Some(Checkpoint::Finish) => end_sitting = true,
                    Some(Checkpoint::KeepGoing) => s.checkpoint_dismissed = true,
                    None => {}
                }
                ui.add_space(spacing::gap(3));
            }

            // A new card on screen resets the reveal and the answer-timer; the same card across
            // frames keeps them, so a reveal survives a repaint.
            // The reveal's animation resets on this same test (ADR-0037 §4): one id, snapped when
            // the card changes. Keyed on the card instead, egui's animation map — which is never
            // evicted — would retain an entry per card for the life of the process; left unreset,
            // the next card's answer is drawn fading out for the whole duration.
            let card_changed = s.shown != Some(offered.card);
            if card_changed {
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
            //
            // **The card is handed both faces and how far open it is** (ADR-0037 §3), rather than
            // being handed the answer only once it is due. Room can only be kept for a face the card
            // knows about, and keeping it is what stops the prompt jumping and what holds the type
            // tier across the reveal (§5). `t` is 0 until the tap, so nothing of the answer is on
            // screen before it — ADR-0006 §4, pinned by test.
            let badge_text = box_badge_wording(!offered.is_new, offered.box_);
            let t = crate::motion::reveal_progress(ui, card_changed, s.revealed);
            if surface::card(
                ui,
                &rendered.prompt,
                Some(rendered.answer.as_str()),
                Some(badge_text.as_str()),
                surface::REVIEW_HEIGHT,
                t,
            )
            .clicked()
            {
                s.revealed = true;
            }

            if s.revealed {
                // **Edit note rides directly under the card, and the grades sit on the reach line**
                // (ADR-0035 §1, §2). The order is still the reading order — prompt, answer, then
                // the controls — but the *distance* is not: the page's leftover height falls
                // between the card and the grades rather than below everything, so the controls
                // pressed on every card land where the thumb already is and the one pressed rarely
                // does not.
                //
                // **Above the slack rather than below the grades, and above the card was the other
                // candidate.** Both keep it out of the thumb's zone; this one is cheaper, because
                // nothing above it moves when it appears. Placed above the card it has to reserve
                // an empty row before the reveal or the card jumps down at the exact moment the eye
                // goes to the answer — judged in the hand and rejected there (ADR-0035 §2).
                //
                // Offered **only now the card is revealed** (ADR-0029 §1). The honest diagnosis of
                // most leeches is a defective card (ADR-0010 §7), and its three named forms —
                // ambiguous, too large, testing two facts at once — are all judgements about the
                // *pair*, so all three are post-reveal findings already.
                //
                // **Nothing flips the card here, and that absence is the decision.** ADR-0021 §6's
                // "entering the editor counts as a reveal" is retired with the pre-reveal control
                // it existed for: a full-width button under the card, which is itself the reveal
                // target, made a mis-tap spend the reveal on a card nobody chose to look at. So
                // ADR-0006 §4's "no grading before the answer is seen" now holds because there is
                // **no route** into the editor before the reveal — put this control back outside
                // the `revealed` branch and the guarantee is silently false again, with no rule
                // left to catch it.
                //
                // An edit that makes the card dormant still needs no mechanism: the next frame
                // re-derives the queue and simply does not offer it (ADR-0006 §2).
                ui.add_space(spacing::gap(3));
                if full_width_button(ui, "Edit note").clicked() {
                    edit_request = Some(offered.card.note);
                }

                ui.add_space(frame::slack_above(
                    frame::page_room(ui),
                    grade_cluster_height(touch),
                    spacing::gap(3),
                ));
                let pressed = grade_buttons(ui, &offered, today, touch);

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

/// What the reviewer answered the 10-minute checkpoint with.
enum Checkpoint {
    Finish,
    KeepGoing,
}

/// The 10-minute checkpoint (ADR-0006 §1, ADR-0034 §4): a courtesy check-in on **one line**, above a
/// card that stays on screen and stays gradeable.
///
/// The two controls keep their borders. A frameless pair was drawn and rejected on the one ground a
/// picture is good for — *it is not obvious that they are clickable* — so what makes this compact is
/// that each takes only the room its label needs, not that it stopped looking like a control.
fn checkpoint(ui: &mut egui::Ui) -> Option<Checkpoint> {
    let mut answer = None;
    spacing::row(ui, 2, |ui| {
        field_label(ui, "10 minutes so far.");
        if controls::snug(ui, "Finish here").clicked() {
            answer = Some(Checkpoint::Finish);
        }
        if controls::snug(ui, "Keep going").clicked() {
            answer = Some(Checkpoint::KeepGoing);
        }
    });
    answer
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
        // **The floor gets the screen** (ADR-0034 §3). This is the only Review state with no card
        // and no work in it, and it was drawn as one body sentence tucked under the heading — a
        // state given no more room than a form label. Centred at the display tier it reads as an
        // answer rather than as an absence, which is what it is: nothing is due because the work is
        // done.
        ReviewState::CaughtUp => {
            caught_up(ui);
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

/// The entrance: **one primary way in, with the sizes as a quiet second line** (ADR-0034 §2).
///
/// The picker used to be a wrapped row of four equal controls — `5` `10` `20` `All n` — with no way
/// in that was *the* way in, asking for a decision before anything had happened. The sitting size is
/// a decision most days do not want to make, so the whole queue becomes the default and the shorter
/// sittings stay one tap away.
///
/// **The second line is the link accent's first caller** (`theme::link`, ADR-0030 §5). At weak-text
/// weight it was very nearly invisible, which would have made the sizes technically present and
/// practically gone; that trade is what ADR-0034 §2 decides, and it is the reason the accent wakes.
/// The line is only drawn when it offers something — a queue of three has no *shorter* sitting to
/// pick, and a row reading `or a shorter sitting: 5 10 20` above three cards states work that does
/// not exist.
fn count_buttons(ui: &mut egui::Ui, available: usize) -> Option<usize> {
    let mut chosen = None;
    ui.add_space(spacing::gap(3));

    if controls::wide_primary(ui, &start_wording(available)).clicked() {
        chosen = Some(available);
    }

    // Strictly *shorter* — an option equal to the queue is the primary said twice.
    let shorter: Vec<usize> = [5usize, 10, 20]
        .into_iter()
        .filter(|option| *option < available)
        .collect();
    if !shorter.is_empty() {
        ui.add_space(spacing::gap(2));
        spacing::row(ui, 2, |ui| {
            field_label(ui, "or a shorter sitting:");
            for option in shorter {
                if controls::text_action(ui, &option.to_string()).clicked() {
                    chosen = Some(option);
                }
            }
        });
    }
    chosen
}

/// The primary's label: what pressing it actually does. Names the count so the default is stated
/// rather than implied — `Start — all 6` is a promise about the sitting, `Start` is not.
fn start_wording(available: usize) -> String {
    if available == 1 {
        "Start — the one card".to_owned()
    } else {
        format!("Start — all {available}")
    }
}

/// The grade controls: ***Forgot* held apart, the three passes in one segmented row**
/// (ADR-0034 §1). Returns the grade pressed, if any.
///
/// **The shape is an argument about the scale, not about space.** *Forgot* is a different kind of
/// answer — *I did not know this* — and the three passes are degrees of one answer. Four stacked
/// full-width controls say those are four rungs of a single ladder, which puts the failure grade at
/// the bottom of a scale it is not on. Holding it apart and segmenting the rest says what is true,
/// and buys back the vertical budget the card spends as a side effect rather than as the reason.
///
/// The row survives a **fourth pass grade** and that was measured rather than hoped: three segments
/// are 208px at the judging width and 163px at the application's own; four are 154 and 118, and a
/// label with its interval still fits inside 118. So a later change to
/// [ADR-0001](../../../../docs/adr/0001-scheduling-algorithm-and-grade-scale.md)'s scale does not
/// have to reopen this arrangement.
fn grade_buttons(ui: &mut egui::Ui, offered: &Offered, today: i64, touch: bool) -> Option<Grade> {
    let label = |ui: &egui::Ui, grade: Grade, name: &str| {
        let days = session::interval_preview(offered, grade, today);
        controls::grade_label(ui, name, &format!("{days}d"))
    };

    let forgot = label(ui, Grade::Forgot, "Forgot");
    let mut pressed = controls::control_job(ui, forgot, ui.available_width())
        .clicked()
        .then_some(Grade::Forgot);

    const PASSES: [(Grade, &str); 3] = [
        (Grade::Barely, "Barely"),
        (Grade::Good, "Good"),
        (Grade::Easy, "Easy"),
    ];

    // **Under a thumb the passes stack; under a pointer they stay a row** (ADR-0035 §3). A thumb
    // travels up and down freely and sideways badly, so the two ends of a segmented row flip
    // between comfortable and a stretch depending on which hand holds the phone — which is not a
    // sizing problem and cannot be fixed by a bigger target. A pointer has no such axis and crosses
    // 208px for nothing, so it keeps the row ADR-0034 §1 chose.
    //
    // *Forgot* is held apart either way, which is the half of §1 that survives: **three** units
    // here, because in a stack the gap carries the whole distinction on its own rather than sharing
    // it with a change of shape.
    if touch {
        ui.add_space(spacing::gap(3));
        for (i, (grade, name)) in PASSES.iter().enumerate() {
            if i > 0 {
                ui.add_space(spacing::gap(1));
            }
            let job = label(ui, *grade, name);
            if controls::control_job(ui, job, ui.available_width()).clicked() {
                pressed = Some(*grade);
            }
        }
        return pressed;
    }

    // Two units hold the passes apart from *Forgot*. Three was the old stacked break and it is more
    // than a row needs: the row is already a different shape, so the gap only has to separate, not
    // to carry the whole distinction on its own.
    ui.add_space(spacing::gap(2));

    let labels = PASSES
        .iter()
        .map(|(grade, name)| label(ui, *grade, name))
        .collect();
    if let Some(i) = controls::segmented(ui, labels) {
        pressed = Some(PASSES[i].0);
    }
    pressed
}

/// How tall the grade cluster draws, which [`frame::slack_above`] needs **before** drawing it.
///
/// Computed rather than measured: the cluster is a fixed composition of controls at a fixed height
/// with stated gaps, so its height is arithmetic. A prototype that varied the composition had to
/// remember last frame's measurement and carry a frame of lag; one composition needs neither.
fn grade_cluster_height(touch: bool) -> f32 {
    if touch {
        // Forgot, the break, then three passes a unit apart.
        controls::HEIGHT * 4.0 + spacing::gap(3) + spacing::gap(1) * 2.0
    } else {
        // Forgot, the break, then the segmented row.
        controls::HEIGHT * 2.0 + spacing::gap(2)
    }
}

/// The caught-up floor: **the mark, then the statement**, centred and given the screen
/// (ADR-0034 §3, [ADR-0038 §3](../../../../docs/adr/0038-the-mark-and-the-icon-rule.md)).
///
/// **The display tier is used here for something that is not a card face**, which narrows
/// [ADR-0032 §1](../../../../docs/adr/0032-the-type-scale-and-the-rhythm.md)'s *"the text actually
/// being read"* the same way [ADR-0033 §4](../../../../docs/adr/0033-the-card.md) narrowed it for a
/// card that will not fit. The scale has four sizes and nothing between 20 and 40; at 20 the whole
/// content of this screen is set at the size of the word *Review* three lines above it, which reads
/// as a caption rather than as the state. #124's variant E reached for 24 and the scale does not
/// have it.
///
/// **The mark's first appearance anywhere inside the application**, and the only one. Three other
/// homes were weighed and rejected on [The Craft](https://github.com/amin-bf/cairn/issues/149): a
/// nav strip is chrome seen four hundred times a sitting, a splash is a delay added on purpose, and
/// Settings is where you go to change things. This is the app's one *you are done* moment.
///
/// **It appears whenever nothing is due — including on a fresh install, where nothing has been
/// earned**, and that is what keeps it clear of ADR-0001 §3's quiet constraint. A picture that shows
/// up when you have done nothing cannot be a reward for doing something.
fn caught_up(ui: &mut egui::Ui) {
    ui.add_space(spacing::gap(8));
    ui.vertical_centered(|ui| {
        crate::icon(
            ui,
            fonts::MARK,
            typography::MARK,
            ui.visuals().weak_text_color(),
        );
        // **`gap(8)` is the second one on this screen**, the lead above being the first, and that is
        // rhythm rather than a copy-paste: the mark and the sentence are two objects, where the
        // sentence and its footnote are one thing said twice. Recorded because equal gaps at the top
        // of a hierarchy are exactly what a later reader tidies away.
        ui.add_space(spacing::gap(8));
        ui.label(bidi::job(
            "All caught up.",
            egui::FontId::proportional(typography::DISPLAY),
            ui.visuals().text_color(),
        ));
        ui.add_space(spacing::gap(2));
        field_label(ui, "Nothing is due right now.");
    });
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
                    false,
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

    /// **ADR-0006 §1, and the reason it went ten months unenforced.** The 10-minute checkpoint
    /// surfaces *"without hiding the card underneath — the reviewer can still grade what they're
    /// looking at while deciding"*.
    ///
    /// Until #134 the checkpoint was an `else if` arm that drew *instead of* the card, contradicting
    /// the ADR in writing. Nothing failed and nothing could have: `checkpoint_due` needs ten real
    /// minutes to elapse, which no capture run waits for and no test had ever forced — so the one
    /// state that breaks the guarantee was the one state nobody had ever looked at.
    #[test]
    fn the_ten_minute_checkpoint_never_hides_the_card() {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let mut coll = Collection::open(data.path(), state.path()).unwrap();
        coll.create_note("basic", &[("Front", "der Hund"), ("Back", "the dog")])
            .unwrap();

        const TODAY: i64 = 20_514;
        let now_ms = TODAY * 86_400_000 + 4 * 3_600_000;

        let mut sitting = Sitting::new(10, HashSet::new());
        // Wind the clock past the checkpoint rather than waiting for it — the whole point is that
        // waiting is what nobody ever did.
        sitting.started = Instant::now() - Duration::from_secs(700);
        assert!(
            sitting.checkpoint_due(),
            "the fixture must actually reach the checkpoint, or this test proves nothing"
        );
        sitting.revealed = true;

        let ctx = egui::Context::default();
        crate::theme::install(&ctx, crate::theme::ThemeChoice::Dark);
        crate::typography::install(&ctx);
        crate::spacing::install(&ctx);

        let mut sitting = Some(sitting);
        let mut showing_leeches = false;
        let mut pointer = None;
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
                    false,
                );
            })
        };

        // The first frame binds the card to the sitting and clears `revealed`; the second is the one
        // to assert on, with the card shown and the checkpoint still up.
        let _ = frame(&mut sitting, &mut coll);
        sitting.as_mut().unwrap().revealed = true;
        let drawn = drawn_text(&frame(&mut sitting, &mut coll));

        assert!(
            drawn.contains("10 minutes so far."),
            "the checkpoint should be on screen — drew: {drawn}"
        );
        assert!(
            drawn.contains("der Hund"),
            "ADR-0006 §1: the card stays visible underneath the checkpoint — drew: {drawn}"
        );
        assert!(
            drawn.contains("Good"),
            "ADR-0006 §1: the card stays *gradeable* while the checkpoint is up — drew: {drawn}"
        );
    }

    /// The entrance never offers a sitting longer than the queue, and never offers one **equal** to
    /// it either — that is the primary said a second time in a quieter voice.
    ///
    /// The shipped picker capped its options and the second line this replaced it with did not, so
    /// a queue of six was offered `5 10 20`: two of them work that does not exist.
    #[test]
    fn the_entrance_offers_only_sittings_shorter_than_the_queue() {
        for (available, expected) in [(6usize, vec![5usize]), (25, vec![5, 10, 20]), (3, vec![])] {
            let shorter: Vec<usize> = [5usize, 10, 20]
                .into_iter()
                .filter(|option| *option < available)
                .collect();
            assert_eq!(
                shorter, expected,
                "with {available} available the shorter sittings should be {expected:?}"
            );
        }
    }

    /// The primary names the count, so the default it commits to is stated rather than implied.
    #[test]
    fn the_entrance_names_what_starting_commits_to() {
        assert_eq!(start_wording(6), "Start — all 6");
        assert_eq!(start_wording(1), "Start — the one card");
    }

    /// **ADR-0035 §3, and the only thing that keeps the two axes apart.** Under a thumb every grade
    /// is a full-width control; under a pointer the three passes share one row.
    ///
    /// Asserted on the **widths actually drawn**, not on the branch taken, because the defect this
    /// guards against is a control that renders perfectly at the wrong width. Deleting the `touch`
    /// argument and always drawing the row compiles, passes every other test, and silently returns
    /// the handset to a grade row whose two ends flip between comfortable and a stretch depending on
    /// which hand is holding the phone — the finding #141 was opened for.
    #[test]
    fn the_grades_stack_under_a_thumb_and_stay_a_row_under_a_pointer() {
        const WIDTH: f32 = 392.0; // the handset's column: 448dp less two 28dp margins.

        fn control_widths(width: f32, touch: bool) -> Vec<f32> {
            let ctx = egui::Context::default();
            crate::theme::install(&ctx, crate::theme::ThemeChoice::Dark);
            crate::typography::install(&ctx);
            crate::spacing::install(&ctx);

            let offered = Offered {
                card: CardRef {
                    note: NoteId([0; 16]),
                    ordinal: 0,
                },
                box_: 1,
                is_new: true,
                memory: None,
                last_day: 0,
            };
            let out = ctx.run_ui(Default::default(), |ui| {
                ui.set_width(width);
                grade_buttons(ui, &offered, 0, touch);
            });

            fn walk(shape: &egui::Shape, fill: egui::Color32, into: &mut Vec<f32>) {
                match shape {
                    egui::Shape::Rect(r) if r.fill == fill => into.push(r.rect.width().round()),
                    egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| walk(s, fill, into)),
                    _ => {}
                }
            }
            let mut widths = Vec::new();
            for clipped in &out.shapes {
                walk(
                    &clipped.shape,
                    crate::theme::control_fill(&crate::theme::cairn_dark()),
                    &mut widths,
                );
            }
            widths
        }

        let stacked = control_widths(WIDTH, true);
        assert_eq!(
            stacked.len(),
            4,
            "a thumb gets four separate controls — drew {stacked:?}"
        );
        assert!(
            stacked.iter().all(|w| *w == WIDTH.round()),
            "every stacked grade takes the whole column — drew {stacked:?}"
        );

        let row = control_widths(WIDTH, false);
        assert_eq!(
            row.len(),
            4,
            "a pointer still gets four controls, three of them sharing a row — drew {row:?}"
        );
        assert!(
            row.iter().filter(|w| **w == WIDTH.round()).count() == 1,
            "only *Forgot* spans the column under a pointer — drew {row:?}"
        );
        assert!(
            row.iter().filter(|w| **w < WIDTH.round() / 2.0).count() == 3,
            "the three passes share one row, so each is well under half the column — drew {row:?}"
        );
    }

    /// The cluster's height is what [`frame::slack_above`] is given, so the two must agree or the
    /// bottom edge lands somewhere the ADR does not describe. Computed rather than measured, which
    /// makes this checkable without a window — and worth checking, because the arithmetic is the
    /// kind that stays plausible while being wrong by one gap.
    #[test]
    fn the_cluster_height_matches_what_the_cluster_draws() {
        let stacked = controls::HEIGHT * 4.0 + spacing::gap(3) + spacing::gap(1) * 2.0;
        let row = controls::HEIGHT * 2.0 + spacing::gap(2);
        assert_eq!(grade_cluster_height(true), stacked);
        assert_eq!(grade_cluster_height(false), row);
        assert!(
            grade_cluster_height(true) > grade_cluster_height(false),
            "the stack is the taller of the two, which is what the slack has to absorb"
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
