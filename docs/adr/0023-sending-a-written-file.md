# ADR-0023: Sending a written file

- **Status**: Accepted
- **Date**: 2026-08-01
- **Resolves**: [Decide: whether the app helps send an exported deck file](https://github.com/amin-bf/leitner/issues/70)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Related**: [ADR-0003 §2](0003-client-stack.md) (manifest plus one `.so`),
  [ADR-0008 §2, §10](0008-the-deck-export-format.md) (the artifact that leaves the machine; the
  extension and the `mimetype` member), [ADR-0009 §4](0009-crate-and-workspace-layout.md) (the
  platform seam), [ADR-0010](0010-leeches.md) (*detect and surface, never intervene*),
  [ADR-0014 §8](0014-when-parameter-optimisation-runs.md) (when divergence is allowed),
  [ADR-0016 §5, §6](0016-backup-and-restore.md) (the three-operation seam; the archive is written
  when the user asks), [ADR-0022 §10](0022-the-import-preview-and-export-report.md) (what the
  application says after an export)
- **Evidence**: [`docs/research/android-outbound-share/`](../research/android-outbound-share/README.md)

## Context

[ADR-0008 §2](0008-the-deck-export-format.md) calls a deck file **"the one artifact that leaves the
machine and arrives with someone who does not have our application"** — and the entire outbound path
stopped at the moment it was written. [ADR-0022 §10](0022-the-import-preview-and-export-report.md)
specifies what the application says after an export, and the answer was a location: the file is in
the documents folder, or in `Downloads`, and the user is on their own from there.

That was never decided. It was an omission with a specific cause, and the cause did not survive
inspection. [ADR-0016 §5](0016-backup-and-restore.md) ruled out a file **picker** because
`ACTION_CREATE_DOCUMENT` and `ACTION_OPEN_DOCUMENT` deliver through an activity **result**, which
needs a Java subclass, which needs a dex that [ADR-0003 §2](0003-client-stack.md)'s *"manifest plus
one `.so`"* does not ship. That same section then **admitted** the Android launch intent, on the
ground that *"a launch intent is readable from the activity with no result callback and no dex —
unlike the picker above."*

A send is in the second category. [ADR-0022](0022-the-import-preview-and-export-report.md)'s handoff
said so and was careful about what it had: *"That is an argument for looking, not a verification;
`AGENTS.md` rule 9 applies."*

**This ADR looked.** Everything below about Android rests on measurement on the handset — a Pixel 8
Pro, Android 17 / API 37 — in the shipped APK shape, and the handset answered a question it had not
been asked, which is §7 and [#72](https://github.com/amin-bf/leitner/issues/72).

## Decision

### 1. The application helps the artifact leave, and the seam gains a fourth operation

> **There is an outbound affordance. It hands the written file to the surface the platform provides
> for handing files onward, and stops there.**

The argument that had to be beaten is **not** either of the priors this looked like it would fail.
[ADR-0016 §5](0016-backup-and-restore.md) sized its seam deliberately — *"put a named file, get a
named file, list the files we recognise. Nothing else"* — and then made the **count itself**
load-bearing: *"Three operations, not four: there is deliberately no delete."* This is a fourth
operation, and platform cheapness is no answer to an architectural argument.

It earns its place on the artifact's own purpose. A deck file that cannot be sent is
[ADR-0008 §2](0008-the-deck-export-format.md)'s central claim left unimplemented, and on Android
[ADR-0022 §10](0022-the-import-preview-and-export-report.md) already concedes the fallback is
unactionable — it prints *"Saved to Downloads"* precisely because there is nothing better to say.
**The seam was sized against the operations backup needed, not against the operations the deck file
needs**, and the deck file was never asked.

**Neither standing prior reaches it, and both were tested rather than waved past.**

- ***Detect and surface, never intervene*** ([ADR-0010](0010-leeches.md), then
  [ADR-0014 §2](0014-when-parameter-optimisation-runs.md),
  [ADR-0015 §1](0015-the-sync-experience.md),
  [ADR-0019 §3](0019-naming-the-account-at-enrolment.md)) governs the application **speaking or
  acting unbidden**. This affordance is user-initiated and makes no claim about anything. Where it
  *would* bite is §5, and §5 refuses that form.
- **The no-third-speaker rule** (`AGENTS.md`, from [ADR-0015 §1](0015-the-sync-experience.md)) is
  scoped to **sync**: *"exactly two things may speak about sync."* This says nothing about sync.

### 2. Both artifacts, and the archive is the stronger case

> **The affordance covers the `.ldeck` deck file and the `.lcoll` collection archive alike.**

Stated rather than inherited, because the ticket was titled for the deck file and silence would have
left the archive to whoever implemented it.

[ADR-0016 §6](0016-backup-and-restore.md) is the argument. It refused an automatic write on the
ground that an archive in the device's own downloads folder *"is **not** a backup until the user
moves it off the device — that folder dies with the phone… the half that matters is irreducibly a
human act"*, and it recorded as **its own strongest objection** that *"a backup nobody makes is not
a backup."* This does not contradict that section: §6 refused an automatic **write**, and this is
not one. It is the same human act with the trip through a file manager removed, and it raises the
floor §6 conceded it could not raise.

**And the measured collision behaviour bites the archive harder than the deck.** A colliding name
dedupes to `collection.lcoll (1)` — suffix after the extension (§7). Re-writing the same name is the
**exception** for decks and the **rule** for archives, so a user who backs up monthly accumulates
files whose names no longer end in `.lcoll`. What that costs is
[#72](https://github.com/amin-bf/leitner/issues/72)'s; that it makes the send worth *more* on the
archive, not less, is this section's.

### 3. Android: the system share sheet

> **`Intent.createChooser(ACTION_SEND, …)`, carrying the written file's `MediaStore` URI as
> `EXTRA_STREAM`, launched with `startActivity` from the context the JNI shim already holds.**

**Measured, not inferred.** The sheet opened —
`com.android.intentresolver/.ChooserActivityLauncher` became the top resumed activity, populated
with real targets — from an APK containing exactly `AndroidManifest.xml`, one
`lib/arm64-v8a/libleitner_app.so`, and its signature. **No `classes.dex`, no `res/`, no new
permission, no new crate.** So [ADR-0003 §2](0003-client-stack.md)'s property, which
[ADR-0009](0009-crate-and-workspace-layout.md) and
[ADR-0014 §3](0014-when-parameter-optimisation-runs.md) both lean on, is not spent here.

**The intent declares `application/vnd.leitner.deck+zip`** (and the archive's type for a `.lcoll`).
A custom `vnd.` type is not a problem for the chooser — it drew the full sheet for exactly that
type. The type **stored** on the `MediaStore` row is a different matter and is
[#72](https://github.com/amin-bf/leitner/issues/72)'s.

### 4. Desktop: the file, revealed and selected in the file manager

> **The written file, shown selected in the system file manager.**

There is no share sheet to call: of the 28 portals present on a full desktop, **there is no
`org.freedesktop.portal.Share`**. So the desktop's equivalent is not a smaller share sheet, it is a
different surface — and the file manager is where a desktop hands a file onward, by dragging it into
a mail client or a chat window.

**This is a reversal of the first answer this ADR reached, and the reasoning that failed is worth
recording** so it is not reached again. The refusal was *"opening a folder is not sending."* By that
standard **the share sheet is not sending either** — both hand the file to another application and
stop. The right question is which surface each operating system gives for handing a file onward, and
answering it platform by platform is what makes the two arms one decision instead of a divergence.

The second failed argument was [ADR-0022 §10](0022-the-import-preview-and-export-report.md)'s *"a
desktop path is a thing the user can act on."* True, and it does less work than it looks: acting on
it means selecting text and navigating a file manager to it by hand.

**The premise for divergence is met**, which is what
[ADR-0014 §8](0014-when-parameter-optimisation-runs.md) demands before platforms are allowed to
differ — it refused *"softer divergence"* where the premise was refuted by measurement, and here
both halves are measured. It is the mirror of [ADR-0016 §5](0016-backup-and-restore.md) admitting
desktop drag-and-drop **inbound** as additive; a file sitting selected in a file manager is exactly
what makes the outbound drag possible.

**Mechanism, in preference order, and neither costs a dependency this workspace does not have:**

| | Interface | Note |
|---|---|---|
| Preferred | `org.freedesktop.FileManager1.ShowItems` | Takes **URIs**, so it needs no file-descriptor passing. D-Bus-activatable |
| Portal | `org.freedesktop.portal.OpenURI.OpenDirectory` (v5) | Takes an **fd**, so no path crosses the boundary |
| Fallback | `xdg-open` on the containing directory | Opens the folder **without selecting the file** |

**The fallback's degradation is not stated to the user.** It is invisible unless you already knew,
and a sentence explaining it would cost more attention than the difference is worth.

**Rejected: `org.freedesktop.portal.Email`.** It exists and it would attach the file to a new
message — by **choosing one channel on the user's behalf**, which is the opposite of what a share
sheet does. Worse than the status quo, not better.

### 5. It never fires by itself

> **The affordance is an action the user takes. Nothing opens on its own when an export finishes.**

This is the one form of the feature where *detect and surface, never intervene* genuinely bites. A
chooser or a file manager that appears unasked **takes the screen**, and it would be the first thing
in this specification to act unbidden.

Two further grounds, so this does not rest on the prior alone. **Export has honest non-send uses** —
archiving a deck, or moving it off later by cable. And for the `.lcoll` archive, auto-firing would
quietly undo [ADR-0016 §6](0016-backup-and-restore.md): that section refused to automate the write
because the off-device move is *"irreducibly a human act"*, and firing the sheet automatically
automates precisely the step it called irreducible. **One tap is the improvement; zero taps is the
application deciding.**

### 6. What the export report says now

[ADR-0022 §10](0022-the-import-preview-and-export-report.md)'s handoff anticipated that *"if an
outbound share affordance is adopted, §10's location line is what it replaces."*

> **It is not replaced. Both lines stand, and the report gains an action beside them.**

**Because the affordance is declinable.** Backing out of the chooser is one gesture, and the file
exists either way — so a report that spent the location on a send button would leave everyone who
declines not knowing where their file went. §10's stated job is untouched by this ADR: *"the user
chose neither the name nor the location, and the application is the only thing that knows either."*

If anything §7 strengthens it. The name §10 reads back may genuinely be `French A1.ldeck (1)`, so
the line reports something the user could not otherwise learn.

### 7. Facts the implementation must not rediscover

Each of these is a **silent** failure — it produces no error, or an error at the wrong moment.

1. **The context is `android.app.Application`, not the Activity.** `instanceof android.app.Activity`
   is **false** for the handle `ndk_context::android_context().context()` returns — the same handle
   `leitner-store::platform::android` already uses. So **`FLAG_ACTIVITY_NEW_TASK` is mandatory**:
   without it `startActivity` throws `AndroidRuntimeException: Calling startActivity() from outside
   of an Activity context…`, through `ContextImpl` rather than `Activity`.
2. **The flags go on the chooser, not on the intent being chosen.** `Intent.createChooser` returns a
   **fresh** `Intent` and inherits neither `FLAG_ACTIVITY_NEW_TASK` nor
   `FLAG_GRANT_READ_URI_PERMISSION`. Setting the grant only on the inner intent fails *after* the
   user has picked an application — the worst available moment.
3. **We cannot enumerate the handlers, and must not try.** `queryIntentActivities` returned **1**
   against a sheet showing five applications, because Android 11+ package visibility hides them —
   `AppsFilter` logs `BLOCKED` for the mail, drive, files and browser packages. Counting or naming
   targets would need a `<queries>` element in the manifest. **Nothing here needs one**: the chooser
   is a system component and is not filtered.
4. **The chooser's own file-preview line shows the `MediaStore` row id**, not the display name, for a
   URI whose stored type is `application/octet-stream`. A recipient querying the URI resolves the
   real name correctly, so this is cosmetic — recorded because a bare number where a filename belongs
   looks like our defect and will be reported as one.

## Amendments to accepted ADRs

### [ADR-0016 §5](0016-backup-and-restore.md) — the seam is four operations, and its Android put is verified

**The seam gains `hand_off`.** `leitner-export::platform` becomes **put, get, list, hand_off**, under
the same three-`#[cfg]`-arm discipline with the third a `compile_error!`.
**`leitner-store::platform` still keeps exactly two functions**, so
[ADR-0009 §4](0009-crate-and-workspace-layout.md)'s erosion signal is preserved intact and still
means what it says.

**The name is `hand_off` rather than `send`** for the reason §2 of that ADR gives about the
`progress` profile: *a name describing half a payload is how the wrong selection rule gets
implemented*. `send` is accurate on Android and false on the desktop, where nothing is sent — and an
implementer reading `send` on the desktop arm will reach for
[the mail portal §4 rejects](#4-desktop-the-file-revealed-and-selected-in-the-file-manager).

**And §5's unverified Android put is verified.** That section carried *"It is not verified on the
handset — see Open items, and `AGENTS.md` rule 9"* against its `MediaStore` insert. Insert returned a
URI, `openOutputStream` accepted the bytes, and a read-back reported them. **No permission was
requested and none was needed**, at API 37, exactly as §5 predicted. That open item is discharged.

### [ADR-0022 §10](0022-the-import-preview-and-export-report.md) — the report gains an action, and its collision row is discharged

§6 above amends §10: both lines stand and an action joins them.

**Its *Open items* row *"`MediaStore` collision behaviour on the handset — whether a colliding
display name overwrites, dedupes or fails"* is answered: it dedupes.** §10 was written to be correct
under all three outcomes and it is, so nothing there needs changing. **The guess inside it was wrong
in an instructive direction** — it wrote the deduped example as `French A1 (1).ldeck`, and the
handset produces **`French A1.ldeck (1)`**, suffix after the extension, verified on disk. §10's
decision to state *"the name the platform actually wrote, never the name requested"* is what makes
that knowable at all, and it is vindicated. What the mangled name costs is
[#72](https://github.com/amin-bf/leitner/issues/72)'s.

> **Corrected by [ADR-0024 §4](0024-identifying-a-written-file.md): the suffix-after-extension
> behaviour is conditional, not unconditional.** It happens only when the insert declares a media
> type that **disagrees with the name** — as the probe here did, asking for
> `application/vnd.leitner.deck+zip` on a `.ldeck`. Measured across three declarations: with no type,
> or with the agreeing `application/octet-stream`, the same collision yields
> `probe72-x (3).ldeck` — **the extension survives**. ADR-0024 §4 therefore has the Android write
> declare no `mime_type` at all, and the deduped name becomes `French A1 (1).ldeck` after all. **The
> original guess was right and the measurement that appeared to refute it was reading our own
> declaration back.** Everything else here stands, including that reading the name back is what makes
> any of it knowable.

## Requirements this places on downstream tickets

### [#72 — how a written file is identified again](https://github.com/amin-bf/leitner/issues/72)

1. **This ADR fixes the outbound intent's declared type and nothing about the stored type.** The
   chooser accepts `application/vnd.leitner.deck+zip`; the `MediaStore` row does not keep it. Those
   are different surfaces and only the first is settled here.
2. **`hand_off` is specified against the URI the write returned**, so whatever #72 decides about
   naming or re-insertion changes what is handed off, not how.

## Consequences

- **A fourth seam operation, and the count is no longer the invariant.**
  [ADR-0016 §5](0016-backup-and-restore.md)'s *"three operations, not four"* was an argument about
  **delete**, which remains absent for its own reasons. The invariant that survives is
  *opaque, minimal, enumerable* — not the number three. Anything reading the count as the rule will
  now read it wrong.
- **The two platform arms do different things, and that is the decision rather than a compromise.**
  Android sends; the desktop reveals. A future reader finding this asymmetric should re-read §4
  before "fixing" it — symmetry here would mean picking a mail client for the user.
- **The desktop arm degrades silently where no file manager is on the bus.** `xdg-open` opens the
  folder without selecting the file, and §4 deliberately says nothing about it.
- **`leitner-export` gains no dependency.** Both preferred desktop routes take URIs or file
  descriptors over D-Bus, and the Android route is one more call through the JNI shim that
  [ADR-0007 §6](0007-the-local-store.md) already established.
- **`AGENTS.md` client-stack rule 8 is not touched.** Nothing here types text.
- **The one thing this does not do is send.** Both arms hand off and stop; no arm confirms delivery,
  and the application never learns whether the user completed the share or backed out of it. There
  is deliberately no report afterwards — that would be a claim about something we cannot observe,
  which is [ADR-0015 §1](0015-the-sync-experience.md)'s reason for having no *"in sync"* state.

## Open items handed onward

| Item | Owner |
|---|---|
| **What identifies a written file when Android keeps neither our media type nor our extension** — measured here, decided there | [#72 — how a written file is identified again](https://github.com/amin-bf/leitner/issues/72) |
| ~~**Whether a recipient can read the bytes.**~~ The grant is set correctly and metadata resolves, but completing a share into a real application would have sent a file to a real contact from the owner's own accounts, so it was deliberately not done | **Discharged** — see below |
| **The API 24–28 path**, narrowed from *"below API 29"*. `MediaStore.Downloads` and the permission-free insert are API 29+; `min_sdk_version` is 24. **29 itself is now measured** and behaves as §7 and [ADR-0024 §4](0024-identifying-a-written-file.md) describe, so the gap is 24–28 exactly. Inherited from [ADR-0016 §5](0016-backup-and-restore.md) rather than created here | Implementation |
| Visual treatment of the affordance — where the action sits and what it is labelled | **Out of scope** — *the visual design pass*, ruled out by [the map](https://github.com/amin-bf/leitner/issues/1) on 2026-07-31 |

### The recipient read, discharged

> **The bytes arrive.** A `.ldeck` handed off from the handset reached a **second device that has
> never held this application**, byte for byte — same SHA-256, same four members, `mimetype` still
> first and uncompressed
> ([#98](https://github.com/amin-bf/leitner/issues/98)).

**What blocked it was the recipient, not the measurement.** This ADR did not decline to look; it
declined to *send a file to a person* from the owner's own accounts to find out. A second handset the
owner controls removes that reason entirely, and nothing else about the item changed.

**It is a different claim from the one already recorded, which is why it was worth the trip.** §7's
metadata resolution proves a recipient can *read the URI*; it does not prove the stream behind it
opens. The receiving device stated the exact byte count in its accept prompt **before** the transfer
— the sender having read the stream through `FLAG_GRANT_READ_URI_PERMISSION` — and the file that
landed hashed identically. So the grant is not merely accepted, it is honoured.

**And §7's fourth fact stops being an inference.** It reasoned that the chooser previewing a bare
`MediaStore` row id is cosmetic *because* "a recipient querying the URI resolves the real name
correctly". The display name travelled with the file, dedupe suffix and all. That was the one step in
the argument nobody had watched happen.

This is also what turns [ADR-0008 §2](0008-the-deck-export-format.md)'s *"the one artifact that leaves
the machine and arrives with someone who does not have our application"* from the premise this crate
was built on into something the shipped path has done.
