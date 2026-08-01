# Export

Deck files: the `.ldeck` container, what goes in it, and the policy deciding what an imported file is
allowed to change. This is constraint 2 of the map made concrete — decks are portable and
publishable.

Depends on `content`. Will depend on `log` once [#37](https://github.com/amin-bf/leitner/issues/37)
specifies the progress profile — which is why this is a peer of `replay` rather than a module inside
`content`.

**Bound by** [ADR-0008](../../../docs/adr/0008-the-deck-export-format.md), whose glossary this file
supersedes; also by [ADR-0005](../../../docs/adr/0005-the-deck-model.md) (deck identity) and
[ADR-0002 §9](../../../docs/adr/0002-the-card-model.md) (the container must be able to carry binary
from day one, even though no media ships) and
[ADR-0011 §7](../../../docs/adr/0011-new-card-rate-and-daily-limits.md), which fixes `notes.jsonl`
line order as `(position, note id)` — ADR-0008 §12 demanded byte-for-byte determinism without ever
saying what order the lines took — and
[ADR-0020 §4](../../../docs/adr/0020-protection-at-rest.md): **no archive this crate writes is ever
encrypted**, on a reason worth knowing before it looks like an omission. A key protecting a file that
travels must reach every device that opens it, and with no server there is no channel to send it on but
the one it is protecting; a user-supplied key is refused separately by ADR-0020 §3. Finally by
[ADR-0022](../../../docs/adr/0022-the-import-preview-and-export-report.md), which specifies what these
artifacts *say*: an import is gated behind a declinable preview of its effects, and an export reports
where the file went, since with no picker the user chose neither its name nor its location.

## Language

**Deck file**:
A `.ldeck` zip archive carrying one or more decks' content and **no review progress**. The artifact
handed to another person.
_Avoid_: Export, bundle, package — all of which also name the act rather than the thing.

**Profile**:
Which payload a container carries: `deck` or `collection`. Declared in the manifest, and
distinguished to the operating system by extension. The **profile is the discriminator that selects
the stamp rule** — import restamps, restore preserves — which is why it is a profile and not a flag.

**Collection archive**:
A `.lcoll` zip archive carrying the whole collection: the log verbatim, plus everything that settles,
minus device identity and credentials. The artifact a user keeps for themselves.
_Avoid_: Backup file — the artifact is not a backup until the user moves it off the device.

**Collection profile**:
The `collection` payload. Its selection rule is *everything that settles, plus the log, minus device
identity and credentials* — **not** "all decks". It carries what the deck profile deliberately drops:
unfiled notes, stamps byte for byte, per-deck revisions, suspensions, the new-card rate, device
labels.

**Collection id**:
A UUIDv4 identifying a collection across devices and files. Minted once, **adopted and never
re-minted** — the exact opposite of a writer id's never-adopt rule. Of record in
[`store`](../../store/src/CONTEXT.md), where it is minted; named here because it is what the
container carries.

**User-files seam**:
`leitner-export`'s own `platform` module: **put, get, list, hand_off**, three `#[cfg]` arms with a
`compile_error!` third. How an artifact reaches a place the user can see, and then leaves it.
Deliberately [ADR-0013](../../../docs/adr/0013-the-sync-transport.md) §1's shape reused; the fourth
operation is [ADR-0023](../../../docs/adr/0023-sending-a-written-file.md)'s.

**Hand off**:
Giving the written file to whatever surface the platform provides for passing a file onward — the
system share sheet on Android, the file manager with the file **selected** on the desktop. It is not
*sending*: both arms stop at the hand-over and the application never learns what happened next.

**Revision**:
A per-deck monotonic integer declared by the file, advancing only when the deck's content digest
changes. Compared **only within one deck id's lineage** — it is meaningless across decks.

**Tombstone**:
A note id marked deleted, carrying no content, by which an author retracts a note from a published
deck. The only kind of deletion that travels; deck deletion never does.

**Acquired kind definition**:
A kind definition a collection holds because it arrived in an imported file, for a kind the running
build does not ship. Read-only, and never displaces a shipped definition.

**Update path / create path**:
The two import branches, selected by whether the file's deck id is already held. Authority follows
deck id, never a user's choice.

**Import plan**:
What an import *would* do to this collection, stated as effects — notes created, notes skipped as
already held, notes moving deck, tombstones that match a note we hold, a deck about to be renamed, a
deck about to be left empty, a kind about to be adopted. **Derived on every read, never stored**: it
is a projection of the log, and a cached one can be falsified by a merge landing underneath it.
_Avoid_: Import summary, import result — both name something produced after the fact, and there is
nothing after the fact.

**Preview**:
The import plan, shown, with the import declinable. The one gate in this specification, and it exists
because a regretted import is the one destructive act no archive and no peer can undo.
_Avoid_: Confirmation dialog — this repo refuses those twice, and for reasons that do not reach here.

**Gate / describe**:
The two stages of reading a file. The **gate** reads the central directory only and refuses a file
this build must not act on — unknown format integer, wrong profile, revision below the one held, a
path rule broken. The **describe** stage inflates `notes.jsonl` and diffs it against the collection
to build the plan. A refusal must never require inflating a payload.

**Authoring value**:
A value on the deck-id-keyed slot that belongs to *publishing* a deck rather than to using it:
`{revision, digest}`, author, description, licence. Never exported as deck content, never in the log,
and **must sync** — otherwise one author's two devices emit inconsistent files as routine behaviour.
The slot's other half is **personal** values, whose syncing is still open and which do not yet exist.

## Rules that are easy to break silently

- **A per-deck progress export does not exist, and cannot.** The log has no deck column by design,
  so scoping progress to a deck means filtering by *current* membership — the same log exported
  twice a week apart, with one note moved, yields different "progress for the same deck". Progress
  is coherent collection-wide and incoherent deck-wide.
- **On import, the file wins for everything it carries.** Settling stamps are per-collection
  counters and are meaningless compared across collections, so import is policy, not merge.
- **Stamps do not travel.** Import restamps only what actually changes.
- **The deck profile carries no log, so it carries no writer ids.** The disclosure question answers
  itself; do not add a policy to contain it, and do not add a log member to the deck profile.
- **One container, not two.** A backup artifact reuses this container with a different profile.
  Two containers means two parsers, two versioning stories and two sets of path-traversal rules.
- **Restore is a merge and never removes anything.** A replace cannot stick: every device holds the
  whole log, merge is set union, so the next sync re-merges whatever a wipe removed. It follows that
  backup protects against **loss, not against unwanted change** — an overwritten field carries a
  newer stamp and must win.
- **The `collection` profile does not inherit byte-for-byte determinism.** That rule exists for an
  artifact sent to strangers; a personal archive needs the creation date it forbids. **Minimal
  disclosure still binds both** — never auto-populate an author name, a device label or any ambient
  identity.
- **Never put a credential in an archive**, and never put `writer_id` or `seq_highwater` in one. A
  restored device must mint a fresh writer id, or two devices become one writer and the union drops
  reviews.
- **No file picker, on either platform.** Activity *results* need a Java subclass and therefore a
  dex, which spends the Gradle-free APK. Launch intents, dropped files and **outbound sends** are
  fine; results are not. The send was **measured** in the shipped APK shape — manifest plus one
  `.so`, no `classes.dex`, no `res/`
  ([evidence](../../../docs/research/android-outbound-share/README.md)) — so the picker's
  disqualifying property genuinely does not reach it.
- **`hand_off` never fires by itself, and the two arms are not a divergence to fix.** A sheet or a
  file manager opening unasked takes the screen. Android sends and the desktop reveals because there
  is no share portal on the desktop and nothing to drag on Android; making them symmetric means
  picking a mail client for the user.
- **The import plan is derived on every read, never cached.** A stored plan is a stored projection of
  the log — the thing ADR-0004 exists to prevent — and a sync landing while the preview is on screen
  can falsify it. Derived, promise and effect cannot diverge, which is why nothing is reported after
  an import commits.
- **The preview states effects, not file contents.** The manifest's counts are the wrong numbers in
  exactly the cases that matter: a file whose notes you almost all hold already, and a file whose
  retractions match nothing you have. The manifest is for **gating**; the payload is for describing.
- **A shipped kind winning over the file's definition is silent.** An unknown kind being *adopted* is
  stated. Announcing the override describes a non-event and advertises the option that rule exists to
  remove — reordering a kind's `cards` list is the most destructive edit in this codebase.
- **Every string from a file is hostile.** Author, description, licence and deck names render as
  plain text, never Markdown, length-bounded — the preview is the one screen showing a stranger's
  strings before the user has agreed to anything. Deck names are also sanitised **outbound**, since a
  filename is derived from one.
- **Read back what the platform wrote.** Never echo the requested filename: `MediaStore` may
  overwrite, dedupe or fail on a collision, and which it does is unverified on the handset.
- **The application never deletes a file it wrote or imported.** The seam has no delete; the list
  grows and tidying it is the file manager's job.
