# leitner-app

## Writing conventions

**Name the fact, not the product.** Prior art is cited by what it establishes and why, never by a
bare product name standing in for the explanation. "X does it this way" tells a reader nothing
unless they already know X; it makes the document depend on knowledge that isn't in it.

A named application may appear **only alongside the substance** — the mechanism, the reasoning, and
a primary source — so the passage stands on its own and a reader who has never used that
application loses nothing. Research notes in `docs/research/` are where this most often applies:
they exist to carry evidence, so the evidence must be written out, not pointed at.

Everywhere else — ADRs, `CONTEXT.md`, issues, code, commit messages — prefer stating the finding
and its source directly. If a fact only exists as "that app does X", find the underlying source or
argue the trade-off on its own merits.

This applies to every agent working in this repo, on every artifact that persists.

## Landing work

Tickets here are worked in **parallel worktree sessions**, each branching from `origin/main` and
never seeing what merged afterwards. Two things break silently because of it.

### Rules that are easy to break silently

1. **If commit signing fails, stop and ask. Never fall back to `--no-gpg-sign`.** Every commit on
   `main` is signed, and this repo merges with merge commits rather than squashing, so unsigned work
   lands on `main` as-is and someone has to clean it up. The passphrase comes from a GUI prompt, so
   an unattended session gets `gpg: signing failed: Timeout` and **no commit object is written** —
   the work is not lost, it simply has not been committed yet. Land everything that needs no commit
   first (tracker updates, verification runs) so the pause blocks only the commit, then wait.
   To repair history that already went unsigned:
   `git rebase --exec 'git commit --amend --no-edit -S' origin/main` then
   `git push --force-with-lease`.
2. **Re-check the highest ADR number immediately before committing**, not when you start writing.
   `git fetch origin && git ls-tree --name-only origin/main docs/adr/` — a parallel session may have
   taken your number, and the worktree's own `docs/adr/` is a stale answer that looks authoritative.
   When renumbering, **sed only your own files**: `AGENTS.md` and older ADRs may already reference
   the *other* ADR at that number, and a blind replace corrupts those links. Re-read whatever landed
   meanwhile, too — an ADR that merged mid-session may place requirements on the ticket you are
   resolving.

## Start with the context map

**[`CONTEXT-MAP.md`](./CONTEXT-MAP.md) is the entry point to the codebase** — the six crates, the
eight contexts, and an index saying which ADR sections bind which context. `docs/adr/` is over 2,600
lines; the index is what stops "read the ADRs" from meaning all of them.

Read this file first, then the context map, then the `CONTEXT.md` for the area you are touching.
`docs/research/` is the evidence trail for reopening a decision, not reading for implementing one.

## The workspace

Six crates, laid out in [ADR-0009](./docs/adr/0009-crate-and-workspace-layout.md) and extended by
[ADR-0013 §11](./docs/adr/0013-the-sync-transport.md):
`leitner-core` (the domain, pure), `leitner-store` (SQLite and the platform seam), `leitner-export`
(the `.ldeck` container), `leitner-sync` (publishing to the remote; holds the network dependencies),
`leitner-app` (egui, lib + cdylib), `leitner-desktop` (a shim, forced by `cargo-apk`).

Contexts are **modules, not crates** — with two exceptions, both for the same reason: `export` and
`sync` hold dependencies `leitner-core` may not have. A context becomes a crate only when it must
carry one. Vocabulary lives in a `CONTEXT.md` beside the code; decisions live system-wide in
`docs/adr/`, and context-scoped `docs/adr/` directories are not used here.

### Rules that are easy to break silently

1. **`leitner-core` has no dependencies, and adding one is an ADR-sized decision.** Its empty
   `[dependencies]` is what makes `cargo test -p leitner-core` need no database, no window and no
   handset. `rusqlite` belongs in `leitner-store`; `egui` and `eframe` belong in `leitner-app`.
2. **Time and identity are values, never injected traits.** Replay needs no clock at all — day
   numbers are frozen on the row at write time, and fuzz is seeded from card identity. The two call
   sites that need "now" take it as a parameter. A `SystemTime::now()` inside `leitner-core` breaks
   the property that two devices replaying one log agree.
3. **There is no fake store.** Store tests open a real SQLite database in a temp directory, because
   the design *is* WAL, `BEGIN IMMEDIATE`, `ATTACH` and `INSERT OR IGNORE`.
4. **A new ADR must be added to `CONTEXT-MAP.md`'s index.** One that is not there is invisible to
   the agent it was written for.

## The card model

A note holds content; a card is a generated view of it, identified by
`CardRef { note: NoteId, ordinal: u16 }` — eighteen canonical bytes, no standalone card id
([ADR-0002](./docs/adr/0002-the-card-model.md)). The ordinal is a **slot assigned by the kind
definition**, chosen in [ADR-0017](./docs/adr/0017-card-slots.md).

### Rules that are easy to break silently

1. **A slot number is a card's identity. Never change one, never reuse one for a different question.
   List order is free.** Slots are drawn from one namespace shared by *every* kind, so `basic` and
   `basic-reverse` both declare slot 0 for Front→Back — deliberately, because it is the same card, and
   that sharing is what lets a note gain its reverse direction without orphaning its history. Change a
   slot and you silently retype every accumulated review onto the wrong card, which the log cannot be
   edited to repair. This replaces ADR-0002 §4's old "never reorder the `cards` list" rule: **reading
   the list index instead of the slot field is now the defect.**
2. **Two tests guard rule 1, and they are the only reason it is enforceable.** Slot uniqueness across
   the shipped definitions, and slot immutability against a checked-in golden `slot → (prompt, answer)`
   list. Both need no database, no window and no handset. Deleting or weakening either returns the most
   destructive edit in this codebase to being a prohibition nobody is pointed at.
3. **Cloze blanks live above the high bit: blank `n` is slot `0x8000 | n`.** Fixed-arity slots occupy
   `0x0000–0x7FFF`. The partition is what makes the two schemes unable to collide, so never allocate a
   fixed-arity slot with the top bit set, and never read a cloze ordinal as a blank number without
   masking (`ordinal & 0x7FFF`). A raw log row for cloze blank 1 reads `ordinal 32769`; that is correct.
4. **`CardRef` carries no kind discriminator, and adding one is wrong rather than merely expensive.**
   Kind-scoped identity orphans the reviews of a `basic` note that gains its reverse — the most likely
   kind change there is, and one where reattachment is *correct* — with the same silence as the hazard
   it would fix. When two kinds should share history, give them the same slot.
5. **An imported kind definition's slots are never validated, and this is safe for one reason.** A card
   is `(note UUID, slot)`, so two kinds sharing a slot collide only within *the same note* — and the
   kind dropdown offers the shipped kinds plus the note's own current kind, so a note can never be
   switched *into* an acquired kind. Add acquired kinds to that dropdown and the importer suddenly owes
   a check it does not have.

## Authoring and the note list

The app has three top-level destinations — **Review, Notes, Settings** — and the **note list** is the
browse surface all authoring hangs off, specified in
[ADR-0021](./docs/adr/0021-note-ordering-saving-and-the-note-list.md). Until it existed, two ADRs were
already leaning on a screen nobody had built.

### Rules that are easy to break silently

1. **Reordering a note writes exactly one value. Never renumber.** `position` is an **order key with
   infill**, not an integer, precisely so a move costs one write. A bulk rewrite — a "tidy up
   positions", a compaction, a migration that redistributes keys — is N independent writes on ADR-0004
   §7's surface, and **order is a gestalt, so one lost value scrambles the whole list**: two devices
   reordering concurrently agree on neither order and nothing reports it. The integer looks simpler and
   is the trap; ADR-0011 §7's *"need not be dense"* was a permission its own high-water counter never
   let anyone use.
2. **The note list has one order and no sort control.** A sort makes reordering meaningless while it is
   active — a drag inside an alphabetical view has no definable result — and nothing fails when someone
   adds one. Filters narrow; nothing re-sorts. Text search is the answer to "find this note".
3. **Never put schedule information on the note list — not even aggregated.** A note generates several
   cards in several boxes, so any per-note figure is boxes **counted**, which ADR-0001 §3 forbids
   outright. This is constraint 4 deciding a rendering, and it reads as a helpful addition every time.
4. **Never bind a key to "the last field".** Which field is last is a property of the **kind
   definition**, which is *data*, so a kind gaining a field silently changes what the key does with no
   code change and nothing failing — and ADR-0008 §7 lets a note carry an *acquired* kind, putting that
   in a stranger's hands. Enter is inert in every single-line field, the last included; the *New note*
   rhythm is a modifier chord.
5. **The editor holds no unsaved state, and that is a durability rule rather than a preference.**
   Autosave is per field on blur or a short idle. Client-stack rule 10 means a backgrounded Android app
   is **frozen, then possibly killed** — so under an explicit save, putting the phone down mid-note is
   the standard way to lose work, with no error and no chance for the app to warn. Adding a Save button
   also un-does ADR-0012 §5, which deliberately moved the only decision a save could carry onto the
   ambient warning.
6. **"Never sync while the review screen is up" does not mean "nothing may change the queue
   mid-session".** A note edited from the review screen changes it immediately and correctly. That rule
   (sync rule 2 below, ADR-0015 §6) bans an **unannounced** recompute caused by another device — not
   the visible result of the user's own act on the card in front of them. Read broadly, it deletes
   mid-review editing as a violation, which is the predictable mistake.
7. **Opening the editor from the review screen counts as a reveal.** The editor shows the back, so
   without this ADR-0006 §4's *"self-grading can't happen before the answer is seen"* is quietly false.
   The alternatives both need state this design does not have: skipping a card ungraded needs an
   in-session deferred set, which ADR-0006 §2 proved does not exist, and flagging it for later is the
   stored *"since you last looked"* ADR-0010 §9 refused.

## The client stack

**egui / eframe**, chosen in [ADR-0003](./docs/adr/0003-client-stack.md). One binary per platform,
no webview, no IPC. Setup and commands are in [`README.md`](./README.md).

**The targets are desktop and Android.** The web target was ruled out of scope in
[ADR-0007 §1](./docs/adr/0007-the-local-store.md) — a browser cannot be the system of record for an
app whose only copy of the data is local. Rules below that mention the web build are retained
because they are validated findings, not because a web build ships.

### Rules that are easy to break silently

1. **All user-visible text goes through the bidi helper.** egui places text runs left-to-right in
   logical order, so a plain `ui.label("…")` renders Persian and Arabic with the words backwards, and
   Arabic-Indic digits reversed. Build a `LayoutJob` with sections in visual order instead. A
   `ui.label` on card content is a bug, not a style choice.
2. **`TextEdit` needs the same treatment, via `.layouter()`** — it lays out its own text and
   otherwise bypasses the helper. Note that caret and selection are then in visual order while the
   buffer is logical, so RTL editing is imprecise; design around it rather than fighting it.
3. **The storage seam is a compile-time `#[cfg]`.** Keep it that way. Never introduce a runtime
   platform check — the whole stack choice rests on wrong platform code failing the build. It is two
   functions in `leitner-store::platform`, and a **third function appearing there means the seam is
   eroding**. A `#[cfg(target_os)]` anywhere else in the workspace is a defect.
4. **Immediate mode has nowhere to `await`.** Spawn the future, store a handle, read the result on a
   later frame, and call `ctx.request_repaint()` on completion or the result sits unseen until the
   next input event.
5. **The desktop binary lives in its own crate, and that crate stays empty.** `cargo-apk` panics
   after signing when one crate has both a cdylib and a bin — the APK is fine, the exit code is not,
   and CI breaks. So never add a `[[bin]]` to `leitner-app`, and never put logic in
   `leitner-desktop`: code there is never compiled for Android and never runs on the handset.
   It takes `eframe` by re-export from `leitner-app`, not as its own dependency.
6. **`eframe`'s dependency is split per target** — its default `accesskit` feature is rejected
   alongside `android-native-activity`.
7. **Fonts are ours to ship — and must be installed on the first frame, not in `CreationContext`.**
   egui bundles only Hack, Ubuntu-Light and Noto Emoji. Register any added face in **every** family
   you use, including `Monospace`, or text silently renders as boxes. Registering during creation
   breaks the web build: wgpu panics with "Tried to update a texture that has not been allocated
   yet", glow renders everything near-black. Defer it one frame.
8. **Android text input is ASCII-only, and cannot be fixed here.** winit's Android backend handles
   only motion and key events — it has no IME path, so composed text never reaches the app. This is
   not the activity backend: GameActivity was tried and reverted (see
   [`prototypes/egui-slice/android/README.md`](https://github.com/amin-bf/leitner/blob/prototypes/issue-8/prototypes/egui-slice/android/README.md)). Never design a feature that requires typing non-Latin
   text on Android.
9. **Verify Android on the real handset.** The emulator is x86_64; the Pixel 8 Pro is arm64-v8a only.
10. **A backgrounded Android app is frozen, not slowed.** The process moves to the `/background`
    cpuset (~13× the CPU time) and is then frozen outright — `isFrozen=true`, `utime` stopped dead —
    so long work *stops* rather than running slowly: 303 s of wall clock for 4.3 s of work, measured
    in [`docs/research/fsrs-on-device/`](./docs/research/fsrs-on-device/README.md). It may also be
    killed outright, since a process holding hundreds of megabytes is a prime low-memory-killer
    target. So long work is started **from the foreground, by a user action**, on a worker thread
    polled by the frame loop (rule 4), with **nothing persisted until it completes** — then a frozen
    or killed run holds no partial state and the recovery action is to press the button again. Never
    schedule it, never take a wakelock, and never promise the user that a started job is still
    progressing. [ADR-0014 §3](./docs/adr/0014-when-parameter-optimisation-runs.md).

## The local store

Two SQLite files, chosen in [ADR-0007](./docs/adr/0007-the-local-store.md): `collection.db` is
authoritative, `derived.db` is a disposable cache attached to the same connection.

### Rules that are easy to break silently

1. **`INSERT OR IGNORE` is for merge-ingest only. Our own writes use plain `INSERT`.** On another
   device's rows it is the union merge; on our own it silently drops a review.
2. **Never take the next sequence number from `MAX(seq) WHERE writer = me`.** After a merge that
   continues *another* device's numbering, which is the duplicate-writer failure the design exists to
   prevent. Use `local.seq_highwater` — and a log row above it means someone else is writing as us,
   so mint a new writer id.
3. **Every write transaction is `BEGIN IMMEDIATE`.** Sequence allocation is a read-modify-write; a
   deferred transaction loses updates between two processes.
4. **Only `log.line` is authoritative.** Every other column, and everything in `derived.db`, is
   derived and may be dropped and rebuilt. Derived columns do **not** have to round-trip.
5. **The writer marker lives outside the backup set** — `getNoBackupFilesDir()` on Android,
   `$XDG_STATE_HOME` on desktop. Move it into the data directory and a restored phone becomes a
   duplicate writer.

## Deck export

A `.ldeck` file is a **zip archive** carrying deck content and never review progress, chosen in
[ADR-0008](./docs/adr/0008-the-deck-export-format.md).

### Rules that are easy to break silently

1. **A shipped kind definition always wins; an imported file may never overwrite one.** A file's
   definition is used only for kinds this build does not ship. Break this and an import becomes a
   remote path to reordering a kind's `cards` list, which silently retypes every accumulated review
   onto the wrong card and cannot be repaired from the log.
2. **Never write an ADR-0004 §7 stamp into a *deck* export.** A stamp is a counter plus a **writer
   id**, which is a device fingerprint, and its counter is meaningless outside its own collection.
   Import assigns fresh local stamps — and only to values whose content actually differs, or every
   import floods the user's own devices with edits. The **`collection` profile is the exact opposite**
   and carries stamps byte for byte, because a restore does not cross a collection boundary; the
   profile is what selects the rule, which is why it is a profile and not a flag.
3. **Import branches on deck id, and the branches have opposite rules.** Id already held → the file
   wins and may move notes into that deck (ADR-0005 §9). Id new → notes already held are never touched
   or moved (ADR-0005 §2). Applying one rule everywhere lets a stranger's file take notes out of decks
   the user already has.
4. **The importer accepts only the known member names and the `media/` prefix**, rejecting absolute
   paths, `..` segments and symlink entries. Zip path traversal is this container's classic defect.
5. **A *deck* export must be byte-for-byte deterministic** — fixed member order, pinned timestamps,
   fixed deflate level, no extra fields. Zip's per-member timestamps otherwise make identical content
   export as different bytes, which leaks build time and breaks "same revision, same file". This binds
   the `deck` profile only: a collection archive carries a creation date instead (ADR-0016 §11).
6. **The revision advances only when the content digest changes**, not on every export. Incrementing
   per export inflates the counter and makes relaying an unmodified deck emit a phantom revision that
   competes with the original author's next one.
7. **`zip`'s `deflate-flate2` feature does not compile** — it selects no zlib backend. Use
   `--no-default-features --features deflate-flate2-zlib-rs`; the `deflate` umbrella feature builds but
   drags in zopfli for an encoder we do not need.
8. **An import is gated, and the plan behind the gate is derived on every read.** Caching it stores a
   projection of the log — the thing ADR-0004 exists to prevent — and a sync landing while the preview
   is on screen falsifies it. Derived, promise and effect cannot diverge, which is why **nothing is
   reported after an import commits** ([ADR-0022 §1, §5](./docs/adr/0022-the-import-preview-and-export-report.md)).
9. **The manifest gates; the payload describes.** A refusal — unknown `format`, wrong profile,
   revision below the one held, a broken path rule — must never require inflating a payload. The
   preview then states **effects on this collection**, not the manifest's counts, which are the wrong
   numbers in exactly the cases the preview exists for: how many notes are genuinely new after the
   collision skip, how many move deck, and how many tombstones match a note we actually hold.
10. **Every string arriving in a file is hostile.** Author, description, licence and deck names render
    as plain text, never Markdown, length-bounded — the preview shows a stranger's strings *before* the
    user has agreed to anything. Deck names are sanitised **outbound** too, since the export filename
    is derived from one.
11. **Read back the filename the platform wrote; never echo the one requested.** The Android put is a
    `MediaStore` insert, and the user chose neither name nor location, so the report is the only way
    they can find the file at all. **Measured on the handset** — it **dedupes**, and the suffix lands
    *after* the extension: `French A1.ldeck (1)`, not `French A1 (1).ldeck`
    ([evidence](./docs/research/android-outbound-share/README.md)). `MediaStore` also **discards the
    media type we declare**, deriving it from the extension instead, and a type the name disagrees
    with makes it **rename the file**. What that costs the extension-matched launch filter is
    [#72](https://github.com/amin-bf/leitner/issues/72)'s.

## Sync

The log is published to **storage we do not own** — a personal cloud drive's application data folder
— as immutable objects under a per-writer namespace, chosen in
[ADR-0013](./docs/adr/0013-the-sync-transport.md). The remote is a **rendezvous point, not a system
of record**: `collection.db` is authoritative and every device holds the whole log, so deleting the
remote costs one republish and no data.

### Rules that are easy to break silently

1. **The `log` and `state` roll-ups are opposite, and one direction destroys review history.**
   `…/log/` merges losslessly — ADR-0004 §10 forbids compaction. `…/state/` merges by keeping only
   the winning stamp per key, because that is what settling means. Apply the state rule to the log
   and reviews are gone with nothing downstream able to notice.
2. **Write the merged object before deleting, and delete only what it covers.** Deletion in the
   application data folder is **permanent** — files there cannot be trashed
   (`notSupportedForAppDataFolderFiles`), so ordering is the only protection. A `404` on a key that
   was listed a moment ago means *list again*, never *attempt recovery*.
3. **Nothing published is ever rewritten, and no code may use a conditional write.** Every key has
   exactly one author, so compare-and-swap protects against nothing here — and two of three servers
   tested silently ignored the precondition while returning success, so a design that *depends* on
   one cannot tell. Adding a shared key reopens a hazard this design is simply not exposed to.
4. **One writer, one namespace.** A device writes only under its own prefix, for the collection's
   lifetime. This is the invariant every other property rests on, including rule 3.
5. **`K` (the roll-up fan-in) is not a compatibility constant, but the key format is.** Readers merge
   by set union over sequence ranges and never assume a layout, so `K` may be tuned freely; the
   fixed-width zero-padded range in the key may not, because the listing *is* ADR-0004 §2's version
   summary and it works by lexicographic sort.
6. **Never present the sync folder as a backup.** It is hidden, it is deleted when the user removes
   the app's data, and backup is the separate `.lcoll` archive specified in
   [ADR-0016](./docs/adr/0016-backup-and-restore.md).

## Backup and restore

A **collection archive** is a `.lcoll` file — the same zip container as a deck file, carrying a
different profile — holding the log verbatim plus everything that settles, and written only when the
user asks. Chosen in [ADR-0016](./docs/adr/0016-backup-and-restore.md). Sync does **not** discharge
it: sync is opt-in, so a never-enrolled user has nothing, and sync propagates a deletion rather than
archiving against it.

### Rules that are easy to break silently

1. **Restore is a merge and never removes anything, and a replace is not implementable.** Every
   device holds the whole log and merge is set union, so a wipe-then-install is undone by the next
   sync — the peers still hold every row. It follows that backup protects against **loss, not against
   unwanted change**: an overwritten field carries a newer stamp and must win, or ADR-0004 §7's
   causality rule breaks. Say this to users; "restore" universally implies replacement.
2. **A writer id is never adopted; a collection id is never re-minted.** The two halves of identity
   take *opposite* rules, and swapping them is silent in both directions — adopt a writer id and two
   devices become one writer with reviews dropped on merge; re-mint a collection id and the check
   that tells an archive of yours from a stranger's stops working. The gate is one rule at both the
   restore and the enrolment seam: **an empty collection adopts, a non-empty one refuses.** "Empty"
   means no log rows under this device's own writer id and nothing on the mutable surface — *not* "no
   notes", or a fresh install can never join an existing account.
3. **The `collection` profile does not inherit ADR-0008 §12's byte-for-byte determinism.** That rule
   exists so an artifact sent to strangers does not leak build time; a personal archive needs the
   creation date it forbids, or a user cannot tell two backups apart. **Minimal disclosure still
   binds both profiles** — never auto-populate an author name, a device label, or any ambient
   identity.
4. **`derived.db` stays outside the backup set**, beside the writer marker. It is disposable by
   design, so backing it up protects nothing while spending the 25 MB platform quota and hastening
   the cutoff — which arrives after roughly **nine months** of heavy use, not the two years ADR-0007
   §6 states (it read that ADR's raw-interchange row where Auto Backup covers files on disk).
5. **No file picker, on either platform, and no text field for a filename or a passphrase.** Activity
   *results* need a Java subclass and therefore a dex, spending ADR-0003's Gradle-free APK; launch
   intents and dropped files need neither and are fine. And rule 8 of the client stack makes Android
   text input ASCII-only, so a passphrase set on the desktop in the user's own language cannot be
   typed on the phone — which is why **archives are never encrypted**.
6. **The platform seam rule is per crate.** `leitner-store::platform` keeps exactly two functions;
   a crate needing the platform for an unrelated reason gets its own module under the same three-arm
   discipline. `leitner-export` has one — put, get, list, **hand_off**
   ([ADR-0023 §1](./docs/adr/0023-sending-a-written-file.md)). The count is **not** the invariant:
   ADR-0016 §5's *"three operations, not four"* was an argument about **delete**, which is still
   absent. *Opaque, minimal, enumerable* is what binds.
7. **`hand_off` is named for what it does on both platforms, and the two arms differ on purpose.**
   Android launches the system share sheet; the desktop reveals the file **selected** in the file
   manager, because no `org.freedesktop.portal.Share` exists. Calling it `send` invites an
   implementer to reach for the mail portal on the desktop arm, which
   [ADR-0023 §4](./docs/adr/0023-sending-a-written-file.md) rejects — attaching to a message picks
   one channel on the user's behalf, which is what a share sheet exists not to do. **It never fires
   by itself**: a chooser or a file manager opening unasked takes the screen, and nothing in this
   specification acts unbidden.
8. **On Android the context is `android.app.Application`, not the Activity.** `ndk_context` hands
   back an `Application`, so `startActivity` **requires `FLAG_ACTIVITY_NEW_TASK`** or it throws. Both
   that flag and `FLAG_GRANT_READ_URI_PERMISSION` go on the **chooser** intent —
   `Intent.createChooser` returns a fresh `Intent` and inherits neither. Setting the grant only on
   the inner intent fails *after* the user has picked an application.

### What the user sees

[ADR-0015](./docs/adr/0015-the-sync-experience.md) settles the surface and
[ADR-0019](./docs/adr/0019-naming-the-account-at-enrolment.md) adds the account name to it. Five
rules fail silently, four of them because the reasoning is invisible from the code that would break
them.

1. **Exactly two things may speak about sync: a dead grant, and ADR-0004 §8's clock-skew warning.**
   A network failure never speaks — offline is normal and nagging about it is the defect. There is
   **no status icon, no badge, no success toast, and no "in sync"**: after a sync the app knows every
   writer's highest *published* sequence and never whether another device has reviewed since, so
   claiming devices agree is unknowable. The resting statement is the fact *"last caught up ⟨when⟩"*.
   Every future feature will have a reason to want a third speaker; each one is a defect.
2. **Never start a sync while the review screen is up; let one in flight finish.** This is not a lock
   on reviewing — the app never blocks that, ever — it is what stops a merge recomputing every
   `(S, D)` mid-session, which ADR-0014 called locally unfixable. **It works only because there is no
   background sync**, so that absence is load-bearing and not a limitation to lift.
3. **There is no delete-remote-data control, and adding one destroys other devices' rows.** The
   `drive.appdata` grant reaches the whole app folder, so a delete from one device removes namespaces
   whose rows a device that never fetched them will never see again. It looks like a courtesy and
   there is nothing to reclaim — a few hundred objects, ~47.5 MB per decade. Disconnect drops the
   local grant and deletes nothing.
4. **A wrong-account enrolment cannot be checked by any code, and the defence is two sentences the
   user reads.** A device pointed at the wrong account gets an empty folder, indistinguishable from
   being the first to enrol — so enrolment *ends by stating what it found*, **prefixed with the
   account it connected as**: *"Connected as you@example.com. This is the first device here"* versus
   the devices it met ([ADR-0019 §1](./docs/adr/0019-naming-the-account-at-enrolment.md)). Delete
   either half and you remove a guard that has no replacement. **No check can substitute**, and
   reaching for one is the natural mistake in two directions: ADR-0016 §10's identity check does not
   cover it, because **every collection id agrees** — all the devices hold the same collection and
   merely cannot see each other — and neither would a check on the *account*, because there is no
   peer, no namespace and no published byte to compare against. The failure is **reachability, not
   identity**, and the only comparand that exists is the user's memory of the last enrolment.
   The two halves do different jobs and neither is redundant: **the "first device here" sentence
   detects, the account address diagnoses.** Without the address the user infers "wrong account" from
   "first device here" — which almost nobody does, and every likelier hypothesis (folder cleared,
   other device reset, sync broken) **routes to a repair that cannot work**, so each failed attempt
   convinces them the collection is gone.
5. **The account address lives with the credential and never on ADR-0004 §7's mutable surface.** It
   is a property of the *grant*, not the collection, so it is never published and never enters either
   export profile — [ADR-0016 §4](./docs/adr/0016-backup-and-restore.md)'s `collection` rule excludes
   credentials, which is what keeps it out without a clause naming it. Put it on the mutable surface
   and it lands in every `.lcoll` **and** on the remote, buying nothing: a published address is
   unreadable by the device that needs it, which is looking at a different folder.

## Protection at rest

**Nothing is encrypted, anywhere**, decided once for all four artifacts in
[ADR-0020](./docs/adr/0020-protection-at-rest.md): `collection.db`, the drive credential, the `.lcoll`
archive, and the log published to the drive. Three earlier ADRs each acted on this default while
deferring the reasoning; the reasoning now lives in one place, and it is made of refusals — the kind of
decision that erodes fastest when met without it.

### Rules that are easy to break silently

1. **Never ask the user for a secret — no passphrase, no PIN, no unlock code — and note that the
   ASCII-input argument is not why.** [ADR-0016 §8](./docs/adr/0016-backup-and-restore.md) rejected a
   *passphrase* because client-stack rule 8 makes it untypeable on the handset, and that argument
   genuinely **does not reach a PIN**, since digits are ASCII. Reopening on that gap is the predictable
   mistake. Two independent reasons refuse anyway. **There is nowhere to recover a forgotten secret
   from** — no server, no account, no escrow, by construction — so a secret turns a design premised on
   never losing data into one where forgetting loses all of it. And **a PIN is safe on a phone only
   because the hardware refuses to be asked quickly**; data resting on a provider's disks cannot borrow
   that, so against unlimited offline guessing a six-digit space is a routine computation and the PIN
   protects nothing from the one adversary that motivated it.
2. **An application-held key is not the way around rule 1.** For the two local artifacts it buys no
   adversary the platform does not already answer — the desktop key is a file beside the data it
   protects. For the two that travel it is *circular*: with no server there is no channel to distribute
   the key except the one being protected, so the key ships beside the ciphertext and the design is
   plaintext with extra steps.
3. **A lock screen over the app is theatre, and it may never be described as protection.** The file sits
   in plaintext beside it, so anything opening the *file* walks past. If it ever ships as a
   user-interface convenience it may not be worded to imply the data is encrypted.
4. **The one route to delete published data is the provider's own settings, and its absence from this
   app is deliberate.** [ADR-0015 §10](./docs/adr/0015-the-sync-experience.md) forbids an in-app control
   because the grant reaches the whole folder, so a delete from one device destroys namespaces another
   device has never fetched. That prohibition is only tolerable because the user *does* have a route —
   the folder goes on uninstall, or is deleted manually from the provider's connected-applications
   settings. Anyone reading the missing control as an oversight should read that section before adding
   one. **Sync settings must keep naming that route and the name this app appears under**, since the
   folder is hidden and cannot be navigated to; the exact menu wording is a third party's UI and is
   verified at implementation, never pinned in our documents.
5. **Never claim the Android backup is unreadable by the provider — that guarantee is conditional and
   this project's own floor breaks it.** Auto Backup is always encrypted in transit and at rest, but
   under *operator-held* keys; the stronger layer, keyed to the lock screen, needs **Android 9+ (API
   28) and a lock screen actually set**. `min_sdk_version = 24`, so API 24–27 handsets — and anyone
   with no lock screen — have their collection *and* the sync refresh token held under keys the
   operator manages. Backup stays **on** regardless: refusing it would spend protection against loss,
   which is the thing this design actually fears, to buy confidentiality conceded everywhere else.
   Note two asymmetries with the sync folder — a device backup has **no per-app deletion path** and
   **survives uninstall**, and there is **no enrolment moment** at which to tell the user any of it.
   Evidence in [`docs/research/auto-backup-at-rest/`](./docs/research/auto-backup-at-rest/README.md).
6. **Enrolment states what leaves the device, and that clause is not a status message.**
   [ADR-0015 §5](./docs/adr/0015-the-sync-experience.md) holds the number of things that may speak about
   sync at two, and it means *ambient* speech — icons, badges, toasts. This clause rides the one-time
   consent moment alongside ADR-0015 §7's existing sentence, for the same reason that one exists: an
   exposure the user can never afterwards discover gets one sentence at the moment of choice. Do not
   promote it to a surface, and do not delete it as redundant.

## Agent skills

### Issue tracker

Issues live as GitHub issues on `amin-bf/leitner`, managed via the `gh` CLI. See `docs/agents/issue-tracker.md`.

**File issues directly — this repo overrides the global "don't create GitHub issues unprompted" rule.**
Work here is charted as wayfinder maps whose tickets *are* issues, so a rule against filing them
unprompted contradicts the workflow: it turns every graduated ticket into a round trip, and a sweep that
surfaces four decisions has to ask four times before the map can record them. Create the issue, then say
what you created and why. The global rule still holds outside a map — a bug you noticed in passing, a
"we should probably…", anything Amin would file himself — where the point is that filing it is a claim
on his backlog rather than a step in work already agreed.

### Triage labels

The five canonical triage roles, using the default label strings. See `docs/agents/triage-labels.md`.

### Domain docs

Multi-context — a root `CONTEXT-MAP.md` pointing at per-context `CONTEXT.md` files. See `docs/agents/domain.md`.
