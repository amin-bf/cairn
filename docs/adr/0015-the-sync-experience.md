# ADR-0015: The sync experience

- **Status**: Accepted
- **Date**: 2026-07-31
- **Resolves**: [Decide: the sync experience](https://github.com/amin-bf/leitner/issues/40)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Related**: [ADR-0013](0013-the-sync-transport.md) (the transport, and the four requirements it
  placed here), [ADR-0014](0014-when-parameter-optimisation-runs.md) (the mid-session queue shift,
  handed here whole), [ADR-0004](0004-the-review-event-log.md) (row identity, device labels, the
  clock-skew residual), [ADR-0003](0003-client-stack.md) (the Gradle-free APK, no Android IME),
  [ADR-0006](0006-the-review-session-experience.md) (the session, derived position),
  [ADR-0009](0009-crate-and-workspace-layout.md) (the platform seam),
  [ADR-0010](0010-leeches.md) (detect and surface, never intervene),
  [ADR-0008](0008-the-deck-export-format.md) (the Android intent filter for a deck file),
  [ADR-0016](0016-backup-and-restore.md) (collection identity and its check; landed just after this
  ADR and answered both of its handoffs — see the *Requirements* section)

## Context

[ADR-0013](0013-the-sync-transport.md) settled the **mechanism** — a key-value namespace on a
personal cloud drive's application data folder, one writer per namespace, nothing ever rewritten,
the listing as the version summary — and deliberately left everything the user sees to this ticket,
naming four requirements. [ADR-0014](0014-when-parameter-optimisation-runs.md) added a fifth, and
called it structurally unfixable where it stood.

Two things about this decision are worth stating before the decisions themselves.

**Almost nothing here is a data problem.** [ADR-0004 §2](0004-the-review-event-log.md) makes merging
set union on `(writer, sequence)`; there is no conflict resolution because two rows with the same
pair are the same row. Everything below is therefore about what a person is told and when — which is
why it took a conversation rather than a measurement.

**Two of the ticket's questions dissolved rather than resolving**, and both dissolutions are load
bearing. The ticket asked how a device presents *"you are behind"*; §4 finds that state is transient
by construction and the app is never entitled to rest in it. And [ADR-0013](0013-the-sync-transport.md)'s
handoff asked how long a handset left alone goes between background syncs; §2 finds there are none,
so the question has no subject.

## Decision

### 1. Divergence is accepted, always, and nothing is ever locked

> **There is no state in which the application declines to let the user review.**

The alternative carried on the map since charting was pessimistic single-writer: sync on open and on
close with the UI locked while it runs, so two devices never diverge. It loses on four counts, in
descending strength.

**A lock needs a lock-holder, and this transport structurally cannot be one.**
[ADR-0013 §1](0013-the-sync-transport.md) is four operations wide — put, get, list, delete — with no
locking, no transaction and no server-side computation of any kind. A lock is a *shared* key, and
[§1](0013-the-sync-transport.md)'s single-author-per-key property is what removes every need for a
conditional write. Introducing one key with two writers would reopen, for a UI nicety, the exact
hazard #33 measured: two of three servers returning `201 Created` while silently ignoring the
precondition.

**An offline device cannot be locked at all**, so the guarantee is not merely weaker than it looks —
it is unenforceable. The map fixes offline-by-default as the operating mode rather than a feature,
which makes the unenforceable case the *normal* one.

**There is no conflict to prevent.** Two devices reviewing simultaneously produce a union, not a
collision.

**And it trades a non-problem for a real one.** A user in a drawer, on a plane, or with a broken
grant would be blocked from the application's only function.

**The accepted cost, stated rather than glossed: two diverged devices can review the same card on the
same day, and replay sees both.** [ADR-0001 §5](0001-scheduling-algorithm-and-grade-scale.md) already
makes same-session re-shows real logged rows with `delta_t = 0`, so the second grading is absorbed as
a re-show rather than corrupting anything. It perturbs one card's stability slightly and later
reviews pull it back — **bounded and self-limiting**, the same shape as
[ADR-0004 §8](0004-the-review-event-log.md)'s clock-skew residual, and a dividend of the same
replay-not-migrate property that created the exposure.

This also settles the ticket's *plane case* outright. There is nothing left of it: you cannot sync
before use, so you diverge, and diverging is cheap.

### 2. Sync runs on three foreground triggers, and there is no background sync at all

[ADR-0013](0013-the-sync-transport.md)'s first requirement gave this ticket the promise *"sync when
the user opens the app, and opportunistically before that,"* bounded by Android listing network as
**Disabled** in the Rare and Restricted app-standby buckets.

**The ceiling is one step earlier than that, and it is ours rather than the platform's.** A scheduled
job or a foreground service needs Java, a `classes.dex` and a Gradle project — precisely what
[ADR-0003](0003-client-stack.md) measured as its prize (an APK that is a manifest plus one `.so`,
against 44 committed generated files for each alternative stack) and what
[ADR-0014 §3](0014-when-parameter-optimisation-runs.md) already refused to spend on a 4.3 s job. The
standby-bucket ceiling would only bite if we had a scheduler to be throttled. We do not, and we have
now twice decided not to build one. **So the *"opportunistically before that"* half of the promise is
not ours to make.**

That is not a loss worth reopening, on one argument: **a device that is not being used has nothing to
publish, and nobody is waiting to read from it.** Background sync's entire value is a device that is
*in use but not open*, which is not a state this application has. The drawer phone's user opens the
app when they return to it; its local log was complete the whole time.

> **Three triggers, and nothing else: the app becoming active, session end, and explicit user
> action.**

1. **The app becomes active** — launch, or the window or app regaining focus — subject to a
   **recency floor** so that alt-tabbing does not hammer the remote. This is the trigger that
   matters, because it puts the queue in front of the user already current. The floor is a
   *debounce*, not a schedule; [ADR-0014 §7](0014-when-parameter-optimisation-runs.md) already
   sanctions the notion, allowing its leading sync to be skipped when one is already recent.
2. **Session end** — the first moment the local log holds something worth publishing.
3. **Explicit user action** — a control in sync settings, and
   [ADR-0014 §7](0014-when-parameter-optimisation-runs.md)'s leading step of Optimise.

**Rejected: a timer while the app is open.** A clock in a codebase that has spent real effort keeping
clocks out — [ADR-0004 §4](0004-the-review-event-log.md) freezes day numbers at write time precisely
so nothing recomputes them — for marginal freshness. *Focus regained* covers the desktop
sit-open-all-day case with a user-caused trigger and no schedule.

**Rejected: publishing per graded card.** It multiplies exactly the object count
[ADR-0013 §5](0013-the-sync-transport.md) exists to bound, for a latency nobody is waiting on.

**Rejected: sync on close.** It is *safe* — an interrupted publish means the next one covers an
overlapping range, and [ADR-0013 §5](0013-the-sync-transport.md)'s rule 1 makes overlap free because
merge is set union — but on Android "close" means **frozen**
([`AGENTS.md`](../../AGENTS.md) client-stack rule 10), so it may simply not complete, and it adds
nothing over session end.

**Rejected: a session-start trigger.** It puts a network round trip between the user's intent and
their first card, and trigger 1 already covers it in the overwhelming case.

**The consequence, recorded so it is not mistaken for an oversight**: a user who reviews and then
force-quits without ending a session publishes on next launch instead. That is why the answer is not
"on every write".

### 3. The promise is "your devices catch up when you open the app"

> **Never "automatic", never "always in sync", never "in the background".**

This is a rule rather than a preference, because it is the kind of copy written later by someone who
was not in this conversation. It appears on the enrolment screen and in sync settings, and nowhere
else — a promise repeated in tooltips is a promise nobody reads.

### 4. The resting surface states a fact, never a claim — and "you are behind" dissolves

The ticket asked how a device presents *"you are behind"*, now that
[ADR-0013 §6](0013-the-sync-transport.md) prices the answer at one listing and one round trip.
**The application can never honestly rest in that state**:

- If it can reach the remote, the handshake *is* the listing, so discovering it is behind and fixing
  it are one operation separated by seconds. **"Behind" is a progress state, not a warning.**
- If it cannot reach the remote, **it does not know** whether it is behind. Nothing has told it.

So the sentence is one the app is either about to make untrue, or is not entitled to say.

**"In sync" is a claim the app cannot back, and this is constraint 4's shape on a new surface.** After
a successful sync it knows every writer's highest *published* sequence and that it holds all of them
— that much is real. It does **not** know whether another device has reviewed since it last
published, and it never can. A green checkmark would be read as *"all my devices agree"*, which is
unknowable, exactly as a box badge would be read as a claim about the queue.

> **The resting surface is one line in sync settings: "Last caught up ⟨when⟩." No checkmark, no "in
> sync", no "up to date", and no persistent status indicator anywhere in the chrome.**

A sync in flight shows transient, non-blocking progress where it was triggered, and gates nothing
(§1). This follows [ADR-0010 §9](0010-leeches.md)'s leech pointer and
[ADR-0014 §2](0014-when-parameter-optimisation-runs.md)'s optimisation nudge, each surfaced in
exactly one place.

**Accepted cost**: a user whose sync is merely offline for a month gets no signal, and *"Last caught
up 4 weeks ago"* sits in settings where they are not looking. They are reviewing fine, their log is
complete, and there is nothing for them to do — but this is a deliberate choice to let a long silence
pass unremarked, not an omission.

### 5. Silence on network failure, speech on auth failure — and only two things ever speak

**The rule keys on the *kind* of failure, not its duration**, and the application can tell them
apart: an expired grant is a specific response, not a timeout.

| | Surface |
|---|---|
| **Network failure** — offline, timeout, DNS | **Nothing, ever.** Offline is normal and must never nag. |
| **Auth failure** — the grant died | A **persistent, non-modal notice**. This is the one sync state the user must act on. |

**No threshold and no failure counter anywhere in this**, for
[ADR-0014 §2](0014-when-parameter-optimisation-runs.md)'s reason: there is no defensible number, and
inventing one is worse than keying on something real.

Exactly two things are permitted to use that notice channel: the dead grant above, and
[ADR-0004 §8](0004-the-review-event-log.md)'s clock-skew warning (§11). Anything else that wants to
speak about sync is a defect against §4.

### 6. Sync never starts while the review screen is up

[ADR-0014](0014-when-parameter-optimisation-runs.md)'s first requirement handed this over whole and
called it locally unfixable: [ADR-0006 §2](0006-the-review-session-experience.md) derives session
state from the log every frame, so a new parameter vector recomputes every `(S, D)` mid-session, and
blocking review during an optimisation run cannot help because **a merge landing another device's
vector is the identical event with no local trigger to gate on.**

That reasoning was correct and was written before §2 existed. **§2's foreground-only triggers supply a
local trigger after all** — not a gate on review, which §1 forbids, but a gate on *sync*:

> **We never start a sync while the review screen is up. A sync already in flight finishes normally.**

This costs nothing. The user was already made current by the becoming-active sync *before* the
session started, so what is deferred is a *re*-sync; and
[ADR-0006 §1](0006-the-review-session-experience.md) bounds a session at a chosen count with a
ten-minute checkpoint, after which §2's session-end trigger fires immediately.

The exposure then collapses to cases the user causes:

- **Leaving the session, pressing Sync or Optimise, and returning** — possible because
  [ADR-0006 §2](0006-the-review-session-experience.md) stores no session position.
  [ADR-0014 §4](0014-when-parameter-optimisation-runs.md)'s completion message already states that
  every due date moved. Their action, their answer.
- **An in-flight sync landing in the first seconds of a session.** Accepted rather than fixed:
  holding fetched rows unapplied makes the local log and what we have downloaded disagree, which is
  real machinery for a window a few cards wide. **Only the *starting* of a sync is suppressed.**

**Deliberately not suppressed: a sync landing while the count picker is up**, changing the cap that
[ADR-0006 §1](0006-the-review-session-experience.md) derives from what is due. The user has committed
to nothing, and picking from the true number is what syncing before a session is *for*.

**The knock-on is worth recording because it inverts how §2 reads.** This answer exists only because
§2 chose foreground-only triggers. Background sync would have made the deferral unenforceable, so a
decision taken on packaging grounds turns out to buy the session's stability — and anyone later
reading "no background sync" as a limitation to be lifted would not see that it is load-bearing.

### 7. Enrolment is opt-in, lives in settings, and is surfaced by the empty state

[ADR-0013 §8](0013-the-sync-transport.md) fixed the device flow and that sync never blocks first use.

**No first-run prompt.** Someone installing a flashcard application should not meet an authorisation
flow before their first card.

**But burying it fails the second-device user, whose entire reason for installing is sync.** So it is
surfaced in the **empty-collection state**, in [ADR-0006](0006-the-review-session-experience.md)'s
style of explicit worded states: *"Nothing here yet — create a deck, import one, or set up sync to
bring in another device's collection."* A worded empty state, not a wizard.

**The screen states the scope in plain words.** Granting drive access is the friction point and the
true answer is unusually good: `drive.appdata` reaches a hidden folder only this application can see,
not the user's files. **Only `drive.appdata` is requested** — not `email` — so the consent screen asks
for exactly one thing.

> **Amended by [ADR-0019 §4](0019-naming-the-account-at-enrolment.md): the requested set is `openid
> email drive.appdata`, so the consent screen asks for two things.** `profile` is declined — display
> name and picture have no diagnostic value and are exactly the ambient identity
> [ADR-0016 §11](0016-backup-and-restore.md) keeps out. `openid` is required before `email` may be
> included. The plain-words rule above is unchanged and now covers both: *"see your email address"* is
> a claim a user can evaluate. **The property this section's narrow scope was protecting survives** —
> all three scopes are non-sensitive, so verification remains not mandatory and
> [ADR-0013 §3](0013-the-sync-transport.md)'s *no verification-time endpoint, therefore no server*
> still holds.

**After enrolment, the application states what it found.** This is the most load-bearing part of §7,
because **enrolling a second device against the wrong account is undetectable**:
[ADR-0013 §10](0013-the-sync-transport.md) scopes the folder per (account, application), so a wrong
account presents an **empty folder — identical to being the first device to enrol.** No collection
identity fixes this; an empty folder carries no information. The only defence is a fact stated at the
one moment the user could notice:

> *"This is the first device here"* — versus — *"Found 2 other devices: Laptop, Pixel."*

A user who expected to join an existing collection and reads the first sentence has caught it.
Detect and surface, in [ADR-0010](0010-leeches.md)'s shape.

> **Amended by [ADR-0019 §1](0019-naming-the-account-at-enrolment.md): each sentence is prefixed with
> the account.** *"Connected as `you@example.com`. This is the first device here."* The address is also
> kept in §12's settings screen — **not shown once and discarded**, because the failure is discovered
> months later, when a second device disagrees, and a once-only message is gone by then.

**[ADR-0016 §13](0016-backup-and-restore.md) landed after this ADR and answered the handoff below
`no`, with a sharper reason than the one above** — worth recording, because it explains why no
mechanism of this shape could ever have worked. **In a wrong-account enrolment every collection id
agrees.** The devices on the right account and the device on the wrong one hold the *same*
collection; they simply cannot see each other. **The failure is one of reachability, not identity**,
so an identity check was never going to catch it. The sentence therefore stands as the whole defence,
and the one thing that *would* detect it is naming the **account** on the enrolment screen — which
costs the `email` or `profile` scope this section deliberately declines. That trade is live and owned
by neither ADR; it is in *Open items* rather than reversed here.

> **Amended by [ADR-0019 §2](0019-naming-the-account-at-enrolment.md), which took the trade — and
> corrected this paragraph's premise while doing so.** Naming the account is **not** *"the one thing
> that would detect it"*: the sentence above already detects it, in both cases that occur — a second
> device told it is the first, and a re-enrolment after
> [ADR-0013 §3](0013-the-sync-transport.md)'s 7-month token death told the same. **What the sentence
> cannot do is diagnose.** The user must infer *"wrong account"* from *"first device here"*, and every
> competing hypothesis — folder cleared, other device reset, sync broken — is more intuitive and
> **routes to a repair action that cannot possibly work**, so each failed attempt raises their
> confidence that the collection is gone. The account name is bought for **diagnosis**; detection was
> already paid for. This sentence therefore stands, but is no longer the *whole* defence.

**Enrolment also runs [ADR-0016 §10](0016-backup-and-restore.md)'s identity check**, and both
outcomes are moments this ADR owns:

- **An empty collection adopts the id it meets, silently.** A brand-new install has already minted
  one, so a plain *"ids differ → refuse"* would stop a fresh device ever joining an existing account —
  the commonest path there is.
- **A non-empty collection refuses any id but its own, and the refusal must name the mismatch *and*
  state the way out** — archive, clear data, restore, enrol. A refusal that only says no leaves the
  user holding a device that will not sync, which is the failure this check exists to prevent turned
  into a different one.

This is the exception to §5's *only two things speak* rule and does not widen it: it is not a resting
notice but the immediate result of an action the user just took, in the flow they took it in.

**The collection id is published as one small immutable object per writer prefix**
([ADR-0016 §3](0016-backup-and-restore.md)), under
[ADR-0013 §4](0013-the-sync-transport.md)'s never-rewritten rule. **It is not on the mutable surface
and must never be made to settle** — an id that settles is an id that can change, which is the one
thing it exists not to do.

**The words.** [`crates/sync/src/CONTEXT.md`](../../crates/sync/src/CONTEXT.md) already rules out
*login*, *sign-in* and *pairing* — there is no account of ours and no device-to-device step. The
action is **"Set up sync"**. The provider is named, because the user must recognise which account
they are choosing; that is the interface requiring it, not a breach of the name-the-fact rule, which
governs prose leaning on a product instead of explaining itself.

### 8. Devices label themselves at enrolment

[ADR-0004 §3](0004-the-review-event-log.md) says *"a device meeting an unfamiliar writer asks what to
call it."* **Asking a user to name a stranger is a puzzle they cannot solve.** Asking them to name
**the device in their hand** is trivial — and because labels ride
[ADR-0004 §7](0004-the-review-event-log.md)'s mutable surface, every other device then already knows
it.

> **Each device is labelled by its user at enrolment, and the label syncs.**

§3's ask-about-a-stranger path narrows to a writer whose label never arrived, and to §3's reinstall
case, where typing the same name is exactly what groups several writer ids under one label.

**No default derived from hostname or device model.** That would be a platform function, and
[ADR-0009 §4](0009-crate-and-workspace-layout.md) calls a third function in the seam the tell that it
is eroding. The user types it, once per device.

**Labels needed a new justification and have one.** §4 removed the reason
[ADR-0004 §3](0004-the-review-event-log.md) gave for their existence — *"'you are behind your laptop'
is a sentence; 'you are behind `7f3a-b21c`' is not"* — and that sentence no longer gets said. They
survive on two uses: the device list in sync settings (§12), and **§11's clock-skew warning, which
must name the offending device.**

### 9. The Android text-input limitation is stated in advance, at the point of failure

[`AGENTS.md`](../../AGENTS.md) client-stack rule 8 records that winit's Android backend has no IME
path, so composed text never reaches the application. **The failure mode is the problem, not the
limitation**: a user switches to a non-Latin keyboard, types, and **nothing happens at all.** We
receive no events, so we cannot detect the attempt and cannot report it when it occurs. It can only
be stated **in advance**, or it reads as a bug.

**One correction to the ticket's premise.** It holds that sync is the only route for non-Latin
content and that *"there is no second route and no fallback."* That is true of **authoring** and
false of **receiving**: [ADR-0008 §10](0008-the-deck-export-format.md) specifies an Android intent
filter matching a `pathPattern` for the extension, so a deck file opens from a file manager or a mail
attachment. An Android-only user with non-Latin content is locked out of writing their own, not out
of the application.

Stating it where it bites means the Android editor says something the desktop editor does not —
**platform-varying UI**, which collides with [`AGENTS.md`](../../AGENTS.md) client-stack rule 3.
[ADR-0012](0012-the-note-authoring-experience.md)'s phone layout dodges this by keying on width, but
*"can this device type Persian"* is not a width property.

> **A compile-time capability constant, and the note editor varies on it. The Android editor carries
> a standing quiet line: this device types Latin text only; author other scripts on the desktop and
> they sync here.**

The argument is one this repository has already won once.
[ADR-0003](0003-client-stack.md) chose its whole stack partly because *"a `#[cfg]` the compiler checks
beats a runtime `if` nobody checks."* Rule 3 exists to stop **behaviour** diverging silently between
platforms; a constant whose only job is to make a limitation **visible** is the inverse of what it
guards. §15 amends [ADR-0009 §4](0009-crate-and-workspace-layout.md) to say so explicitly rather than
leaving the next agent to weigh a rule against its purpose.

**Rejected: stating it only where it is true on both platforms** (sync settings and enrolment). Zero
platform code, and it leaves the silent-nothing failure fully intact — the Android user meets the
explanation late or never.

**Rejected: showing the line on both platforms.** No `cfg`, and on desktop it is a permanent
statement about a limitation the reader does not have.

**Nothing about sync changes for this user**, and they are the strongest case against §4's silence:
the desktop-authors / phone-reviews user depends on sync more than anyone, and if it fails on the
network for a month, new cards stop arriving with the application saying nothing. §4 holds anyway —
they notice the absence, settings holds the answer, and adding a nag for one user class breaks a rule
that is right for everyone.

### 10. Disconnect is the only control; revocation is explained, not owned; nothing is deleted

Three things a user might mean by "stop syncing", with different blast radii.

**Disconnect drops the local grant and stops syncing. It deletes nothing and revokes nothing.** It is
the only one of the three the application can do cleanly, and it is reversible: re-enrol and the
device resumes under its existing writer id, since
[ADR-0004 §3](0004-the-review-event-log.md) ties that id to the install rather than to the grant.

**Revocation is not ours to offer, and its shape must be met before it is needed.**
[ADR-0013 §9](0013-the-sync-transport.md): every device holds a token issued against one client id,
so the account's security page lists this application once, with one button — **revoking for a lost
phone logs out every device the user owns.** That is genuinely surprising, and the moment to learn it
is not while replacing a stolen phone. Sync settings states it plainly and points at where it is
done, rather than pretending to a control we do not have.

**There is no "delete my synced data" control**, and this is a deliberate refusal:

- The `drive.appdata` grant reaches the **whole** application folder, so a delete from one device
  destroys **other writers' namespaces**. Every device holds the whole log — but only the rows it has
  already fetched. A device that never fetched writer `A`'s rows loses them permanently if `A` is
  also gone.
- Its only conceivable benefit is reclaiming space, and there is none to reclaim:
  [ADR-0013 §5](0013-the-sync-transport.md) bounds live objects at a few hundred per collection, and
  #33 measured **47.5 MB per decade**.
- The capability already exists on the user's side, and offering it here invites tidying up something
  [ADR-0013 §2](0013-the-sync-transport.md) made disposable on purpose.

**What we give instead is a name and a navigation path — not a folder path, because there is not
one.** `appDataFolder` is hidden ([ADR-0013 §3](0013-the-sync-transport.md)), so it does not appear in
the user's file view and cannot be navigated to. What exists is the drive's **connected-applications
settings**, which carries a delete-hidden-application-data action per application. Sync settings names
that route and **the name we appear under**.

**That name is a fourth console trap** in [ADR-0013 §3](0013-the-sync-transport.md)'s series: it is
the consent screen's application name, a setting no code path in this repository can validate. If it
does not match what the user knows the application as, *"find it in the list"* fails with nothing
anyone can act on. The exact menu wording is a third party's UI, expected to drift — **verified at
implementation, never pinned in this document.**

The tension, stated rather than buried: a privacy-minded user asking *"delete everything you have put
in my drive"* is told to do it themselves. That is correct — it is their storage, the folder's
documented purpose is to hold our data, and [ADR-0013 §2](0013-the-sync-transport.md) means nothing
there is precious — but it is a refusal of a reasonable-sounding feature, not an oversight.

### 11. The clock-skew warning names the device, and never offers the repair inline

[ADR-0004 §8](0004-the-review-event-log.md) shipped *"guard on write, detect on merge and warn
without blocking, collection-wide cutoff as the only repair"* and never assigned the warning a
surface. It is a merge-time event, so it is this document's.

> **It uses §5's persistent non-modal channel, names the device by its §8 label, states the fact, and
> does not offer the cutoff.**

The repair discards good history alongside bad — [ADR-0004 §8](0004-the-review-event-log.md)'s
accepted residual in full — so it lives in sync settings behind an explanation of what it costs, and
never one tap from a notice the user met by surprise. **Detect and surface, never intervene**: the
third instance after [ADR-0010](0010-leeches.md) and
[ADR-0014](0014-when-parameter-optimisation-runs.md).

**It is dismissible and keyed to the rows that triggered it**, so it does not re-fire on every
subsequent merge of a log that still contains them. A warning that returns forever is one the user
learns to ignore, which is the failure mode this whole document has been avoiding.

### 12. What the sync settings screen holds

One screen, and everything sync-shaped is on it:

| | Source |
|---|---|
| **"Last caught up ⟨when⟩"** | §4 |
| **The connected account address** — *"Connected as `you@example.com`"* | [ADR-0019 §1](0019-naming-the-account-at-enrolment.md); a standing fact, and the only cross-device comparison available to a human |
| **Sync now** | §2's third trigger |
| **The device list** — labels grouped per [ADR-0004 §3](0004-the-review-event-log.md), several writer ids under one label, each with "last published ⟨when⟩" | §8; read straight off [ADR-0013 §6](0013-the-sync-transport.md)'s listing, so it costs no extra request |
| **Disconnect** | §10 |
| **How revocation works, and where** | §10 |
| **The history cutoff**, behind its explanation | §11 |
| **The desktop-authoring statement** | §9 |

### 13. Cold start is a load, not a lock

A freshly enrolled device fetching everything is the one genuinely long operation in the feature, and
it does not violate §1. **On a fresh device there is nothing to review until it lands**, so a progress
screen is the honest state rather than a block — nothing is being withheld. **On a device that
already held content and then enrolled**, review continues and rows land as they arrive, which is
§1's accepted divergence.

[ADR-0013 §5](0013-the-sync-transport.md) is what makes this bounded: a few hundred objects rather
than the ~43,800 that one-object-per-sync would have produced over a decade.

**Roll-up remains invisible**, discharging [ADR-0013](0013-the-sync-transport.md)'s fourth
requirement: it is not a user-facing operation, has no progress to report, and its failure mode is a
retry.

## Amendments to accepted ADRs

### 14. [ADR-0004 §3](0004-the-review-event-log.md) — devices label themselves, and labels have a new justification

§3 describes a device *"meeting an unfamiliar writer"* and asking the user what to call it. §8 above
inverts the default: **a device is labelled by its user at enrolment and the label syncs**, so the
ask-about-a-stranger path is the exception — a writer whose label never arrived, or §3's reinstall
case.

§3's stated reason for labels existing — *"'you are behind your laptop' is a sentence"* — **no longer
holds**, because §4 finds that sentence is one the application is never entitled to rest on. Labels
are retained on two different grounds: §12's device list, and §11's clock-skew warning, which must
name a device to be actionable. The identity split itself (machine-owned writer ids, human-owned
labels, never adopted) is untouched.

### 15. [ADR-0009 §4](0009-crate-and-workspace-layout.md) — the platform prohibition is on behaviour, not on capability

§4 and [`AGENTS.md`](../../AGENTS.md) client-stack rule 3 forbid a `#[cfg(target_os)]` outside
`leitner-store::platform` and call a third function arriving there the tell that the seam is eroding.
That rule stands, and its scope is now stated: **it prohibits platform-conditional *behaviour* and a
growing function seam. A compile-time constant that names a platform *capability*, so the interface
can state a limitation the user would otherwise meet as silence, is permitted.**

The distinction is the one [ADR-0003](0003-client-stack.md) already won the stack decision on — a
`#[cfg]` the compiler checks over a runtime `if` nobody checks. A capability constant makes a
limitation visible; the rule exists to stop divergence becoming invisible. §9 is its first and, for
now, only use.

### 16. [ADR-0013](0013-the-sync-transport.md) — two corrections

**Requirement 1's ceiling is ours, not the platform's.** It bounds the promise by Android's standby
buckets, periodic-work floor and foreground-service caps. §2 finds the binding constraint one step
earlier: **no background scheduling mechanism is available to us at all** without spending
[ADR-0003](0003-client-stack.md)'s Gradle-free APK, which
[ADR-0014 §3](0014-when-parameter-optimisation-runs.md) already declined to do. The platform ceilings
are real and never reached, because nothing schedules. The promise narrows accordingly (§3).

**§3 gains a fourth console trap**: the consent screen's application name is what the user must
recognise in their drive's connected-applications list (§10). Like the Production publishing status
and the *TVs and Limited Input devices* client type, it lives in a console, cannot be validated by
anything in this repository, and fails by making a documented route un-followable rather than by
producing an error.

## Requirements this places on downstream tickets

### [#37 — backup and restore](https://github.com/amin-bf/leitner/issues/37) — both answered

Written while #37 was open; [ADR-0016](0016-backup-and-restore.md) landed shortly after and answered
both by name in its §13, **one of them negatively**. Recorded as asked and answered rather than
edited away, because the negative answer is the more useful of the two.

1. ~~**§10 refuses a delete-remote-data control** on the reasoning that the sync namespace is
   disposable; if #37 introduces an artifact that is *not* disposable, that reasoning does not
   transfer.~~ — **Correct, and honoured.** [ADR-0016 §5](0016-backup-and-restore.md)'s seam is
   **put, get, list and deliberately no delete**, diverging from
   [ADR-0013 §1](0013-the-sync-transport.md)'s four operations by one, in the safe direction: *there,
   deletion is safe because the objects are disposable; here, it is refused because the artifact is
   not.*
2. ~~**§7's "what we found" statement is the only guard against a wrong-account enrolment**; check
   whether collection identity makes the case detectable.~~ — **Answered `no`, structurally.**
   [ADR-0016 §13](0016-backup-and-restore.md) found that in a wrong-account enrolment **every
   collection id agrees**, because all the devices hold the same collection and merely cannot see
   each other; the failure is **reachability, not identity**. §7's sentence stands as the whole
   defence, and the live trade it leaves — naming the account, at the cost of a scope
   [ADR-0013 §8](0013-the-sync-transport.md) declined — is in *Open items*.
   **Since taken by [ADR-0019](0019-naming-the-account-at-enrolment.md)**, which widened the
   reachability finding to rule out *any* application-side check and bought the account name for
   **diagnosis rather than detection**. §7's sentence stands, no longer alone.

## Consequences

- **Exactly two things are permitted to speak about sync** — a dead grant and a clock-skew warning.
  Any third is a defect against §4, and this is the rule most likely to erode, because every future
  feature has a reason to want a badge.
- **The absence of background sync is load-bearing, not a limitation to lift.** §6's session
  stability depends on it. Anyone reopening it inherits the mid-session queue shift
  [ADR-0014](0014-when-parameter-optimisation-runs.md) called unfixable.
- **The application never claims two devices agree**, because it cannot know. Any future surface that
  implies it breaks §4.
- **A wrong-account enrolment remains undetectable**, mitigated only by a sentence. This is the
  weakest point in the document and is recorded as such — and
  [ADR-0016 §13](0016-backup-and-restore.md) has since shown it is **structural rather than a gap**:
  every collection id agrees in that case, so the failure is reachability, not identity, and no check
  of that shape could have caught it. The remaining lever is naming the account, which costs a scope.
- **[`AGENTS.md`](../../AGENTS.md) gains sync-experience rules**, because three of the above fail
  silently: the two-speakers rule, "never start a sync while the review screen is up", and the
  no-delete-remote-data refusal, whose reasoning is invisible from the code that would implement it.
- **A visual design pass now owes three surfaces it does not know about**: sync settings, the
  non-modal notice channel, and the cold-start progress state.

## Open items handed onward

| Item | Owner |
|---|---|
| ~~How long a handset left alone goes between successful background syncs~~ — **dissolved in §2**: there are none | — |
| The recency floor's value for §2's trigger 1 | Implementation; a debounce, not a compatibility constant |
| Exact copy for the drive's connected-applications route (§10) — a third party's UI, expected to drift | Implementation |
| Visual treatment of sync settings, the notice channel and cold-start progress | Map fog — *a visual design pass* |
| ~~Whether collection identity makes a wrong-account enrolment detectable~~ — **answered `no` by [ADR-0016 §13](0016-backup-and-restore.md)**: every id agrees, so the failure is reachability rather than identity | — |
| ~~**Whether to name the account on the enrolment screen**, at the cost of the `email` or `profile` scope §7 and [ADR-0013 §8](0013-the-sync-transport.md) both decline~~ — **taken by [ADR-0019](0019-naming-the-account-at-enrolment.md)**: it is named, on the enrolment screen *and* in §12's settings, at the cost of `openid email` (not `profile`); *"the only lever left"* was **wrong** — §7's sentence already detects, and the name is bought for diagnosis | — |
