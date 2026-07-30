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
from day one, even though no media ships).

## Language

**Deck file**:
A `.ldeck` zip archive carrying one or more decks' content and **no review progress**. The artifact
handed to another person.
_Avoid_: Export, bundle, package — all of which also name the act rather than the thing.

**Profile**:
Which payload a container carries: `deck` (specified) or `progress` (reserved for #37). Declared in
the manifest, and distinguished to the operating system by extension.

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
