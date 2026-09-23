//! The **Settings** destination: the reset control, the new-card-rate and optimise controls, the
//! sync surface, and the **temporary** specimens. The specimens are development controls, not
//! specified features — each keeps the doc comment marking it so.

use cairn_core::content::{DeckId, NoteId};
use cairn_core::log::{DEFAULT_NEW_CARD_RATE, DayScale};
use cairn_store::Collection;

use crate::screens::enrolment::enrolment_screen;
use crate::{
    bidi, body, compact_button, field_label, file_bench, fixtures, fonts, full_width_button,
    heading, inbound, listing, optimise, sync, text_field,
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

/// What the **file list** carries between frames (ADR-0022 §11, ADR-0041 §5).
#[derive(Default)]
pub(crate) struct FileList {
    /// The rows from the last enumeration, or `None` when the list has to be read again — which it is
    /// the first time Settings draws after being opened, and after a file set lands. Only names and
    /// manifest summaries are held, never the bytes: describing a row inflates no payload (ADR-0022
    /// §11), and the bytes are re-read from the seam when a row is opened.
    ///
    /// **This is a listing, not a plan**, so holding it is not ADR-0022 §5's stored projection: it
    /// says which files exist and what they claim to be, and nothing about what they would do to the
    /// collection. The plan is derived afresh on every frame the preview draws.
    rows: Option<Vec<listing::Listed>>,
    /// Why the list could not be read, or why an opened row could not be — the seam's refusal,
    /// verbatim. Empty when there is nothing to say.
    said: String,
}

impl FileList {
    /// Read the list again the next time it draws. Called whenever Settings is left, so opening it
    /// always shows the files as they are now rather than as they were the last time.
    pub(crate) fn forget(&mut self) {
        self.rows = None;
        self.said.clear();
    }
}

/// **Temporary, and not a specified feature.** What the fixture bench carries between frames — the
/// last thing an install had to say, verbatim. Held rather than logged because a handset run has no
/// console the person holding it can read (as [`HandOff`]), and this is the one control here whose
/// silent failure would be a *plausible* screen rather than a blank one.
#[derive(Default)]
pub(crate) struct Bench {
    said: String,
}

/// **Temporary, and not a specified feature.** What the development controls at the bottom of
/// Settings asked the application to do, once the collection borrow has ended.
///
/// Both arms close the open connection and unlink the databases, which cannot happen while
/// [`settings_screen`] holds `&mut Collection` — so the controls only *ask*, exactly as the reset
/// control has always done, and [`CairnApp`](crate::CairnApp) acts on the way out.
pub(crate) enum BenchRequest {
    /// Return this device to a first launch — the collection deleted and reseeded.
    Reset,
    /// Replace the collection with a pre-made one (`fixtures`).
    Install(fixtures::Fixture),
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
    bench: &mut Bench,
    now_ms: i64,
) -> Option<BenchRequest> {
    heading(ui, "Settings");
    ui.add_space(spacing::gap(2));

    if *setting_up {
        enrolment_screen(ui, setting_up);
        return None;
    }

    // **Appearance sits first, directly under the heading**, and it is the one control here that is
    // about *this device* rather than about the collection — everything below it (the rate, the
    // scheduler, sync) syncs, and this deliberately does not (ADR-0036 §3). Putting the device-local
    // item at the top is a first cut at the ordering #121 suspects this screen of lacking.
    //
    // It also makes the control **reachable by the capture harness at every width**, which is what
    // made anyone look: a control placed below prose sits at a y that depends on where that prose
    // wraps, so the storyboard clicked the right place at 1280 and empty page at 560 — producing
    // seven perfectly valid dark captures of a light storyboard, with nothing failing. Nothing above
    // a one-word heading can wrap, so this position is the same at both.
    theme_control(ui, coll);
    ui.add_space(spacing::gap(3));
    ui.separator();
    ui.add_space(spacing::gap(2));

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

    // **The file list** — a specified block, where two development specimens stood (ADR-0022 §9,
    // §11; ADR-0041 §5). Where on Settings it finally sits is the Settings slice's (#152); it goes
    // above the development controls because it is not one of them.
    ui.add_space(spacing::gap(3));
    ui.separator();
    ui.add_space(spacing::gap(2));
    file_block(ui, file_list, inbound);

    ui.add_space(spacing::gap(3));
    ui.separator();
    ui.add_space(spacing::gap(2));
    let mut request = reset_control(ui).then_some(BenchRequest::Reset);

    ui.add_space(spacing::gap(3));
    ui.separator();
    ui.add_space(spacing::gap(2));
    if let Some(chosen) = fixture_bench(ui, bench, coll, file_list) {
        request = Some(chosen);
    }

    ui.add_space(spacing::gap(3));
    ui.separator();
    ui.add_space(spacing::gap(2));
    rendering_specimen(ui);

    ui.add_space(spacing::gap(3));
    ui.separator();
    ui.add_space(spacing::gap(2));
    handoff_specimen(ui, handoff);

    request
}

/// **Temporary, and not a specified feature.** The fixture bench: the collection states the shipping
/// seed never produces, one tap each (`fixtures`).
///
/// **This is the handset half of a bench whose other half is a binary.** The desktop harness installs
/// a fixture from *outside*, into the scratch data directory it already owns; Android has no such
/// route, because `data_dir` is `getFilesDir()` and nothing outside the app may write it — and #141
/// found that even an uninstall does not clear it, because ADR-0007 §6 deliberately puts it in the
/// Auto Backup set. A thumb tapping a button is the only way in, which is why this block exists
/// rather than a flag.
///
/// The **checkpoint** is the one control here that installs no rows. ADR-0006 §1's ten-minute
/// check-in hangs off a sitting's monotonic clock rather than off the log, so no pre-made collection
/// reaches it; this shortens it for the rest of the process, which is the difference between a
/// decided state that gets looked at and one that does not.
///
/// The **file sets** sit under the fixtures, and they are the one control here that acts on the spot:
/// a set only *adds* files through the user-files seam and reads the collection to check its plans,
/// so it closes no connection and needs nothing from the caller. On a handset this row is the only
/// way a file set lands at all — `MediaStore` is writable by the application and by nothing outside
/// it (`file_bench`).
///
/// Returns what the person asked for; the caller acts once the collection borrow has ended.
fn fixture_bench(
    ui: &mut egui::Ui,
    bench: &mut Bench,
    coll: &Collection,
    file_list: &mut FileList,
) -> Option<BenchRequest> {
    let mut request = None;

    field_label(ui, "Fixtures (temporary)");
    ui.add_space(spacing::gap(1));
    body(
        ui,
        "Development control — replaces the collection with a pre-made one, to reach a screen the \
         shipping seed cannot. Rows other devices hold come back on the next sync.",
    );
    ui.add_space(spacing::gap(1));

    spacing::row_wrapped(ui, 1, |ui| {
        for fixture in fixtures::Fixture::ALL {
            if compact_button(ui, fixture.label()).clicked() {
                bench.said = format!("Installing {} — {}…", fixture.key(), fixture.reaches());
                request = Some(BenchRequest::Install(fixture));
            }
        }
    });

    ui.add_space(spacing::gap(1));
    spacing::row_wrapped(ui, 1, |ui| {
        for set in file_bench::FileSet::ALL {
            if compact_button(ui, set.label()).clicked() {
                // Said in full either way, for the reason the fixture verdict is: a set built against
                // the wrong collection is refused, and the refusal names the fixture it needs.
                bench.said = match set.install(coll) {
                    Ok(landed) => format!("{} — {landed}", set.key()),
                    Err(message) => message,
                };
                // The list above describes the files that were there; read it again.
                file_list.forget();
            }
        }
    });

    ui.add_space(spacing::gap(2));
    if fixtures::checkpoint_is_shortened() {
        body(ui, "The 10-minute checkpoint is shortened for this run.");
    } else if full_width_button(ui, "Shorten the checkpoint (temporary)").clicked() {
        fixtures::set_checkpoint_after(fixtures::BENCH_CHECKPOINT_SECONDS);
    }

    if !bench.said.is_empty() {
        ui.add_space(spacing::gap(1));
        body(ui, &bench.said);
    }
    request
}

/// **Temporary, and not a specified feature.** What the last fixture install had to say — the state
/// it reached, or why it did not. Called by [`CairnApp`](crate::CairnApp) after it has acted, because
/// the install needs the collection this screen was holding.
pub(crate) fn bench_said(bench: &mut Bench, said: String) {
    bench.said = said;
}

/// **The file list** — a specified block on Settings (ADR-0022 §9, §11; ADR-0041 §5), and the call
/// site [`cairn_export::platform::list`] waited for from #108 to #167.
///
/// **It says what the list *is*, never what is missing.** Scoped storage grants this application its
/// own `MediaStore` rows and nothing else, so a `.cdeck` another application dropped in `Downloads` is
/// invisible to the query — not unreadable, *absent* (ADR-0024 §3). That absence is the platform, not
/// a defect to explain, so the wording is *"the files this app wrote"* and never invites anyone to
/// drop a file in a folder and expect it here. On the desktop the same list is a real folder scan,
/// and the wording is true there too. The empty state cannot point at an export screen, because there
/// is none yet (the map's *Out of scope*), so it says only what is so.
///
/// **It reads itself.** The list is enumerated the first time the block draws after Settings opens,
/// never behind a button: a list that has to be asked for is a development control.
///
/// **Each row is described from its own bytes, never its extension** (ADR-0024 §1, deck-export rule
/// 13), from the manifest's counts, inflating no payload (ADR-0022 §11). A file we wrote but can no
/// longer parse is **listed and marked unreadable**, never hidden: hiding it sends a user after a
/// permissions problem that does not exist.
///
/// **Opening a row is one mechanism, not two.** It re-reads the bytes through
/// [`cairn_export::platform::get`] and hands them to [`listing::select`], producing the same
/// [`inbound::Inbound`] a drop or a launch intent produces — and holding one is what makes the preview
/// take the screen (ADR-0022 §5, §6).
fn file_block(ui: &mut egui::Ui, state: &mut FileList, inbound: &mut Option<inbound::Inbound>) {
    use cairn_export::platform;

    field_label(ui, "Files");
    ui.add_space(spacing::gap(1));
    body(
        ui,
        "The files this app wrote. Open one to see what importing it would do.",
    );
    ui.add_space(spacing::gap(2));

    if state.rows.is_none() && state.said.is_empty() {
        match platform::list() {
            Err(e) => state.said = format!("Could not list the files: {e}"),
            Ok(names) => {
                // A file we cannot even read back still earns a row, marked unreadable, rather than
                // vanishing (ADR-0022 §11).
                let rows = names
                    .iter()
                    .map(|name| match platform::get(name) {
                        Ok(bytes) => listing::describe(name, &bytes),
                        Err(_) => listing::Listed {
                            name: name.clone(),
                            summary: cairn_export::Summary::Unreadable,
                        },
                    })
                    .collect();
                state.rows = Some(rows);
            }
        }
    }

    if !state.said.is_empty() {
        body(ui, &state.said);
    }
    let Some(rows) = &state.rows else {
        return;
    };
    if rows.is_empty() {
        body(ui, "This app hasn't written any files yet.");
        return;
    }

    // The row opened this frame, re-read below the borrow so `state.rows` is not held across the
    // `get`. Only one row can be pressed per frame.
    let mut opened: Option<String> = None;
    for (i, row) in rows.iter().enumerate() {
        if i > 0 {
            ui.add_space(spacing::gap(1));
        }
        let (mark, caption) = row_description(&row.summary);
        if crate::controls::row_marked(ui, mark, &row.name, &caption) {
            opened = Some(row.name.clone());
        }
    }

    if let Some(name) = opened {
        // Re-read at the moment of opening: the preview plans the file as it is now.
        match platform::get(&name) {
            Ok(bytes) => *inbound = Some(listing::select(&name, bytes)),
            Err(e) => state.said = format!("Could not read {}: {e}", bidi::isolate(&name)),
        }
    }
}

/// A listed file's picture and caption, **from its manifest** (ADR-0022 §11): the kind of file first,
/// in words — the picture beside it never replaces them (ADR-0041 §4) — then what the file says it
/// holds. These are the file's own counts, not what importing it would do; the preview says that.
fn row_description(summary: &cairn_export::Summary) -> (char, String) {
    use crate::screens::import::{counted, grouped};
    use cairn_export::Summary;
    match summary {
        Summary::Deck {
            decks,
            notes,
            retractions,
        } => {
            let mut caption = if *decks == 1 {
                format!("deck · {}", counted(*notes, "note"))
            } else {
                format!(
                    "deck · {} decks, {}",
                    grouped(*decks),
                    counted(*notes, "note")
                )
            };
            if *retractions > 0 {
                caption.push_str(&format!(" · {}", counted(*retractions, "retraction")));
            }
            (fonts::DECK, caption)
        }
        Summary::Collection {
            created,
            notes,
            reviews,
        } => (
            fonts::ARCHIVE,
            format!(
                "collection archive · {} · {}, {}",
                calendar_date(created),
                counted(*notes, "note"),
                counted(*reviews, "review")
            ),
        ),
        Summary::Other(_) => (fonts::UNREADABLE, "another kind of file".to_owned()),
        Summary::Unreadable => (fonts::UNREADABLE, "unreadable".to_owned()),
    }
}

/// An archive's ISO-8601 creation instant as the date ADR-0022 §11 draws — *1 September 2026*. It is a
/// stranger's string (§7), so anything that does not parse is shown as it came rather than guessed at.
fn calendar_date(iso: &str) -> String {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let parts = iso.get(..10).map(|d| d.split('-').collect::<Vec<_>>());
    if let Some([y, m, d]) = parts.as_deref()
        && let (Ok(y), Ok(m), Ok(d)) = (y.parse::<u32>(), m.parse::<usize>(), d.parse::<u32>())
        && (1..=12).contains(&m)
        && (1..=31).contains(&d)
    {
        return format!("{d} {} {y}", MONTHS[m - 1]);
    }
    iso.to_owned()
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
/// **Appearance: System, Light or Dark** (ADR-0036 §3).
///
/// ADR-0030 §2 removed OS-theme following because only a dark palette was drawn, and following it
/// would have handed a light-preferring user stock egui by omission. Both palettes are drawn now, so
/// the behaviour returns — but as a **choice** rather than as obedience, because the case the OS
/// cannot serve is the one a reading app most needs: a dark room and a desktop set to light.
///
/// **Device-local, and that is the decision, not an implementation detail.** It rides the `local`
/// table rather than the settings singleton, so it never syncs — a desktop under a lamp and a
/// handset in bed want opposite answers, and a synced theme would have each clobber the other.
///
/// Selection is drawn with `selectable_label`, the same way the nav row marks the current
/// destination — the app's one existing way of saying *this is the one you are on*.
fn theme_control(ui: &mut egui::Ui, coll: &mut Collection) {
    let stored = coll.theme_preference().ok().flatten();
    let current = crate::theme::ThemeChoice::parse(stored.as_deref());

    field_label(ui, "Appearance");
    let mut chosen = None;
    spacing::row(ui, 1, |ui| {
        for (choice, label) in [
            (crate::theme::ThemeChoice::System, "System"),
            (crate::theme::ThemeChoice::Light, "Light"),
            (crate::theme::ThemeChoice::Dark, "Dark"),
        ] {
            if ui
                .selectable_label(current == choice, crate::text(ui, label))
                .clicked()
            {
                chosen = Some(choice);
            }
        }
    });

    if let Some(choice) = chosen
        && choice != current
    {
        // A failed write is dropped rather than surfaced, as at the rate site above: the re-read on
        // the next frame then shows the choice did not take. **The install happens either way** —
        // the palette the user just asked for is applied now, and a store that would not record it
        // reverts on the next launch rather than refusing the click.
        let _ = coll.set_theme_preference(choice.as_str());
        crate::theme::install(ui.ctx(), choice);
    }

    ui.add_space(spacing::gap(1));
    // Said where the choice is, because it is the half a user cannot see: this machine only.
    body(
        ui,
        "This machine only — your appearance does not follow the collection to another device.",
    );
}

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
