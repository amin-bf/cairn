# ADR-0016: Backup and restore

- **Status**: Accepted
- **Date**: 2026-07-31
- **Resolves**: [Decide: backup and restore](https://github.com/amin-bf/cairn/issues/37)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/cairn/issues/1)
- **Related**: [ADR-0004 §7](0004-the-review-event-log.md) (the mutable surface, deletion, the stamp
  rule), [ADR-0007 §6](0007-the-local-store.md) (the writer marker, Auto Backup),
  [ADR-0008](0008-the-deck-export-format.md) (the container, the reserved profile, determinism),
  [ADR-0009 §4](0009-crate-and-workspace-layout.md) (the platform seam),
  [ADR-0013](0013-the-sync-transport.md) (the transport, one account is one collection),
  [ADR-0014 §2](0014-when-parameter-optimisation-runs.md) (the nudge form),
  [ADR-0015](0015-the-sync-experience.md) (what the app is entitled to say; the two questions §13
  answers)

## Context

The ticket asked how a user gets their collection back after losing, replacing or wiping a device,
and it was blocked deliberately: [#33](https://github.com/amin-bf/cairn/issues/33) and
[#39](https://github.com/amin-bf/cairn/issues/39) had to land first, because a transport that keeps
a full copy somewhere the user controls would make backup a side effect and dissolve the question.

That transport landed. [ADR-0013 §7](0013-the-sync-transport.md) publishes not only the log but the
**mutable surface** per writer — and note content lives on that surface
([ADR-0004 §7](0004-the-review-event-log.md)) — so an enrolled account holds content *and* progress,
and a fresh install signing in rebuilds everything. The dissolution case is stronger than the ticket
anticipated.

It still fails, for reasons §1 sets out. Three earlier decisions then constrain every answer below.
**The container is not open**: [ADR-0008 §1](0008-the-deck-export-format.md) rejected "letting a
backup artifact invent its own container later" and reserved a profile slot for this ticket.
**Restore is already safe**: [ADR-0007 §6](0007-the-local-store.md)'s writer marker turns a restored
collection into a clean fork rather than a duplicate writer. And **the platform seam is already
contradictory**: [ADR-0013 §12](0013-the-sync-transport.md) recorded that ADR-0009's two instructions
about platform capabilities cannot both be followed, routed around it, and predicted that *"the next
ticket to need one will hit it head on with no equivalent escape."* This is that ticket — §5.

Two arithmetic findings turned up in the sources rather than in the reasoning, and both are recorded
as amendments: ADR-0007 §6 reads the wrong row of its own table, and `derived.db` is spending the
backup quota it warns about.

## Decision

### 1. The archive exists; sync does not discharge this ticket

> **A whole-collection archive is a specified artifact, separate from sync.**

Sync covers device loss for an enrolled user, and covers it well. Three failures survive it, and they
are different in kind:

- **The never-enrolled user.** [ADR-0013 §8](0013-the-sync-transport.md) makes sync opt-in and says it
  "never blocks first use," because offline-by-default is the map's operating mode rather than a
  feature. Reviewing for years with no account is a **supported configuration**, and for that user
  sync is not a weak backup story — it is the absence of one.
- **Loss of the account or the grant.** Revocation is all-or-nothing (§9 there), the folder is deleted
  if the user removes the app's data, and §2's *"losing the remote costs one republish rather than any
  data"* is true only while a local copy survives. The device-lost-and-account-lost path takes both.
- **Deletion.** Sync propagates a delete to every device in seconds, and
  [ADR-0004 §7](0004-the-review-event-log.md) accepts that the content is gone — *"undeleting restores
  the schedule but not the text, which must come from a backup or an export."* An accepted ADR already
  spends this artifact by name.

**And the sync folder must not be presented as a backup**, which [ADR-0013 §3](0013-the-sync-transport.md)
fixed as a constraint rather than an option: it is hidden, it holds a published projection, and it is
deleted when the user removes the app's data.

**The honest limit is recorded in §4**: this protects against loss, not against unwanted change.

### 2. The archive is a third profile in the `.ldeck` container: `collection`

[ADR-0008 §1](0008-the-deck-export-format.md) reserved a **`progress`** profile for this ticket. The
slot is used and the name is wrong: the artifact carries content as well, and a name describing half a
payload is how the wrong selection rule gets implemented. It is the **`collection`** profile.

**The selection rule is not "all decks."** It is *everything that settles, plus the log, minus device
identity and credentials* — a different rule, not a wider one, because the deck profile deliberately
drops most of what a restore needs:

| | Deck profile | `collection` profile |
|---|---|---|
| Unfiled notes | never exported ([§8](0008-the-deck-export-format.md)) | **carried** — else backup silently drops notes |
| ADR-0004 §7 stamps | do not travel ([§3](0008-the-deck-export-format.md)) | **carried byte for byte** |
| `{revision, digest}` per deck | never exported ([§9](0008-the-deck-export-format.md)) | **carried** — else the next export emits a phantom revision |
| Suspension ([ADR-0010 §5](0010-leeches.md)) | never exported | **carried** |
| New-card rate ([ADR-0011 §5](0011-new-card-rate-and-daily-limits.md)) | not content | **carried** |
| Device labels ([ADR-0004 §3](0004-the-review-event-log.md)) | never exported | **carried** |
| The log | absent | **carried verbatim**, as received |
| `writer_id`, `seq_highwater` | absent | **never** — [ADR-0007 §6](0007-the-local-store.md) requires a fresh id |
| The sync credential | absent | **never** |

**A distinct profile rather than a flag, because the rule that differs is destructive in one
direction.** [ADR-0008 §3](0008-the-deck-export-format.md) calls import and restore *"same container,
opposite rule"* — import restamps, restore preserves. A flag inside one profile is ignorable, and
ignoring it produces a restored device whose fresh counters outrank genuinely later edits still held
on another device: silent, and unrecoverable. As a **profile**, a reader that does not understand
`collection` refuses at [§7](0008-the-deck-export-format.md)'s hard gate instead of half-reading the
file.

**The rule is "the log verbatim, plus everything that settles" — not "everything we cannot
reconstruct."** [ADR-0014](0014-when-parameter-optimisation-runs.md) supplies the reason by name: its
**fitted-over count** rides inside a `config-set` row and is stated to be *"not… a derived column that
a restore may recompute,"* because §6 there establishes that recomputing it reports a fit that never
happened. The set of things that *look* derivable but are not is exactly where a selection rule fails
silently, so the rule is stated positively.

### 3. A collection has an identity, adopted and never re-minted

> **A collection id, minted once as a UUIDv4, carried in the `collection` profile.**

This is the map's *Collection identity* fog, which
[ADR-0008 §3](0008-the-deck-export-format.md) called load-bearing — telling an import from a restore
"requires knowing which collection a payload came from" — and which
[ADR-0013 §10](0013-the-sync-transport.md) half-answered and explicitly declined to finish.

§2's profile answers *which rule*. It cannot answer *whose collection*, and the gap is one case: a
device holding collection X is handed a `collection` archive from collection Y. The profile says
"restore", so Y's stamps travel byte for byte — and Y's counters are meaningless in X, precisely the
comparison [ADR-0005 §9](0005-the-deck-model.md) spends a paragraph forbidding. Nothing corrupts; the
user silently acquires a stranger's collection. With an id it is a refusal, not a judgement.

**Identity now has two halves, and they behave oppositely.** This is worth stating together because
each is a rule an agent will otherwise apply to the wrong one:

| | Writer id ([ADR-0004 §3](0004-the-review-event-log.md), [ADR-0007 §6](0007-the-local-store.md)) | Collection id |
|---|---|---|
| On finding one you did not mint | **never adopt** — mint fresh | **always adopt** — never re-mint |
| Why | a sequence number promises sole authorship, which only continuous possession establishes | every device of one collection must agree, or the check is worthless |

**Where it lives.** Minted at first launch beside the writer marker, held in `local`. It is **not** on
the mutable surface — it must never settle, since two devices settling on different ids for one
collection is the failure it exists to prevent — and **not** in the log, since it is not an input to
replay. Through sync it rides as one small immutable object under each writer's own prefix,
consistent with [ADR-0013 §1](0013-the-sync-transport.md)'s single-author-per-key property and §4's
never-rewritten rule. Every writer publishes the same value; two different values is an alarm that
[ADR-0013 §10](0013-the-sync-transport.md) says this transport cannot produce.

**Rejected: a confirmation prompt instead.** Cheaper, and it asks the user to know something they
cannot know — which is how the wrong stamp rule gets applied with a click.

### 4. Restore is a merge, never a replace

> **Restore adds. It never removes anything the device already holds.**

**A replace cannot stick, and that is structural.** Every device holds the whole log, it is never
compacted ([ADR-0004 §10](0004-the-review-event-log.md)), and merge is set union on `(writer, seq)`.
So a device that wipes its collection and installs an archive has the **next sync re-merge everything
it just removed** — the remote and every peer still hold those rows, and nothing in this design can
propagate a deletion of log rows. The mutable surface goes the same way: the peers' newer stamps win,
so a deletion flag returns with them.

[ADR-0013 §2](0013-the-sync-transport.md) states this pointing the other way — losing the remote
"costs one republish rather than any data at all." The symmetric consequence is that **losing data on
purpose does not work either**. A restore that silently undoes itself on the next sync is worse than
no restore at all.

**Merge-restore discharges [ADR-0004 §7](0004-the-review-event-log.md), and the mechanism is worth
spelling out** because it looks like it should not work:

> A deleted note *"keeps only its marker, not its content."* The content is **discarded, not
> superseded by a competing value**. So an archive predating the delete carries those field values
> with old stamps and meets nothing to lose to — they land. Undelete the note, restore, and the text
> is back. It is [ADR-0002 §7](0002-the-card-model.md)'s *"history reattaches by itself"* applied to
> content instead of schedule, and it needs no new mechanism.

**Accepted cost: this protects against loss, not against unwanted change.** A bad edit — right text
typed over with wrong text — is a settled value carrying a newer stamp, and the archive's older stamp
must lose or [ADR-0004 §7](0004-the-review-event-log.md)'s causality rule is broken. The same is true
of a regretted import. Recorded plainly because "backup" invites the belief that any past state can be
recovered, and here only *destroyed* state can be.

**Rejected: offering replace for the unsynced case**, where it is genuinely safe because no peer
exists to re-merge from. An operation that is correct in one configuration and silently self-reversing
in another is the conditional rule this specification has refused repeatedly.

### 5. Artifacts reach the filesystem through a put/get/list seam, and this resolves ADR-0013 §12

Nobody owned this. [ADR-0008](0008-the-deck-export-format.md) specifies the container's bytes and
never says how a file reaches the user's filesystem, so the deck file has the same gap; it simply
never had to be faced. A backup that cannot leave the device is not a backup.

**A file picker is unavailable at a price this specification has already refused.**
`ACTION_CREATE_DOCUMENT` and `ACTION_OPEN_DOCUMENT` deliver through an activity *result*.
`android.app.NativeActivity` does not forward results to native code and the native glue has no path
for one, so catching it requires our own Java subclass — which requires a dex, a build system and
`res/`, overturning [ADR-0003 §2](0003-client-stack.md)'s measured *"manifest plus one `.so`"*
property that [ADR-0009](0009-crate-and-workspace-layout.md) and
[ADR-0014 §3](0014-when-parameter-optimisation-runs.md) both lean on.

> **The seam is three operations: put a named file, get a named file, list the files we recognise.
> Nothing else. No picker, no dialog, no path typed by a user.**

> **[ADR-0022](0022-the-import-preview-and-export-report.md) says what those three surface.** §11:
> the **list** describes each file from its own manifest rather than by filename. §10: because there
> is no picker and no typed text, **the user chose neither the name nor the location**, so the
> post-export report states both — and states the name the platform *actually wrote*, since this
> section records that the `MediaStore` path is unverified on the handset and a colliding display
> name may overwrite, dedupe or fail. §6: all three entry points above produce **one** screen, which
> the launch intent requires to be cold-start capable.
>
> **One thing this section decided against is now a live question rather than a settled one.** The
> disqualifying property above is delivery through an activity **result** — and a *send* intent has
> no result, exactly like the launch intent this section admits two paragraphs down. Whether the
> application helps a user send an exported deck was never asked; it is
> [#70](https://github.com/amin-bf/cairn/issues/70)'s.

This is deliberately [ADR-0013 §1](0013-the-sync-transport.md)'s shape — *"put an object, get an
object, list a prefix, delete an object. Nothing else"* — reused rather than reinvented. This
specification has already priced an opaque, minimal, enumerable seam once and liked the result; using
one idea twice is cheaper than two. **Three operations, not four: there is deliberately no delete**,
for the reason §13 gives.

| Target | Put | List / get |
|---|---|---|
| Desktop | write into the user's documents directory | scan documents and downloads |
| Android | insert into `MediaStore` `Downloads` via `ContentResolver`, write to the returned URI | query `MediaStore` for our extensions |

> **The Android list is narrowed by [ADR-0024 §3](0024-identifying-a-written-file.md): it returns
> only files this application wrote.** Measured — a `.ldeck` placed in `Downloads` by another package
> is invisible to us, while a control file we wrote ourselves appears in the same query. The *"no
> permission at API 29+"* claim above was measured for the **put** and does not extend to reading;
> scoped storage grants an application its own rows and nothing else, and `READ_MEDIA_*` covers
> images, video and audio rather than documents. **So the list is not a view of the `Downloads`
> folder**, and a deck someone sends can never appear in it.
>
> **The put is unaffected**, and so is the refusal of the picker below — which now carries a second
> load: with no picker and no folder visibility, an **intent filter is the only inbound door**, which
> is what forces ADR-0024 §2's broad media-type filter. The Android write also stops declaring a
> `mime_type` (ADR-0024 §4), which is what keeps the extension on a deduped name.

The Android side needs **no activity result and no permission at API 29+**, and is reachable by JNI
from the Context the existing shim already obtains. It is not verified on the handset — see *Open
items*, and `AGENTS.md` rule 9.

> **Verified, and the seam is now four operations wide — [ADR-0023](0023-sending-a-written-file.md).**
> The `MediaStore` put works exactly as written: insert returned a URI, `openOutputStream` accepted
> the bytes, a read-back reported them, and **no permission was requested or needed** at API 37
> ([evidence](../research/android-outbound-share/README.md)). This paragraph's caveat is discharged.
>
> **The count in this section is no longer the invariant.** ADR-0023 §1 adds a fourth operation,
> `hand_off`, having found that *"three operations, not four"* was an argument about **delete** — which
> remains absent — and that this seam was sized against what **backup** needs rather than what the
> deck file needs. What still binds is *opaque, minimal, enumerable*.
>
> **The picker argument holds and is now bounded by measurement.** A *send* was launched from
> `NativeActivity` in the shipped APK shape — manifest plus one `.so`, no `classes.dex`, no `res/` —
> so it really is in the launch-intent category this section admits rather than the result category
> it refuses. **One correction to the mechanism named above**: the context the shim obtains is
> `android.app.Application`, **not** the Activity, so `startActivity` requires
> `FLAG_ACTIVITY_NEW_TASK`.
>
> **And the extension this section leans on does not survive the write.** `MediaStore` derives the
> media type from the extension and discards ours, and a colliding name is deduped to
> `archive.lcoll (1)` — a name that no longer ends in `.lcoll`, and which the *"intent filter on the
> extension"* above will therefore not match. That is
> [#72](https://github.com/amin-bf/cairn/issues/72)'s, not ADR-0023's.

**Three additional entry points, each costing nothing:**

- **Desktop drag-and-drop.** egui surfaces dropped files directly, with no operating-system dialog and
  no seam function. A laptop user's archive lives in `~/backups` or on a stick, not in a downloads
  folder, so the symmetric list is genuinely worse there —
  [ADR-0014 §8](0014-when-parameter-optimisation-runs.md) refused "softer divergence" because the
  premise was refuted by measurement, and here the premise is true. It is **additive**: Android has
  nothing to drag, so it degrades to the list rather than diverging.
- **Android launch intent.** An intent filter on the extension lets a file manager open an archive,
  and a *launch* intent is readable from the activity with no result callback and no dex — unlike the
  picker above. [ADR-0008 §10](0008-the-deck-export-format.md) already requires such a filter for
  `.ldeck`; this adds the second extension to the same mechanism.

  > **The entry point survives; the mechanism is replaced by
  > [ADR-0024 §2](0024-identifying-a-written-file.md).** *"An intent filter on the extension"* cannot
  > work — no filename reaches a filter. The filter matches on **media type**
  > (`application/octet-stream`, plus the precise `application/vnd.leitner.deck+zip`) for both
  > `ACTION_VIEW` and `ACTION_SEND`, and the file is identified by content once opened. Both
  > extensions land on the same mechanism exactly as this bullet intended — it is simply not the
  > extension doing the matching.
- **Text is never typed.** No filename field, no path field. `AGENTS.md` rule 8 makes Android text
  input ASCII-only, so any such field is broken for the users this application exists for.

**On the seam rule, this ticket resolves the contradiction
[ADR-0013 §12](0013-the-sync-transport.md) recorded** rather than routing around it, by adopting the
fix that ADR's own text sketched: *"the seam rule is per crate rather than per workspace."* These three
functions live in a `platform` module **inside `leitner-export`**, under the same discipline —
three `#[cfg]` arms, the third a `compile_error!`. **`leitner-store::platform` keeps exactly two
functions**, so [ADR-0009 §4](0009-crate-and-workspace-layout.md)'s erosion signal is preserved
intact and means what it says.

### 6. The archive is written when the user asks, and the nudge states a fact

> **No schedule, no automatic write. An action in settings, and a subtitle beneath it.**

**The argument that decides it is specific to backup.** An archive written to the device's own
downloads folder is **not a backup until the user moves it off the device** — that folder dies with
the phone. So automating the write automates the half with no value, while the half that matters is
irreducibly a human act. Worse, it would manufacture exactly the false belief
[ADR-0007 §6](0007-the-local-store.md) warns about: files accumulating, nothing protected.

A real scheduler is disqualified independently.
[ADR-0014 §3](0014-when-parameter-optimisation-runs.md) rejected a foreground service or scheduled job
because [ADR-0003](0003-client-stack.md)'s prize is a Gradle-free APK, and a weekly background backup
spends it outright.

**The nudge takes [ADR-0014 §2](0014-when-parameter-optimisation-runs.md)'s form** — an action that is
never conditioned, and beneath it a subtitle carrying no threshold, no badge, no colour and no verb:

> Last backup 3 March. 1,240 reviews since.

and where none exists:

> No backup yet. 4,200 reviews across 812 notes.

**It appears in settings and nowhere else — specifically not at the end of a session.**
[ADR-0010 §9](0010-leeches.md) placed a pointer there and
[ADR-0014 §2](0014-when-parameter-optimisation-runs.md) already refused the slot on the ground that
"a second one competing for it devalues both." A third is not arguable.

**Where the precedent does not transfer, stated so it is not borrowed:**
[ADR-0014 §1](0014-when-parameter-optimisation-runs.md)'s deciding argument was **contention** — every
device crossing a threshold, training, and writing a competing `config-set` row. Backup writes nothing
to the log and nothing that settles, so no contention argument exists here. This rests entirely on the
off-device point above.

**Accepted cost, and it is the strongest objection in this ADR: a backup nobody makes is not a
backup.** [ADR-0014 §1](0014-when-parameter-optimisation-runs.md) accepted the same for optimisation,
but could afford to because the floor was a good one. Here the floor is §7's Auto Backup and nothing
else. The judgement is that an automatic local write does not raise that floor — it makes the nothing
feel like something.

### 7. Auto Backup stays on, and the application states the size fact

**Kept on.** For one user it is the only thing between them and total loss: never enrolled, never made
an archive, phone in a puddle. [ADR-0007 §6](0007-the-local-store.md) already paid the entire cost of
making that restore *safe* — the writer marker turns it into a clean fork — and
[ADR-0013 §9](0013-the-sync-transport.md) deliberately routes the refresh token through it so a
replaced phone arrives already authorised. Disabling it to prevent a false belief would punish exactly
the users it protects, and `android:allowBackup="false"` is a free manifest attribute, so this is a
choice rather than a constraint.

**The cutoff is stated, because it is observable by us.** `onQuotaExceeded()` needs a backup agent
class, which needs a dex; we do not need it. We know our own file sizes and the quota is a documented
platform constant, so on Android the subtitle gains one line once it is true:

> Your collection is 31 MB. Android's automatic backup stops above 25 MB.

Two facts, no verb, no colour. [ADR-0014 §2](0014-when-parameter-optimisation-runs.md) refuses
thresholds in a nudge, and the distinction defended here is that it refused them because *"there is no
defensible number to use"* — a documented platform constant is the one case where there is.

**How urgent this is: see the ADR-0007 §6 amendment below.** The crossing is roughly nine months of
heavy use, not two years.

> **Upheld and qualified by [ADR-0020 §6](0020-protection-at-rest.md), which supplies a second reason
> to keep it on and one fact this section does not state.** The second reason: refusing backup where
> the platform's stronger encryption is unavailable would spend protection against *loss* — the thing
> this ADR exists for — to buy confidentiality the specification concedes at every other artifact.
>
> **The fact: the payload is not guaranteed to be unreadable by the provider.** It is always encrypted
> in transit and at rest, but under operator-held keys; the layer keyed to the device lock screen needs
> **Android 9+ (API 28) and a lock screen actually set**, and this project's `min_sdk_version = 24`
> puts part of its own supported range permanently outside that. So a user on an API 24–27 handset, or
> with no lock screen, has their collection **and** the refresh token this section deliberately routes
> through the backup set held under keys the operator manages. Two asymmetries with the sync folder
> come with it: a device backup has **no per-application deletion path** and **survives uninstall**.
> [Evidence](../research/auto-backup-at-rest/README.md).

### 8. The archive is not encrypted

**Not even optionally.** This is the one artifact designed to leave the device and land on storage we
do not control, so the case for encryption is the strongest anywhere in this specification. It loses
on a repo-specific correctness argument:

> `AGENTS.md` rule 8 — **Android text input is ASCII-only and cannot be fixed here.** A user who sets
> a passphrase in their own language on the desktop **cannot type it on the phone**, so the archive
> becomes unopenable on the platform that most needs it. That is a correctness failure, not an
> inconvenience, and it lands on precisely the users this application exists for.

Three supporting grounds. **A passphrase-protected backup is the most reliable way to lose data
permanently**, and this design has nowhere to recover from — no server, no account, no escrow, by
construction. **The archive carries no credential** (§2), so the worst case is disclosure of the
user's own study material, not account takeover. And
[ADR-0008 §10](0008-the-deck-export-format.md) makes inspectability a feature — *"renaming a deck file
to `.zip` and looking inside is intended"* — which encryption forfeits for the artifact most likely to
need forensic recovery.

**Accepted cost, recorded rather than argued away.**
[ADR-0013 §9](0013-the-sync-transport.md)'s *"encrypting the token while the data it guards lies in
plaintext beside it is theatre"* does **not** transfer here, and this ADR declines to borrow it: that
argument works because both objects sit in the same app-private directory. **This archive travels and
`collection.db` does not**, so a plaintext archive in a downloads folder is genuinely more exposed than
anything else in the design. The map's *Local encryption / device passcode* fog is the honest home for
the general question, and if it lands it must cover the local store and the travelling archive
together.

> **Landed as [ADR-0020](0020-protection-at-rest.md), which upholds this conclusion and **does not
> keep its reasoning**.** It covers four artifacts rather than two — the store, the credential, this
> archive, and the log published to the drive, which this ADR did not count.
>
> **The ASCII argument above is no longer load-bearing, and that matters.** It kills a *passphrase*
> and it does **not reach a PIN**, because digits are ASCII — so a reader who finds this section
> convincing will one day notice the gap and reopen the question from its weakest point. ADR-0020 §3
> refuses on two grounds that hold regardless of what can be typed. **The loss asymmetry**, which is
> this section's own first supporting ground promoted to the decisive one. **And arithmetic**: a
> short numeric secret is safe on a handset only because the hardware refuses to be asked quickly,
> and data resting on a provider's disks cannot borrow that, so against unlimited offline guessing a
> six-digit space is routine.
>
> **This section's caveat was correct and is answered.** ADR-0013 §9's theatre argument indeed does
> not transfer to an artifact that travels; ADR-0020 §4 supplies one that does — **a key protecting a
> travelling artifact must reach every device that opens it, and with no server the only channel is
> the one being protected**.

### 9. `.lcoll`, self-identifying from its first bytes

The extension is **`.lcoll`**. The first member of the archive is `mimetype`, `stored` with no
compression, containing `application/vnd.leitner.collection+zip`.

This is [ADR-0008 §10](0008-the-deck-export-format.md)'s mechanism unchanged, for the reason it gives:
a zip archive's header says nothing about what it contains, so a type marker at a fixed byte offset is
what lets content sniffing identify a file whose extension was mangled or stripped. §10 states that a
distinct extension per profile is *"how the operating system and the user tell a deck file from a
whole-collection artifact **before** opening it"*, and hands the naming here.

**Rejected: `.leitner`.** Friendlier to a user staring at a downloads folder, and actively wrong in a
design with two file types that §10 spent its length making distinguishable.

### 10. Mismatched collection ids: an empty collection adopts, a non-empty one refuses

One rule, applied identically at both seams — restoring an archive, and enrolling a transport:

> **A collection that has authored nothing adopts the identity it meets. A collection that has
> authored something refuses any identity but its own.**

"Authored nothing" is sharp on purpose: **no log rows under this device's own writer id, and nothing
on the mutable surface.** Not "no notes" — a user who imported a deck and reviewed nothing has
authored nothing and must still be able to adopt.

| Device holds | Meets | Outcome |
|---|---|---|
| nothing | archive X | adopt X |
| X | archive X | merge; stamps byte for byte |
| X | archive Y | **refuse**, naming the mismatch |
| X | empty remote folder | publish; that account now holds X |
| X | remote folder holding X | normal sync |
| X | remote folder holding **Y** | **refuse**, naming the mismatch |
| fresh install, minted Z, authored nothing | remote folder holding X | **adopt X** |

That last row is the trap this rule exists for. §3 mints an id at first launch, so a brand-new install
already has one, and a naive "ids differ → refuse" would mean **a fresh install can never enrol into
an existing account** — the most common real path there is.

**This upgrades [ADR-0013 §10](0013-the-sync-transport.md) from a structural accident to a checked
invariant.** Its guarantee — one account is one collection — rests on the application data folder
being scoped per account and application, which really means *"you cannot merge two collections
because you cannot see the other one."* That stops being true the moment a user has two accounts and
picks the wrong one. The id makes it a check rather than a property of what happens to be visible.

**The way out of a refusal is always available and must be stated in the interface**, or a user is
left holding a device that will not talk to their account: make an archive, clear the app's data,
restore, enrol.

### 11. The manifest carries a creation date, and determinism is not inherited

[ADR-0008 §12](0008-the-deck-export-format.md) makes export **byte-for-byte deterministic** — member
timestamps pinned to a constant, no extra fields — so that build time does not leak and *"same
revision, same file"* is a property rather than an approximation. **That reasoning is entirely about an
artifact sent to strangers.**

The `collection` profile is the opposite artifact in every respect. It goes to nobody, it has no
revision, and **a backup without a date is close to useless**: §6's subtitle needs one, and a user
with three archives in a downloads folder needs to tell them apart before restoring the wrong one.
Determinism buys this profile nothing and costs it the one piece of metadata it needs.

> **The `collection` profile carries a creation timestamp in its manifest and does not inherit
> [ADR-0008 §12](0008-the-deck-export-format.md)'s determinism.**

Stated as an explicit non-inheritance rather than by silence, because §12 reads as a container-wide
rule and the next agent will assume it applies. **The disclosure half of §12 is inherited unchanged**:
no author name, no device label, no ambient identity ever auto-populated.

**Restore previews before it acts**, which [ADR-0008 §2](0008-the-deck-export-format.md) already
bought — the manifest is readable from the zip central directory *"without inflating the payload, so
an import can be previewed."* Before anything merges:

> Collection archive, 3 March 2026. 812 notes, 4,200 reviews.

This is also where §10's gate fires: a mismatched collection id is reported here, by name, rather than
after the fact.

> **Confirmed — not amended — by [ADR-0022 §12](0022-the-import-preview-and-export-report.md), which
> specifies a far richer preview for a deck import and explains why this one stays a single line.**
> The rule generating both: *a preview states effects in proportion to what can be lost.* §4 makes
> restore a merge that only ever adds — it cannot delete a note, rename a deck or move anything — so
> there are no destructive effects to enumerate and a description of the *file* is the whole of what
> is useful. A deck import can do all three, which is what buys ADR-0022 §3's line set. **Recorded
> because the asymmetry reads as an oversight**, and an agent bringing this line "into line" would be
> adding machinery to describe consequences that cannot occur.
>
> **The date this section puts in the manifest also becomes visible one step earlier**: ADR-0022 §11
> describes each file in §5's list from its own manifest, so three archives are told apart *before*
> one is opened — which is what this paragraph's *"needs to tell them apart before restoring the wrong
> one"* actually asked for, and could not get from filenames this ADR never specifies.

### 12. What a restore restores, stated plainly

- **Comes back**: every review ever recorded; all note content and tags; decks and membership;
  suspensions; the new-card rate; acquired kind definitions; per-deck revisions; and the scheduler
  parameters including [ADR-0014](0014-when-parameter-optimisation-runs.md)'s fitted-over count.
- **Does not come back**: **this device's identity.** It mints a fresh writer id
  ([ADR-0007 §6](0007-the-local-store.md)), so a restored device is a **clean fork rather than a
  resurrection** — deliberate, and the thing that stops two devices silently becoming one writer.
- **Is not removed**: anything the device already held. Restore is a merge (§4); it only ever adds.

**The last line is the one the interface must say**, because "restore" universally implies replacement
and here it does not.

### 13. Answering ADR-0015's two handoffs, one of them negatively

[ADR-0015](0015-the-sync-experience.md) landed in a parallel session and handed this ticket two
questions by name. Both are answered here, and the first is answered **no**.

**"Does collection identity make a wrong-account enrolment detectable?" — No, and the reason is
structural rather than a gap in §3.** ADR-0015 anticipated this correctly: a wrong account presents
an **empty folder identical to being first to enrol**, and an empty folder carries no information to
compare an id against. But the sharper statement is worth recording, because it explains why no
mechanism of this shape could have worked:

> **In a wrong-account enrolment every collection id agrees.** The devices on the right account and
> the device on the wrong one all hold the *same* collection — they simply cannot see each other. The
> failure is one of **reachability, not identity**, so an identity check was never going to detect
> it. §3 answers "is this the same collection?", and here the answer is *yes*.

So ADR-0015's sentence — the app states what it found after enrolling — **stands as the whole
defence**, and this ADR does not replace it. The one thing that would detect the case is naming the
**account** on the enrolment screen, which costs an `email` or `profile` scope that
[ADR-0013 §8](0013-the-sync-transport.md) deliberately did not request. That is a live trade, owned
by neither ADR, and it is recorded in *Open items* rather than decided here.

> **Widened by [ADR-0019 §3](0019-naming-the-account-at-enrolment.md): the reachability finding rules
> out *any* application-side check, not only an identity one.** In a wrong-account enrolment there is
> no peer, no namespace and no published byte to compare against, so **a check on the account address
> is void too** — publishing *"this device connected as X"* into the application data folder is
> unreadable by the device that needs it, which is looking at a different folder. **The only comparand
> that exists is the user's own memory of the previous enrolment.** ADR-0019 nonetheless takes the
> trade, on a corrected premise: naming the account is not *"the one thing that would detect"* the
> case — ADR-0015 §7's sentence already detects it — it is what lets a user **diagnose** it, instead of
> reaching for repairs that cannot work.

**"The no-delete reasoning does not transfer to an artifact that is not disposable." — Correct, and
it is already honoured.** ADR-0015 refuses a delete-remote-data control partly because the sync
namespace is disposable and there is nothing to reclaim. An archive is the opposite: for the
never-enrolled user of §1 it may be the only copy in existence. §5's seam is therefore **put, get,
list — and deliberately no delete**, unlike [ADR-0013 §1](0013-the-sync-transport.md)'s four
operations which this ADR otherwise copies.

> **The application never deletes a user's archive.** It writes into a user-visible folder, and
> removing files from a user-visible folder is the file manager's job. A delete in this seam would
> let the application destroy the one artifact that exists to survive the application.

The divergence from ADR-0013 §1's shape is one operation, in the safe direction, for a reason that is
the exact inverse of that ADR's: **there, deletion is safe because the objects are disposable; here,
deletion is refused because the artifact is not.**

## Amendments to accepted ADRs

| ADR | What changes | Why |
|---|---|---|
| [ADR-0007 §6](0007-the-local-store.md) | The Auto Backup quota is crossed in **about nine months** of heavy use, not "about two years". | §6 reads the wrong row of [§10](0007-the-local-store.md)'s own table: it used the **raw interchange** figure (11 MB/yr → 2.3 years) where Auto Backup covers **files on disk** (~33 MB/yr → ~0.76 years). The hazard it identified is roughly 2.5× more urgent than it states. |
| [ADR-0007 §6](0007-the-local-store.md) | **`derived.db` moves out of the backup set**, to `getNoBackupFilesDir()` beside the writer marker. | §3 makes it disposable and rebuildable by design, and §6 puts it where Auto Backup collects it — so the cache is uploaded, counts against the 25 MB quota, and accelerates the silent cutoff, to protect a file whose defining property is that losing it costs nothing. `ATTACH` is indifferent to the path, so this needs no new mechanism. |
| [ADR-0008 §1](0008-the-deck-export-format.md) | The reserved **`progress`** profile is specified as **`collection`**. | §2 above: it carries content as well as the log, so the reserved name understates the payload by half — and a name describing half a payload is how the wrong selection rule gets implemented. |
| [ADR-0008 §12](0008-the-deck-export-format.md) | Byte-for-byte determinism binds the **`deck`** profile only; the `collection` profile carries a creation date. Minimal disclosure still binds both. | §11 above: determinism exists so an artifact sent to strangers does not leak build time and keeps "same revision, same file". A personal archive has no revision, goes to nobody, and needs the date §12 forbids. |
| [ADR-0013 §12](0013-the-sync-transport.md) | The recorded contradiction is **resolved for platform *functions***, by adopting the fix its own text sketched: the seam rule is **per crate**. `leitner-export` gets its own three-arm `platform` module; `leitner-store::platform` keeps exactly two functions. | §5 above. ADR-0013 routed around it via the device flow and predicted the next ticket would meet it head on. It did. |

**Two ADRs resolved ADR-0013 §12 in parallel, on different facets, and they compose.**
[ADR-0015](0015-the-sync-experience.md) amends [ADR-0009 §4](0009-crate-and-workspace-layout.md) to
say the prohibition is on *behaviour and a growing function seam, rather than on a capability
constant*; this ADR says the *function* seam is per crate. Neither weakens
`leitner-store::platform`'s two-function limit, which is the property both are protecting, and
between them §12's contradiction is now discharged from both sides. Recorded because two independent
resolutions of one recorded contradiction is exactly the situation a later reader would otherwise
mistake for a conflict.

**No amendment is needed elsewhere.** [ADR-0004 §7](0004-the-review-event-log.md) is **discharged
rather than changed** — its *"must come from a backup or an export"* now names a specified artifact,
and §4 above shows the mechanism works because deletion discards rather than supersedes.
[ADR-0013 §3](0013-the-sync-transport.md)'s "the sync folder is not a backup" is honoured, not
revisited. [ADR-0009 §4](0009-crate-and-workspace-layout.md)'s two-function limit on
`leitner-store::platform` is **preserved intact**, which is the point of §5's per-crate reading.

## Requirements this places on downstream tickets

### [#40 — the sync experience](https://github.com/amin-bf/cairn/issues/40)

1. **Enrolment carries §10's identity check**, and both outcomes are UI moments: an empty collection
   adopting silently, and a non-empty one refusing. A refusal must name the mismatch and state the way
   out (archive, clear data, restore, enrol) or the user is stuck.
2. **The collection id is published as one small immutable object per writer prefix** (§3), under
   [ADR-0013 §4](0013-the-sync-transport.md)'s never-rewritten rule. It is not on the mutable surface
   and must never be made to settle.

### Implementation

1. **Verify the `MediaStore` path on the real handset before building on it** — `AGENTS.md` rule 9.
   §5's claim that an insert plus a `ContentResolver` write needs no activity result and no permission
   at API 29+ is reasoned from the platform's documented scoped-storage behaviour and **has not been
   run on the Pixel 8 Pro**. If it fails, §5's seam is unchanged and only its Android arm is in
   question.
2. **The `.lcoll` intent filter needs a `pathPattern` alongside the media type**, exactly as
   [ADR-0008 §10](0008-the-deck-export-format.md) records for `.ldeck` — custom extensions have no
   reliable extension-to-type mapping on Android.

## Glossary

New terms are of record in the `CONTEXT.md` files, per
[ADR-0009 §6](0009-crate-and-workspace-layout.md): **collection archive**, **collection profile** and
**collection id** in [`export`](../../crates/export/src/CONTEXT.md), which owns the container; the
collection id's *storage* is noted in [`store`](../../crates/store/src/CONTEXT.md) beside the writer
marker it is minted with.

## Consequences

- **The `export` crate gains a dependency on `log`**, which `CONTEXT-MAP.md` predicted in as many
  words — *"will depend on `log` once #37 specifies the progress profile"* — and which is why it is a
  peer of `replay` rather than a module inside `content`.
- **`leitner-store::platform` stays at two functions**, and the erosion signal
  [ADR-0009 §4](0009-crate-and-workspace-layout.md) attached to a third still means what it says.
  Every future platform capability now has a home that does not require breaking it.
- **The deck file gains a delivery mechanism it never had.** §5's seam was specified for the archive
  and answers `.ldeck` identically; ADR-0008 had specified bytes with no way to write them anywhere a
  user could reach.
- **Two file types now exist**, and both are self-identifying from their first bytes. Any third pays
  the same cost.
- **Nothing in this ADR makes the remote authoritative for anything**, so
  [ADR-0013 §2](0013-the-sync-transport.md)'s disposability property — which most of that ADR's
  reasoning rests on — is untouched.
- **`AGENTS.md` gains a backup section**, because four of the rules above fail silently: restore is a
  merge, the two identity halves behave oppositely, the collection profile does not inherit
  determinism, and `derived.db` must stay outside the backup set.

## Open items handed onward

| Item | Owner |
|---|---|
| Running an archive write and a restore on the real handset, including the `MediaStore` path | Implementation |
| ~~**Whether the enrolment screen should name the account it connected as**, which is the only thing that would detect [ADR-0015](0015-the-sync-experience.md)'s wrong-account case (§13)~~ — **decided by [ADR-0019](0019-naming-the-account-at-enrolment.md)**: it does, and it persists in sync settings; the scope is `openid email` (`profile` declined) and all three are non-sensitive, so [ADR-0013 §3](0013-the-sync-transport.md)'s *no verification, therefore no server* survives. *"The only thing that would detect"* is **corrected**: it diagnoses, and §13 above is widened accordingly | — |
| ~~Whether the archive should ever be encrypted — must be answered together with the local store, never alone~~ — **discharged** by [ADR-0020 §3 §4](0020-protection-at-rest.md), which answered it together with three others rather than two | — |
| ~~**Whether the 25 MB Auto Backup quota is measured before or after compression.** The platform documentation is silent, and §7's nine-month estimate moves by an order of magnitude on the answer. Surfaced by [ADR-0020](0020-protection-at-rest.md)'s evidence~~ — **measured: BEFORE compression.** §7 stands as written and §6's nine-month estimate is confirmed. A 40 MiB payload compressing 158× (`gzip -9`) and 11,022× (`zstd -19`) was rejected, and the transport named the uncompressed figure in its own pre-flight log line; the framework hands the transport an uncompressed total *before* any data is streamed, so compression cannot participate. **The unit is tar-stream bytes** — on-disk size plus a 512-byte header per file, each file padded to 512 — so §7's *"we know our own file sizes"* arithmetic should carry that small overhead. [Evidence](../research/auto-backup-quota/README.md) | — |
| ~~**Quota failure is silent** — the whole package is rejected, signalled only by a callback needing a dex ADR-0003 does not ship and by two log lines, with no documented user notification. So §7's *"states the size fact"* cannot be driven by the platform telling us~~ — **confirmed by measurement**: no notification was posted by any backup component across the over-quota runs, though the provider owns channels capable of it. §7's decision to state the size fact in the application therefore stands on an observation rather than an absence in the documentation. **One correction rides with it**: the two published log lines are *not* equivalent — *"Transport quota exceeded for package"* is the quota signal, while *"Transport rejected backup of … , skipping"* fires for a generic package rejection and was observed on a **1 KB** payload during the transport's post-failure backoff. Anything greping the log must match the first. [Evidence](../research/auto-backup-quota/README.md) | — |
| Media, if audio on cards is ever built: the archive inherits the size, and §6's manual write becomes a much larger ask | **Out of scope** — [the map](https://github.com/amin-bf/cairn/issues/1) ruled audio out on 2026-07-31; this row is one of the two reasons it recorded for leaving it *de-risked* rather than free |
