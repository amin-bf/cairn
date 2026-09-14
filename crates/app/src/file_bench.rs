//! **Temporary, and not a specified feature.** The file bench: the files an import arrives *from*,
//! installed beside a fixture the way a fixture is installed beside the application.
//!
//! # Why this exists
//!
//! The fixture bench (`fixtures`) reaches states *inside* the application. The file surface — the
//! file list and the import preview behind it (ADR-0022) — is about what is *outside* it: a place
//! with files in it. Until this module, reaching one meant a throwaway recipe writing files into a
//! directory named on the command line (`docs/design/file-surface-before-2026-09-05/`), and every
//! plan it produced said *new deck*, because nothing it wrote could name anything a collection held.
//! The lines ADR-0022 exists for — *updating a deck you already have*, *N of your notes will be
//! deleted*, *moving in from X*, *renaming your X*, *X will be left empty* — had never been drawn.
//!
//! # The route
//!
//! **The files are written through the user-files seam itself** — [`platform::put`], the call an
//! export makes — and never copied into a folder by the harness. That one choice settles both
//! platforms:
//!
//! - **On desktop** the seam writes into `$XDG_DOCUMENTS_DIR`, which `capture-desktop.sh` points into
//!   its scratch profile, so `cairn-fixture files <set>` installs a set before the app starts.
//! - **On a handset** nothing outside the application can write `MediaStore` — but the application
//!   can, and [`platform::list`] there returns exactly the rows the application inserted (ADR-0024
//!   §3). So the bench's Settings block reaches the phone, which a harness copying files into a
//!   directory never could have.
//!
//! # A file set verifies itself
//!
//! As a fixture does, and for the same reason. After writing, [`FileSet::install`] lists the files
//! back through the seam, re-reads each, and plans it through [`inbound::read`] against the live
//! collection — the exact path a selected row takes — and **fails unless every file plans the way
//! the set says it plans**. That is also what makes a set's dependency on its fixture impossible to
//! miss: the update file takes the update path only against [`Fixture::Decks`], so a set installed
//! beside any other collection is refused rather than photographed as a list of *new deck* plans
//! under names that promise the other half.
//!
//! # Installing twice is harmless
//!
//! The seam has no delete (ADR-0016 §5), and on a handset nothing else can remove a row, so a set
//! must survive being installed again. Every file is built from fixed ids and is **byte-identical on
//! every build**. A name already present with those bytes is left alone; a name present with
//! *different* bytes is refused rather than written, because the seam would dedupe the write to
//! `French A1 (1).cdeck` and the list would then hold two files where a storyboard names one.

use std::collections::BTreeMap;
use std::fmt;

use cairn_core::content::{DeckId, NoteId};
use cairn_core::identity::CollectionId;
use cairn_export::{
    CollectionArchive, DeckContent, DeckExport, DeckPlan, Header, Metadata, MovingIn, NoteContent,
    Path, Plan, Profile, Refusal, Tombstone, build_collection, build_deck, collection_filename,
    deck_digest, export_filename, next_revision, platform,
};
use cairn_store::Collection;

use crate::fixtures::{DECK_NOTES, DECKS, Fixture, bench_id, deck_note_id};
use crate::{inbound, listing};

/// **Temporary, and not a specified feature.** A set of files, named by what planning them reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSet {
    /// **Four files, chosen to span what a file can do to a collection rather than to look like a
    /// downloads folder.** One deck file that updates a deck [`Fixture::Decks`] holds and draws every
    /// destructive line; one file carrying three decks, one of them held and two new; a collection
    /// archive, the other profile in the same container; and a file this application wrote and can no
    /// longer parse, which is listed and marked unreadable rather than hidden (ADR-0022 §11).
    Imports,
}

impl FileSet {
    /// Every set, in the order the Settings block draws them.
    pub const ALL: [FileSet; 1] = [FileSet::Imports];

    /// The name a storyboard and a command line use. Stable, for the reason [`Fixture::key`] is.
    pub fn key(self) -> &'static str {
        match self {
            FileSet::Imports => "imports",
        }
    }

    /// The button label on Settings — what the files will be, not what the button does.
    pub fn label(self) -> &'static str {
        match self {
            FileSet::Imports => "Files to import",
        }
    }

    /// What planning these files reaches, in one line.
    pub fn reaches(self) -> &'static str {
        match self {
            FileSet::Imports => {
                "an update that renames, deletes and moves notes in; three decks in one file; an \
                 archive; an unreadable file"
            }
        }
    }

    /// The set a storyboard or a command line named, or `None` — refused rather than guessed.
    pub fn parse(key: &str) -> Option<FileSet> {
        FileSet::ALL.into_iter().find(|s| s.key() == key)
    }

    /// The fixture these files are built against. Their plans are only the plans the set names when
    /// the collection is this one, and [`FileSet::install`] checks that rather than trusting it.
    pub fn against(self) -> Fixture {
        match self {
            FileSet::Imports => Fixture::Decks,
        }
    }

    /// The files, built. Deterministic: the same bytes on every call and every build.
    pub fn files(self) -> Result<Vec<BenchFile>, String> {
        match self {
            FileSet::Imports => Ok(vec![
                BenchFile {
                    name: export_filename(&[UPDATE_NAME]),
                    bytes: update_file()?,
                    wants: Wants::Update,
                },
                BenchFile {
                    // Named by the seam's own rule, and the first deck is the Persian one — so the
                    // list carries a right-to-left **filename**, not only right-to-left content.
                    name: export_filename(&[DECKS[1].1, GERMAN.1, DUTCH.1]),
                    bytes: several_file()?,
                    wants: Wants::Several,
                },
                BenchFile {
                    name: collection_filename(),
                    bytes: archive_file(),
                    wants: Wants::Archive,
                },
                BenchFile {
                    name: UNREADABLE_NAME.to_owned(),
                    bytes: UNREADABLE_BYTES.to_vec(),
                    wants: Wants::Unreadable,
                },
            ]),
        }
    }

    /// Write this set through the user-files seam and verify it landed — listed under the names it
    /// was written with, and each file planning against `coll` the way the set says.
    ///
    /// Idempotent (see the module's *Installing twice is harmless*), so a handset that has already
    /// installed the set can install it again after a fixture reset without growing a second copy.
    pub fn install(self, coll: &Collection) -> Result<Landed, String> {
        let files = self.files()?;
        let present = platform::list().map_err(|e| e.to_string())?;

        let mut written = 0;
        for file in &files {
            if present.contains(&file.name) {
                let held = platform::get(&file.name).map_err(|e| e.to_string())?;
                if held != file.bytes {
                    return Err(format!(
                        "file set '{}': a different file called \"{}\" is already there, and \
                         writing would dedupe this one to a second name",
                        self.key(),
                        file.name
                    ));
                }
                continue;
            }
            let put = platform::put(&file.name, &file.bytes).map_err(|e| e.to_string())?;
            if put.name != file.name {
                return Err(format!(
                    "file set '{}': asked for \"{}\", written as \"{}\"",
                    self.key(),
                    file.name,
                    put.name
                ));
            }
            written += 1;
        }

        // Read back **through the seam**, not from the bytes just built: what a storyboard or a thumb
        // meets is what `list` and `get` return, and that is what has to be checked.
        let mut found = Vec::new();
        for name in platform::list().map_err(|e| e.to_string())? {
            if files.iter().any(|f| f.name == name) {
                let bytes = platform::get(&name).map_err(|e| e.to_string())?;
                found.push((name, bytes));
            }
        }

        let mut landed = self.check(coll, &found)?;
        landed.written = written;
        Ok(landed)
    }

    /// Hold what the seam returned against what this set promises. Split from [`FileSet::install`]
    /// so a test can reach it without a process-global environment variable.
    fn check(self, coll: &Collection, found: &[(String, Vec<u8>)]) -> Result<Landed, String> {
        let files = self.files()?;
        for file in &files {
            let Some((name, bytes)) = found.iter().find(|(n, _)| *n == file.name) else {
                return Err(format!(
                    "file set '{}': \"{}\" is not listed",
                    self.key(),
                    file.name
                ));
            };
            // The path a selected row takes, not a shortcut to the planner (ADR-0022 §5).
            let report = inbound::read(&listing::select(name, bytes.clone()), coll)
                .map_err(|e| e.to_string())?;
            if let Err(wanted) = file.wants.holds(report.sniffed.as_ref(), &report.outcome) {
                return Err(format!(
                    "file set '{}': \"{name}\" wanted {wanted}, and does not — these files are \
                     built against the '{}' fixture",
                    self.key(),
                    self.against().key()
                ));
            }
        }
        Ok(Landed {
            files: files.len(),
            written: 0,
        })
    }
}

/// Install `set` beside the collection in the platform's directories — the **outside** way in, and
/// the whole of what `cairn-fixture files <set>` does.
///
/// **It wipes nothing and needs the fixture installed first.** On desktop the files go wherever
/// `$XDG_DOCUMENTS_DIR` points, so redirect it before running this by hand, exactly as
/// `XDG_DATA_HOME` and `XDG_STATE_HOME` are redirected before `cairn-fixture` (client-stack rule 21):
/// unredirected, the set lands in the operator's real `~/Documents`. It only ever adds, and refuses
/// rather than overwrites.
pub fn install_into_platform_dirs(set: FileSet) -> Result<Landed, String> {
    let data = cairn_store::platform::data_dir().map_err(|e| e.to_string())?;
    let state = cairn_store::platform::state_dir().map_err(|e| e.to_string())?;
    let coll = Collection::open(&data, &state).map_err(|e| e.to_string())?;
    set.install(&coll)
}

/// One file of a set: the name it is written under, its bytes, and what planning it must reach.
pub struct BenchFile {
    pub name: String,
    pub bytes: Vec<u8>,
    wants: Wants,
}

/// What a set reached — reported by the bench the way [`fixtures::Reached`](crate::fixtures::Reached)
/// is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landed {
    /// Files listed and planning as the set says.
    pub files: usize,
    /// How many of them this install wrote — zero on a second install, which is the point.
    pub written: usize,
}

impl fmt::Display for Landed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} files listed, {} written now, each planning as the set says",
            self.files, self.written
        )
    }
}

/// What one file must plan as. Checked field by field against **the tables below** rather than
/// against literals, so the tables stay the one statement of what each file does.
#[derive(Debug, Clone, Copy)]
enum Wants {
    Update,
    Several,
    Archive,
    Unreadable,
}

impl Wants {
    fn holds(
        self,
        sniffed: Option<&Profile>,
        outcome: &Result<Plan, Refusal>,
    ) -> Result<(), &'static str> {
        let deck = Some(&Profile::Deck);
        let reached = match self {
            Wants::Update => sniffed == deck && outcome.as_ref().is_ok_and(update_holds),
            Wants::Several => sniffed == deck && outcome.as_ref().is_ok_and(several_holds),
            Wants::Archive => {
                sniffed == Some(&Profile::Collection) && *outcome == Err(Refusal::WrongProfile)
            }
            Wants::Unreadable => sniffed.is_none() && *outcome == Err(Refusal::Unreadable),
        };
        match (reached, self) {
            (true, _) => Ok(()),
            (false, Wants::Update) => Err(
                "an update of a held deck that renames it, deletes held notes, moves notes in from \
                 a deck and from unfiled, and leaves that deck empty",
            ),
            (false, Wants::Several) => Err(
                "three decks with no header — one held and gaining notes, two new, one of those \
                 skipping a note already held",
            ),
            (false, Wants::Archive) => Err("a collection archive, refused as the wrong profile"),
            (false, Wants::Unreadable) => Err("no container at all, refused as unreadable"),
        }
    }
}

fn update_holds(plan: &Plan) -> bool {
    let [deck] = plan.decks.as_slice() else {
        return false;
    };
    deck.id == fixture_deck(0)
        && deck.path == Path::Update
        && deck.renamed_from.as_deref() == Some(DECKS[0].1)
        && deck.already_yours == UPDATE_KEPT.len()
        && deck.new_notes == UPDATE_NEW.len()
        && deck.deleted == UPDATE_RETRACTED.len()
        && deck.moving_in == moving_in_from(&UPDATE_MOVED)
        && plan.emptied_decks == emptied_by(&UPDATE_MOVED)
        && !deck.no_change
        && !deck.revision_conflict
}

fn several_holds(plan: &Plan) -> bool {
    let find = |id: DeckId| plan.decks.iter().find(|d| d.id == id);
    let shape = |d: Option<&DeckPlan>, path: Path, new: usize, yours: usize| {
        d.is_some_and(|d| {
            d.path == path
                && d.new_notes == new
                && d.already_yours == yours
                && d.moving_in.is_empty()
                && d.deleted == 0
                && d.renamed_from.is_none()
        })
    };
    plan.decks.len() == 3
        && plan.header == Header::default()
        && plan.emptied_decks.is_empty()
        && shape(
            find(fixture_deck(1)),
            Path::Update,
            PERSIAN_NEW.len(),
            PERSIAN_KEPT.len(),
        )
        && shape(
            find(parse(GERMAN.0)),
            Path::Create,
            GERMAN_WORDS.len(),
            GERMAN_HELD.len(),
        )
        && shape(find(parse(DUTCH.0)), Path::Create, DUTCH_WORDS.len(), 0)
}

/// The moving-in lines a set of moved rows produces, in the planner's order — grouped by the deck
/// each row is filed under in [`DECK_NOTES`], unfiled as `None`.
fn moving_in_from(rows: &[usize]) -> Vec<MovingIn> {
    let mut by_source: BTreeMap<Option<String>, usize> = BTreeMap::new();
    for &row in rows {
        let from = DECK_NOTES[row].0.map(|d| DECKS[d].1.to_owned());
        *by_source.entry(from).or_default() += 1;
    }
    by_source
        .into_iter()
        .map(|(from, count)| MovingIn { from, count })
        .collect()
}

/// The held decks a set of moved rows drains — every deck all of whose notes are among them. Sorted,
/// as the planner reports them.
fn emptied_by(rows: &[usize]) -> Vec<String> {
    let mut emptied: Vec<String> = (0..DECKS.len())
        .filter(|&d| {
            let filed: Vec<usize> = (0..DECK_NOTES.len())
                .filter(|&i| DECK_NOTES[i].0 == Some(d))
                .collect();
            !filed.is_empty() && filed.iter().all(|i| rows.contains(i))
        })
        .map(|d| DECKS[d].1.to_owned())
        .collect();
    emptied.sort();
    emptied
}

// --- The files ---------------------------------------------------------------------------------

/// The update file's deck name: **not** the name [`Fixture::Decks`] holds, so the plan states a
/// rename — ADR-0005 §9's *"the file wins"*, and the one effect the ADR concedes *"will feel lost"*.
const UPDATE_NAME: &str = "French A1";

/// Rows of [`DECK_NOTES`] under *Français* the update carries **live**: *already yours*.
const UPDATE_KEPT: [usize; 7] = [0, 1, 2, 4, 5, 11, 12];

/// Rows under *Français* the update carries as **tombstones** — held notes, so they bite (ADR-0008
/// §5). Together with [`UPDATE_KEPT`] this is every note the deck holds, which a test pins: a row in
/// neither would stay in the deck silently and make the file a picture of a partial update.
const UPDATE_RETRACTED: [usize; 3] = [18, 22, 23];

/// Held rows **filed elsewhere** that the update carries into *Français*: all four of *Vocabulaire de
/// la Révolution française*, which therefore **left empty** — and one unfiled note, so the preview
/// states both shapes of move, *from X* and *from an unfiled note*.
const UPDATE_MOVED: [usize; 5] = [14, 15, 16, 17, 24];

/// Notes held nowhere: *new*.
const UPDATE_NEW: [(&str, &str); 6] = [
    ("la boîte aux lettres", "the letterbox"),
    ("le volet", "the shutter"),
    ("la gouttière", "the gutter"),
    ("l'escalier", "the staircase"),
    ("le palier", "the landing"),
    ("la sonnette", "the doorbell"),
];

/// Rows under *فارسی* the three-deck file carries: all of them, so the Persian deck updates with
/// nothing moved or deleted — the quiet update, beside the loud one.
const PERSIAN_KEPT: [usize; 5] = [6, 7, 8, 9, 10];

const PERSIAN_NEW: [(&str, &str); 3] = [("دریا", "sea"), ("کوه", "mountain"), ("آسمان", "sky")];

/// A deck id the fixture does **not** hold, so its deck takes the create path.
const GERMAN: (&str, &str) = ("f1c70000-0005-4000-8000-000000000005", "Deutsch");

const GERMAN_WORDS: [(&str, &str); 6] = [
    ("der Bahnhof", "the station"),
    ("die Brücke", "the bridge"),
    ("der Schlüssel", "the key"),
    ("das Fenster", "the window"),
    ("die Straße", "the street"),
    ("der Wald", "the forest"),
];

/// A held, unfiled row the **new** German deck also carries. On the create path a held id is skipped
/// and never moved (ADR-0005 §2), so it reads *already yours* under a deck the user has never had —
/// the one line where the two paths disagree about the same note.
const GERMAN_HELD: [usize; 1] = [13];

const DUTCH: (&str, &str) = ("f1c70000-0006-4000-8000-000000000006", "Nederlands");

const DUTCH_WORDS: [(&str, &str); 4] = [
    ("de fiets", "the bicycle"),
    ("het huis", "the house"),
    ("de gracht", "the canal"),
    ("het brood", "the bread"),
];

/// A file this application wrote and can no longer parse (ADR-0022 §11).
const UNREADABLE_NAME: &str = "Chapters 1-4.cdeck";
const UNREADABLE_BYTES: &[u8] = b"was a deck once, now truncated";

/// The note-id spaces the files' own notes take, beside [`deck_note_id`]'s `0x01`.
const UPDATE_NEW_SPACE: u8 = 0x02;
const PERSIAN_NEW_SPACE: u8 = 0x03;
const GERMAN_SPACE: u8 = 0x04;
const DUTCH_SPACE: u8 = 0x05;

/// The update file: one deck, a header, and every destructive line.
fn update_file() -> Result<Vec<u8>, String> {
    let mut notes = held_rows(UPDATE_KEPT.iter().chain(&UPDATE_MOVED));
    notes.extend(new_rows(UPDATE_NEW_SPACE, &UPDATE_NEW));
    let deck = DeckContent {
        id: fixture_deck(0),
        name: UPDATE_NAME.to_owned(),
        notes: positioned(notes),
        tombstones: UPDATE_RETRACTED
            .iter()
            .map(|&row| Tombstone {
                id: deck_note_id(row),
            })
            .collect(),
    };
    // The file's own claims, shown as a header (ADR-0022 §7). The three-deck file carries none, so
    // the pair shows both states of it.
    let header = Metadata {
        author: "Marjan Rahimi".to_owned(),
        description: "A1 vocabulary for the first ten chapters.".to_owned(),
        licence: "CC BY-SA 4.0".to_owned(),
    };
    deck_file(&header, vec![deck])
}

/// ADR-0008 §8's multi-deck file: the Persian deck held, two decks new.
fn several_file() -> Result<Vec<u8>, String> {
    let mut persian = held_rows(PERSIAN_KEPT.iter());
    persian.extend(new_rows(PERSIAN_NEW_SPACE, &PERSIAN_NEW));
    let mut german = held_rows(GERMAN_HELD.iter());
    german.extend(new_rows(GERMAN_SPACE, &GERMAN_WORDS));
    let dutch = new_rows(DUTCH_SPACE, &DUTCH_WORDS);

    let deck = |id: DeckId, name: &str, notes| DeckContent {
        id,
        name: name.to_owned(),
        notes: positioned(notes),
        tombstones: Vec::new(),
    };
    deck_file(
        &Metadata::default(),
        vec![
            deck(fixture_deck(1), DECKS[1].1, persian),
            deck(parse(GERMAN.0), GERMAN.1, german),
            deck(parse(DUTCH.0), DUTCH.1, dutch),
        ],
    )
}

/// A collection archive — the other profile in the same container. Nothing about its contents is
/// read on the way to its refusal, so it carries no rows; its counts are only what a manifest says.
fn archive_file() -> Vec<u8> {
    build_collection(&CollectionArchive {
        collection_id: &CollectionId(bench_id(0x09, 1).0),
        created: "2026-09-01T00:00:00Z",
        notes: DECK_NOTES.len(),
        reviews: DECK_NOTES.len() * 2,
        log: &[],
        mutable: &[],
    })
}

fn deck_file(header: &Metadata, decks: Vec<DeckContent>) -> Result<Vec<u8>, String> {
    let exports = decks
        .into_iter()
        .map(|content| {
            let digest = deck_digest(&content).map_err(|e| e.to_string())?;
            let revision = next_revision(None, &digest);
            Ok(DeckExport { content, revision })
        })
        .collect::<Result<Vec<_>, String>>()?;
    build_deck(header, &exports).map_err(|e| e.to_string())
}

/// Held rows of [`DECK_NOTES`] at their published ids, carrying the content the fixture holds — so a
/// kept note changes nothing but its deck, and the plan's lines are about membership alone.
fn held_rows<'a>(rows: impl Iterator<Item = &'a usize>) -> Vec<NoteContent> {
    rows.map(|&row| {
        let (_, front, back) = DECK_NOTES[row];
        basic(deck_note_id(row), front, back)
    })
    .collect()
}

fn new_rows(space: u8, words: &[(&str, &str)]) -> Vec<NoteContent> {
    words
        .iter()
        .enumerate()
        .map(|(i, (front, back))| basic(bench_id(space, i as u8 + 1), front, back))
        .collect()
}

fn basic(id: NoteId, front: &str, back: &str) -> NoteContent {
    NoteContent {
        id,
        position: String::new(),
        kind: "basic".to_owned(),
        fields: vec![
            ("Back".to_owned(), back.to_owned()),
            ("Front".to_owned(), front.to_owned()),
        ],
    }
}

/// Give each note a key in the order it was listed. The key fixes emission order only and is not
/// itself exported (ADR-0008 §12 as amended by ADR-0021 §3).
fn positioned(mut notes: Vec<NoteContent>) -> Vec<NoteContent> {
    for (i, note) in notes.iter_mut().enumerate() {
        note.position = format!("a{i:03}");
    }
    notes
}

fn fixture_deck(index: usize) -> DeckId {
    parse(DECKS[index].0)
}

/// A deck id this module writes as a literal. The literals are pinned canonical by a test, so this
/// cannot fail on a build that passed it.
fn parse(id: &str) -> DeckId {
    DeckId::parse_canonical(id).expect("the bench's deck ids are canonical UUID text")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const NOW_MS: i64 = 20_514 * 86_400_000 + 4 * 3_600_000;

    fn installed(fixture: Fixture) -> (TempDir, TempDir, Collection) {
        let data = TempDir::new().unwrap();
        let state = TempDir::new().unwrap();
        let mut coll = Collection::open(data.path(), state.path()).unwrap();
        fixture.install(&mut coll, NOW_MS).unwrap();
        (data, state, coll)
    }

    fn as_found(set: FileSet) -> Vec<(String, Vec<u8>)> {
        set.files()
            .unwrap()
            .into_iter()
            .map(|f| (f.name, f.bytes))
            .collect()
    }

    /// **Every file plans the way its set says, against the fixture it is built for.** The assertion
    /// the bench rests on, as `every_fixture_lands_where_it_says_it_lands` is for the other half.
    #[test]
    fn every_file_plans_as_its_set_says_against_its_fixture() {
        for set in FileSet::ALL {
            let (_d, _s, coll) = installed(set.against());
            let landed = set
                .check(&coll, &as_found(set))
                .unwrap_or_else(|e| panic!("{}: {e}", set.key()));
            assert_eq!(landed.files, 4);
        }
    }

    /// **Against any other collection the set is refused**, which is what stops a storyboard that
    /// forgot its `fixture` line from photographing *new deck* under the update file's name.
    #[test]
    fn against_any_other_fixture_the_set_is_refused() {
        for fixture in Fixture::ALL
            .into_iter()
            .filter(|f| *f != FileSet::Imports.against())
        {
            let (_d, _s, coll) = installed(fixture);
            let refused = FileSet::Imports.check(&coll, &as_found(FileSet::Imports));
            assert!(
                refused.is_err(),
                "{} is not the fixture these files name, and the set installed anyway",
                fixture.key()
            );
        }
    }

    /// **The update file draws every line ADR-0022 exists for**, asserted as the shape the ticket
    /// asked for and not only through the tables — a table edit that quietly removed the rename or
    /// the empty deck would still agree with itself.
    #[test]
    fn the_update_file_draws_every_destructive_line() {
        let (_d, _s, coll) = installed(Fixture::Decks);
        let file = FileSet::Imports.files().unwrap().remove(0);
        let report = inbound::read(&listing::select(&file.name, file.bytes), &coll).unwrap();
        let plan = report.outcome.unwrap();
        let deck = &plan.decks[0];

        assert_eq!(deck.path, Path::Update, "updating a deck you already have");
        assert!(deck.already_yours > 0 && deck.new_notes > 0);
        assert!(deck.deleted > 0, "tombstones that bite");
        assert!(
            deck.moving_in.iter().any(|m| m.from.is_some())
                && deck.moving_in.iter().any(|m| m.from.is_none()),
            "notes moving in from a deck and from unfiled: {:?}",
            deck.moving_in
        );
        assert!(deck.renamed_from.is_some(), "a rename");
        assert!(!plan.emptied_decks.is_empty(), "a deck left empty");
        assert_ne!(plan.header, Header::default(), "and a header to separate");
    }

    /// The tables say what their comments say. Each is a claim about [`DECK_NOTES`], which lives in
    /// another module and can be edited without reading this one.
    #[test]
    fn the_tables_span_what_they_say() {
        let filed = |d: usize| -> Vec<usize> {
            (0..DECK_NOTES.len())
                .filter(|&i| DECK_NOTES[i].0 == Some(d))
                .collect()
        };

        let mut francais: Vec<usize> = UPDATE_KEPT
            .iter()
            .chain(&UPDATE_RETRACTED)
            .copied()
            .collect();
        francais.sort();
        assert_eq!(francais, filed(0), "kept and retracted are all of Français");

        assert!(
            filed(2).iter().all(|i| UPDATE_MOVED.contains(i)),
            "every Révolution note moves, so the deck is left empty"
        );
        assert!(
            UPDATE_MOVED.iter().any(|&i| DECK_NOTES[i].0.is_none()),
            "and one unfiled note moves"
        );
        assert_eq!(PERSIAN_KEPT.to_vec(), filed(1), "all of فارسی");
        for &i in &GERMAN_HELD {
            assert!(
                DECK_NOTES[i].0.is_none(),
                "the German deck's held note is unfiled"
            );
            assert!(
                !UPDATE_MOVED.contains(&i),
                "and not also moved by the update"
            );
        }

        for id in [GERMAN.0, DUTCH.0] {
            assert!(DeckId::parse_canonical(id).is_some(), "{id} is canonical");
            assert!(
                DECKS.iter().all(|(held, _)| *held != id),
                "{id} is new to the fixture"
            );
        }
    }

    /// **Byte-identical on every call**, which is what lets a second install recognise its own files
    /// rather than dedupe them. Nothing here reads a clock or entropy; this keeps it that way.
    #[test]
    fn the_files_are_the_same_bytes_every_time() {
        let once = as_found(FileSet::Imports);
        let again = as_found(FileSet::Imports);
        assert_eq!(once, again);
        let names: std::collections::HashSet<&str> = once.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names.len(), once.len(), "no two files share a name");
    }

    #[test]
    fn an_unknown_file_set_key_is_refused() {
        assert_eq!(FileSet::parse("imports"), Some(FileSet::Imports));
        assert_eq!(FileSet::parse("import"), None);
    }
}
