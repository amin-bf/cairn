# cairn

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

### Branch names

**`Domain/PascalCaseDescription`** — one slash, no spaces, no kebab-case, and **no ticket number**.
It is the pull-request title with the spaces taken out: the PR convention is
`Domain / Title Case Description`, so `Client Stack / Read the IME Insets and Stop the Per-Tap
Keyboard Re-Pop` branches as `ClientStack/ReadImeInsetsAndStopKeyboardRePop`.

Derive it from the title you intend to give the PR, never from the issue you are resolving. A
ticket-numbered branch — `sandcastle/issue-84`, `worktree-wayfinder-75-…` — names **the tracker
rather than the work**, and the tracker is the one thing a PR already links. Nothing fails when a
branch is misnamed, which is why it needs saying: the cost is paid later and by someone else, when
the branch list has stopped being a map of the system and become a list of who happened to open what.

`ClientStack/…`, `NoteAuthoring/…`, `DeckExport/…`, `Sync/…`, `AgentStandards/…` — the same domains
the PR titles already use. Branch and PR then agree on which part of the system the work belongs to.

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
3. **A prototype is preserved as a tag and never merged.** Throwaway code built to answer a design
   question — variants to react to, capture harnesses, spikes — is tagged
   **`prototypes/issue-<N>`** and left contained in no branch. `git tag -l 'prototypes/*'` is the
   inventory; `prototypes/issue-8`, `-11`, `-20`, `-28`, `-67`, `-120` and `-124` are the existing
   ones.

   `main` keeps the validated decision, not the options that lost. Merging a prototype puts a
   throwaway binary and dozens of capture PNGs into the tree of every future checkout, permanently,
   to preserve something a tag already preserves.

   **Split the branch before opening the pull request.** Prototype work usually contains a little
   that genuinely belongs on `main` — a bug fixed in a tool the repo keeps, say. Cherry-pick that
   onto its own branch and PR only that; tag the rest. #124 is the worked example: two capture-harness
   fixes went to `main` as their own PR, and five Review variants with fifty captures became
   `prototypes/issue-124`.

   **A tag is not out of reach, and "the next session cannot see it" is not a reason to merge.**
   Tags are fetched by every clone, so a later worktree reads a prototype without merging anything:

   ```sh
   git show prototypes/issue-124:docs/design/prototype-124/README.md
   git checkout prototypes/issue-124 -- crates/desktop/src/bin/review-prototype.rs
   ```

   This is written down because the convention lived **only in `git tag -l`**, and an agent that did
   not think to look there argued its way into a pull request that merged a prototype into `main`.

4. **Anything captured from real hardware is redacted before the commit that introduces it.** This
   repository is **public** and a push is the point of no return, so *after* is not a repair.

   A capture off a physical device carries system chrome belonging to whoever owns the device rather
   than to the application — the status bar and its notification icons above all. An emulator carries
   none of it, which is exactly why this is easy to miss: every capture convention here was built
   against emulated and nested-compositor screens, and the exposure appears the first time a session
   photographs real hardware. The same holds for anything else lifted off a real machine — crash
   tombstones, bug reports, log dumps naming unrelated applications, and absolute paths carrying a
   username.

   **The obvious remedies do not work, so do not plan to rely on them.** A commit that has been
   pushed stays served by its SHA. Force-pushing a rewritten branch, rebasing the commit away,
   closing the pull request and deleting the branch all leave it reachable through the pull-request
   refs, which the host keeps independently of any branch. Only the host's support team can purge
   those. **Redaction before `git add` is the only control that works** — treat the first `git add`
   as the irreversible step, not the merge.

   **Paint over in place; do not crop.** A write-up that measures its own image is falsified by
   cropping and unaffected by painting. Read the band's height off the device instead of guessing,
   and say in the prose that the band is a redaction so a later reader does not diagnose it as a
   rendering defect:

   ```sh
   adb shell dumpsys window | grep -m1 'type=statusBars'   # frame=[0,0][1344,151] → 151
   magick shot.png -fill black -draw "rectangle 0,0 1344,151" shot.png
   ```

   **If it happens anyway, do not write down where.** Naming the commit, branch or pull request in a
   durable document — this file, an ADR, a design readme, a commit message — converts the record into
   a pointer to the very thing being protected, and publishes it to a far wider audience than the
   original slip ever reached. What is unreachable in practice is protected mostly by nobody knowing
   where to look. Fix it forward, tell the repository owner directly, and leave the location out of
   every artifact that persists.

## Start with the context map

**[`CONTEXT-MAP.md`](./CONTEXT-MAP.md) is the entry point to the codebase** — the six crates, the
eight contexts, and an index saying which ADR sections bind which context. `docs/adr/` is over 2,600
lines; the index is what stops "read the ADRs" from meaning all of them.

Read this file first, then the context map, then the `CONTEXT.md` for the area you are touching.
`docs/research/` is the evidence trail for reopening a decision, not reading for implementing one.

## The workspace

Six crates, laid out in [ADR-0009](./docs/adr/0009-crate-and-workspace-layout.md) and extended by
[ADR-0013 §11](./docs/adr/0013-the-sync-transport.md):
`cairn-core` (the domain, pure), `cairn-store` (SQLite and the two directory lookups), `cairn-export`
(the `.cdeck` container), `cairn-sync` (publishing to the remote; holds the network dependencies),
`cairn-app` (egui, lib + cdylib), `cairn-desktop` (a shim, forced by `cargo-apk`).

Contexts are **modules, not crates** — with two exceptions, both for the same reason: `export` and
`sync` hold dependencies `cairn-core` may not have. A context becomes a crate only when it must
carry one. Vocabulary lives in a `CONTEXT.md` beside the code; decisions live system-wide in
`docs/adr/`, and context-scoped `docs/adr/` directories are not used here.

### Rules that are easy to break silently

1. **`cairn-core` has exactly one dependency — `fsrs` — and adding a second is an ADR-sized
   decision.** What makes `cargo test -p cairn-core` need no database, no window and no handset is
   not the *count* but [ADR-0027 §2](./docs/adr/0027-the-scheduler-dependency.md)'s test: the spec
   must name the crate, it must be computation and nothing else, it must build for every target, it
   must be pinned exactly, and it needs its own ADR. `rusqlite` belongs in `cairn-store`; `egui`
   and `eframe` belong in `cairn-app`. **`rand`, `serde`, `rayon` and `ndarray` are in the lockfile
   transitively and are not thereby available** — finding one there is not a precedent for reaching
   for it, and `log` must still never be a *direct* dependency here (it would shadow the `log`
   context module).
2. **Time and identity are values, never injected traits.** Replay needs no clock at all — day
   numbers are frozen on the row at write time, and fuzz is seeded from card identity. The two call
   sites that need "now" take it as a parameter. A `SystemTime::now()` inside `cairn-core` breaks
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
7. **The review screen's *edit this note* appears only after the reveal, and restoring it before the
   reveal re-opens a hazard whose guard no longer exists.** [ADR-0029](./docs/adr/0029-editing-a-note-from-the-review-screen.md)
   narrowed ADR-0021 §6's *"at any point in the card's life"* and **retired** its *"entering the editor
   counts as a reveal"* rule in the same move — that rule had exactly one customer, the pre-reveal
   edit, and after the reveal it is a no-op. So ADR-0006 §4's *"self-grading can't happen before the
   answer is seen"* now holds because **there is no route into the editor before the reveal**, not
   because a clause about the editor's side-effect holds. Put the control back in the pre-reveal state
   and the guarantee is silently false again, with nothing failing and no rule left to catch it. The
   alternatives to the old rule both needed state this design does not have: skipping a card ungraded
   needs an in-session deferred set, which ADR-0006 §2 proved does not exist, and flagging it for later
   is the stored *"since you last looked"* ADR-0010 §9 refused. **What ADR-0029 §2 gives up** — *"is
   this prompt answerable?"* is unaskable once you know the answer — is accepted, not overlooked.

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
   **That job's sections are pushed by hand, and `LayoutJob::append` may never come back.** It merges
   into the previous section whenever the format matches — sensible for sections carrying only a
   colour, fatal where **the section boundaries are the reordering**. Merged, the paragraph becomes
   one shaping run, harfrust infers RTL for it, and it reverses the order the helper just put the
   words in. What hid that is rule 7's other half: runs are re-split by **face** below the merge, so
   wherever the spaces come from a different face than the words the order survives by accident. It
   did in `Proportional` and `Monospace`, and did not in `bold`, where one face owns both — Persian
   rendered backwards there and only there. Every test asserting on the job's *text* passes either
   way; only laying it out through real faces tells them apart.
   **Card content and chrome take two different builders, and mixing them fails silently.** A field
   is restricted Markdown (ADR-0002 §8): `**bold**` in the shipped face, `` `code` `` in `Monospace`,
   `*italic*` as a shear. So **card content — anything a note holds — goes through `bidi::markdown_job`,
   and app chrome (labels, badges, headings, the import preview) through `bidi::job`**, which renders
   every marker as itself. Route card content through `job` and `**bold**` shows literally (issue
   #104); route a stranger's string — an import preview — through `markdown_job` and you have handed a
   file the power to style the screen it is being previewed on, which ADR-0022 §7 forbids. Both build
   the same bidi-ordered sections; the only difference is whether the markers are interpreted.
2. **`TextEdit` needs the same treatment, via `.layouter()`** — it lays out its own text and
   otherwise bypasses the helper. Note that caret and selection are then in visual order while the
   buffer is logical, so RTL editing is imprecise; design around it rather than fighting it.
3. **The storage seam is a compile-time `#[cfg]`.** Keep it that way. Never introduce a runtime
   platform check — the whole stack choice rests on wrong platform code failing the build. It is two
   functions in `cairn-store::platform`, and a **third function appearing there means the seam is
   eroding**. A `#[cfg(target_os)]` anywhere else in the workspace is a defect. **The vendored
   dependency of rule 12 is outside this rule** — it is not a workspace member and it is not our code,
   so the `#[cfg]` you will find there is correct. Said explicitly because a correct instance of a
   construct used as a defect signal is how the signal quietly stops meaning anything.
4. **Immediate mode has nowhere to `await`.** Spawn the future, store a handle, read the result on a
   later frame, and call `ctx.request_repaint()` on completion or the result sits unseen until the
   next input event.
5. **The desktop binary lives in its own crate, and that crate stays empty.** `cargo-apk` panics
   after signing when one crate has both a cdylib and a bin — the APK is fine, the exit code is not,
   and CI breaks. So never add a `[[bin]]` to `cairn-app`, and never put logic in
   `cairn-desktop`: code there is never compiled for Android and never runs on the handset.
   It takes `eframe` by re-export from `cairn-app`, not as its own dependency.
6. **`eframe`'s dependency is split per target** — its default `accesskit` feature is rejected
   alongside `android-native-activity`.
7. **Fonts are ours to ship — and must be installed on the first frame, not in `CreationContext`.**
   egui bundles only Hack, Ubuntu-Light and Noto Emoji. Register any added face in **every** family
   you use, or text silently renders as boxes. **That is three families, not the two this rule used
   to name** — `Proportional`, `Monospace` and ADR-0012 §8's `bold`, enumerated once in
   `fonts::families()`, which install, the coverage test and the specimen all read so a fourth cannot
   be added to only one of them. Bold is the one to watch: it is built from scratch rather than
   appended to, so nothing of egui's sits behind it. **And within a family, order decides which face
   is reached** — first match wins, and more than one shipped face carries the Arabic script, so
   DejaVu listed ahead of Noto meant Noto was never reached and Persian was drawn by DejaVu's
   afterthought Arabic. A glyph existing is not the same as the right face drawing it, and the
   difference is invisible to anyone who does not read the script. Registering during creation
   breaks the web build: wgpu panics with "Tried to update a texture that has not been allocated
   yet", glow renders everything near-black. Defer it one frame.
   **A fourth face ships and it carries no script: `Cairn Icons`**
   ([ADR-0038 §1](./docs/adr/0038-the-mark-and-the-icon-rule.md)). An icon is a **glyph**, reached by
   falling through like any missing character, so no call site selects a family and an icon at `BODY`
   *is* `BODY` — and an icon's size is a **font** size, named in `typography` with the rest. Its code
   point is **private use**, which is what makes it safe to append last everywhere: it shadows
   nothing and nothing shadows it, so this rule's ordering hazard cannot arise for an icon. It goes
   into `bold` as the *regular* cut — the one stated exception to "bold holds the bold cuts and
   nothing else", because a mark has no weight. The face is **generated** from
   `crates/app/res/drawable/ic_launcher_monochrome.xml` by `scripts/build-icon-face.py`; run
   `--check` after touching the drawable, or the claim that the glyph is the launcher's four stones
   quietly stops being true. **Two things fail silently here.** The coverage test's filter was an
   *allowlist* of letters and digits while its own comment described a denylist, so a private-use
   code point was skipped and the test would have passed on a family the face never reached — it is
   a denylist now, and a new specimen row needs no edit there. And **an icon standing on its own must
   be allocated its ink, not its line box**: `ui.label` allocates the *family's* `row_height`, which
   at size 150 puts 109px of stones in a 172px row and adds 53px of unchosen space before the stated
   gap (ADR-0032 §2). Call `crate::icon` for a picture standing alone; `ui.label` stays correct for
   an icon **inline** with its word, which is the case the whole route exists for.
8. **Android text input is ASCII-only, and cannot be fixed here.** winit's Android backend handles
   only motion and key events — it has no IME path, so composed text never reaches the app. This is
   not the activity backend: GameActivity was tried and reverted (see
   [`prototypes/egui-slice/android/README.md`](https://github.com/amin-bf/cairn/blob/prototypes/issue-8/prototypes/egui-slice/android/README.md)). Never design a feature that requires typing non-Latin
   text on Android. **This rule is also rule 12's tripwire**: the patch there is justified by the
   absence of an IME path, so the day winit grows one, that patch becomes a bug and must be re-judged
   rather than re-applied. Whoever retires this rule owns that.
9. **Verify Android on the real handset.** The emulator is x86_64; the Pixel 8 Pro is arm64-v8a only.
   `cargo apk build` needs a **JDK on `PATH`** — `apksigner` is a `java` wrapper, and its absence
   surfaces only at the signing step, *after* a full NDK compile, as
   `apksigner: line 97: exec: java: not found`.
   **And an intent fired from `am` is not the intent an application sends.**
   `FLAG_GRANT_READ_URI_PERMISSION` reaches the intent's `data` URI and its `ClipData`, never a bare
   Parcelable extra; real senders escape that only because `Activity.startActivity` migrates
   `EXTRA_STREAM` into the clip on the way out, and **`am start --eu` does not**. So a shell-fired
   `ACTION_SEND` measures the harness rather than the application — and where the receiving code
   degrades to a refusal by design, it presents as the file simply never arriving, with nothing in
   `logcat` either. **Send a file the application itself wrote**, which needs no grant at all, to tell
   a broken reader from a harness that cannot hand one over
   ([ADR-0024 §5.7](./docs/adr/0024-identifying-a-written-file.md)).
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
11. **The soft keyboard is invisible to the app unless it asks, and the failure is unreachability,
    not occlusion.** Rule 8's gap has a second half: winit's Android backend reports no **insets**
    either, and the window is enforced edge-to-edge, so `adjustResize` is inert — nothing resizes and
    nothing is reported. egui then sizes its `ScrollArea` to a viewport taller than the visible one,
    the content fits, and there is **no scroll range**, so the covered band cannot be reached at all.
    Measured on the handset: **923dp of usable height down to 565dp, 39% of the screen, silently**.
    So the `ui` crate carries a one-function `platform` seam returning the IME and system-bar insets
    and reserves the band — a **third** per-crate seam under
    [ADR-0016 §5](./docs/adr/0016-backup-and-restore.md), not a widening of `cairn-store`'s two.
    **That seam's return type must distinguish "this platform has no soft keyboard" from "the keyboard
    is down"** — collapsing both to zero, as ADR-0025 §2 first wrote it, makes every gate on "the
    keyboard is down" permanently true off Android
    ([ADR-0026 §5](./docs/adr/0026-the-per-tap-keyboard-re-pop.md)).
    **Three guards come with all this and an implementation missing any of them is visibly broken**:
    keep the focused field inside the shrunken viewport **in the same frame it shrinks** — a `TextEdit`
    publishes `output.ime` only while its rect is visible, `egui-winit` turns that absence into
    `hide_soft_input`, which collapses the inset, which restores the viewport, which shows the field
    again — **surrender focus when a focused field is scrolled *completely* out of view**, which
    is the same loop entered from the other end — and **raise the keyboard from a discrete press on a
    text field** (rule 12), without which it never comes back after a manual dismiss.
    [ADR-0025 §1 §2 §3](./docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md),
    [ADR-0026 §4 §5](./docs/adr/0026-the-per-tap-keyboard-re-pop.md).
12. **One dependency is vendored and patched, and a bump is not just a version change.** As published,
    `egui`'s `TextEdit` calls `request_focus` on *every* pointer interaction with no `has_focus` check,
    `request_focus` interrupts IME composition unconditionally, and `egui-winit` implements that
    interruption as `set_ime_allowed(false)` then `(true)` — which winit's Android backend maps onto
    `hide_soft_input`/`show_soft_input`. So **every tap into a text field dismisses and reopens the
    keyboard**, measured at **6 hides / 17 shows for three taps on the already-focused field**, and the
    inset collapse it causes throws away the scroll position that rule 11 exists to make meaningful.
    It buys nothing here: rule 8's missing IME path means there is never a composition to interrupt.
    **There is no fix above the dependency** — the interrupt flag is private, its setter is public and
    its clearer is not, and nothing hooks the platform output before `egui-winit` reads it. So we carry
    a **verbatim copy of the published crate** with that one block behind `#[cfg(not(target_os =
    "android"))]`, wired by `[patch.crates-io]`, pinned exactly, verifiable by recursive diff against a
    pristine copy. **The patch is bound to the block's shape, not its line number: if a release
    restructures it, re-judge rather than re-apply** — a guard applied to a block that no longer means
    the same thing looks healthy in a diff. Routine bumps need the diff and the shape check; the handset
    measurement only when either is unhappy. **Both checks are one command — `scripts/verify-vendor.sh`**
    — and the handset one is `scripts/measure-ime-requests.sh`, so neither is a procedure to reconstruct.
    The reasoning, the rejected alternatives and the exit condition live in
    [`vendor/PATCH.md`](./vendor/PATCH.md); read it before bumping the egui family.
    [ADR-0026](./docs/adr/0026-the-per-tap-keyboard-re-pop.md).
13. **Colour is named in exactly one place — the `theme` module — and every screen reads the *ambient*
    visuals.** [ADR-0030 §1](./docs/adr/0030-the-first-finish-pass-decisions.md) puts the palette
    behind one function producing an `egui::Visuals`, installed once. A `Color32::from_rgb`, a
    `ui.visuals_mut()` tweak, or a hard-coded shade **anywhere else renders fine to the author and
    drifts the palette one screen at a time, with nothing failing** — which is exactly why it needs a
    rule. Ask for a role (`ui.visuals().text_color()`, `weak_text_color()`, `hyperlink_color`), never
    a value. The white in `fonts.rs`'s coverage probe is not an exception — it draws nothing a user
    sees. **The contrast floor is 7:1 for text against its surface** (§3): a text colour added to the
    palette that clears less is a defect the floor catches. Two carve-outs, both recorded in §3:
    **weak text** (the derived `weak_text_color()`, ~5.6:1) stays below the floor on purpose, because
    §4 wants the box badge quiet and a 7:1 weak text is a loud one — it is a pre-existing weakness, not
    a defect, held only against stock. And the **non-text** pairs are out of scope, so do not "fix" a
    decorative stroke to reach 7:1 — the one exception being the hover stroke, which #115 lifted back
    over **3:1** (not 7:1) because it was the lone pair the palette *regressed*.
14. **The app pins dark and does not follow the OS theme — and pinning is two acts, not one.**
    [ADR-0030 §2](./docs/adr/0030-the-first-finish-pass-decisions.md): install the dark palette as the
    visuals **and** disable theme-following. Set only the first and an OS theme change silently
    restores stock egui — the 5.12:1 body the palette exists to fix — with nothing failing and no test
    covering it, because the drafted palette is dark only. Dropping system-following is a deliberate,
    recorded behaviour change, not a default to reach past: a light palette is a separate finish pass,
    and restoring following is its job, not something to re-add without one.
15. **The page frame is named in exactly one place — the `frame` module — and a screen asks for a
    frame, never a number.** [ADR-0031](./docs/adr/0031-the-page-frame.md) is rule 13 one layer down:
    a literal `28.0` of horizontal padding or a hand-rolled `min(available, 640.0)` **renders fine to
    the author and drifts the layout one screen at a time, with nothing failing** — and #123 found the
    app already paying that for spacing at ~60 call sites. `frame::wide_column` has exactly one
    legitimate caller, the editor; a second is the frame eroding into a per-screen preference.
16. **Under a frame, `ui.available_width()` is the column and not the window — so an arrangement
    threshold must say which it means.** This is the trap #131 found and it is the shape to watch for
    rather than the one instance. The editor's side-by-side test read `available_width()` and was
    correct only because the app had no frame; the moment one existed it measured the column, a 640
    column is not `>= 640`, and **every desktop would have shown the phone's `Write | Cards` toggle**
    with nothing failing and no test covering it. Arrangement decisions read
    `ctx().viewport_rect()` — not `available_width()`, and not `content_rect()`, whose safe-area
    insets are vertical on every device that has them, so subtracting them makes a layout decision out
    of a notch. And `frame::cap_for` is read by **both** the nav row and the screen, because two call
    sites naming their own number is how the nav silently drifts out of alignment with the content
    (ADR-0031 §3).
17. **Type is named only in `typography`, spacing only in `spacing`, and a stated gap is the whole
    gap.** [ADR-0032](./docs/adr/0032-the-type-scale-and-the-rhythm.md) is rule 13 a third time, with
    one deliberate break in the pattern. Type follows it: four sizes, ambient roles, and a literal
    passed to a `FontId` outside that module is the defect — control text is an **alias** of body, so
    diverging them means breaking a test rather than adding a constant, and `display` has exactly one
    caller (the card face). **The rhythm breaks it, and the reason has to travel with the rule** or it
    reads as an oversight: `item_spacing` is **zeroed** and every gap says its own size, because an
    ambient gap cannot express a gap *smaller* than itself and would force every site to name a number
    that is not the gap it wants. So `ui.add_space(spacing::gap(n))`, never a float, and **`gap` takes
    an integer so a half-step will not compile** — a unit that permits halves is a four-unit wearing an
    eight label.
    **Three things fail silently here.** egui adds `item_spacing` *before* every stated gap — its docs
    say so — so any width arithmetic written against the old behaviour is wrong by 8 per gap; that is
    what left the two-column editor 8px outside its own frame, off-centre, with nothing failing.
    **Any widget pair that relied on the ambient 3px fuses** the moment it is zeroed, and a 3px
    separation nobody decided is not a design to preserve — state it. And **a `TextStyle::Name` that
    was never installed panics rather than falling back**, which is the loudness this crate wants: a
    defensive resolve would draw the 40px card face at stock's 13 on any path that skipped `install`.
    Both modules install through `all_styles_mut` — `Style` is per-theme exactly like `Visuals`
    (rule 14), and neither type nor spacing differs between themes, so writing every slot makes
    ADR-0030 §2's trap inapplicable instead of merely avoided.
    The page margin is **exempt** from the grid on purpose (ADR-0032 §3) — the frame is its own value
    family — and the numbers are **logical pixels**, which is `dp` and `pt`, so they carry to a native
    client unchanged.
18. **A card face must reset `halign` and set a wrap width, and no French fixture will tell you
    otherwise.** `bidi` sets `halign = RIGHT` as a **direction marker**, never the alignment, and says
    every caller resets it — an RTL galley left that way spans **negative x**, so a button centring it
    draws the text off its own face. `card_face` was the caller that never did, and at stock's 13px the
    ~118px overhang merely looked off-centre; the display tier took it to **−455px**, clean off the
    window. A `LayoutJob` also wraps at `f32::INFINITY`, so nothing had ever been asked to fit.
    **The reason this survived so long is the fixture**: the seed collection is French, so no capture in
    this repository had ever put right-to-left text on a card face. The Persian storyboard now visits
    the card pane for exactly that, and `surface`'s
    `a_right_to_left_face_is_drawn_inside_the_card` pins it. Any new surface drawing note content owes
    the same two lines and the same capture.
19. **Ask `surface` for a card, and never draw note content on a rect you filled yourself.**
    [ADR-0033](./docs/adr/0033-the-card.md) is rule 13 a fourth time, for the card: one object with two
    faces divided by a hairline, on a fill *darker* than the page, with the box badge inside it.
    `surface::card` is the only implementation and both callers — review and the editor's card pane —
    go through it, which is what finally makes ADR-0012 §1's *"drawn the way review draws it"* literal
    rather than approximate.
    **Three things it settles are easy to get wrong again.** The face **steps down** display → heading
    → body when the content does not fit and **stops at body**, growing instead of shrinking further —
    a paragraph card at 40px is the whole 560×860 window with *Edit note* below the fold, and a card
    face smaller than prose has stopped serving the reader it was sized for. The badge takes the corner
    reading does **not** begin at — top-right in Latin, **top-left in Persian** — governed by the
    *prompt* so it cannot change sides at the reveal; a corner that is quiet in one script is the first
    thing seen in the other, and ADR-0030 §4 calls it a *quiet aside*. And **the controls must stay
    quieter than the card** (ADR-0033 §3): blurred until nothing is legible, filled grade buttons
    outweigh every candidate card, so a Review screen whose controls beat its card has failed the ADR
    whatever #134 draws.
20. **`clear_color` is the page, and deleting it fails silently.** `eframe::App::ui` hands you a `Ui`
    with *"no margin or background color"* — its words — so without an override every screen is drawn
    on eframe's own `rgba(12, 12, 12, 180)`, which is `#080808` and **below every rung of the stone
    ramp**. That is not a darker theme: `theme::card_fill` measures **1.07:1** against it, so a card
    drawn as a well inverts into a raised surface, and the nav strip — a real panel, so it does read
    `panel_fill` — becomes *lighter* than the page under it. It went unnoticed through every capture
    this repository holds. This is rule 13's blind spot rather than a breach of it: the rule governs
    colours this crate **names**, and says nothing about ones the renderer supplies by default, where
    nothing in the source is wrong and the screen is still a colour nobody picked (ADR-0033 §2).
21. **A screen the seed cannot reach is photographed from a *fixture*, never from a prototype and
    never by extending the seed.** Every capture run is a first launch — the harness wipes the whole
    data directory — so the shipping seed's six due cards were once the only collection anything was
    ever photographed against, and #134 ended up shipping three decided states whose only pictures
    were of something that is not the application. `crates/app/src/fixtures.rs` holds pre-made
    collections instead, installed from outside by `cairn-fixture` on desktop and from a temporary
    Settings block on the handset, where `getFilesDir()` is unwritable from outside and an uninstall
    is not a first launch either (#141). **`CairnApp::open_store` stays untouched**, which is what
    keeps every capture taken before a fixture existed a picture of the same thing. A storyboard
    **names its own fixture** on a `fixture <name>` line, because a storyboard run without the
    collection it needs produces a full set of valid captures of the seed under the fixture's names.
    Adding a state means adding a fixture that **asserts what it reached** — the intervals come from
    `fsrs`, so a fixture landing where it says is a claim about a pinned dependency rather than about
    our code, and it has already been wrong once (store rule 6).
    **`cairn-fixture` wipes the platform data directory before it installs**, which is correct inside
    the harness (it owns a scratch profile) and destroys the operator's own collection outside it.
    Redirect `XDG_DATA_HOME` and `XDG_STATE_HOME` before running it by hand — the same two bases
    `capture-desktop.sh` redirects, and the only thing standing between a bench run and someone's
    review history.
    **And every control below the last card is now reached by `%BY-n%`, never a literal y**
    ([ADR-0038 §5](./docs/adr/0038-the-mark-and-the-icon-rule.md)). ADR-0035 §1 anchors a screen's
    final control to a line above the **bottom of the page**, so its y is a function of the window
    height — and §1 is a *page* rule now, not a Review one, so this reaches the leech entrance on the
    caught-up floor as well as the grade cluster. A literal y lands on empty page at every other
    window size and the run produces perfectly valid captures of the *previous* screen under the next
    one's names. That silent miss has now arrived from four different sides (#122, #143, #153, #155);
    `%CX%`, `%LX+n%` and `%BY-n%` exist to close each of them.
22. **A fixture may not depend on anything the card *identity* decides, because identity is random
    per build.** A `NoteId` is `uuid_v4` from the OS, so it is stable across **devices sharing a
    collection** — which is all any of the rules below it were ever for — and freshly random across
    **builds of the same fixture definition**. **Four** things read it on the way to a screen, and a
    fixture that leaves any of them to decide is photographing a coin flip rather than a state:
    - `replay::leeches` breaks a rank tie on `CardRef::encode`. #160 found all three leeches given
      identical histories, so both real keys tied, and every capture of the leech screen this
      repository held showed an order the run had picked at random — two captures from *one* run
      disagreed with each other. A fixture whose screen shows an order must **break the real keys**.
    - `session::compose` breaks the **due queue's** tie the same way, so it decides *which card a
      sitting opens on*. This is the one that is still live: `backlog`'s twenty-five cards share one
      history, so `checkpoint.txt` run twice photographs two different cards — measured, `le phare`
      and `la marée` from the same storyboard minutes apart.
    - `scheduling`'s interval fuzz is seeded from `CardRef::encode` (ADR-0027 §5, ADR-0001 §7), so a
      card scheduled close to due is due on some builds and not others — and, through `due_day`, it
      also perturbs the queue order above. The pre-#160 leech recovery left as little as **two days**
      of margin against a fuzz that swings about three; it had simply not lost the toss yet. Fixtures
      now leave **at least five days**, asserted directly, because `Fixture::check` only ever sees
      the build it ran on and fails *intermittently* otherwise.
    - `screens::review`'s suspended section sorts on identity too. No fixture suspends anything yet,
      so this one is a trap rather than a defect — the first fixture that does inherits it.

    All four are the same shape as store rule 6 and the `%CX%` family: nothing errors, nothing looks
    wrong, and the artifact is a valid picture of something nobody chose. **Measure, do not reason**
    — the intervals are `fsrs`'s, so a new fixture's margin is found by installing it a few hundred
    times, not by argument, and a fixture whose order matters is checked by *installing it twice*.

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
6. **Backdated rows must be appended oldest-first across the *whole* collection, not per card.**
   `append_review` guards on write and rewrites any instant at or below the highest already in the log
   to `highest + 1` (ADR-0004 §8, so a backwards clock cannot sort into the past) — and the day number
   is derived from the instant it actually stored, not the one you passed. The guard compares against
   the entire log, so building one card's history and then starting the next card silently stamps
   every later row a millisecond after the newest one already there. Nothing errors and nothing looks
   wrong: the leech fixture's four failure days at 80, 60, 40 and 20 days ago collapsed onto one
   recent day, leaving a card with **one** failure day and a collection that was not the state it
   claimed. `fixtures::History` gathers every review before writing any, for exactly this. Anything
   constructing history — a fixture, a test, an import — owes the same ordering.

## Deck export

A `.cdeck` file is a **zip archive** carrying deck content and never review progress, chosen in
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
    they can find the file at all. **Measured on the handset** — it **dedupes**
    ([evidence](./docs/research/android-outbound-share/README.md)). `MediaStore` also **discards the
    media type we declare**, deriving it from the extension instead.
12. **The Android write declares no `mime_type`, and that is what keeps the extension.** A declared
    type that disagrees with the name is the *only* reason a collision produces `French A1.cdeck (1)`
    instead of `French A1 (1).cdeck` — measured across three declarations
    ([evidence](./docs/research/android-file-identity/README.md)). `MediaStore` stores
    `application/octet-stream` either way, so the declaration buys nothing and costs the extension.
13. **A file is identified by its bytes, never by its name — but whether it *reaches* us is decided by
    the name.** The `mimetype` member is stored first and uncompressed so the type sits at a fixed
    offset, and on Android that is the *sole* authority over a file's **profile**: `.cdeck` and
    `.ccoll` both store as `application/octet-stream`, and a file arriving through a share may have no
    usable name. But identification is downstream of arrival, and **arrival is gated by the
    extension**: `MediaStore` derives the stored type from the extension, so a byte-identical deck
    named `.txt` types as `text/plain`, the broad filter never fires, and no sniff can recover it — it
    is unreachable, not merely misnamed (Pixel 8 Pro, API 37). A **stripped** name still types as
    `application/octet-stream` and still arrives. So the extension survives as a display string, as the
    `LIKE` clause the list enumerates with, and as the reachability gate — never as the thing that
    decides a file's profile ([ADR-0024 §1](./docs/adr/0024-identifying-a-written-file.md)).
14. **The Android file list shows only files this application wrote, and the interface must not imply
    otherwise.** Scoped storage gives us our own `MediaStore` rows and nothing else — a `.cdeck` another
    application put in `Downloads` is **invisible**, not merely unreadable, and `READ_MEDIA_*` does not
    cover documents. A deck someone sends can therefore only arrive through an **intent filter**, which
    is why the manifest declares the broad `application/octet-stream` filter for `ACTION_VIEW` and
    `ACTION_SEND` and accepts appearing in the Open-with sheet for unrecognised files
    ([ADR-0024 §2, §3](./docs/adr/0024-identifying-a-written-file.md)).
15. **Never match an intent filter on the extension.** No filename reaches one: `MediaStore` URIs and
    a real file manager's alike carry a **row id** in the path. A `pathPattern` is also ignored
    outright unless the filter declares a host, and `cargo-apk` drops the `\` escape — so verify the
    emitted `AndroidManifest.xml`, never the source. The filter matches on **type**, but the extension
    still gates arrival one step upstream: it is what `MediaStore` derives that type from (rule 13), so
    the extension decides reachability without the filter ever reading it.

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
   the app's data, and backup is the separate `.ccoll` archive specified in
   [ADR-0016](./docs/adr/0016-backup-and-restore.md).

## Backup and restore

A **collection archive** is a `.ccoll` file — the same zip container as a deck file, carrying a
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
6. **The platform seam rule is per crate.** `cairn-store::platform` keeps exactly two functions;
   a crate needing the platform for an unrelated reason gets its own module under the same three-arm
   discipline. `cairn-export` has one — put, get, list, **hand_off**
   ([ADR-0023 §1](./docs/adr/0023-sending-a-written-file.md)) — and `cairn-app` has the **third**,
   a single function returning the window's insets ([ADR-0025 §2](./docs/adr/0025-the-authoring-screen-under-a-soft-keyboard.md),
   client-stack rule 11). The count is **not** the invariant: ADR-0016 §5's *"three operations, not
   four"* was an argument about **delete**, which is still absent. *Opaque, minimal, enumerable* is
   what binds. **Three modules is not itself the erosion signal** — a *fourth function* inside any one
   of them is; a crate answering a question that is genuinely its own is the rule working, and reading
   the module count as the limit is what would push the next such question back into the store.
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
   and it lands in every `.ccoll` **and** on the remote, buying nothing: a published address is
   unreadable by the device that needs it, which is looking at a different folder.

## Protection at rest

**Nothing is encrypted, anywhere**, decided once for all four artifacts in
[ADR-0020](./docs/adr/0020-protection-at-rest.md): `collection.db`, the drive credential, the `.ccoll`
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

Issues live as GitHub issues on `amin-bf/cairn`, managed via the `gh` CLI. See `docs/agents/issue-tracker.md`.
Issue URLs everywhere — decided ADRs and recorded evidence included — name `amin-bf/cairn`, so
nothing depends on the host's rename redirect. **Old-name prose does remain in those files and is not
a defect**: [ADR-0028 §4](./docs/adr/0028-the-application-is-named-cairn.md) freezes the *claims* a
document makes, never the addresses it cites.

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
