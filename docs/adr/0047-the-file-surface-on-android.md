# ADR-0047: The file surface on Android — the system picker in, the save dialog out

- **Status**: Accepted
- **Date**: 2026-10-29
- **Resolves**: [Native Android: What the File Surface Is Once the System Picker Exists](https://github.com/amin-bf/cairn/issues/196)
- **Map**: [Map: Cairn as a Native Android App in Kotlin](https://github.com/amin-bf/cairn/issues/193)
- **Decided by**: the agent, under [ADR-0045 §7](0045-the-native-android-client.md) — every answer
  below either follows the platform's documented practice for storage access or is forced by an
  earlier ADR, so none was put to the repository owner
- **Amends, on Android only**: [ADR-0016 §5](0016-backup-and-restore.md) (the seam's put, get and
  list), [ADR-0022 §6 §10 §11](0022-the-import-preview-and-export-report.md) (the entry points, the
  report, the list), [ADR-0024 §3](0024-identifying-a-written-file.md) (the list's scope),
  [ADR-0041 §5](0041-the-file-surface.md) (the file list block)
- **Related**: [ADR-0045 §6](0045-the-native-android-client.md) (the picker and a typed filename,
  lifted on Android), [ADR-0023](0023-sending-a-written-file.md) (the hand-off),
  [ADR-0024 §1](0024-identifying-a-written-file.md) (identity is in the bytes),
  [Native Android: Where the Line Between Kotlin and Rust Sits](https://github.com/amin-bf/cairn/issues/195)
  (who does the I/O — §6)

## Context

ADR-0041's file surface was built around two absences. With no picker, the only way to reach a file
the user had not just received was a list of the files this application wrote, read from its own
`MediaStore` rows (ADR-0024 §3), each described from its manifest (ADR-0022 §11). With no typed
filename, export put the file in `Downloads` under a name the application derived, and the report's
whole job was to say what that name turned out to be and where (ADR-0022 §10). ADR-0045 §6 removes
both absences on Android. The question was whether the list survives beside the picker, and whether
export still goes to `Downloads` or wherever the user saves it.

The platform's answer is the Storage Access Framework: `ACTION_OPEN_DOCUMENT` to open a document,
`ACTION_CREATE_DOCUMENT` to save one, both needing no permission, both reaching every provider the
device has (local storage, a cloud drive, a stick), and both returning a document the application
may read or write once. That is the established practice ADR-0045 §7 says to follow; what follows is
what it does to the surface Cairn already specified.

## Decision

### 1. The picker replaces the list on Android

> **Settings' *Files* block on Android holds an *Open a file…* action where the list was. It opens
> the system picker. There is no list of files beside it.**

The list existed because nothing else could reach a file (ADR-0022 §11: *"on Android the list is the
primary way in"*), and it could only ever show what this application wrote. The picker reaches those
files too — the egui build's exports are ordinary documents in `Downloads` — and every file the list
could never show. Its own *Recent* root lists the documents just opened or saved, which is the one job
a list of our own would still do.

**Rejected: the list beside the picker.** After §2 nothing writes `MediaStore` rows any more, so the
list would hold only the egui build's old exports and never grow: a block that shrinks to its empty
state on every new install, offering a narrower door to files the picker already opens.

**Rejected: a recent-files list of our own**, kept by persisting the picker's grants. It re-implements
the picker's *Recent* root, and a persisted grant goes stale silently when the file is moved or
deleted, so the list would show files it can no longer open.

**What the list gave that is lost, and where it went.** The list described each file from its
manifest before it was opened, so three archives could be told apart by date (ADR-0022 §11). The
preview still states what a file is before anything is applied (ADR-0022 §3, §12), and §3 puts the
date in the name an archive is offered under.

### 2. Every file opened is offered, and the bytes decide

> **The picker asks for any type (`*/*`). Whatever is chosen is read once, sniffed, and taken to the
> one import preview — a file that is not ours meets the preview's refusal (ADR-0022 §4).**

On the device `.cdeck` and `.ccoll` are both `application/octet-stream`, and a cloud provider may type
them as anything, so no type filter can select them and ADR-0024 §1 already makes the bytes the sole
authority. **This closes a hole the filters cannot**: ADR-0024 §1's correction found a deck named
`.txt` typed `text/plain` and never offered to us. The picker offers it; the sniff identifies it.

The grant is used once and not persisted, since nothing is kept to reopen (§1). The **intent
filters stay as they are** (ADR-0024 §2): *Open with* and the share sheet are how a file arrives from
another application, and the picker is how the user goes looking for one. All three reach the same
cold-start-capable screen, which is ADR-0022 §6's *one screen, three entry points* with the list's
place taken by the picker.

### 3. Export saves through the system save dialog, under a name the user may change

> **Exporting a deck file or writing a collection archive opens the system save dialog, pre-filled
> with the name the application would have used. The user picks the place and may edit the name.
> Cancelling writes nothing and says nothing.**

The pre-filled name for a deck file is ADR-0022 §10's derived one (`French A1.cdeck`,
`French A1 and 2 more.cdeck`), sanitised outbound exactly as before, since a deck name still arrived
from a stranger. **An archive is offered as `Cairn backup 2026-10-29.ccoll`**, the date being the archive's creation date (the one in its
manifest, ADR-0016 §11), written year first so a folder of them sorts by age. `collection.ccoll`
relied on the list to tell archives apart; with the list gone, the name is the first place a user
sees which archive is which.

The document is created as `application/octet-stream`, the type the platform gives our files anyway,
for ADR-0024 §4's reason: a declared type that disagrees with the name is what moved the dedupe suffix
past the extension. Whether a given provider keeps the typed extension, and how it renames on a
collision, is measured on the handset when the export slice is built (`AGENTS.md` rule 9), not
assumed here. **The application does not police the name**: a name typed without the extension is
the user's choice, and the file is still identified by its bytes.

**Rejected: keep writing to `Downloads` and leave the dialog out.** ADR-0045 §6 lifted the refusal
precisely so the user could choose; a silent `Downloads` write was the workaround for not being able
to ask.

### 4. The report states the name written, and the place stops being its job

> **On Android the export report states the name the provider actually gave the document, and no
> location line. The hand-off stays beside it.**

ADR-0022 §10's rule holds: read back what was written, never echo what was asked, because a provider
may still rename. But its premise — *the user chose neither the name nor the location, so the
application is the only thing that knows either* — no longer holds on Android: the user has just
chosen both, and a document gives the application no path it could print. *Saved to Downloads* would
now often be false.

**The hand-off is unchanged** (ADR-0023): a share action beside the report, handing on the document
just saved with a read grant, never firing by itself.

**Rejected: share straight from a cache, without saving.** ADR-0023 §6 kept the report's lines beside
the send because *"a report that spent the location on a send button would leave everyone who
declines unable to find their file"*; a share with no saved file is that case again.

### 5. The desktop is unchanged

The desktop keeps ADR-0016 §5 as written — the list, the documents folder, drag-and-drop, and the
report's path line — until a desktop effort decides otherwise (the map's *Out of scope*). Every
amendment below is scoped to Android.

### 6. What this needs from the core, and what hangs on #195

The Android file surface calls **none of `cairn-export::platform`'s four operations**: the picker and
the save dialog hold the document, and the share is an intent. What it needs from the core is
**bytes in and bytes out** — summarise, plan and apply an import from a file's bytes; build a deck
file or an archive as bytes; the two suggested names.

[Where the Line Between Kotlin and Rust Sits](https://github.com/amin-bf/cairn/issues/195) decides
the rest: whether the document's stream is read in Kotlin and passed across as bytes or handed to Rust
as a file descriptor (an archive is a decade of log rows, so this is a memory question as well as a
binding one), and what becomes of the seam's Android arm, which the egui build still calls until it
retires (ADR-0045 §5).

> **Note (29.10.2026):** #195 has since been decided by
> [ADR-0046 §5](0046-the-line-between-kotlin-and-rust.md): Kotlin reads the document and the bytes
> cross both ways, and the seam's Android arms stay, uncalled by the Kotlin app, until the egui
> build retires.

## Amendments to accepted ADRs

All on Android only; the desktop reads each as written.

- **[ADR-0016 §5](0016-backup-and-restore.md)** — the seam's **put, get and list** are replaced by
  the save dialog and the picker; there is no Android list. *Hand off* stays, now handing on a
  saved document. ADR-0045 §6 lifted the section's refusals; this says what replaces them.
- **[ADR-0022 §6](0022-the-import-preview-and-export-report.md)** — the three entry points are the
  **picker**, the *Open with* filter and the share filter. One screen, cold-start capable, unchanged.
- **[ADR-0022 §10](0022-the-import-preview-and-export-report.md)** — the name is a suggestion the
  user may edit; the report keeps the name line and drops *Saved to Downloads*.
- **[ADR-0022 §11](0022-the-import-preview-and-export-report.md)** and
  **[ADR-0024 §3](0024-identifying-a-written-file.md)** — there is no list on Android, so the rule
  that it must not pretend to be a folder view has nothing to bind there.
- **[ADR-0041 §5](0041-the-file-surface.md)** — the *Files* block on Android is the *Open a file…*
  action and the export actions; the list's line, empty state and captions are the desktop's.

## Consequences

- No storage permission is requested, as before; the Storage Access Framework needs none.
- A file sent from another application or saved by one is now reachable on Android without the
  *Open with* sheet — the gap ADR-0024 §3 recorded as unfixable.
- The `MediaStore` facts in `AGENTS.md` (deck-export rules 11–15) and backup rule 5 describe the egui
  build; they are re-worded when it retires (the map's fog, *Retiring the egui Android build*), not
  here.
- **Open, for the build**: how the device's providers treat a typed extension and a colliding name
  under `ACTION_CREATE_DOCUMENT`; and whether a shared document URI from a cloud provider reaches
  the receiving application as reliably as a `MediaStore` URI did (ADR-0023's recipient read).
