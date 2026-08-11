//! The **Settings** destination: the reset control, the new-card-rate and optimise controls, the
//! sync surface, and the **temporary** specimens. The specimens are development controls, not
//! specified features — each keeps the doc comment marking it so.

use cairn_core::content::{DeckId, NoteId};
use cairn_core::log::{DEFAULT_NEW_CARD_RATE, DayScale};
use cairn_store::Collection;

use crate::screens::enrolment::enrolment_screen;
use crate::{
    bidi, body, field_label, fonts, full_width_button, heading, inbound, listing, optimise, sync,
    text_field,
};
use crate::{spacing, typography};

/// **Temporary, and not a specified feature.** What the hand-off specimen carries between frames.
#[derive(Default)]
pub(crate) struct HandOff {
    /// The name the platform reported for the last successful [`cairn_export::platform::put`] —
    /// **the written one, never the requested one** (ADR-0022 §10). This is what `hand_off` is then
    /// asked for, so the specimen exercises the read-back rather than asserting it.
    written: Option<String>,
    /// The last thing either button had to say, verbatim: a read-back name or a refusal. Held rather
    /// than logged because a handset run has no console the person holding it can read.
    said: String,
}

/// **Temporary, and not a specified feature.** What the file-list specimen carries between frames.
#[derive(Default)]
pub(crate) struct FileList {
    /// The rows from the last enumeration — `None` until the list button has been pressed once, so an
    /// empty list reads as *"nothing here"* rather than *"not asked yet"*. Only names and sniffs are
    /// held, never the bytes: the row description is the cheap sniff and inflates nothing (ADR-0022
    /// §11); the bytes are re-read from the seam when a row is selected.
    rows: Option<Vec<listing::Listed>>,
    /// The last thing the enumeration had to say — a seam refusal, verbatim. Held rather than logged
    /// because a handset run has no console (as [`HandOff`]).
    said: String,
}

/// The **Settings** destination (ADR-0021 §1), holding the sync surface (ADR-0015 §12, ADR-0019 §1).
///
/// This renders the *surface* — the words and the refusals — for the not-yet-enrolled device: the
/// promise, the entry to enrolment, and the durable removal route. The enrolled surface (the resting
/// "Last caught up ⟨when⟩", the connected account, Sync now, the device list, Disconnect and the
/// history cutoff) is modelled and proven in `sync`, but it needs a live grant, and the device flow
/// that obtains one carries the network this environment lacks (ADR-0013 §11) — so it is wired when
/// that mechanism lands, not faked here. What is fixed now is what each surface *says*.
// Each screen threads its own `&mut` slice of `CairnApp` state plus the frame's `now_ms`; grouping
// them behind a struct would only relocate the same fields, not reduce them.
#[allow(clippy::too_many_arguments)]
pub(crate) fn settings_screen(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    setting_up: &mut bool,
    rate_buffer: &mut Option<String>,
    optimise_job: &mut Option<optimise::OptimiseJob>,
    optimise_done: &mut bool,
    handoff: &mut HandOff,
    inbound: &mut Option<inbound::Inbound>,
    file_list: &mut FileList,
    now_ms: i64,
) -> bool {
    heading(ui, "Settings");
    ui.add_space(spacing::gap(2));

    if *setting_up {
        enrolment_screen(ui, setting_up);
        return false;
    }

    new_card_rate_control(ui, coll, rate_buffer);
    ui.add_space(spacing::gap(3));
    ui.separator();
    ui.add_space(spacing::gap(2));

    optimise_control(ui, coll, optimise_job, optimise_done, now_ms);
    ui.add_space(spacing::gap(3));
    ui.separator();
    ui.add_space(spacing::gap(2));

    // The promise, worded once (ADR-0015 §3) — never "automatic", never "in the background".
    body(ui, sync::PROMISE);
    ui.add_space(spacing::gap(2));

    // "Set up sync" is the entry, not "login" or "pairing" (ADR-0015 §7): there is no account of ours
    // and no device-to-device step.
    if full_width_button(ui, sync::SET_UP_SYNC).clicked() {
        *setting_up = true;
    }

    ui.add_space(spacing::gap(3));
    // The removal route and the app name, kept permanently because the folder is hidden and cannot be
    // navigated to (ADR-0015 §10, ADR-0020 §4). Disconnect is the only control this app owns.
    body(ui, &sync::revocation_and_removal());

    ui.add_space(spacing::gap(3));
    ui.separator();
    ui.add_space(spacing::gap(2));
    let reset = reset_control(ui);

    ui.add_space(spacing::gap(3));
    ui.separator();
    ui.add_space(spacing::gap(2));
    rendering_specimen(ui);

    ui.add_space(spacing::gap(3));
    ui.separator();
    ui.add_space(spacing::gap(2));
    handoff_specimen(ui, handoff);

    ui.add_space(spacing::gap(3));
    ui.separator();
    ui.add_space(spacing::gap(2));
    // The list is the other half of the inbound path: it enumerates the files this application wrote
    // and, on selection, funnels one into the very same `inbound` the specimen below reads (#108).
    file_list_specimen(ui, file_list, inbound);

    ui.add_space(spacing::gap(3));
    ui.separator();
    ui.add_space(spacing::gap(2));
    inbound_specimen(ui, coll, inbound.as_ref());

    reset
}

/// **Temporary, and not a specified feature.** The file-list specimen: the first call site of
/// [`cairn_export::platform::list`] (issue #108), which until now had none. It enumerates the files
/// this application wrote, describes each **from its own bytes**, and on selection hands one to the
/// same inbound read an arriving file takes.
///
/// **It says what the list *is*, never what is missing.** Scoped storage grants this application its
/// own `MediaStore` rows and nothing else, so a `.cdeck` another application dropped in `Downloads` is
/// invisible to the query — not unreadable, *absent* (ADR-0024 §3). That absence is the platform, not
/// a defect to explain, so the wording is *"the files this application wrote"* and never invites a
/// user to drop a file in a folder and expect it here. On the desktop the same list is a real folder
/// scan, and the wording is true there too.
///
/// **Each row is described from its sniff, never its extension** (ADR-0024 §1, deck-export rule 13):
/// enumeration is by `.cdeck`/`.ccoll`, but a `.cdeck` may carry a collection archive and only the
/// `mimetype` member tells them apart — so both profiles appear, told apart by the bytes. A file we
/// wrote but can no longer parse is **listed and marked unreadable**, never hidden (ADR-0022 §11):
/// hiding it sends a user after a permissions problem that does not exist.
///
/// **Selecting a row is one mechanism, not two.** It re-reads the bytes through
/// [`cairn_export::platform::get`] and hands them
/// to [`listing::select`], producing an [`inbound::Inbound`] the [`inbound_specimen`] below then plans
/// against the live collection — the same gate-and-describe read a drop or a launch intent reaches
/// (ADR-0022 §5). The row description stays the cheap sniff so enumerating the whole list inflates
/// **zero payloads**; only the one selected file is inflated to a plan.
fn file_list_specimen(
    ui: &mut egui::Ui,
    state: &mut FileList,
    inbound: &mut Option<inbound::Inbound>,
) {
    use cairn_export::platform;

    body(
        ui,
        "Development control — the files this application wrote, and only those. It is not a view of \
         the downloads folder: a file another application put there cannot appear here, and that is \
         the platform, not a fault. Each row is described from its own bytes; selecting one previews \
         it below through the same read an arriving file takes.",
    );
    ui.add_space(spacing::gap(1));

    if full_width_button(ui, "List the files (temporary)").clicked() {
        state.said.clear();
        match platform::list() {
            Err(e) => {
                state.rows = None;
                state.said = format!("Could not list the files: {e}");
            }
            Ok(names) => {
                // Read each file back and sniff its profile — the seam hands whole bytes, but only
                // the fixed-offset `mimetype` header is read; no payload is ever inflated (ADR-0022
                // §11). A file we cannot even read back still earns a row, marked unreadable, rather
                // than vanishing.
                let rows = names
                    .iter()
                    .map(|name| match platform::get(name) {
                        Ok(bytes) => listing::describe(name, &bytes),
                        Err(_) => listing::Listed {
                            name: name.clone(),
                            sniffed: None,
                        },
                    })
                    .collect();
                state.rows = Some(rows);
            }
        }
    }

    ui.add_space(spacing::gap(2));

    if !state.said.is_empty() {
        body(ui, &state.said);
        return;
    }

    let Some(rows) = &state.rows else {
        body(ui, "Not listed yet — press the button.");
        return;
    };

    if rows.is_empty() {
        body(
            ui,
            "No files this application has written yet. Writing one (above) puts it here.",
        );
        return;
    }

    // The row selected this frame, re-read below the borrow so `state.rows` is not held across the
    // `get`. Only one row can be pressed per frame.
    let mut selected: Option<String> = None;
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            ui.add_space(spacing::gap(1));
        }
        spacing::row(ui, 1, |ui| {
            if ui.button("Preview").clicked() {
                selected = Some(row.name.clone());
            }
            field_label(ui, &format!("{} — {}", row.name, listed_label(row)));
        });
    }

    if let Some(name) = selected {
        // Re-read the bytes at selection and route them through the arriving path — one mechanism,
        // not two (ADR-0022 §5). A read that fails between listing and selecting says so plainly.
        match platform::get(&name) {
            Ok(bytes) => *inbound = Some(listing::select(&name, bytes)),
            Err(e) => state.said = format!("Could not read \"{name}\": {e}"),
        }
    }
}

/// A listed file's row description, drawn **from its sniff** (ADR-0024 §1). Both profiles are named,
/// and a file we wrote but can no longer parse is *unreadable* — the honest word for a row that stays
/// on the list rather than disappearing from it (ADR-0022 §11).
fn listed_label(listed: &listing::Listed) -> &'static str {
    use cairn_export::Profile;
    match &listed.sniffed {
        Some(Profile::Deck) => "deck",
        Some(Profile::Collection) => "collection archive",
        Some(Profile::Other(_)) => "another kind of file",
        None => "unreadable",
    }
}

/// **Temporary, and not a specified feature.** The rendering specimen: every script the shipped faces
/// exist for, drawn in every family they are registered into, so issue #97's criteria can be read off
/// one screen by someone who reads the script.
///
/// **It is here because the handset cannot be asked any other way.** Client-stack rule 8 makes Android
/// text input ASCII-only — there is no IME path, so a Persian sentence can never be *typed* on the
/// device — and a screenshot compared against a reference image only tells a non-reader that something
/// shaped like Persian appeared. So the strings ship in the binary, each above what it must read, and
/// the judgement handed over is the one only a reader can make.
///
/// **Three families, drawn one under the other on purpose.** A face is resolved per family and per
/// character, so the same string can be right in `Proportional` and wrong in `Monospace` or in
/// [`fonts::bold_family`] with nothing failing anywhere (client-stack rule 7). Stacking them puts the
/// three renderings of one string side by side, which is the only way a wrong *face* — as opposed to a
/// missing glyph — shows up at all: it draws, it just draws in the wrong hand.
///
/// It goes through [`bidi::job`] like every other string in the app, because half of what is being
/// checked is the ordering that helper produces (client-stack rule 1) rather than the glyphs alone.
fn rendering_specimen(ui: &mut egui::Ui) {
    body(
        ui,
        "Development control — every script the shipped faces exist for, in every family. Each line \
         below is the same text drawn by a different family; check it against the caption above it.",
    );
    ui.add_space(spacing::gap(2));

    for (caption, specimen) in fonts::SPECIMENS {
        field_label(ui, caption);
        ui.add_space(spacing::gap(1));
        for (i, family) in fonts::families().into_iter().enumerate() {
            if i > 0 {
                ui.add_space(spacing::gap(1));
            }
            spacing::row(ui, 1, |ui| {
                // The family's own name, in the family itself: a tag drawn in some *other* face
                // would be naming a rendering it is not part of.
                //
                // **These two read the scale's constants rather than a `TextStyle`** because the
                // whole point of the control is to draw one text in a *chosen* family, which a
                // resolved text style cannot express — it carries its own. Reaching for the
                // constants is the sanctioned way past that (ADR-0032 §1); a literal here would not
                // be (this is a real screen, unlike `fonts`'s coverage probe, which draws nothing a
                // user sees).
                ui.label(bidi::job(
                    &family_tag(&family),
                    egui::FontId::new(typography::SMALL, family.clone()),
                    ui.visuals().weak_text_color(),
                ));
                ui.label(bidi::job(
                    specimen,
                    egui::FontId::new(typography::HEADING, family.clone()),
                    ui.visuals().text_color(),
                ));
            });
        }
        ui.add_space(spacing::gap(2));
    }
}

/// **Temporary, and not a specified feature.** The hand-off specimen: the two user-files calls issue
/// #98 asks to be verified on the handset, behind **two separate buttons**.
///
/// **It is here because nothing else reaches them.** [#88](https://github.com/amin-bf/cairn/issues/88)
/// landed `cairn-export` and its four-operation seam but deferred the export *screen* to the visual
/// pass, so `put` and `hand_off` have no call site in this crate — and every one of #98's criteria is
/// about what those two calls do at runtime on a real `MediaStore`. A seam with no caller cannot be
/// verified by holding the phone.
///
/// **Two buttons rather than one, and that is the point rather than a convenience.**
/// [ADR-0023 §5](../../../docs/adr/0023-sending-a-written-file.md) says the affordance *never fires by
/// itself*: nothing opens when an export finishes. A specimen that wrote and then shared in one press
/// would satisfy every other criterion while making that one unobservable — the sheet would appear
/// either way, and no one watching could tell which rule was in force.
///
/// **It reports the name it was given back, never the one it asked for**
/// ([ADR-0022 §10](../../../docs/adr/0022-the-import-preview-and-export-report.md)), and it shows both
/// so the difference is legible: press it twice and the second write collides, which is the whole of
/// [ADR-0024 §4](../../../docs/adr/0024-identifying-a-written-file.md)'s claim that declaring no media
/// type is what keeps the extension. The bytes are identical across presses on purpose — same name,
/// same content — so the collision is the one that ADR's probe measured and not a different event.
fn handoff_specimen(ui: &mut egui::Ui, state: &mut HandOff) {
    body(
        ui,
        "Development control — the two user-files calls, one per button. Write puts a real .cdeck \
         through the seam and states the name the platform wrote back. Hand off opens the system \
         share sheet for it, and only when pressed: writing never opens anything.",
    );
    ui.add_space(spacing::gap(1));

    if full_width_button(ui, "Write a deck file (temporary)").clicked() {
        state.said = match specimen_deck() {
            Err(e) => format!("Could not build the file: {e}"),
            Ok(bytes) => {
                let requested = cairn_export::export_filename(&[SPECIMEN_DECK_NAME]);
                match cairn_export::platform::put(&requested, &bytes) {
                    Err(e) => format!("Could not write it: {e}"),
                    Ok(written) => {
                        let said = format!(
                            "Asked for \"{requested}\" — written as \"{}\".",
                            written.name
                        );
                        state.written = Some(written.name);
                        said
                    }
                }
            }
        };
    }

    ui.add_space(spacing::gap(1));

    if full_width_button(ui, "Hand it off (temporary)").clicked() {
        state.said = match &state.written {
            None => "Nothing written yet — write a deck file first.".to_owned(),
            Some(name) => match cairn_export::platform::hand_off(name) {
                Ok(()) => format!("Handed \"{name}\" onward. Nothing is reported after this."),
                Err(e) => format!("Could not hand it off: {e}"),
            },
        };
    }

    if !state.said.is_empty() {
        ui.add_space(spacing::gap(2));
        body(ui, &state.said);
    }
}

/// **Temporary, and not a specified feature.** The inbound specimen: it states what the platform
/// handed the application at launch or by a drop, and what was decided about it — the action, whether
/// a name came with it, the sniffed profile, and the plan behind the gate or the refusal in its place
/// (acceptance of #107). It is what makes [#99](https://github.com/amin-bf/cairn/issues/99)'s
/// on-device criteria readable off one screen by someone holding the phone.
///
/// **It is here because the preview *screen* is the visual design pass's, ruled out of scope by the
/// map.** So this ends where #98's specimen did — a development control that reports, not the
/// ADR-0022 surface. What it reports is real: a `.cdeck` opened from a file manager, or shared from a
/// messaging application, or dropped on the desktop window, reaches this through the same
/// identification and plan path as the real importer will.
///
/// **The plan is derived here, every frame, and never cached** ([ADR-0022 §5](../../../docs/adr/0022-the-import-preview-and-export-report.md)):
/// the application holds the arrived *file* ([`inbound::Inbound`]) across frames, and this calls
/// [`inbound::read`] fresh on each draw, so a sync landing while the specimen is on screen changes
/// the numbers rather than staling them. Every string the file carries is already bounded plain text
/// (ADR-0022 §7) and is drawn through [`bidi::job`] like all chrome — a stranger's string can never
/// style the screen it is previewed on (client-stack rule 1).
fn inbound_specimen(ui: &mut egui::Ui, coll: &Collection, inbound: Option<&inbound::Inbound>) {
    body(
        ui,
        "Development control — what the platform handed us to open. Drop a .cdeck on this window, or \
         open one from a file manager or a share on the handset; this states what arrived and what \
         an import would do, derived fresh and never held.",
    );
    ui.add_space(spacing::gap(2));

    let Some(inbound) = inbound else {
        body(ui, "Nothing has arrived yet.");
        return;
    };

    field_label(ui, &format!("Arrived: {}", inbound.arrival.label()));
    match &inbound.name {
        Some(name) => field_label(ui, &format!("Name it gave: \"{name}\"")),
        // A share may carry none, and identity never needs it (ADR-0024 §1).
        None => field_label(ui, "Name it gave: none — identified by its bytes."),
    }

    // Derived on the spot against the collection as it stands (ADR-0022 §5), never stored.
    match inbound::read(inbound, coll) {
        Err(e) => body(
            ui,
            &format!("Could not read the collection to diff against: {e}"),
        ),
        Ok(report) => {
            field_label(
                ui,
                &format!("Sniffed: {}", profile_label(report.sniffed.as_ref())),
            );
            ui.add_space(spacing::gap(2));
            match &report.outcome {
                Err(refusal) => body(ui, &refusal_wording(refusal)),
                Ok(plan) => {
                    // One unit between statements. The preview is the one screen a stranger's file
                    // writes onto (ADR-0022 §10), so its lines must not run together into a single
                    // block a reader skims — and they did exactly that once the ambient 3px went to
                    // zero. This screen is **not reachable by the capture harness** (it needs a file
                    // dropped on the window, which synthetic input cannot produce), so nothing would
                    // have photographed the regression.
                    for (i, line) in plan_lines(plan).into_iter().enumerate() {
                        if i > 0 {
                            ui.add_space(spacing::gap(1));
                        }
                        body(ui, &line);
                    }
                }
            }
        }
    }
}

/// What the sniff said the file is, for the specimen (ADR-0024 §1). `Other` carries the type it
/// declared; `None` is a file that is not a sniffable container at all.
fn profile_label(profile: Option<&cairn_export::Profile>) -> String {
    use cairn_export::Profile;
    match profile {
        Some(Profile::Deck) => "a deck".to_owned(),
        Some(Profile::Collection) => "a collection archive".to_owned(),
        Some(Profile::Other(media)) => media.clone(),
        None => "not a recognised container".to_owned(),
    }
}

/// The refusal shown in place of a preview (ADR-0022 §4). One plain message each, **with no detail
/// that reads as an invitation to repair the file** — the classic zip-traversal defect, and the
/// message is not a diagnostic channel for whoever built it.
fn refusal_wording(refusal: &cairn_export::Refusal) -> String {
    use cairn_export::Refusal;
    match refusal {
        Refusal::Unreadable => "This file could not be read as a deck.".to_owned(),
        Refusal::UnknownFormat(_) => "This file needs a newer version of the app.".to_owned(),
        Refusal::WrongProfile => "This is a collection archive, not a deck file.".to_owned(),
        Refusal::BrokenPath => "This file is not put together the way a deck is.".to_owned(),
        // Named as older, never as damaged (ADR-0022 §4), and it names the held deck.
        Refusal::Older { deck } => {
            format!("This is an older copy of \"{deck}\" than the one you have.")
        }
    }
}

/// The plan's effect lines, one string each so [`body`] lays each through the bidi helper (ADR-0022
/// §3, client-stack rule 1). A line that does not apply is **absent, never shown as zero** — a screen
/// of zeroes buries the one line that is not. The last line is always present (ADR-0022 §3).
fn plan_lines(plan: &cairn_export::Plan) -> Vec<String> {
    use cairn_export::Path;
    let mut lines = Vec::new();

    let header = &plan.header;
    if !header.author.is_empty() {
        lines.push(format!("by {}", header.author));
    }
    if !header.description.is_empty() {
        lines.push(header.description.clone());
    }
    if !header.licence.is_empty() {
        lines.push(header.licence.clone());
    }

    for deck in &plan.decks {
        let path = match deck.path {
            Path::Update => "updating a deck you already have",
            Path::Create => "new deck",
        };
        lines.push(format!("{} — {path}", deck.name));

        if deck.no_change {
            lines.push("Nothing will change.".to_owned());
            continue;
        }

        let mut counts = Vec::new();
        if deck.new_notes > 0 {
            counts.push(format!("{} new", deck.new_notes));
        }
        if deck.already_yours > 0 {
            counts.push(format!("{} already yours", deck.already_yours));
        }
        if !counts.is_empty() {
            lines.push(counts.join(", "));
        }
        for moving in &deck.moving_in {
            let from = moving.from.as_deref().unwrap_or("an unfiled note");
            lines.push(format!("{} moving in from {from}", moving.count));
        }
        if deck.deleted > 0 {
            lines.push(format!("{} of your notes will be deleted", deck.deleted));
        }
        if let Some(from) = &deck.renamed_from {
            lines.push(format!("renaming your \"{from}\" to \"{}\"", deck.name));
        }
        if deck.revision_conflict {
            lines.push("same revision as yours, different content".to_owned());
        }
    }

    if !plan.adopted_kinds.is_empty() {
        lines.push(format!(
            "adds a card type this build does not have: {}",
            plan.adopted_kinds.join(", ")
        ));
    }
    for emptied in &plan.emptied_decks {
        lines.push(format!("\"{emptied}\" will be left empty"));
    }

    // Always present, even when it is the only line (ADR-0022 §3): "Import" implies risk to a
    // schedule, which ADR-0005 §9 makes structurally impossible.
    lines.push("Your review history is untouched.".to_owned());
    lines
}

/// The specimen deck's display name — the filename derives from it, sanitised outbound.
const SPECIMEN_DECK_NAME: &str = "Specimen";

/// Fixed ids, so every press builds **byte-identical** content and a second write is a true
/// same-name collision rather than a new file. `cairn-core` never mints an id (ADR-0009 §8), and a
/// specimen has no collection to take one from.
const SPECIMEN_DECK_ID: DeckId = DeckId([
    0x98, 0x0d, 0xec, 0x00, 0x40, 0x00, 0x40, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
]);
const SPECIMEN_NOTE_ID: NoteId = NoteId([
    0x98, 0x0d, 0xec, 0x00, 0x40, 0x00, 0x40, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
]);

/// A real `.cdeck` — the actual container, not a stand-in. What is being verified is what the
/// platform does with the bytes and the name, so a placeholder payload would still exercise the
/// seam; a real one additionally lets whoever receives the share open it.
fn specimen_deck() -> Result<Vec<u8>, cairn_export::ExportError> {
    let content = cairn_export::DeckContent {
        id: SPECIMEN_DECK_ID,
        name: SPECIMEN_DECK_NAME.to_owned(),
        notes: vec![cairn_export::NoteContent {
            id: SPECIMEN_NOTE_ID,
            position: "n".to_owned(),
            kind: "basic".to_owned(),
            fields: vec![
                ("Front".to_owned(), "specimen front".to_owned()),
                ("Back".to_owned(), "specimen back".to_owned()),
            ],
        }],
        tombstones: Vec::new(),
    };
    let digest = cairn_export::deck_digest(&content)?;
    let revision = cairn_export::next_revision(None, &digest);
    cairn_export::build_deck(
        &cairn_export::Metadata::default(),
        &[cairn_export::DeckExport { content, revision }],
    )
}

/// The short name of a family, for the specimen's row tag.
fn family_tag(family: &egui::FontFamily) -> String {
    match family {
        egui::FontFamily::Proportional => "prop".to_owned(),
        egui::FontFamily::Monospace => "mono".to_owned(),
        egui::FontFamily::Name(name) => name.to_string(),
    }
}

/// **Temporary, and not a specified feature.** A development control that returns this device to a
/// **first launch** — the collection deleted and reseeded exactly as [`CairnApp::open_store`] does it
/// on a fresh install — so an on-handset verification run does not need a cable and `run-as` to get back
/// to a known state. Returns whether it was pressed.
///
/// **It is a reset, not a delete, and it is not a step towards a user-facing one.** Nothing in this design
/// removes data: [ADR-0016 §1](../../../docs/adr/0016-backup-and-restore.md) establishes that restore is
/// a merge and a replace is *not implementable*, because every device holds the whole log and merge is
/// set union — so a wipe here is undone by the next sync from any peer that still holds those rows. It
/// is honest only as what it says it is: a local reset on a device being tested against.
/// [ADR-0015 §10](../../../docs/adr/0015-the-sync-experience.md) separately forbids a control that
/// deletes *published* data, which this does not touch.
fn reset_control(ui: &mut egui::Ui) -> bool {
    body(
        ui,
        "Development control — returns this device to a first launch, seed and all. Rows other \
         devices hold come back on the next sync.",
    );
    ui.add_space(spacing::gap(1));
    full_width_button(ui, "Reset the collection (temporary)").clicked()
}

/// The new-card-rate control (ADR-0011 §3): a plain integer field, with the consequence explained
/// where it is set — no modal, no automatic mode. The buffer is seeded from the stored rate on first
/// show and committed on a completed edit (blur), clamped and defaulted in the store; **zero is a
/// legal value and the backlog answer**, so an empty or unparsable field is left for the user to
/// finish rather than snapped to a number. It never enters the log and never exports (ADR-0011 §5).
fn new_card_rate_control(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    rate_buffer: &mut Option<String>,
) {
    // Seed the buffer from the stored rate the first time this screen is shown.
    let buffer = rate_buffer.get_or_insert_with(|| {
        coll.new_card_rate()
            .unwrap_or(DEFAULT_NEW_CARD_RATE)
            .to_string()
    });

    field_label(ui, "New cards a day");
    let resp = text_field(ui, buffer);
    // Commit on blur: a completed edit that parses writes the (clamped) rate back; zero is kept.
    if resp.lost_focus()
        && let Ok(rate) = buffer.trim().parse::<u32>()
    {
        // A failed write is dropped rather than surfaced: the re-read below then reflects the
        // unchanged stored value, so the field simply shows the edit did not take. Surfacing write
        // errors is a later ticket, as at the review grade site.
        let _ = coll.set_new_card_rate(rate);
        // Reflect the clamp back into the buffer so an out-of-range entry shows what was stored.
        *buffer = coll
            .new_card_rate()
            .unwrap_or(DEFAULT_NEW_CARD_RATE)
            .to_string();
    }
    ui.add_space(spacing::gap(1));
    // The consequence, stated where the choice is (ADR-0011 §3, §4): this is the only enforced limit,
    // and zero is how a backlog is cleared before turning it back on.
    body(
        ui,
        "The only limit in the app. Set it to zero to clear a backlog, then turn it back on.",
    );
}

/// The parameter-optimisation control (ADR-0014 §2, §3, §4). **The action is always present** — a
/// button that is sometimes absent teaches the feature does not exist — with the fact-only nudge
/// beneath it. Pressing it starts a worker thread the frame loop polls; while it runs, the button is
/// replaced in place by the two-phase progress and a Cancel (§4), and **nothing is written until it
/// completes**. On completion the fitted vector is written — skipped if unchanged (§5) — and the
/// factual completion message shown, which makes no quality claim (§4).
///
/// The words and the run's shape are proven in `optimise`; this is the egui wiring the visual pass
/// refines. ADR-0014 §7's *sync, then train* is a no-op here: no transport is enrolled in this build,
/// and an offline device optimising on local history is a fine outcome — the leading sync is a
/// sequence, never a gate.
fn optimise_control(
    ui: &mut egui::Ui,
    coll: &mut Collection,
    job: &mut Option<optimise::OptimiseJob>,
    done: &mut bool,
    now_ms: i64,
) {
    field_label(ui, "Scheduler");

    if let Some(running) = job.as_mut() {
        // A run is in flight: keep the frame loop turning so `poll` is reached, then render the phase.
        ui.ctx().request_repaint();
        match running.phase() {
            optimise::Phase::Preparing => {
                spacing::row(ui, 1, |ui| {
                    ui.add(egui::Spinner::new());
                    body(ui, "Preparing…");
                });
            }
            optimise::Phase::Training { current, total } => {
                let fraction = if total == 0 {
                    0.0
                } else {
                    current as f32 / total as f32
                };
                ui.add(egui::ProgressBar::new(fraction).show_percentage());
            }
        }
        if full_width_button(ui, "Cancel").clicked() {
            running.cancel();
        }
        // Poll once this frame. On completion, write the vector (unchanged ones write nothing, §5) and
        // drop the job. A cancelled or failed run yields `None`: nothing to write, recover by pressing
        // the button again.
        if let Some(result) = running.poll() {
            if let Some(outcome) = result {
                // A failed write is dropped rather than surfaced, matching the review-grade site; the
                // nudge simply re-reads the unchanged row next frame.
                let _ = coll.set_scheduler_parameters(
                    outcome.parameters.weights(),
                    outcome.fitted_over,
                    now_ms,
                    DayScale::default(),
                );
                *done = true;
            }
            *job = None;
        }
        return;
    }

    // At rest: the always-present action, the fact-only nudge, and the completion message if a run
    // just finished (ADR-0014 §2, §4).
    if full_width_button(ui, "Optimise").clicked() {
        *done = false;
        let lines = coll.log_lines().unwrap_or_default();
        *job = Some(optimise::OptimiseJob::start(lines));
        ui.ctx().request_repaint();
    }
    ui.add_space(spacing::gap(1));

    let nudge = coll
        .log_lines()
        .map(|lines| {
            let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
            optimise::nudge_text(&cairn_core::replay::optimisation_nudge(&refs))
        })
        .unwrap_or_default();
    body(ui, &nudge);

    if *done {
        ui.add_space(spacing::gap(1));
        body(ui, optimise::COMPLETION_MESSAGE);
    }
}
