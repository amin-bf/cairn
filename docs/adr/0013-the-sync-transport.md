# ADR-0013: The sync transport

- **Status**: Accepted, with one verification outstanding (§8)
- **Date**: 2026-07-31
- **Resolves**: [Decide: the sync transport](https://github.com/amin-bf/leitner/issues/39)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Evidence**: [`docs/research/sync-transport/`](../research/sync-transport/README.md)
  ([Research: sync transport over storage we do not own](https://github.com/amin-bf/leitner/issues/33))
- **Related**: [ADR-0004](0004-the-review-event-log.md) (row identity, the version summary, the
  mutable surface, the interchange form), [ADR-0007](0007-the-local-store.md) (the authoritative
  local copy, the writer marker outside the backup set),
  [ADR-0009](0009-crate-and-workspace-layout.md) (the platform seam, the anticipated sixth crate),
  [ADR-0011 §5](0011-new-card-rate-and-daily-limits.md) (the rate rides the mutable surface)

## Context

Standing constraint 3 of the map says sync must not be foreclosed, and
[ADR-0004 §2](0004-the-review-event-log.md) made it concrete: every row carries `(writer id,
sequence number)`, that pair *is* the row's identity, merging is set union with duplicates dropped,
and scanning reconstructs the `{writer → highest sequence}` summary that answers *"am I behind?"*.
The ADR deliberately declined to say what carries any of that between devices.

[#33](https://github.com/amin-bf/leitner/issues/33) gathered the facts and left four families live —
a git remote, a rented object store, rented WebDAV, and a personal cloud drive through its own API —
after **structurally disqualifying** one: a folder another application keeps in step **cannot answer
"am I behind?" even in principle**, because a device sees only its own local replica and a directory
listing reports what has *arrived*, never what exists elsewhere. ADR-0004 §2's handshake has no
counterparty there. That is a missing capability, not a cost, and it is not revisited below.

It also removed three things the ticket expected to decide this. **Money discriminates nothing** —
$0.000 to $0.004 a month across every metered candidate, because ingress is free everywhere and the
data sits inside every free tier. **Conditional writes are unnecessary** under a per-writer keyspace.
And **Android caps unattended sync identically for every candidate**, so the platform constrains what
the app may *promise*, not which store it should use.

Two of its findings bind everything below. **"One writer, one namespace" is the invariant the design
rests on**, whichever store wins — per-writer *files* do not prevent publishing conflicts, because a
content-addressed store's compare-and-swap is on the *ref* rather than the file. And **declining to
need conditional writes is a prize rather than a saving**: two of three WebDAV servers tested
silently ignored the precondition in the data-losing direction, returning `201 Created` while
overwriting, so a client cannot tell from a `2xx` that the condition was ever evaluated.

One question the ticket did not ask turned out to gate several answers: **how many people will ever
run this app.** The honest answer is *unknown, possibly many*. §2 is why that did not have to be
resolved.

## Decision

### 1. The transport is a key-value namespace, and one writer owns one prefix

> **Four operations: put an object, get an object, list a prefix, delete an object. Nothing else.**

Objects are named, opaque and whole. There is no append, no partial write, no locking, no
transaction, and no server-side computation of any kind. Every key lives under exactly one writer's
prefix, so **every key has exactly one possible author, for the lifetime of the collection**.

This is the whole interface, and it is deliberately the *least* any candidate offers. Three of #33's
four live families implement it directly — a rented object store natively, rented WebDAV over HTTP
verbs, a personal cloud drive through its own API — and one crate already implements all of them
behind a single trait and `cargo check`s clean for `aarch64-linux-android`.

**Choosing this shape is what rejects git**, and that is the real fork in this decision rather than a
four-way choice between stores. Git is not a key-value store; its unit of concurrency control is the
ref. Two devices committing entirely disjoint per-writer files to one branch collide — the second
push is rejected — and recovering the premise costs **per-writer branches**, a design commitment the
ticket did not know it was making.

**Rejected: a git remote.** Its advantages are real and, here, small: the cheapest handshake measured
anywhere (~1.3 KB and two requests, with the version summary encodable in the ref *name* so no
objects move at all), and content-hash integrity giving a definitive corruption answer nothing else
offers. Against that, its costs concentrate precisely where this application is weakest — **the
handset**:

- **The Android library choice is a fork with no good arm.** `libgit2` builds, at the price of three
  vendored C libraries and an OpenSSL compiled from source for every ABI in every CI run, and it is
  the only copyleft licence in either stack (GPLv2 with a linking exception written to permit exactly
  this use, so not a blocker — but the only one). The pure-Rust alternative fails to compile at all
  on current versions and must be pinned back, and its SSH transport works by **spawning an external
  `ssh` program**, which Android forbids: `execve()` on files in the app home directory is refused.
- **A killed clone deletes the target directory entirely.** There is no resume, so a phone that drops
  off Wi-Fi during its first sync restarts from zero every time, leaving ~25 MB of orphaned temporary
  packs behind on each attempt.
- **Local corruption is not repaired by refetching.** Negotiation is ref-based, so the remote
  concludes the client already holds the missing object and sends nothing. Recovery is a full
  re-clone — which loops straight back into the previous point.
- **None of it is verified on the handset**, and `AGENTS.md` rule 9 requires that before anything is
  designed on it.

Integrity is also worth less here than it looks. It matters most when the unit of data is precious
and unreconstructible; ours is a set of self-describing rows where a malformed line is already
discarded silently ([ADR-0004 §11](0004-the-review-event-log.md)) and **every row exists on the
device that wrote it**.

### 2. Transport is the one decision on this map that is cheap to reverse, and that is load-bearing

Almost everything else in this specification is locked by the log being append-only and
un-migratable. This is not, and the reason is structural: **every device holds the complete log
forever** — never compacted ([ADR-0004 §10](0004-the-review-event-log.md)), authoritative in
`collection.db` ([ADR-0007](0007-the-local-store.md)) — and merge is set union on `(writer,
sequence)`.

So switching stores means each device republishes from its own local copy into a new namespace.
Nothing migrates, no history is stranded, and the worst cost is re-enrolling each device once. **The
remote is a rendezvous point, not a system of record.** It stands in the same relation to
`collection.db` as `derived.db` does: disposable, rebuildable, and never the truth.

Two things follow, and both are used below. The unknown audience has to be *survivable*, not
*answered* — a far weaker requirement, and the reason it is safe to ship exactly one backend. And
losing the remote entirely, to a bug, a revoked grant or a user deleting it, costs one republish
rather than any data at all.

### 3. The backend is Google Drive's application data folder

All three personal-drive candidates offer an app-scoped folder, a public OAuth client with PKCE, and
a change cursor cheaper than a listing. They differ irreversibly on exactly one axis: **ceilings the
developer cannot lift, counted against our single registered client ID across everyone who ever
installs the app.**

| | Ceiling on people who install the app | Refresh token when unused | Folder |
|---|---|---|---|
| **Google Drive** (`appDataFolder`) | **none** — the scope is *non-sensitive*, so verification is not mandatory | **dies after 6 months** | hidden |
| Dropbox (App folder) | **50, then new links freeze** pending human review | long-lived, no stated expiry | visible; *"users can contribute by moving files into it"* |
| OneDrive (`approot`) | none found — *low confidence*, not exhausted | no specified lifetime | visible |

**The decision is an asymmetry between two failures, not a sum of advantages.** Each serious
candidate has one bad case and they are not comparable in kind. Google's is that a device left in a
drawer for seven months asks the user to sign in again — the data is untouched, the local log is
complete, and the remedy is a tap, performed by the person holding the phone at the moment they were
going to open the app anyway. Dropbox's is that person number 51 cannot connect **and nothing that
person does fixes it**; it is unblocked only by a human at the provider approving a request from us.
One fails open and recoverably; the other fails closed and unrecoverably, on the exact axis this
project cannot predict.

**The non-sensitive scope is the rarest thing in the survey, and its value is not the saved
paperwork.** No verification means **no verification-time endpoint** — no domain-ownership proof, no
hosted policy a reviewer fetches, no callback. Every one of those would have been a server, which the
destination forbids.

**The hidden folder is correct here, and #33's framing of it is rejected.** That note called it *"a
backup hazard, not a backup"* because the user cannot copy the contents out. That reasoning assumes
the remote is an archive; §2 establishes it is not. Under the disposability principle, *visible* is
the liability — a visible app folder invites a user to reorganise it mid-sync — and a hidden one
cannot be casually disturbed. Backup proper is
[#37](https://github.com/amin-bf/leitner/issues/37)'s, and it must not be quietly satisfied by a sync
folder.

**Two traps that live in a console, not in the repository**, so nothing here will ever catch them:

- **The project must be in Production publishing status.** A project left in "Testing" issues refresh
  tokens expiring in **7 days**, turning enrolment into a weekly chore that would read as a
  mysterious bug. Production is available precisely because the scope is non-sensitive.
- **100 refresh tokens per end-user account per client ID**, each re-authorisation consuming one and
  the oldest invalidated at the ceiling. At 2–5 devices this never binds; it is recorded because
  ADR-0004 §3 mints a fresh writer id per install, so reinstalls are not rare events.

**Rejected: Dropbox**, on the ceiling above. Its unauthenticated long-poll — the only push-shaped
mechanism anywhere in the research, blocking up to 480 seconds with no credential in flight — is
genuinely elegant and buys less than it appears to, because Android lists network as **Disabled** in
the Rare and Restricted standby buckets, which is exactly where a drawer device sits. It would help
only on desktop, the machine most likely to have the app open already.

**Rejected: OneDrive.** Nothing it wins on, and the most unknowns: no consumer review gate was found,
but the research marks that *low confidence* from unexhausted documentation, and its rate limits were
never pinned at all.

**Rejected: a rented object store.** It is the cleanest technical fit — it *is* this seam natively,
with strong read-after-write and list-after-write, lexicographic ordering, `start-after`, and entity
tag plus size in the listing. It has the worst human story in the survey, which #33 flagged as the
finding most likely to disqualify the family: an account signup with a payment card even on a free
tier, a bucket, a minted key, a secret shown exactly once, and then a ~25-character key ID and
~31-character secret **typed into the phone**. Only one of three providers exposes prefix scoping and
an expiry as form fields; on another it takes hand-written policy JSON where writing only the `Allow`
and omitting the explicit `Deny` produces a silently ineffective policy. This design asks a language
learner to become a storage administrator.

**Rejected: rented WebDAV.** The most conventional credentials in the survey — app-specific
passwords, mandatory under two-factor authentication, individually revocable — and the only family
the user can self-host. It costs a paid account, a URL and username and password typed per device,
and a weak read side: the standardised incremental listing is **refused on the files endpoint**
(`415`, *"The {DAV:}sync-collection REPORT is not supported on this url"*), leaving a full `PROPFIND`
at 245–286 bytes per entry, on a server that in one test **offered no compression even when gzip was
requested**.

Both remain implementable behind §1's seam if a reason ever appears. Neither ships.

### 4. Nothing published is ever rewritten

> **Every object is written once, at one key, and never modified. Only roll-up (§5) deletes.**

Each publish writes a *new* object holding exactly the rows that writer has produced since its last
publish, keyed by the sequence range it covers:

```
w<writer id>/log/<start seq>-<end seq>.jsonl.zst      fixed-width, zero-padded
```

Immutability is not tidiness; it removes a class of failure rather than making one cheaper. An object
written once can never be observed half-written, is safe to cache forever, needs no validator, and —
combined with §1's single-author-per-key property — closes the last opening through which anyone
would reach for a conditional write.

**Conditional writes appear nowhere in this design, and that is a decision rather than an
omission.** They solve lost updates to a *shared* key; under a per-writer keyspace no key ever has
two writers, so the failure they prevent cannot occur. Adding one would make the realistic case
*worse*, not better: a client retrying after an ambiguous timeout, re-uploading byte-identical
content, is rejected rather than succeeding harmlessly. And it would expose us to the hazard #33
measured — two of three servers returning `201 Created` while ignoring the precondition and
overwriting the file, indistinguishable from success. **We are not mitigating that hazard; we are
never exposed to it.**

**Compression is `zstd`, and it is a container rather than a re-encoding**, so ADR-0004 §11's
relay-byte-for-byte rule is untouched: the exact interchange bytes come back out. Naming the
compressor is not a detail — see the amendment in §12.

### 5. Roll-up merges by count, never by a clock, and it is the only deletion

**Cold start is the operation that decides segment granularity — not bytes, not money, not storage.**
A fresh device, or one whose change cursor has expired (documented as `410 Gone`, requiring full
re-enumeration), must fetch everything. At one object per sync that is on the order of **43,800
objects for a decade**, and the cost is one request each. Rolled up it is a few hundred. Every
byte-level consideration in the research is a rounding error beside that.

So objects merge, recursively, by a rule with no calendar in it:

> **When a writer holds K objects covering adjacent sequence ranges at the same level, it writes one
> object covering their union and deletes exactly the objects it merged.** Default `K = 32`.

Levels are implicit — an object's range says how much it covers, and nothing needs to be labelled.
The bound this gives is the point: a decade of one writer's syncs is ~21,900 level-0 segments, which
fits in **four levels of at most 32 objects each, so at most ~128 live objects per writer** and a few
hundred across a collection. Each row is rewritten once per level it climbs, so lifetime upload
amplification is about **4×** — against 1,800 MiB per device per month for whole-log republish.
Compression improves as it climbs, since the measured ratio is a function of block size: **3.0–4.1×**
for a sync-sized chunk, 5.4× at a thousand rows, 6.6× at five thousand, **12.01×** at a writer-year.

**`K` is not a compatibility constant.** A device using a different value still produces objects any
reader can consume, because readers merge by set union and never assume a layout. Tuning it later is
free, which is why a default is pinned rather than argued.

**Rejected: a calendar trigger.** Monthly or daily buckets work and measure well, but they drag a
clock into the transport layer, and this repository has spent real effort keeping clocks out —
ADR-0004 §4 freezes day numbers at write time precisely so nothing recomputes them, and ADR-0009
records that *replay needs no clock at all*. A count trigger is also self-scaling: a heavy user rolls
up often, a light user has few objects to begin with.

**Rejected: whole-log republish.** Ruled out by uplink rather than by money — **1,800 MiB per device
per month** at six syncs a day against 0.2 MiB for segments, a factor of roughly nine thousand, and a
mobile-data bill that appears on no pricing page. The monetary bill is identical either way.

**Rejected: time-bucketing with a rewritten open bucket.** Cheap on upload and expensive on
*download*: every peer re-fetches the whole open bucket each time it changes, so late in a month a
device pulls hundreds of kilobytes to learn about a few hundred bytes of new rows. It also
reintroduces a mutable object, undoing §4.

**Two rules that must be stated because their failure is silent:**

1. **Write the merged object first, then delete — and delete only what it covers.** A reader that
   listed before a roll-up and fetches a key deleted since receives `404`, whose correct handling is
   *re-list*, not *recover*. Merge being set union on `(writer, sequence)` means reading the merged
   object and the segments yields the same set, so overlap is free and the operation is idempotent.
2. **Deletion in the application data folder is permanent.** Files there cannot be trashed —
   attempting it returns `notSupportedForAppDataFolderFiles`. There is no undo, so rule 1 is a rule
   and not a preference.

Even so, the blast radius is bounded by §2: a botched roll-up costs a republish, never a review.

### 6. The listing is the version summary

Because the sequence range is fixed-width and zero-padded in the key, keys sort lexicographically in
numeric order, and **the highest end-sequence under a writer's prefix *is* that writer's entry in
ADR-0004 §2's `{writer → highest sequence}` summary**.

So the handshake needs no manifest object, no head pointer that can be torn, and no extra request. It
is the listing we were going to issue anyway. ADR-0004 §2 put the summary "in the sync handshake,
never on a row"; here the handshake turns out to need no payload of its own at all.

**The change cursor is an optimisation over an authoritative listing, never a replacement for it.**
That ordering matters because cursors are documented to expire, and the recovery from an expired
cursor is a full re-enumeration — which must therefore always work, and does, because the listing is
the source of truth and §5 keeps it small.

### 7. The mutable surface rides the same transport, and its roll-up is the opposite rule

[ADR-0004 §7](0004-the-review-event-log.md) handed this onward by name — snapshot or change stream —
and #33 sharpened it from a question about bytes into one about concurrency control: its entire
"conditional writes are unnecessary" finding holds **only if the mutable surface is also published
per writer**. A single shared document that every device overwrites *is* a contended key, the store's
latest-write-wins rule applies literally, and updates are lost.

> **Per writer: a change stream of stamped assignments, under `w<writer id>/state/…`, on §4's
> immutable objects and §5's roll-up.**

**Snapshot and change stream converge here, and that is why the question dissolves.** §7 gives every
mutable value a stamp — a counter plus the writer id — and a writer's own counter is monotone, so
**within one writer's own stream an earlier assignment to a key always loses to that writer's later
assignment to the same key**. Compacting a writer's stream to *the latest value it assigned to each
key it ever touched* therefore discards nothing any reader could use — and that compacted form *is* a
per-writer snapshot. Deltas per sync, snapshot as the roll-up result, one mechanism.

This introduces **no new concepts**: the same keyspace, the same immutability, the same count
trigger. It also discharges #33's proviso outright, which is what keeps §4's conditional-write
conclusion true.

**The two roll-ups are opposite, and confusing them is destructive in both directions.** This is the
sharpest edge in the ADR:

| | `…/log/` | `…/state/` |
|---|---|---|
| Roll-up is | **lossless** — every row survives | **lossy by design** — superseded values are dropped |
| Fixed by | ADR-0004 §10, the log is never compacted | §7 above: only the latest stamp per key can win |

An agent applying the log's never-compact rule to the state stream builds unbounded growth. An agent
applying the state stream's compaction to the log **silently destroys review history**, which is the
worst outcome available in this codebase. Two prefixes, two rules, and they are written next to each
other here and in `crates/sync/src/CONTEXT.md` for that reason.

**A writer's state object is bounded by what that writer edited, not by collection size.** The naive
worry — the mutable surface is every field of every note, so this is huge — is true of the
*collection* and false of any one writer's view of it. A phone used only for reviewing touches almost
nothing and carries a near-empty state object however many notes exist; the desktop, which ADR-0003
already makes the sole authoring surface for non-Latin content, carries the large one.

**Rejected: snapshot-only, republished each sync.** Simpler to describe, and it reintroduces exactly
what §4 removed — an object that is rewritten rather than written once, so peers re-download the
whole thing to learn about one changed tag.

**Rejected: one shared document.** The only shape that reintroduces the concurrency-control problem a
per-writer keyspace removes, and the one #33 warned about by name.

### 8. Enrolment is the device flow, and it costs no platform capability

The client is public: **the client ID is compiled into the binary and there is no secret**, which
RFC 8252 §8.5 not only permits but expects — a secret shipped to every user is not one.

> **The app displays a short code and a URL. The user opens that URL on whatever device suits them,
> enters the code, and the app polls until the grant arrives.**

**Identical on desktop and Android, with zero platform-specific code.** No browser launch, no intent,
no manifest change, no loopback listener, no `Activity` subclass, no `#[cfg(target_os)]`. Given the
map fixes *agents implement this*, a flow with no platform surface is not merely tidier — it is the
difference between one specification and two.

**Rejected: the authorization-code flow with a loopback redirect.** It gives the nicer moment — tap,
browser appears, done — and costs three things the device flow does not. It needs a new platform
capability, `open_url()`, which walks straight into the ADR-0009 contradiction recorded in §12. It
carries an **unverified assumption**: this provider has withdrawn both custom URI schemes and the
copy-paste redirect for some client types, and loopback-from-Android against it was never confirmed.
And it needs the redirect caught in-process, where the device flow needs nothing caught at all.

**The outstanding verification, and the reason this ADR's status carries a caveat.** #33 verified a
device flow against a *git host*, not against this provider, and this provider's limited-input-device
flow publishes a scope allowlist. **Whether `drive.appdata` is on it is not established.** It is
cheap to check and expensive to be wrong about, so it is a verification step rather than an
assumption: if the scope is not permitted, the fallback is the rejected option above, and adopting it
*does* then require amending ADR-0009 §4.

**Neither flow types a credential into our own text field**, which is why the drive family was
attractive in the first place: `AGENTS.md` rule 8 records that Android text input is ASCII-only and
cannot be fixed here, and a six-character code entered on a device of the user's choosing never
approaches that limit.

**Sync is opt-in and never blocks first use.** Offline-by-default is the map's operating mode rather
than a feature, so a user must be able to review for years without connecting anything. Enrolment is
something you go and do.

### 9. The credential lives in a plain file, and revocation is all-or-nothing

**No keyring, no keystore, on either platform.** On Android, app-private internal storage is already
encrypted at API 29+ and unreadable by any other app, and the keystore can only *protect* a secret,
never hold one — so it buys an encryption hop and a JNI surface. On desktop a keyring defends against
other processes running as the same user, and **that threat model is already conceded**, because
`collection.db` sits in the same directory as an unencrypted SQLite file holding the user's entire
review history. Encrypting the token while the data it guards lies in plaintext beside it is theatre.
The map's *Local encryption / device passcode* fog covers both uniformly if it ever lands, which is
the honest place for this.

**The token goes *inside* the Android backup set — deliberately opposite to
[ADR-0007](0007-the-local-store.md)'s writer marker.** That marker is excluded because a restore that
carries it manufactures a duplicate writer, the silent-loss failure ADR-0004 §2's identity scheme
exists to prevent. The refresh token has no such property: a restored phone arriving already
authorised is simply convenient, and it mints a fresh writer id regardless, so the restore is still a
clean fork. The two look inconsistent and are not.

**Revocation is all-or-nothing, and cannot be made otherwise.** Every device holds a token issued
against the same client ID, and the provider's revocation surface is per-application: the account's
security page lists our app once, with one button. Losing a phone means revoking, after which every
remaining device is logged out and must re-enrol.

**Accepted**, on three grounds. The scope is `appDataFolder`, so a stolen token reaches **our app's
own hidden folder and nothing else** — not the user's Drive, not their documents — where an
object-store key in the realistic case reads and deletes an entire bucket. The token sits in
encrypted app-private storage on a device that is itself lock-screen encrypted, so a lost-and-locked
phone is not a realistic compromise. And §2 means nothing is destroyed in any case: re-enrolment
republishes.

Note the blast radius is the *same* as the object-store key rotation that helped disqualify that
family — it is cheaper here (a code rather than a 31-character secret) but it is not narrower, and
that is worth stating plainly rather than claiming a win the design does not have.

**Rejected: registering an OAuth client per device**, which is genuinely narrower and costs each user
creating their own developer project — a setup story worse than the object store already rejected on
setup burden.

### 10. One account is one collection — here, and only here

The application data folder is scoped per (account, application), so **this transport cannot merge
two collections even by accident**. That is a partial gift to the map's *Collection identity* fog,
and the limits of it should be stated: it says nothing about telling an *import* from a *restore*,
which is the form [#37](https://github.com/amin-bf/leitner/issues/37) actually meets — an import
crosses a collection boundary so ADR-0004 §7 stamps reset, a restore re-enters the same collection so
they must travel byte for byte. Nothing here settles that.

### 11. `leitner-sync` is the sixth crate

Anticipated rather than new: `CONTEXT-MAP.md` already records that *"a `sync` context is anticipated,
not created… expect a sixth crate rather than a fifth module"*, on the ground that a network
dependency cannot live in `leitner-core`. HTTP, TLS and OAuth are a much larger version of the bill
`zip` presented when `export` became the fifth crate, and the crate must be testable without a
window, which rules out `leitner-app`. So this ADR realises ADR-0009's own prediction rather than
overturning anything.

## Amendments to accepted ADRs

### [ADR-0004 §11](0004-the-review-event-log.md) — the interchange form must name its compressor

§11 says the interchange form *"compresses about ten to one"*. That is true of a large-window
compressor over large blocks and false otherwise, and #33 measured both ends on rows generated in
§11's exact shape: a decade compresses **11.76× with `zstd -19`** but only **3.99× with `gzip -9`**,
because gzip's 32 KiB window cannot reach back to the repeated writer ids and key names. Segment size
moves it just as far — **12.01×** as one file per writer-year against **5.02×** as daily segments and
**3.04×** for a sync-sized chunk.

**Amended**: the ratio is conditional on the compressor and the block size. This ADR fixes `zstd`
(§4) and a roll-up ladder (§5) that carries blocks up the measured curve. The row size itself needs
no amendment — two slices independently measured **151.4 B** and **152.5 B** against §11's "roughly
150 bytes".

### [ADR-0004 §10](0004-the-review-event-log.md) — the decade projection carries the same condition

§10's *"around 110 MB raw and 15 MB compressed"* per decade is confirmed on the raw figure to within
1%, and the compressed figure silently assumed both conditions above. **Amended**: 15 MB is right for
`zstd` over large blocks, becomes roughly 27 MB under `gzip`, and roughly 22 MB under daily
segmentation. §10's never-compact conclusion is untouched — it never depended on the ratio.

### [ADR-0009 §4](0009-crate-and-workspace-layout.md) — a contradiction on its own seam, recorded

ADR-0009 §4 says **"A third function appearing in this module means the seam is eroding. That is the
signal to stop, not to add it."** Its handoff table, under *Any ticket that adds a platform
capability*, says **"It goes through `leitner-store::platform` or it does not exist. A second
`#[cfg(target_os)]` elsewhere in the workspace is a defect, not a shortcut."**

This was the first ticket to reach for a platform capability that is not storage, and the two
instructions cannot both be followed: the handoff sends `open_url()` into
`leitner-store::platform` — semantically absurd in a storage crate — and §4 forbids it arriving
there.

**This ADR does not resolve the contradiction; §8 routes around it**, because the device flow needs
no platform capability at all. **It is recorded because the next ticket to need one will hit it head
on with no equivalent escape**, and because a contradiction nobody wrote down is one the next agent
rediscovers as a surprise. The shape of the fix, when it is needed: the seam rule is per crate rather
than per workspace — `leitner-store` keeps exactly two functions, and a crate that must touch the
platform for an unrelated reason gets its own module under the same three-arm discipline.

If §8's verification fails and the loopback flow is adopted, **that fix becomes required**, and the
amendment is no longer hypothetical.

## Requirements this places on downstream tickets

### [#40 — the sync experience](https://github.com/amin-bf/leitner/issues/40)

1. **The honest promise is bounded by the platform, not by this transport.** Android lists network as
   **Disabled** in the Rare and Restricted app-standby buckets — where a device left alone lands —
   defers idle apps to roughly one network window a day, floors periodic work at 15 minutes, caps a
   `dataSync` foreground service at six hours in any 24, and forbids starting one from boot. Asking
   for a power-management exemption is store-policy-restricted unless the app's core function is
   adversely affected, which a few-kilobyte sync cannot claim. So the promise available is: **sync
   when the user opens the app, and opportunistically before that.** Anything stronger is
   overpromising, and #33 established it applies identically to every candidate.
2. **"Am I behind?" is answered by a listing and costs one round trip** (§6). Whatever the UI says
   about divergence, it is not paying for the answer.
3. **A device that has been away for months may need to re-enrol** (§3). That is a UI moment, and it
   should read as reconnecting rather than as an error.
4. **Roll-up is invisible and must stay that way.** It is not a user-facing operation, has no
   progress to report, and its failure mode is a retry.

### [#37 — backup and restore](https://github.com/amin-bf/leitner/issues/37)

1. **The sync folder is not a backup and must not be presented as one** (§2, §3). It holds a
   published projection of the log, it is hidden from the user, and it is deleted if the user removes
   the app's data from their Drive.
2. **§10 gives only half of collection identity.** One account is one collection *through this
   transport*; distinguishing an import from a restore still needs the answer #37 owns.
3. **A restore re-enters the same collection**, so ADR-0007's writer-marker exclusion still governs
   the fork, and §9's decision to let the *token* ride the backup set does not weaken it.

### [#42 — when parameter optimisation runs](https://github.com/amin-bf/leitner/issues/42)

1. **Publishing and optimising compete for the same foreground window.** Android freezes a
   backgrounded app outright, so both want the moment the user has the app open. Sync is seconds and
   optimisation is up to 4.3 s at decade scale; ordering them is #42's, but they are the same budget.

## Glossary

Of record in [`crates/sync/src/CONTEXT.md`](../../crates/sync/src/CONTEXT.md), per ADR-0009's rule
that a glossary lives beside the code it describes.

## Consequences

- **The transport surface is four operations wide and holds no domain knowledge.** A second backend
  is an implementation of a trait, not a redesign — which is what makes the unanswered audience
  question survivable.
- **Nothing in this design ever needs a conditional write**, so the silent-precondition hazard #33
  measured is not mitigated but avoided. Any future change that introduces a shared key reopens it.
- **The store may be deleted at any time with no data loss**, which is a property worth defending in
  review: any future feature that makes the remote authoritative for anything breaks §2 and with it
  most of this ADR's reasoning.
- **The terms-of-service exposure disappears with the git branch.** It was entirely about hosts
  forbidding repositories used as personal cloud storage; an application's own data in an application
  data folder is that folder's documented purpose.
- **A sixth crate lands**, and `leitner-core`'s empty dependency list survives another feature.
- **`AGENTS.md` gains a sync section**, because three of the rules above fail silently: the opposite
  roll-up rules, write-before-delete, and permanent deletion in the app data folder.

## Open items handed onward

| Item | Owner |
|---|---|
| Whether `drive.appdata` is permitted in the limited-input-device flow (§8) | Verification before implementation; fallback named |
| Running an enrolment and a sync on the real handset, per `AGENTS.md` rule 9 | Implementation |
| How long a real handset left alone actually goes between successful background syncs | [#40](https://github.com/amin-bf/leitner/issues/40) |
| Tuning `K` (§5) against real object counts | Implementation; not a compatibility constant |
| Media, if audio on cards is ever built — the map's fog notes media is where sync stops being cheap | Map fog |
| A second backend, if the audience ever makes one worthwhile | Not scheduled |
