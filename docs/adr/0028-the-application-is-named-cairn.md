# ADR-0028: The application is named Cairn

- **Status**: Accepted
- **Date**: 2026-08-08
- **Related**: [ADR-0001 §1 §3](0001-scheduling-algorithm-and-grade-scale.md) (the scheduler chosen,
  the graded box system rejected as the engine, and what a box is),
  [ADR-0008 §10 §13](0008-the-deck-export-format.md) (an extension per profile; the `mimetype`
  member as the authority — this ADR **substitutes the extension strings**),
  [ADR-0015 §10](0015-the-sync-experience.md) (sync settings name the provider's
  connected-applications route **and the name we appear under**),
  [ADR-0020 §5](0020-protection-at-rest.md) (which makes that route load-bearing rather than a
  courtesy), [ADR-0024 §1](0024-identifying-a-written-file.md) (identity lives in the bytes; the
  extension gates reachability upstream of the sniff)
- **Evidence**: [`docs/research/scheduling-algorithms/appendix-b-leitner-boxes.md`](../research/scheduling-algorithms/appendix-b-leitner-boxes.md)

The name was chosen before the scheduler was, and every decision since has moved away from it. This
ADR records the replacement, what the name is required to do — one of its jobs is functional and not
a matter of taste — and the one item in the change that cannot be taken back.

## Context

### The old name asserted a mechanism this design rejects

The card-box method as published is governed by **partition capacity**: five partitions sized
1 / 2 / 5 / 8 / 14 cm, reviewed when one fills, cards moving forward or back on a **binary**
remembered/not signal, with intervals **emergent** from deck size and study rate rather than
specified. A commonly described variant instead samples proportionally — all of partition 1, half of
partition 2, a quarter of partition 3 — so lower partitions come round more often. Both readings are
[SECONDARY] at best; the primary source could not be read, the most-cited sentence carries no inline
footnote, and the two academic treatments that model the method collapse grades to binary
deliberately. Sources, tags and caveats are in the evidence file.

Set that against what this application actually does:

| The named method | This application |
|---|---|
| Review triggered by a partition filling | Per-card due date from a memory model |
| Binary remembered / not | Four grades, with only grade 1 a failure ([ADR-0001 §2](0001-scheduling-algorithm-and-grade-scale.md)) |
| Cards move forward and back between partitions | A box is **computed from stability alone**, thresholds 1 / 7 / 30 / 180 days |
| Lower partitions come round more often | *"nothing may state or imply that lower boxes come up more often"* |
| A partition is a queue you can count | *"boxes are never sorted, counted, or presented as a review queue"* |

The right-hand column is not a coincidence of implementation. [ADR-0001 §1](0001-scheduling-algorithm-and-grade-scale.md)
**rejected the graded box system as the engine** on the grounds that adopting it would mean inventing
the core of the product's correctness with no evidence behind it. What survived is a UI bucket that
expresses **durability, never urgency** — and three rules in the scheduling glossary, plus
[ADR-0021](0021-note-ordering-saving-and-the-note-list.md)'s prohibition on aggregating a per-note box
figure, exist to hold it there.

So the name promised, on the window title and in every export filename, the four properties the
design spends four rules forbidding. That is a name doing negative work: the clearest statement of
the product's thesis was contradicted by the first word a user reads.

### One of the name's jobs is functional

[ADR-0015 §10](0015-the-sync-experience.md) requires sync settings to name the provider's
connected-applications route **and the name we appear under**, because the published folder is hidden
and cannot be navigated to. §10 calls that name a *"fourth console trap"*: it is set in a third
party's console, **no code path in this repository can validate it**, and it fails by making a
documented route un-followable rather than by producing an error.

[ADR-0020 §5](0020-protection-at-rest.md) then made that route load-bearing — with nothing encrypted
anywhere, it is the user's only means of removing plaintext a third party would otherwise hold
indefinitely, and §5 says explicitly that §10's naming requirement *"may not be dropped as a nicety"*.

**This places a hard constraint on the name that has nothing to do with branding**: it must be
recognisable, and unambiguous, in a list of unrelated third-party application entries the user is
scanning under stress. A common dictionary word fails this — it is the failure mode §10 describes,
where *"find it in the list"* fails with nothing anyone can act on.

## Decision

### 1. The application is named **Cairn**

A cairn is a marker built one stone at a time, which endures weather and marks a path for whoever
comes next. Each of those maps onto something this design already committed to: **accretive** (the
log is append-only and never compacted — [ADR-0004 §10](0004-the-review-event-log.md)), **durable**
(what a box reports, [ADR-0001 §3](0001-scheduling-algorithm-and-grade-scale.md)), **silent** (it
does not signal; ADR-0015 §5 holds the number of things that may speak about sync at two), and
**local** (it is where it stands — the remote is a rendezvous point, not a system of record).

It satisfies the §10 constraint above: concrete, one word, not a generic term for the category, and
unlikely to be confused with a neighbouring entry in a connected-applications list.

**Rejected: keeping the old name and treating it as arbitrary.** Tenable in principle — many products
carry names that stopped describing them — and rejected because this one does not merely fail to
describe, it **teaches the opposite** of four written rules, and the surfaces that would have to
correct it are exactly the surfaces this design keeps quiet.

**Rejected: a name built on the decay model** — the memory model's stability is defined as the days
for recall probability to fall to 90%, which is a half-life, and a name saying so would be the most
accurate description available of what the application tracks. Rejected on §10's constraint alone:
the strongest candidate in that family collides with a widely known game, and §10's list is exactly
where a collision costs the user the route.

**Rejected: a Persian-rooted name** (`یادگار` — a keepsake, the thing kept in order to remember by),
which fits the domain more closely than the chosen name does and would have suited an application
whose bidi handling is a first-class concern rather than an accommodation. Rejected because it makes
the Persian surface the primary register and the Latin one a transliteration, which is a product
stance this project has not taken and should not take by way of a name.

### 2. The box survives the rename, and recovers its justification

[ADR-0001](0001-scheduling-algorithm-and-grade-scale.md)'s Context opens *"The app is
Leitner-branded and shows users boxes"* — making the branding the stated reason the box exists at
all. Removing the brand therefore looks like it removes the box's warrant. It does not, and the
rename **improves** the box's standing rather than threatening it:

- The scheduling glossary already defines a box **without reference to the method it was named
  after** — a bucket 1 to 5, computed from stability, expressing durability. Nothing in that
  definition depends on the old name being on the window.
- The three rules that constrain a box were written to *suppress* what the old name implied. With the
  name gone, the box's own definition is the only thing teaching, and the rules stop working against
  a headwind.

**The term `Box` is unchanged, and its `_Avoid_` list — level, stage, bucket, interval band — stands.**
An implementer who reads the rename as licence to rename the concept has the direction backwards.

### 3. The container extensions follow the name, and the substitution must be measured

`.ldeck` and `.lcoll` become **`.cdeck` and `.ccoll`**. The extension is a string the user sees, in
the file list, in the export report, and on a file they hand to a stranger; a permanently unexplained
initial in a shipped artifact is not something this repository leaves lying around.

**What must not be assumed is that the substitution is free**, because
[ADR-0024 §1](0024-identifying-a-written-file.md) established by handset measurement that the
extension does two jobs, one of them upstream of everything else:

1. It is the `LIKE` clause the file list enumerates `MediaStore` with.
2. It **gates reachability**. `MediaStore` derives the stored media type from the extension, and only
   a type of `application/octet-stream` causes the broad intent filter to fire. A byte-identical deck
   under an extension the platform *does* recognise types as something else, is never offered to the
   application, and **no sniff can recover it**.

An extension absent from the platform's media-type map is what buys job 2, and whether a given
extension is absent from it is a **fact about a third party's table, not a property that can be
reasoned to**. So the substitution carried the same obligation ADR-0024 itself met: the three-name
comparison on the real handset, re-run for `.cdeck` and `.ccoll`.

**That run is done, and the substitution is free.** Pixel 8 Pro, Android 17 / API 37, one fixture
under five names with `.ldeck` as a positive control and `.txt` as the negative one
([evidence](../research/extension-rename-reachability/README.md)):

| Name | Stored type | Our filters resolve |
|---|---|---|
| `Inbound.cdeck` | `application/octet-stream` | **yes** |
| `Inbound.ccoll` | `application/octet-stream` | **yes** |
| `Inbound.ldeck` *(control)* | `application/octet-stream` | yes |
| `Inbound` *(no extension)* | `application/octet-stream` | yes |
| `Inbound.txt` *(control)* | **`text/plain`** | **no** |

Both new extensions reproduce every column ADR-0024 measured for the old ones. **The claim is bounded
to API 37**, which is the level this run covers; nothing is asserted for 24–36, exactly as ADR-0024
asserted nothing below 29.

**What does not change**: the `mimetype` member remains the sole authority over a file's **profile**,
and neither extension is ever consulted to decide one ([ADR-0024 §1](0024-identifying-a-written-file.md),
[ADR-0008 §13](0008-the-deck-export-format.md)). Two extensions are kept rather than collapsed to one,
because on the desktop — where the media type is not flattened — they are still how a user tells a deck
file from a whole-collection archive before opening it, which is the job
[ADR-0008 §10](0008-the-deck-export-format.md) gave them.

### 3a. The media-type strings carry the name, and they matter more than the extension

`application/vnd.leitner.deck+zip` and `application/vnd.leitner.collection+zip` become
**`application/vnd.cairn.deck+zip`** and **`application/vnd.cairn.collection+zip`**.

This is a larger change than §3 and is easy to mistake for a smaller one. The extension is a display
string and an enumeration hint; **these two strings are the sole authority over what a file is**
([ADR-0024 §1](0024-identifying-a-written-file.md)) — they are the value written into the `mimetype`
member at its fixed offset, and the value the sniff compares against. A vendor tree naming a product
that no longer exists would sit at byte offset 38 of every artifact this application ever writes.

**It is a format change, and the only reason it is cheap is that nothing has shipped.** After the
first archive a user holds, the old string is a compatibility surface: a reader that does not know it
refuses a file that is genuinely ours, and [ADR-0022 §4](0022-the-import-preview-and-export-report.md)'s
refusal is *honest but unhelpful* in exactly the case the sniff exists to handle. **No compatibility
shim is added for the old strings**, and adding one later would be the wrong repair — it would make the
profile authority a set rather than a value, which is what §1 of ADR-0024 spent a handset measurement
establishing it must not be.

The Android manifest's **precise** intent filter declares the deck type and moves with it. The broad
`application/octet-stream` filter is untouched and remains the only inbound door that matters — the
precise one is kept because it costs nothing and a phone-to-phone share declaring our own type matches
it ([ADR-0023 §3](0023-sending-a-written-file.md)).

### 4. Accepted ADRs are not rewritten, and research notes least of all

Every ADR from 0001 onward says *Leitner*, and five of them say `.ldeck`. **None of that prose is
edited.** An ADR records a decision as it was made; the substitution is recorded **here**, and
[`CONTEXT-MAP.md`](../../CONTEXT-MAP.md)'s index is what carries a reader from one to the other.

This binds hardest on `docs/research/`. Those files record **what was measured, under the names it was
measured under** — ADR-0024's three-name comparison used `Inbound.ldeck`, and rewriting that string
would turn a record of a handset run into a claim about a run that never happened. A research note is
evidence, and evidence is not renamed.

**A link is not a record, and conflating the two is the error to avoid in both directions.** An issue
URL names a *resource*, and that resource moved with the repository: `…/leitner/issues/5` and
`…/cairn/issues/5` are the same issue, under the old and new canonical address. Re-pointing it
asserts nothing new and falsifies nothing, so **every `amin-bf/leitner` URL in `docs/adr/` and
`docs/research/` is updated to `amin-bf/cairn`**, frozen prose or not. What this section protects is
the **claim a sentence makes** — a measured filename, a decided extension, a figure read off a device
— not the address at which a cited thing can be fetched.

The practical gain is that no document depends on a rename redirect. That redirect is real and works,
but it survives only while no repository named `leitner` exists under this account again — a condition
nothing in this repository can enforce, check, or notice the loss of.

**Living documents move entirely**: `README.md`, `AGENTS.md`, `CONTEXT-MAP.md` and every `CONTEXT.md`
describe the system as it is now, not as it was decided, and they carry the new name throughout.

### 5. What moves, and the one item that cannot be taken back

| Item | Note |
|---|---|
| `package = "dev.leitner.app"` (`crates/app/Cargo.toml`) | **Irreversible after the first install.** A package id is the identity Android upgrades against; changing it later is a different application with no upgrade path and no access to the old data directory. Nothing has shipped, so this is the last moment it is free. |
| `.ldeck` / `.lcoll` → `.cdeck` / `.ccoll` | Blocked on §3's handset measurement. Two constants in the export crate, plus the `LIKE` clause and the display strings. |
| `application/vnd.leitner.*+zip` → `application/vnd.cairn.*+zip` | **A format change, not a string change** (§3a). The value at the `mimetype` member's fixed offset, and therefore the profile authority. Free only because nothing has shipped; no shim for the old strings. |
| `$XDG_DATA_HOME/leitner/`, `$XDG_STATE_HOME/leitner/` | A rename with no migration presents as **an empty collection and a re-minted writer marker**, not as an error. Either move both directories or leave both; moving the data directory alone makes the device a duplicate writer ([ADR-0007 §6](0007-the-local-store.md)). |
| Six crate names, `LeitnerApp`, `APP_NAME`, the window title | Mechanical. |
| The consent screen's application name | A console setting, **unvalidatable from here** (ADR-0015 §10). It has to match the new name or §10's route breaks — and it breaks silently. |
| Repository `amin-bf/leitner` → `amin-bf/cairn` | Every issue URL in the repository is re-pointed (§4), so **nothing depends on the host's rename redirect**. That redirect exists and works, but survives only while no repository named `leitner` is created under this account again — a condition nothing here can enforce or notice the loss of. |

## Amendments to accepted ADRs

| ADR | What changes | Why |
|---|---|---|
| [0001 Context](0001-scheduling-algorithm-and-grade-scale.md) | *"The app is Leitner-branded and shows users boxes"* — the first clause is no longer true. The box's warrant is now its own definition in the scheduling glossary, not the branding. | §2 above. §1's rejection of the graded box system as the engine, and §3's definition of a box, are untouched — only the sentence explaining *why the app shows boxes at all*. |
| [0008 §10](0008-the-deck-export-format.md) | The extension strings become `.cdeck` and `.ccoll`. The rule — **one extension per profile, and the extension is never the authority on which** — is unchanged. | §3 above. §10's reasoning was never about the letters. |
| [0015 §10](0015-the-sync-experience.md) | *"the name we appear under"* now resolves to **Cairn**. The requirement is unchanged and the trap it names is unchanged. | §1 above. This is the one place where the name is functional rather than cosmetic, which is why §10 constrained the choice. |

## Consequences

- **The product's thesis is stated by its name instead of contradicted by it.** The three box rules
  and ADR-0021's no-aggregation rule still bind — they were never only corrections of the name — but
  they stop being read as arbitrary.
- **`docs/adr/` and `docs/research/` now speak an older name than the code.** This is deliberate (§4)
  and it has a cost: a reader arriving at ADR-0013 sees a name that appears nowhere in the workspace.
  The index in `CONTEXT-MAP.md` is the only thing bridging that, so **an index that does not say so is
  the failure mode**, not a stale ADR.
- **A future extension change is more expensive than this one**, not less. This one is free because
  nothing has shipped; after the first `.cdeck` a user has shared, it is a compatibility surface.
- **The consent-screen name is now a second unvalidatable string that must be changed by hand, in a
  console, at the same time as the code.** Change one and not the other and ADR-0020 §5's only
  plaintext-removal route silently stops being followable.

## Open items handed onward

- ~~**The handset re-measurement of §3**~~ — **run and discharged** at API 37
  ([evidence](../research/extension-rename-reachability/README.md)). Both new extensions store as
  `application/octet-stream` and resolve our filters; the `.txt` control is still unreachable. The
  API 24–36 window stays unmeasured, which is the pre-existing gap rather than one this ADR opened.
- **Whether the directory rename carries a migration or takes the fresh-start** (§5). It is a
  decision about existing local collections, of which there are only developer ones, and it belongs to
  the ticket that performs the rename rather than here.
- **Nothing else.** The name is chosen, what constrained the choice is written down, and the item that
  cannot be taken back is named as such.
