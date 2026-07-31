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
the one it is protecting; a user-supplied key is refused separately by ADR-0020 §3.

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
`leitner-export`'s own `platform` module: **put, get, list**, three `#[cfg]` arms with a
`compile_error!` third. How an artifact reaches a place the user can see. Deliberately
[ADR-0013](../../../docs/adr/0013-the-sync-transport.md) §1's shape reused.

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
  dex, which spends the Gradle-free APK. Launch intents and dropped files are fine; results are not.
