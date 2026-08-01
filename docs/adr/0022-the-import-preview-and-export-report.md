# ADR-0022: The import preview and the export report

- **Status**: Accepted
- **Date**: 2026-08-01
- **Resolves**: [Decide: what an import preview states, and what export reports back](https://github.com/amin-bf/leitner/issues/68)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Related**: [ADR-0002: The card model](0002-the-card-model.md),
  [ADR-0005: The deck model](0005-the-deck-model.md),
  [ADR-0006: The review session experience](0006-the-review-session-experience.md),
  [ADR-0008: The deck export format](0008-the-deck-export-format.md),
  [ADR-0012: The note authoring experience](0012-the-note-authoring-experience.md),
  [ADR-0016: Backup and restore](0016-backup-and-restore.md),
  [ADR-0021: Note ordering, saving and the note list](0021-note-ordering-saving-and-the-note-list.md)
- **Amends**: [ADR-0005 §5](0005-the-deck-model.md), [ADR-0008 §12](0008-the-deck-export-format.md)
  — see [Amendments to accepted ADRs](#amendments-to-accepted-adrs)

## Context

[ADR-0008 §6](0008-the-deck-export-format.md) built a capability deliberately and then nothing used
it. The manifest is readable from the zip central directory **without inflating the payload**, and
that property is the stated reason the container is a zip at all — *"so the application can show
'3 decks, 1,240 notes, 12 retractions, requires kinds: vocab, cloze' before committing to an
import."* No ADR ever said what the preview shows. Its *Open items* row named
[#28](https://github.com/amin-bf/leitner/issues/28), which closed without reaching it.

The gap matters because ADR-0008 gives an import a great deal to be surprising about, and every one
of these is invisible from a file name:

- **Authority follows deck id** (§11), so a file may reorganise notes into a deck it already shares
  identity with. An import can *move* notes between decks the user already holds.
- **A shipped kind definition always wins** (§7), while an unknown kind is **adopted and rendered
  from the file's own definition** — two different outcomes with no visible difference.
- **Note tombstones travel; deck deletion never does** (§5). An import can delete notes.
- **On import, the file wins for everything it carries** ([ADR-0005 §9](0005-the-deck-model.md)),
  including over a user's own rename of the deck — a loss whose fix
  [the map](https://github.com/amin-bf/leitner/issues/1) has ruled out of scope.

Two constraints this inherits rather than reopens. [ADR-0016 §5](0016-backup-and-restore.md) settled
that there is **no file picker on either platform**, so the entry points are a well-known location,
desktop drag-and-drop and [ADR-0008 §10](0008-the-deck-export-format.md)'s Android launch intent. And
this specification has refused a new *speaker* four times ([ADR-0010](0010-leeches.md),
[ADR-0014](0014-when-parameter-optimisation-runs.md), [ADR-0015 §1](0015-the-sync-experience.md),
[ADR-0018](0018-the-card-pane-ordering.md)), so *detect and surface, never intervene* is the prior to
argue against rather than from.

**How the visual design pass bounds this ADR.** Everything below specifies what these surfaces
*state* and *when*. How they look is the visual design pass's, which
[the map](https://github.com/amin-bf/leitner/issues/1) ruled out of scope on 2026-07-31 — the same
split [ADR-0015](0015-the-sync-experience.md) drew for sync settings and the notice channel.

## Decision

### 1. Import is gated: shown, then confirmed, and declining costs nothing

> **An import is previewed before it is applied, and the user can decline.**

**Two rejections of confirmation prompts already stand in this repo, and neither reason reaches
here.** Both are worth restating, because an agent finding them will assume this section contradicts
them.

- [ADR-0012 §5](0012-the-note-authoring-experience.md) refused a modal at save because *"by the time
  you press Save the edit is already made, and a dialog asks for a decision about work you have
  stopped thinking about."* An import is the opposite: nothing has happened yet, and it is the thing
  the user is thinking about at that exact moment.
- [ADR-0016 §3](0016-backup-and-restore.md) refused a confirmation prompt for the collection-id rule
  because *"it asks the user to know something they cannot know — which is how the wrong stamp rule
  gets applied with a click."* Here the user is told what the file will do to their collection,
  which is precisely a thing they *can* be told.

**The argument that decides it is not that destructive operations deserve dialogs.** It is that this
is the only destructive operation in the specification with **no recovery path at all**:

> [ADR-0016 §4](0016-backup-and-restore.md) — backup *"protects against loss, not against unwanted
> change. A bad edit — right text typed over with wrong text — is a settled value carrying a newer
> stamp, and the archive's older stamp must lose… **The same is true of a regretted import.**"*

Everywhere else this design guarantees recovery: a deleted note's history reattaches by itself
([ADR-0002 §7](0002-the-card-model.md)), a deleted deck is fully recoverable by re-import
([ADR-0005](0005-the-deck-model.md)), a wiped device re-merges from its peers
([ADR-0016 §4](0016-backup-and-restore.md)). An import writes newer stamps over the user's own
values, so no archive and no peer can undo it. The gate is where that asymmetry is paid for.

**Declining costs nothing and leaves nothing behind.** No partial state, no record that a file was
looked at, no file removed — [ADR-0016 §13](0016-backup-and-restore.md)'s seam has no delete, and
that is unchanged here.

### 2. The manifest gates; the payload describes

Two stages with different jobs, and collapsing them is the mistake this section exists to prevent.

| Stage | Reads | Job |
|---|---|---|
| **Gate** | central directory only, no payload inflated | refuse a file this build must not act on (§4) |
| **Describe** | `notes.jsonl`, diffed against the collection | state what will happen to **your** collection (§3) |

**The preview states effects, not contents.** The distinction is not stylistic. Three of the facts
that matter most are absent from the manifest entirely:

- **How many notes are actually new.** [ADR-0005 §2](0005-the-deck-model.md) skips colliding ids on
  the create path, so *"1,240 notes"* reads as an import of 1,240 when 1,200 are already held and 40
  arrive.
- **How many notes move deck.** §11's update path relocates notes between decks the user already
  holds, and the manifest carries counts, not membership.
- **How many tombstones bite.** *"12 retractions"* may delete twelve of the user's notes or none.
  Only one of those is worth stopping for, and the manifest cannot tell them apart.

**Inflating `notes.jsonl` to answer them is not a cost worth avoiding.** A deck's note list is a few
hundred kilobytes of JSON Lines; the property ADR-0008 §6 bought was priced against a payload with
megabytes of audio in `media/` and against a `.lcoll` holding a decade of log rows. §11 below is
where that property actually pays.

**Rejected: a manifest-only preview.** It is the cheaper build and it produces a number that is
misleading *exactly* in the cases the preview exists for — a file whose notes you almost all hold
already, and a file whose retractions do not match anything you have.

### 3. What the preview states

Grouped **per deck**, because [ADR-0008 §11](0008-the-deck-export-format.md) selects the update or
create path per deck id and §8's multi-deck files exist precisely to carry an upstream split, which
mixes both.

```
French A1
by Marjan Rahimi · CC BY-SA 4.0
A1 vocabulary for the first ten chapters.
──────────────────────────────────────────
French A1 — updating a deck you already have
  38 new notes, 1,202 already yours
  12 notes moving in from German
  3 of your notes will be deleted
  Renaming your "My French" to "French A1"
  German will be left empty

German — new deck
  204 notes

Adds a card type this build doesn't have: cloze

Your review history is untouched.

                                    [ Import ]  [ Cancel ]
```

Every line discharges a rule that already exists:

| Line | Rule |
|---|---|
| update / new deck | [ADR-0008 §11](0008-the-deck-export-format.md) — authority follows deck id |
| new notes, already yours | [ADR-0005 §2](0005-the-deck-model.md) — a colliding id is the same note, not re-imported, *"the import reports it"* |
| notes moving in from *X* | [ADR-0008 §11](0008-the-deck-export-format.md) update path + [ADR-0005 §9](0005-the-deck-model.md) — membership follows the file |
| *N* of your notes will be deleted | [ADR-0008 §5](0008-the-deck-export-format.md) — *"import reports the tombstone count it applied"* |
| renaming your *X* to *Y* | [ADR-0005 §9](0005-the-deck-model.md) — the deck name is overwritten by the update |
| *X* will be left empty | [ADR-0005 §9](0005-the-deck-model.md) — an emptied deck is *"left alone and surfaced, never auto-deleted"* |
| adds a card type… | [ADR-0008 §7](0008-the-deck-export-format.md) — an unknown kind is adopted from the file's own definition |
| your review history is untouched | [ADR-0005 §9](0005-the-deck-model.md) — progress keys on `CardRef`, which no deck reshuffling can reach |

**Three rules govern the set.**

**A line that does not apply is absent, never shown as zero.** This follows
[ADR-0016 §6](0016-backup-and-restore.md)'s nudge form — facts, no verb, no threshold, no colour —
and a screen of zeroes buries the one line that is not zero.

**The rename line is in, and the map's own scoping is why.** [ADR-0005 §9](0005-the-deck-model.md)
concedes that *"a user's own rename will feel lost"* and shapes a personal display-name override as
the fix; the map ruled that fix out of scope on 2026-07-31, on the ground that nothing was left to
decide. That makes the preview the **only** surface where this can ever be surfaced. Out of scope for
*fixing* is what puts it in scope for *stating*.

**The kind line runs one way only.** An unknown kind being adopted is stated; a shipped definition
winning over the file's is **silent**. §7 calls reordering a kind's `cards` list *"the single most
destructive edit available in this codebase"* and makes shipped-wins the rule that forecloses it.
Announcing *"we are ignoring this file's definition of `vocab`"* describes a non-event and invites
the user to want the option that rule exists to remove.

**The last line is always present**, even when it is the only line. This is
[ADR-0016 §12](0016-backup-and-restore.md)'s move applied to the other operation — there, *"the last
line is the one the interface must say"*, because "restore" implies replacement and here it does not.
"Import" implies risk to a schedule, and [ADR-0005 §9](0005-the-deck-model.md) makes that
structurally impossible. A preview listing only damage reads as more damaging than it is.

**Rejected: the revision number.** *"Revision 4 → 7"* is authoring machinery — §9 keeps it on the
mutable surface and never exports it — and it answers a question about the counter rather than about
the collection. §4 below states the one revision fact a user needs.

**Rejected: per-deck selection within a file.** Letting a user import two of three decks makes the
file's own composition negotiable, which is exactly what [ADR-0005 §9](0005-the-deck-model.md) means
by *"a deck's composition is the author's statement about the material"*. The choice available is the
file, whole or not at all.

### 4. Refusals and the two degenerate cases share this surface

The preview is where a file is first understood, so it is where a file is refused.
[ADR-0016 §10](0016-backup-and-restore.md) set the precedent in the same words this section needs: a
mismatch *"is reported **here**, by name, rather than after the fact."*

| Case | Rule | Behaviour |
|---|---|---|
| Unknown `format` integer | [ADR-0008 §7](0008-the-deck-export-format.md) — the one hard gate | Refuse in place of the preview, with a plain message. Manifest-only, so it is instant. |
| Revision strictly lower than held | [ADR-0008 §4](0008-the-deck-export-format.md) — refuse to go backwards | Refuse, naming it as **older**, never as damaged: *"This file is an older copy of French A1 than the one you have."* |
| Absolute path, `..` segment, symlink, unrecognised member | [ADR-0008 §6](0008-the-deck-export-format.md) — reject outright | Refuse. One message, and **no detail that reads as an invitation to repair the file** — this is the classic defect of this container and the message is not a diagnostic channel for whoever built the file. |
| Wrong profile (a `collection` payload offered to deck import) | [ADR-0008 §1](0008-the-deck-export-format.md) — one container, two profiles | Refuse, naming the profile. Renaming `.lcoll` to `.ldeck` is a thing a curious user does, and [ADR-0008 §10](0008-the-deck-export-format.md) makes the extension a hint rather than a guarantee. |
| **Equal revision, different digest** | [ADR-0008 §4](0008-the-deck-export-format.md) | **Not a refusal — a preview line.** |
| **Equal revision, same digest** | [ADR-0008 §3](0008-the-deck-export-format.md) | **Preview still appears, stating that nothing will change.** |

**Equal revision, different digest is the one revision fact the user sees.** ADR-0008 §4 accepts
equal revisions deliberately — §3's idempotent re-import depends on it — and records that an author
exporting from two offline devices *"can emit two different files both claiming revision 4"*, with
the digest making this *"**reportable** rather than silent."* Nothing else in the specification is a
place to report it. The preview discharges that requirement rather than inventing one.

**The no-op still shows a preview, and this is the common case rather than an edge.** ADR-0008 §3
makes re-importing an unchanged file *"a genuine no-op: silent, idempotent, and producing nothing to
sync"* — where *silent* means it writes nothing and syncs nothing, **not** that the interface says
nothing. Re-opening a file someone sent you is the second-most-likely import there is, and a
double-click that produces no response is indistinguishable from a broken application. The preview
appears, states that nothing will change, and the gate costs one dismissal.

### 5. The plan is derived, never stored — so nothing is owed afterwards

> **The preview *is* the plan, recomputed against the collection as it stands. There is no
> post-import report.**

[ADR-0008 §5 and §11](0008-the-deck-export-format.md) required an import to report its tombstones and
its skipped notes *"before or immediately after committing"*. §1 answered **before**, which raises
whether *after* is still owed. It is not, and the reason is the property that makes the question
disappear.

**A stored plan can go stale.** [ADR-0015 §2](0015-the-sync-experience.md)'s foreground triggers mean
a sync can land while the preview is on screen, and a merge can turn a note the plan called *new*
into one the collection already holds. A plan computed thirty seconds ago and applied verbatim would
then do something other than what it promised — and detecting that divergence, then reporting it,
is a whole mechanism built to mitigate a decision.

**This repo has made the same move twice, and both times it removed the class of problem rather than
mitigating it.**

- [ADR-0006 §2](0006-the-review-session-experience.md): *"There is no session-progress entity
  anywhere… recomputed on every read"* — proven by force-stopping the application on the handset.
- [ADR-0012 §5](0012-the-note-authoring-experience.md): dormancy is *"recomputed from the draft every
  frame, so the warning is a property of the content rather than a check at save time."*

A plan computed once and held is a **stored projection of the log**, which is the precise thing
[ADR-0004](0004-the-review-event-log.md)'s design exists to avoid. Derived, a merge landing underneath
the preview changes the numbers on screen before the user presses Import, and promise and effect
cannot diverge.

**So nothing is owed after the fact.** No confirmation screen, no summary — the numbers were stated
at the moment the user could still say no, which is strictly stronger than the *after* ADR-0008 §11
would have allowed.

**The application returns to the note list**, which is where the imported notes now are.
[ADR-0021 §1](0021-note-ordering-saving-and-the-note-list.md) fixes three top-level destinations —
Review, Notes, Settings — so there is no deck list to return to, and §2 there gives the note list a
**deck filter** that is exactly the right instrument: **set to the imported deck when the file
carried one, left unfiltered when it carried several.** No new mechanism, and the user lands looking
at what arrived rather than at a screen asserting that it did.

**Accepted cost.** The import is computed twice in the common path — once to preview, once to apply
— and on a 5,000-note deck that is two passes over a few hundred kilobytes. Cheaper than the class of
bug a cached plan creates.

### 6. One screen, three entry points, and no mid-review gate

[ADR-0016 §5](0016-backup-and-restore.md) gives import three ways in and no picker: the **list** of
recognised files (§11), **desktop drag-and-drop**, and the **Android launch intent** from a file
manager or a mail attachment. All three produce the same screen.

**One screen, not three presentations.** The launch intent can **cold-start the application directly
into an import**, so one of the three has to work from a cold start regardless; building the
cold-start-capable one and reaching it three ways is the whole of it.

**Not an overlay, and not the notice channel.**
[ADR-0015 §5](0015-the-sync-experience.md) permits exactly two speakers on the persistent non-modal
channel — a dead grant and [ADR-0004 §8](0004-the-review-event-log.md)'s clock-skew warning — and
this is neither. It is also not a *notice*: nobody is being told something in passing, they are being
asked.

**A file arriving mid-review needs no rule, and this is a dividend rather than an omission.**
[ADR-0015 §6](0015-the-sync-experience.md) had to forbid sync during review because a merge has *"no
local trigger to gate on"*. An import has the opposite property — it is always a deliberate act — so
the preview simply takes the screen and the apply happens off the review screen. Returning re-derives
from the log by [ADR-0006 §2](0006-the-review-session-experience.md), which is exactly the safety
ADR-0015 §6 had to legislate for.

**Cancelling from a cold start lands on the count picker** — the same place ADR-0006 §2's force-stop
test lands, for the same reason.

**[ADR-0021 §6](0021-note-ordering-saving-and-the-note-list.md) reached the same conclusion from the
other side and is worth citing**, because together they fix how ADR-0015 §6 is read. It found a note
editable mid-review without breaching that rule, since what ADR-0015 §6 bans is *"an unannounced
recompute caused by another device"* — read more broadly it would delete mid-review editing too. An
import is the same shape: announced, local, and user-initiated. Two independent decisions landing on
that reading is what makes it the reading.

**Accepted cost:** confirming an import begun mid-session loses the chosen count and the ten-minute
timer, both of which ADR-0006 §2 keeps as in-memory state. [ADR-0006 §1](0006-the-review-session-experience.md)
already treats the timer as *"a courtesy check-in, not an enforcement mechanism"*, so preserving them
is not worth state.

### 7. The file's own claims are a header, rendered as plain text

**Author, description and licence are displayed**, above the effect lines and visually separated from
them.

Without this they are **write-only**: [ADR-0008 §12](0008-the-deck-export-format.md) defines all
three and no ADR anywhere displays them, so an author types a licence that no recipient ever sees.
The preview is the moment at which the *file* is the subject, which makes it the surface.

**Kept out of the effect list deliberately.** §2 rests on the distinction between what the file
contains and what it will do; author and description are claims the file makes about itself, and
merging them into a list of consequences blurs the one line the whole screen is drawn along.

**Absent fields are absent.** §12 makes empty the default and a legal state, so a blank `Author: —`
row manufactures the impression that something is missing.

**All three are attacker-controlled text and are rendered as plain text, never as Markdown**, and
length-bounded for display. This is stated because *"agents implement this"*:
[ADR-0002 §8](0002-the-card-model.md) already excludes link and image syntax from the note subset, so
the injection surface is small — but this is the one screen that renders a stranger's strings
**before the user has agreed to anything**, and ADR-0008 §6 already treats this container as hostile
input for path traversal. The same posture, applied to the text. **Deck names are covered by the same
rule**, being authored content that arrives in the same file.

### 8. File metadata is an authoring value: remembered, synced, never exported

> **Author, description and licence are held per deck id on
> [ADR-0005 §5](0005-the-deck-model.md)'s mutable-surface slot, alongside
> [ADR-0008 §9](0008-the-deck-export-format.md)'s `{revision, digest}`.**

ADR-0008 §12 never said whether they persist between exports of the same deck, and the export screen
cannot be specified without knowing.

**The argument is [ADR-0008 §9](0008-the-deck-export-format.md)'s own, unchanged.** That section
syncs the revision with *"no judgement call in this one: if it did not, an author exporting from a
laptop and from a phone would emit conflicting revision-4 files as **routine behaviour** rather than
as §4's rare offline edge."* Metadata has that defect exactly: an author publishing updates from two
devices credits themselves on one file and is anonymous on the other. Re-typing a licence string on
every update is worse on Android, where `AGENTS.md` rule 8 makes text input ASCII-only.

**This does not reopen §12's minimal-disclosure rule.** §12 forbids auto-population *"from an
operating-system user name, a device label, or any other **ambient identity**"* — a name the system
knows about the user without being told. A value the user typed into this field for this deck is
their own prior deliberate act, which is the distinction §12 is already drawing. **Empty remains the
default and a legal, silent state**, and nothing is ever pre-filled for a deck never exported.

**Knock-on, recorded rather than decided**: a Persian-speaking author cannot type a Persian
description on Android at all, `AGENTS.md` rule 8 being unfixable here. This is the map's existing
*"desktop is the sole authoring surface for non-Latin content"* landing in one more place.

### 9. The export screen

Export is **not gated**, and the asymmetry with §1 is principled rather than an oversight: an import
writes irreversibly into the user's collection, and an export writes a file. There is nothing to
decline.

**It lives in Settings, beside [ADR-0016 §6](0016-backup-and-restore.md)'s archive action, as does
§11's file list.** [ADR-0021 §1](0021-note-ordering-saving-and-the-note-list.md) requires every
specified screen to be reachable from one of three destinations, and *"a destination reachable only
by completing a session is not reachable"* — so this has to be said rather than left to the visual
pass. Settings rather than Notes because both are **file** operations over whole decks, sharing a
seam and a container with the archive, where Notes is where individual notes are authored. **Import
needs no home**: ADR-0016 §5's three entry points reach it without a destination, which is why the
list is the only inbound surface needing a place to live.

The screen carries the deck selection ([ADR-0008 §8](0008-the-deck-export-format.md) — one or more
decks), §8's three metadata fields pre-filled from the selected deck, and one required statement:

> **The count of unfiled notes**, whenever it is above zero.
> [ADR-0008 §8](0008-the-deck-export-format.md) requires it so they *"are not silently missed by a
> user who believes they exported everything."*

**It is a collection-wide fact, not a property of the selection.** An unfiled note
([ADR-0005 §8](0005-the-deck-model.md)) is in no deck at all, so no selection can ever reach it —
which means the number does not change as decks are ticked, and phrasing it as though it did would
invite a user to hunt for the selection that includes them.

**Metadata is per deck, and a multi-deck export takes the first selected deck's values**, since the
manifest carries one set of file-level metadata (§12) and the values are stored per deck id (§8).
Recorded because the alternative — merging three authors' fields — has no defensible rule.

### 10. What the application says after an export

The report has one job the import side does not have, and it is the whole content rather than a
courtesy: [ADR-0016 §5](0016-backup-and-restore.md) removed the picker and forbids typed text —
*"No filename field, no path field"* — so **the user chose neither the name nor the location, and the
application is the only thing that knows either.**

Desktop:

```
French A1.ldeck — 1,240 notes, 12 retractions
~/Documents/French A1.ldeck
```

Android:

```
French A1.ldeck — 1,240 notes, 12 retractions
Saved to Downloads.
```

**The filename is derived from the selection.** One deck takes the deck's name; more than one takes
the first selected deck's name and a count — `French A1 and 2 more.ldeck`. It is **sanitised by the
discipline ADR-0008 §6 applies inward**: no path separators, no control characters, no `..`. A deck
name is authored content that arrived from a stranger, so the outbound path is exactly as hostile as
the inbound one.

**No revision in the filename.** [ADR-0008 §9](0008-the-deck-export-format.md) makes an unchanged
re-export byte-identical at the same revision, so a revision in the name would create a second file
where the correct outcome is the same file.

**The report states the name the platform actually wrote, never the name requested.** ADR-0016 §5's
Android put is a `MediaStore` insert, and §5 itself records that the Android side *"is not verified
on the handset"* (`AGENTS.md` rule 9). Whether a colliding display name overwrites, dedupes to
`French A1 (1).ldeck`, or fails differs by platform and API level. Reading back what was written is
correct under **every** outcome, which is why it is specified in place of the collision behaviour.

**The desktop/Android split is deliberate and passes
[ADR-0014 §8](0014-when-parameter-optimisation-runs.md)'s test.** That ADR refused "softer
divergence" because the premise was refuted by measurement; here the premise is true — a desktop path
is a thing the user can act on, and an Android path is not. It is the same asymmetry ADR-0016 §5 used
to admit desktop drag-and-drop as additive.

**No exhortation and no next step.** [ADR-0016 §6](0016-backup-and-restore.md)'s *"not a backup until
the user moves it off the device"* fact does **not** transfer: it exists to correct a false belief
about safety, and a deck file exists to be sent to someone. There is no false belief here to correct.
Whether the application helps send it is [#70](https://github.com/amin-bf/leitner/issues/70)'s.

### 11. The list reads manifests

[ADR-0016 §5](0016-backup-and-restore.md)'s seam is put / get / **list**, and on Android the list is
the primary way in — with no picker, *"query `MediaStore` for our extensions"* is how a user reaches
a file they did not just receive. **Each listed file is described from its own manifest.**

```
French A1.ldeck             deck · 1,240 notes · 12 retractions
French A1 and 2 more.ldeck  deck · 3 decks, 1,802 notes
backup.lcoll                collection · 3 March 2026 · 812 notes, 4,200 reviews
```

**This is where [ADR-0008 §6](0008-the-deck-export-format.md)'s central-directory property earns what
it cost.** §2 above conceded that inflating one deck's `notes.jsonl` is milliseconds, so
read-without-inflating buys little in a single-file preview. Scanning a folder is the other case: *N*
files, *N* manifests, **zero payloads inflated** — including `.lcoll` archives whose payload is a
decade of log rows.

**It also gives [ADR-0016 §11](0016-backup-and-restore.md)'s creation date a place to be seen.** That
section put the date in the `collection` manifest because *"a user with three archives in a downloads
folder needs to tell them apart before restoring the wrong one"* — but **no ADR specifies the `.lcoll`
filename**, so three archives may differ only by whatever suffix the platform appended on collision.
A list of bare filenames leaves §11's stated need unmet at the moment it matters; a list of manifests
meets it *before* a file is opened.

**A file we cannot read is listed, not hidden**, marked `unreadable`, with §4's refusal shown if it
is opened. Hiding it means a user who deliberately put a file there sees an empty list and concludes
the application cannot see the folder — sending them after a permissions problem that does not exist.

**The list grows and is never tidied.** [ADR-0016 §13](0016-backup-and-restore.md) has no delete in
the seam — *"removing files from a user-visible folder is the file manager's job"* — and that is
unchanged. The application never removes an imported file.

### 12. Why restore's preview stays one line

[ADR-0016 §11](0016-backup-and-restore.md)'s restore preview is a single line describing the *file*:

> Collection archive, 3 March 2026. 812 notes, 4,200 reviews.

Read beside §2 and §3 this looks like the weaker form of the same screen, and an agent will be
tempted to bring it into line. **It is correct as written, and the reason is the rule that generates
both.**

> **A preview states effects in proportion to what can be lost.**

[ADR-0016 §4](0016-backup-and-restore.md) makes restore *"a merge, never a replace"* that *"only ever
adds"* — it cannot delete a note, cannot rename a deck, cannot move anything. There are no
destructive effects to enumerate, so a description of the file is the whole of what is useful, and
§10's collection-id gate is the one refusal it needs. An import can do all three, which is what buys
§3's line set.

ADR-0016 §11 is **confirmed, not amended.**

## Amendments to accepted ADRs

### [ADR-0005 §5](0005-the-deck-model.md) — the authoring half of the slot gains three more values

[ADR-0008](0008-the-deck-export-format.md)'s own amendment split the deck-id-keyed slot into
**personal** values (whose syncing remains open) and **authoring** values, which must sync, and put
`{revision, digest}` in the second half.

**Amendment**: **author, description and licence** join the authoring half (§8 above). Like the
revision, they are never exported as deck content, never appear in the review log, and **must** sync
between the user's own devices.

**Why**: the same defect §9 named for revisions — an author exporting from two devices otherwise
emits inconsistent files as routine behaviour.

### [ADR-0008 §12](0008-the-deck-export-format.md) — metadata persists between exports

§12 makes author, description and licence *"optional, default to empty, and never auto-populated"*
and is silent on whether they survive an export.

**Amendment**: they persist per deck id (§8 above), so the export screen pre-fills them for a deck
exported before and leaves them empty for one exported for the first time.

**Why this is not a weakening of §12**: the prohibition is on **ambient** identity — a name the
system knows without being told. A value the user typed for this deck is their own prior deliberate
act. Nothing is ever populated from an operating-system user name or a device label.

## Requirements this places on downstream tickets

### [#70 — sending an exported deck file](https://github.com/amin-bf/leitner/issues/70)

1. §10's report is written on the assumption that the application does **not** help send the file. If
   an outbound share affordance is adopted, §10's location line is what it replaces.
2. The reasoning that removed the file picker does **not** reach a share intent.
   [ADR-0016 §5](0016-backup-and-restore.md) refused `ACTION_CREATE_DOCUMENT`/`ACTION_OPEN_DOCUMENT`
   because they deliver through an activity **result**, needing a dex
   [ADR-0003](0003-client-stack.md) does not ship — and then admitted the launch intent because *"a
   launch intent is readable from the activity with no result callback and no dex"*. A send is in the
   second category. That is an argument for looking, not a verification; `AGENTS.md` rule 9 applies.

### Implementation

1. **The import plan is derived on every read** (§5). A cached plan is a stored projection of the
   log, which [ADR-0004](0004-the-review-event-log.md) exists to prevent.
2. **The gate reads only the central directory** (§2, §4). A refusal must not require inflating a
   payload, or a malformed file is parsed before it is rejected.
3. **Strings from a file are plain text, length-bounded, never Markdown** (§7) — author, description,
   licence and deck names alike.
4. **Filenames are sanitised outbound** (§10), by ADR-0008 §6's inbound rules.
5. **Read back what the platform wrote** (§10). Never echo the requested name.

## Glossary

Terms from this ADR are of record in [`export`](../../crates/export/src/CONTEXT.md), per
[ADR-0009 §6](0009-crate-and-workspace-layout.md), which fixed where contexts live. This ADR keeps
the reasoning behind them.

## Consequences

- An import can be refused after being understood, and understanding it costs the user one screen
  they did not have to seek out.
- The only irreversible operation in this specification is the only one that asks first.
- A user can no longer be surprised by an import deleting their notes, renaming their deck, or moving
  notes between decks — those are the three things the preview exists to state.
- ADR-0008 §5's tombstone report and §11's skipped-notes report are discharged **before** the commit
  rather than after, which the requirement permitted but did not require.
- ADR-0008 §4's *"same revision, different bytes"* condition finally has a surface to be reported on.
- ADR-0008 §12's file metadata acquires both a reader and a memory; without this ADR it was written
  to every deck file and displayed nowhere.
- The read-without-inflating property that justified the zip container pays for itself in the file
  list rather than in the preview it was designed for.
- Re-importing an unchanged file costs one dismissal instead of being silent — the price of the gate,
  paid in the most common repeat case.
- An export tells the user where the file is, which is the only way they can find it: they chose
  neither its name nor its location.

## Open items handed onward

| Item | Owner |
|---|---|
| **Whether the application helps send an exported deck file**, and what it costs on each platform. The picker's disqualifying property — delivery through an activity *result* — does not reach a send intent, and nobody has checked | [#70 — sending an exported deck file](https://github.com/amin-bf/leitner/issues/70) |
| **`MediaStore` collision behaviour on the handset** — whether a colliding display name overwrites, dedupes or fails. §10 is written to be correct under all three, so nothing is blocked; it is the same unverified Android surface [ADR-0016 §5](0016-backup-and-restore.md) records, under `AGENTS.md` rule 9 | Implementation |
| Visual treatment of the preview, the export screen, the export report and the file list. What they *say* and when they appear is settled here; only how they look is open | **Out of scope** — *the visual design pass*, which [ADR-0006 §10](0006-the-review-session-experience.md) opened and [the map](https://github.com/amin-bf/leitner/issues/1) ruled out on 2026-07-31 |
