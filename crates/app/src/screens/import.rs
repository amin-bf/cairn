//! **The import preview** — the screen an arriving file takes before anything is written
//! ([ADR-0022](../../../docs/adr/0022-the-import-preview-and-export-report.md)), drawn by
//! [#167](https://github.com/amin-bf/cairn/issues/167) and recorded in
//! [ADR-0041](../../../docs/adr/0041-the-file-surface.md).
//!
//! **It has no destination.** A drop, a launch intent or a row in the file list puts a file in
//! [`crate::CairnApp`]'s hands, and while one is held this screen takes the page in place of whatever
//! destination was showing (ADR-0022 §6, §9, confirmed by #151). The gate — *Import* and *Cancel*, or
//! a lone *Close* — is drawn by the caller in a band pinned outside the scroll on the reach line
//! (ADR-0039 §8), which is why this module answers [`gate_for`] separately from drawing the body.
//!
//! **The plan is derived by the caller once per frame and handed in, never held** (ADR-0022 §5): the
//! body and the gate must describe the same derivation, and the next frame derives again, so a sync
//! landing under the preview changes the numbers rather than staling them.
//!
//! # Two statements never share a string
//!
//! The specimen this replaces joined a deck's name and the application's words into one line —
//! *"French A1 — updating a deck you already have"* — and a Persian name then decided the direction of
//! the whole line, drawing it as *"updating a deck you already have — فارسی"*. Correct bidirectional
//! ordering of a string that should never have been one paragraph. So a deck's name is **its own
//! line**, aligned to its own direction (ADR-0041 §2), and a name that has to sit inside a sentence —
//! *moving in from X*, *renaming your X* — goes in through [`bidi::isolate`], which orders it as a run
//! of its own.

use cairn_export::{DeckPlan, Header, Path, Plan, Refusal};

use crate::inbound::Report;
use crate::{bidi, body, frame, spacing};

/// What the pinned band offers under the screen (ADR-0022 §1, §4; ADR-0041 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Gate {
    /// *Import* as the primary with *Cancel* beside it as a text action — a plan that changes
    /// something, which is the only state in which an import can be accepted.
    Decide,
    /// A lone *Close*, as the primary. A refusal has one way out, and so does a file whose import
    /// would change nothing: ADR-0022 §4 says the no-op *"costs one dismissal"*, and an *Import* that
    /// writes nothing is a second button whose only effect is to leave the screen.
    Dismiss,
}

/// What the person pressed in the band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pressed {
    Import,
    /// *Cancel* or *Close* — both leave with nothing written and nothing recorded (ADR-0022 §1).
    Leave,
}

/// Draw the gate in the pinned band and say what was pressed (ADR-0041 §3).
///
/// *Import* is the **primary** — the one way forward on a screen with no card (ADR-0034 §2) — and
/// *Cancel* sits beside it as a **text action**, the frameless alternative that is only legible as a
/// control because a primary stands next to it (§5). A lone *Close* is the primary on its own: one
/// way out, with nothing beside it to be confused with.
pub(crate) fn gate(ui: &mut egui::Ui, gate: Gate) -> Option<Pressed> {
    match gate {
        Gate::Dismiss => crate::controls::wide_primary(ui, "Close")
            .clicked()
            .then_some(Pressed::Leave),
        Gate::Decide => {
            let mut pressed = None;
            // *Cancel* takes its own width at the column's end and *Import* every pixel left — the
            // primary's width is what remains, never a number. The primary goes in **first** so the
            // row is a control's height before *Cancel* is placed, and the smaller text centres on it:
            // placed first, *Cancel* set the row to its own height and sat at the top of the band.
            let cancel = crate::bidi::job(
                "Cancel",
                egui::TextStyle::Small.resolve(ui.style()),
                crate::theme::link(ui.visuals()),
            );
            let cancel_width = ui.fonts_mut(|f| f.layout_job(cancel).size().x)
                + 2.0 * ui.spacing().button_padding.x;
            let import_width = ui.available_width() - cancel_width - spacing::gap(1);
            ui.horizontal(|ui| {
                if crate::controls::primary(ui, "Import", import_width).clicked() {
                    pressed = Some(Pressed::Import);
                }
                ui.add_space(spacing::gap(1));
                if crate::controls::text_action(ui, "Cancel").clicked() {
                    pressed = Some(Pressed::Leave);
                }
            });
            pressed
        }
    }
}

/// Which gate a derived report is owed.
pub(crate) fn gate_for(outcome: &Result<Plan, Refusal>) -> Gate {
    match outcome {
        Ok(plan) if !changes_nothing(plan) => Gate::Decide,
        _ => Gate::Dismiss,
    }
}

/// Whether accepting this plan would write nothing: every deck unchanged, and no file-level effect.
fn changes_nothing(plan: &Plan) -> bool {
    plan.decks.iter().all(|d| d.no_change)
        && plan.adopted_kinds.is_empty()
        && plan.emptied_decks.is_empty()
}

/// The screen's heading: the file's name as it arrived — the object the person opened — or, for a
/// share that carried none (ADR-0024 §1), what the bytes say it is.
fn title(report: &Report) -> String {
    match &report.name {
        Some(name) => name.clone(),
        None => "A deck file".to_owned(),
    }
}

/// Draw the screen's body — everything above the pinned gate.
pub(crate) fn import_screen(ui: &mut egui::Ui, report: &Report) {
    ui.add_space(spacing::gap(1));
    aligned(ui, &title(report), egui::TextStyle::Heading);

    match &report.outcome {
        // A refusal replaces the preview entirely: one plain sentence, and no detail that reads as an
        // invitation to repair the file (ADR-0022 §4).
        Err(refusal) => {
            ui.add_space(spacing::gap(2));
            body(ui, &refusal_wording(refusal));
        }
        Ok(plan) => {
            header(ui, &plan.header);
            for (i, deck) in plan.decks.iter().enumerate() {
                if i > 0 {
                    ui.add_space(spacing::gap(3));
                }
                deck_block(ui, deck);
            }

            // The file-level lines, after every deck: the kinds it adopts and the held decks its moves
            // leave empty. Both are about the collection rather than about one deck in the file.
            let mut file_lines = Vec::new();
            if !plan.adopted_kinds.is_empty() {
                file_lines.push(format!(
                    "Adds a card type this build doesn't have: {}",
                    plan.adopted_kinds.join(", ")
                ));
            }
            for emptied in &plan.emptied_decks {
                file_lines.push(format!("{} will be left empty", bidi::isolate(emptied)));
            }
            if !file_lines.is_empty() {
                ui.add_space(spacing::gap(2));
                lines(ui, &file_lines);
            }

            // Always present, even when it is the only line (ADR-0022 §3): "Import" implies risk to a
            // schedule, which ADR-0005 §9 makes structurally impossible.
            ui.add_space(spacing::gap(2));
            body(ui, "Your review history is untouched.");
        }
    }
    ui.add_space(spacing::gap(2));
}

/// **The file's own claims, set apart from the effects** (ADR-0022 §7, ADR-0041 §1): in weak text,
/// above a hairline. They are what a stranger says about their own file, and they are quieter than
/// what the application says about yours — the line the whole screen is drawn along.
///
/// Absent fields are absent (§7), and a file with no header draws no rule: a hairline under nothing
/// divides nothing.
fn header(ui: &mut egui::Ui, h: &Header) {
    let byline = [
        (!h.author.is_empty()).then(|| format!("by {}", bidi::isolate(&h.author))),
        (!h.licence.is_empty()).then(|| bidi::isolate(&h.licence)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    let claims: Vec<String> = [byline, bidi::isolate(&h.description)]
        .into_iter()
        .filter(|s| !is_blank(s))
        .collect();

    if claims.is_empty() {
        ui.add_space(spacing::gap(3));
        return;
    }
    ui.add_space(spacing::gap(1));
    for (i, claim) in claims.iter().enumerate() {
        if i > 0 {
            ui.add_space(spacing::gap(1));
        }
        ui.label(bidi::job(
            claim,
            egui::TextStyle::Body.resolve(ui.style()),
            ui.visuals().weak_text_color(),
        ));
    }
    // The chrome boundary the note list already draws (ADR-0039 §2), `gap(2)` either side.
    ui.add_space(spacing::gap(2));
    frame::rule(ui);
    ui.add_space(spacing::gap(3));
}

/// Whether a string is empty once the isolate marks are ignored — `isolate("")` is two invisible
/// characters, not nothing.
fn is_blank(s: &str) -> bool {
    s.chars().all(|c| matches!(c, '\u{2068}' | '\u{2069}'))
}

/// One deck in the file: its name in the bold face, the path it takes as a caption under it, and its
/// effects indented one step beneath.
///
/// **The name and its caption follow the name's own direction; the effects do not** (ADR-0041 §2).
/// The name is content and the caption is its footnote — the list row's rule (ADR-0039 §4). The
/// effects are the application's own sentences about your collection, so they keep the interface's
/// edge whatever the deck is called.
fn deck_block(ui: &mut egui::Ui, deck: &DeckPlan) {
    let size = egui::TextStyle::Body.resolve(ui.style()).size;
    aligned_job(
        ui,
        bidi::job(
            &deck.name,
            egui::FontId::new(size, crate::fonts::bold_family()),
            ui.visuals().text_color(),
        ),
        bidi::is_rtl(&deck.name),
    );
    let path = match (deck.path, deck.no_change) {
        (_, true) => "a deck you already have",
        (Path::Update, false) => "updating a deck you already have",
        (Path::Create, false) => "new deck",
    };
    aligned_job(
        ui,
        bidi::job(
            path,
            egui::TextStyle::Small.resolve(ui.style()),
            ui.visuals().weak_text_color(),
        ),
        bidi::is_rtl(&deck.name),
    );

    ui.add_space(spacing::gap(1));
    ui.horizontal(|ui| {
        ui.add_space(spacing::gap(2));
        ui.vertical(|ui| lines(ui, &effects(deck)));
    });
}

/// A deck's effect lines, in ADR-0022 §3's order. **A line that does not apply is absent, never shown
/// as zero** — a screen of zeroes buries the one line that is not.
pub(crate) fn effects(deck: &DeckPlan) -> Vec<String> {
    if deck.no_change {
        return vec!["Nothing will change.".to_owned()];
    }
    let mut out = Vec::new();
    match (deck.new_notes, deck.already_yours) {
        (0, 0) => {}
        (0, held) => out.push(format!("{} already yours", grouped(held))),
        (new, 0) => out.push(counted(new, "new note")),
        (new, held) => out.push(format!(
            "{}, {} already yours",
            counted(new, "new note"),
            grouped(held)
        )),
    }
    for moving in &deck.moving_in {
        // *Unfiled* is the note list's own name for notes in no deck (ADR-0039 §5).
        let from = moving
            .from
            .as_deref()
            .map_or_else(|| "Unfiled".to_owned(), bidi::isolate);
        out.push(format!(
            "{} moving in from {from}",
            counted(moving.count, "note")
        ));
    }
    if deck.deleted > 0 {
        out.push(format!(
            "{} of your notes will be deleted",
            grouped(deck.deleted)
        ));
    }
    if let Some(from) = &deck.renamed_from {
        out.push(format!(
            "Renaming your {} to {}",
            bidi::isolate(from),
            bidi::isolate(&deck.name)
        ));
    }
    if deck.revision_conflict {
        out.push("Same revision as yours, with different content".to_owned());
    }
    out
}

/// Body lines, one unit apart. The preview's lines must not run together into a single block a reader
/// skims, which they did once the ambient 3px went to zero (ADR-0032 §2).
fn lines(ui: &mut egui::Ui, text: &[String]) {
    for (i, line) in text.iter().enumerate() {
        if i > 0 {
            ui.add_space(spacing::gap(1));
        }
        body(ui, line);
    }
}

/// The refusal shown in place of a preview (ADR-0022 §4). One plain message each, **with no detail
/// that reads as an invitation to repair the file** — the classic zip-traversal defect, and the
/// message is not a diagnostic channel for whoever built it.
pub(crate) fn refusal_wording(refusal: &Refusal) -> String {
    match refusal {
        Refusal::Unreadable => "This file could not be read as a deck.".to_owned(),
        Refusal::UnknownFormat(_) => "This file needs a newer version of the app.".to_owned(),
        Refusal::WrongProfile => "This is a collection archive, not a deck file.".to_owned(),
        Refusal::BrokenPath => "This file is not put together the way a deck is.".to_owned(),
        // Named as older, never as damaged (ADR-0022 §4), and it names the held deck.
        Refusal::Older { deck } => format!(
            "This is an older copy of {} than the one you have.",
            bidi::isolate(deck)
        ),
    }
}

/// `n` with its thousands grouped — *1,202*, never *1202* (ADR-0022 §3's own figures).
pub(crate) fn grouped(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `n` and its noun, singular for one — *1 note*, *6 new notes*.
pub(crate) fn counted(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{} {noun}s", grouped(n))
    }
}

/// A line in `style` and the body ink, aligned to its own direction.
fn aligned(ui: &mut egui::Ui, text: &str, style: egui::TextStyle) {
    let job = bidi::job(text, style.resolve(ui.style()), ui.visuals().text_color());
    aligned_job(ui, job, bidi::is_rtl(text));
}

/// Draw `job` against the column's right edge when `rtl`, its left otherwise.
///
/// Measured and pushed across rather than laid out right to left: a job carries no alignment, and a
/// right-to-left `Layout` puts the label's *left* edge on the container's right and runs the line off
/// the far side (`bidi::job`'s note). Wrapping is at the column, so a long name still breaks inside it.
fn aligned_job(ui: &mut egui::Ui, mut job: egui::text::LayoutJob, rtl: bool) {
    job.wrap.max_width = ui.available_width();
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    ui.horizontal(|ui| {
        if rtl {
            ui.add_space((ui.available_width() - galley.size().x).max(0.0));
        }
        ui.add(egui::Label::new(galley).selectable(false));
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use cairn_core::content::DeckId;
    use cairn_export::MovingIn;

    fn deck() -> DeckPlan {
        DeckPlan {
            id: DeckId([1; 16]),
            name: "French A1".to_owned(),
            path: Path::Update,
            new_notes: 0,
            already_yours: 0,
            moving_in: Vec::new(),
            deleted: 0,
            renamed_from: None,
            no_change: false,
            revision_conflict: false,
        }
    }

    #[test]
    fn counts_are_grouped_by_thousands() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1202), "1,202");
        assert_eq!(grouped(1_234_567), "1,234,567");
        assert_eq!(counted(1, "note"), "1 note");
        assert_eq!(counted(1240, "note"), "1,240 notes");
    }

    /// ADR-0022 §3's worked example, line for line — with the name inside a sentence isolated, so
    /// the marks are stripped here to compare the words.
    #[test]
    fn the_update_path_states_every_line_the_adr_draws() {
        let plan = DeckPlan {
            new_notes: 38,
            already_yours: 1202,
            moving_in: vec![MovingIn {
                from: Some("German".to_owned()),
                count: 12,
            }],
            deleted: 3,
            renamed_from: Some("My French".to_owned()),
            ..deck()
        };
        let plain: Vec<String> = effects(&plan)
            .iter()
            .map(|l| l.replace(['\u{2068}', '\u{2069}'], ""))
            .collect();
        assert_eq!(
            plain,
            [
                "38 new notes, 1,202 already yours",
                "12 notes moving in from German",
                "3 of your notes will be deleted",
                "Renaming your My French to French A1",
            ]
        );
    }

    /// **Absent, never zero** (ADR-0022 §3): a deck whose file adds nothing new says only what is
    /// true, and an unfiled source is named the way the note list names it.
    #[test]
    fn a_line_that_does_not_apply_is_absent() {
        let plan = DeckPlan {
            already_yours: 7,
            moving_in: vec![MovingIn {
                from: None,
                count: 1,
            }],
            ..deck()
        };
        assert_eq!(
            effects(&plan),
            ["7 already yours", "1 note moving in from Unfiled"]
        );
    }

    #[test]
    fn a_no_op_is_one_line_and_a_lone_dismissal() {
        let plan = DeckPlan {
            no_change: true,
            ..deck()
        };
        assert_eq!(effects(&plan), ["Nothing will change."]);
        let whole = Plan {
            header: Header::default(),
            decks: vec![plan],
            adopted_kinds: Vec::new(),
            emptied_decks: Vec::new(),
        };
        assert_eq!(gate_for(&Ok(whole)), Gate::Dismiss);
        assert_eq!(gate_for(&Err(Refusal::Unreadable)), Gate::Dismiss);
    }
}
