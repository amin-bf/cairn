# ADR-0020: Protection at rest — nothing is encrypted, and no secret is ever asked for

- **Status**: Accepted
- **Date**: 2026-07-31
- **Resolves**: [Decide: what, if anything, is protected at rest](https://github.com/amin-bf/leitner/issues/58)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Evidence**: [`docs/research/auto-backup-at-rest/`](../research/auto-backup-at-rest/README.md)
  ([Research: is Android Auto Backup's payload readable by the provider](https://github.com/amin-bf/leitner/issues/60)),
  and [`docs/research/sync-transport/`](../research/sync-transport/README.md) for the application data
  folder's deletion route
- **Related**: [ADR-0007](0007-the-local-store.md) (`collection.db`, and the open item this
  discharges), [ADR-0013 §3 §8 §9](0013-the-sync-transport.md) (the application data folder, the scope
  set, the credential in a plain file), [ADR-0015 §5 §7 §10](0015-the-sync-experience.md) (what may
  speak about sync, the enrolment closing statement, the absent delete control and the removal route),
  [ADR-0016 §1 §7 §8 §11](0016-backup-and-restore.md) (backup protects against loss, Auto Backup stays
  on, the archive is not encrypted, minimal disclosure),
  [ADR-0019](0019-naming-the-account-at-enrolment.md) (the account address, which lands in the
  credential file this ADR rules on and adds a second clause to the same enrolment sentence)

## Context

**Nothing in this design is encrypted, and until now no ticket had decided that.** Three accepted ADRs
each acted on the default while pointing at a fog patch on the map — *Local encryption / device
passcode* — for the real answer:

- **The credential** — [ADR-0013 §9](0013-the-sync-transport.md) declined a keyring on the ground that
  *"encrypting the token while the data it guards lies in plaintext beside it is theatre"*, and handed
  the general question to the fog.
- **The archive** — [ADR-0016 §8](0016-backup-and-restore.md) declined a passphrase on a correctness
  ground, recorded that §9's *theatre* argument does **not** transfer to an artifact that travels, and
  handed the general question to the same fog.
- **`collection.db` itself** — [ADR-0007](0007-the-local-store.md) held it as an open item and nothing
  ever decided it.

Three deferrals to a fourth thing that did not exist. This ADR is that thing, and its job is less to
change the answer than to make the answer *citable* and to argue the part that was never argued.

## Decision

### 1. The question is asked for four artifacts, not three

The ticket enumerated three. There is a fourth, and it is the one with the strongest case:

| # | Artifact | Where it rests | Who else can read it |
|---|---|---|---|
| i | `collection.db` (and `derived.db`) | app-private storage on the handset; an ordinary directory on the desktop | on the handset, nothing; on the desktop, any process running as the user |
| ii | The drive credential — refresh token **and, since [ADR-0019 §6](0019-naming-the-account-at-enrolment.md), the account address** | beside (i), by [ADR-0013 §9](0013-the-sync-transport.md); on the handset also *inside* the backup set (§6) | as (i) |
| iii | The `.lcoll` archive | wherever the user put it | whoever encounters it there |
| iv | **The published log** | the drive's application data folder, by [ADR-0013 §1 §3](0013-the-sync-transport.md) | **the storage provider, continuously, for as long as sync is on** |

Fixing the enumeration first matters because ADR-0016 §8's instruction — that the local store and the
travelling archive *"must be answered together"* — is worth nothing if the set is incomplete. (iv) is
different in kind from the other three: it is held by a party who is not the user, it is not elective
once sync is enabled, and what it discloses is not only study material but a timestamped behavioural
record of when this person studies and what they repeatedly fail.

**There is also a fifth thing, and it is a *path* rather than an artifact.** On the handset,
[ADR-0016 §7](0016-backup-and-restore.md) keeps Auto Backup on, so (i) and (ii) reach the same provider
as (iv) **without the user enabling anything**. It is not a separate artifact — it is artifacts (i) and
(ii) in a second location — which is why it gets §6 rather than a row here, and why it was the easiest
thing on this page to miss.

### 2. The threat model, stated once

Every artifact question below resolves against this table, so it is written down rather than re-derived:

| Adversary | On the handset | On the desktop |
|---|---|---|
| **A** — another application on the device | blocked; app-private storage is unreadable by other apps and encrypted at rest ([ADR-0013 §9](0013-the-sync-transport.md)) | **nothing.** Any process running as the user reads `collection.db` |
| **B** — another *person*, device unlocked and in hand | nothing | nothing |
| **C** — someone holding the lost or stolen device | covered; file-based encryption keyed to the lock screen | the user's own full-disk encryption, if they enabled it |
| **D** — the provider holding the published log | — | **nothing.** Plaintext, continuously |
| **D′** — the same provider, holding the Android backup | **conditional**; ciphertext under a key the provider cannot access *only* with Android 9+ **and** a lock screen set (§6) | — |
| **E** — whoever encounters the archive where the user left it | — | **nothing** |

**This application defends against none of them by its own effort.** A, C are answered on the handset
by the platform and are unanswerable on the desktop without a secret the user supplies. B is not an
encryption problem at all: the device is unlocked, so any key the application can reach, it can reach.
E is elective — the user chose where that file went.

**D and D′ are where this is a concession rather than a restatement of what the platform already
does**, and both are recorded as concessions rather than glossed. §5 states D's residual and §6 states
D′'s. **D′ is the worse of the two** and the ordering is worth stating, because intuition puts the
travelling archive first: D′ is not elective, it reaches users who enabled nothing, it has no moment at
which the application could tell them, and its payload cannot be deleted per application.

### 3. No user-supplied secret — and the input limitation is not the reason

This is the rule, and it is deliberately about **secrets** rather than about encryption:

> **No passphrase, no PIN, no unlock code, anywhere in this design.** A system with no server, no
> account and no escrow has nowhere to recover a forgotten secret from, so any user-supplied secret
> converts a design premised on never losing data into one where forgetting loses all of it. And a
> secret weak enough to be typed on the handset is too weak against the one adversary that holds
> ciphertext indefinitely.

The two halves close the door from opposite sides, which is why the door stays shut.

**Why the reason had to be re-argued.** [ADR-0016 §8](0016-backup-and-restore.md) killed a *passphrase*
on the ground that Android text input is ASCII-only ([`AGENTS.md`](../../AGENTS.md) client-stack rule 8),
so a passphrase set on the desktop in the user's own language cannot be typed on the phone and the
archive becomes unopenable exactly where it is needed. That argument is correct and it **does not reach
a PIN**, because digits are ASCII. Someone will eventually notice this, conclude the door is open, and
reopen the whole question from the weakest premise. So:

**A PIN that derives a key fails on arithmetic.** A four-to-six-digit PIN is at most one million
candidates. On a handset that number is safe *only because the hardware refuses to be asked quickly* —
unlock attempts are rate-limited and backed off by the platform. **Data resting on someone else's disk
cannot borrow that property.** Adversary D holds the ciphertext indefinitely, offline, with unlimited
guesses and no rate limiter anywhere in the picture; an exhaustive search of a six-digit space is a
bounded and entirely routine computation even behind a deliberately slow derivation. So the PIN-derived
key is **worthless against precisely the adversary that motivated it**, while charging full price
against the others: a forgotten PIN is total, unrecoverable loss. ADR-0016 §8 already calls a
passphrase-protected backup *"the most reliable way to lose data permanently"*; a PIN is that with less
entropy.

**A PIN that merely gates a screen is theatre in [ADR-0013 §9](0013-the-sync-transport.md)'s exact
sense.** The file sits in plaintext an inch away, so anything that opens the *file* walks past the gate
untouched. It would also hand the application a third thing to say about itself when
[ADR-0015 §5](0015-the-sync-experience.md) spent its length holding that number at two.

**Rejected, and recorded because it is the reasonable-sounding version**: a screen gate admitted
honestly as a user-interface convenience that protects nothing. It defends against the *most likely*
intrusion by far — a person, not a process — and users read its absence as carelessness. It loses
because a lock that protects nothing is read by users as a claim that something is protected, and this
specification has no way to make that distinction visible at the moment it matters. If it ever returns,
it returns to the visual design pass as an affordance, and it may never be described in terms that
imply the data is encrypted.

### 4. And no application-held key either — the theatre argument, generalised

The obvious way around §3 is to encrypt under a key the *application* holds, so nothing can be
forgotten. It does not work, and it fails differently for the resting artifacts than for the travelling
ones.

**For (i) and (ii), it buys nothing.** [ADR-0013 §9](0013-the-sync-transport.md)'s argument already
covers this and generalises without modification: on the handset the platform keystore *"can only
protect a secret, never hold one"*, and app-private storage plus lock-screen-keyed encryption already
answer A and C, so the layer is an encryption hop and a foreign-function surface for no adversary. On
the desktop the key is a file beside the data, which any process reading one reads the other.

**For (iii) and (iv), it is circular.** A key protecting an artifact that travels must be present on
every device that opens it. There is no server in this destination — that is fixed by the map, not
incidental — so there is no channel to distribute that key except the channel it is protecting. An
application-held key published alongside the objects it encrypts is not a weaker design than plaintext;
it is plaintext with extra steps.

This is why a keyring, a keystore, a PIN and a passphrase all land in the same place despite being
quite different mechanisms, and it is why §3 is framed on secrets: naming encryption alone invites the
key-the-app-holds workaround, which an implementer will reach for *because* it dodges the loss problem.

### 5. The published log is a genuine concession, and here is its residual

Stated plainly rather than argued away, because §2 conceded it. §6 holds the other one:

> With sync enabled, the storage provider holds the user's **entire review history in plaintext** — every
> grade, every timestamp, every device label — for as long as the folder exists.

Three things bound it, none of which make it disappear:

- **The drive scope reaches our own hidden folder and nothing else** — not the user's documents, not
  the rest of their storage ([ADR-0013 §9](0013-the-sync-transport.md)). **Stated precisely, because
  [ADR-0019 §4](0019-naming-the-account-at-enrolment.md) has since widened the requested set** to
  `openid email drive.appdata`: the *drive* reach is unchanged and still the folder alone, but the
  grant now also yields the account address. ADR-0019 §5 records that consequence for the phishing
  shape; here it means a stolen token discloses the address as well, which is a real addition and a
  small one against a party who already holds the folder's contents.
- **It is elective.** Sync is opt-in ([ADR-0016 §1](0016-backup-and-restore.md)), and a user who never
  enrols publishes nothing.
- **It is removable — by the user, from the provider's own settings, not from this application.** The
  folder *"is deleted when a user uninstalls your app"* and *"Users can also delete your app's data
  folder manually"*, via the provider's connected-applications settings
  ([evidence and citations](../research/sync-transport/object-stores-and-drives.md)).
  [ADR-0015 §10](0015-the-sync-experience.md) already establishes this route and requires sync settings
  to name it — including *"the name we appear under"*, since the folder is hidden and cannot be
  navigated to. **Nothing here is new; what is new is the load it carries.** §10 offered the route as a
  courtesy while refusing an in-app delete control on its own merits. This ADR makes the route
  *load-bearing*: it is the user's only means of removing plaintext that a third party would otherwise
  hold indefinitely, which is a stronger reason for §10's naming requirement than §10 itself gives, and
  a reason it must not be dropped as a nicety.

**Rejected: end-to-end encryption of the published objects**, which is the conventional answer when
publishing to storage one does not own, and which fails here for §4's circularity: the key would have to
reach every device, and the only channel is the one being protected. Reaching for a user-supplied secret
to break the circle lands back in §3.

### 6. Auto Backup is a second path to the same provider, and for some users it carries no guarantee

[ADR-0016 §7](0016-backup-and-restore.md) keeps Android Auto Backup **on**, so on that platform the
collection reaches the provider **even for a user who never enables sync**. This ADR assumed that path
was covered by the platform. **It is covered for most users and not for all**, and the exception is
structural rather than a configuration mistake.
[Evidence](../research/auto-backup-at-rest/README.md).

**The payload is always encrypted in transit and at rest under keys the operator holds.** A second,
stronger layer encrypts it under a key derived from the device's lock-screen secret and escrowed to
hardware enforcing a failed-attempt counter in firmware — a key the provider states it does not know.
That layer is conditional:

> *"The Standard Android Backup system always encrypts backup data in transit and at rest… regardless
> of the Android version… and of whether your device has a lock screen. Starting from Android 9, if
> the device has a lock screen set, then the backup data is not only encrypted, but encrypted with a
> key not known to Google"*
> — [Android backup best practices](https://developer.android.com/privacy-and-security/risks/backup-best-practices)

**Two populations therefore get different answers, and this project's own floor creates the second
one.** Client-side encryption arrived at API 28 (`FLAG_CLIENT_SIDE_ENCRYPTION_ENABLED`, *"Added in API
level 28"* — [BackupAgent](https://developer.android.com/reference/android/app/backup/BackupAgent)),
and this project targets `min_sdk_version = 24`:

| Population | What the provider holds |
|---|---|
| Android 9+, lock screen set, **backup enabled on that version** | **Ciphertext** under a key the provider states it cannot access |
| **No lock screen** (swipe and trusted-context unlock do not count) | The payload under **operator-managed keys** — for threat-modelling, treat as plaintext |
| Any **API 24–27** handset — inside this project's own supported range | as above; the flag does not exist before API 28, so no configuration and no user action can fix it |
| **Enabled backup before Android 9, then upgraded** | as above, *silently* — a current OS version on the box and a readable backup behind it |

The third row is this project's own doing and the fourth is the one nobody would guess: the platform
documents that after an upgrade *"you need to disable and then re-enable data backup"*, because it
*"only encrypts backups with a client-side secret after informing users"* — and whether production
upgrades re-prompt is **not documented**. The evidence marks that silence rather than filling it.

**Decided: Auto Backup stays on, and the residual is recorded rather than mitigated.**

**Rejected: the platform's declarative refusal** — a manifest flag suppressing backup unless
client-side encryption is available. It would mean a user with no lock screen, or on an API 24–27
handset, **gets no backup at all**. That spends protection against *loss* to buy confidentiality this
specification has conceded at every other artifact, and it inverts
[ADR-0016 §1](0016-backup-and-restore.md)'s premise that loss is the enemy and that sync does not
discharge backup. It is also not known to be implementable here: the Android 12+ form requires a
manifest attribute the crate behind ADR-0003's Gradle-free packaging has no field for, which the
evidence flags as needing a build test rather than settling.

**Three things make this residual genuinely worse than §5's, and they are stated plainly rather than
balanced away:**

- **It is not elective.** §5's exposure follows from enrolling in sync. This one is a platform default
  that reaches a user who chose nothing.
- **§7's disclosure clause cannot reach it.** There is no enrolment moment in a platform default, so
  the defence this ADR relies on for the published log is structurally unavailable here. Recorded as a
  gap, not closed.
- **Deletion is worse than for the published log.** §5's route is per-application and removes the
  folder on uninstall. A device backup has **no documented per-application deletion path**, is removed
  only as a whole device backup (or by disabling backup entirely, or by an inactivity auto-erase), and
  **survives uninstall** — where the application data folder does not.

**One thing that looks worse and is not.** [ADR-0013 §9](0013-the-sync-transport.md) deliberately puts
the refresh token *inside* the backup set, so in the operator-readable case the backup carries a bearer
credential as well as the collection. The credential's scope reaches that **same** operator's
application data folder — whose contents that operator already holds under the same keys. It is not a
new party and not a wider radius. It is, however, the reason this section sits in an ADR about
protection rather than in a footnote: two decisions taken separately for good local reasons
([ADR-0013 §9](0013-the-sync-transport.md)'s convenience on restore,
[ADR-0016 §7](0016-backup-and-restore.md)'s protection against loss) compose into a third fact neither
of them states.

### 7. The residual is disclosed once, at enrolment

A concession has two possible homes — recorded in a specification the user never reads, or said to the
user. This one is said.

**Enrolment gains a clause stating what leaves the device and where the user removes it.** It joins the
sentence [ADR-0015 §7](0015-the-sync-experience.md) already requires — the one that ends enrolment by
stating *"the first device here"* versus the devices it met — which
[ADR-0019 §1](0019-naming-the-account-at-enrolment.md) has since prefixed with *"Connected as …"*.

**That makes three facts at one moment, and the accumulation is deliberate rather than unnoticed.**
Each answers a different question — *which account*, *am I alone here*, *what leaves this device* — and
each is unavailable at any later point for its own reason. This is the natural place for the count to
creep, so the bar is stated: **a fourth clause needs a fact the user cannot obtain afterwards and
cannot act on afterwards.** Anything failing that belongs in sync settings or nowhere.

**This is not a third speaker, and [ADR-0015 §5](0015-the-sync-experience.md) is not bent — on that
ADR's own reasoning rather than on a distinction invented here.** §5 holds the number of things that
may speak about sync at two: a dead grant, and
[ADR-0004 §8](0004-the-review-event-log.md)'s clock-skew warning. **ADR-0015 §7 already carves the
exception and states its test**, for the enrolment sentence it requires:

> *"This is the exception to §5's* only two things speak *rule and does not widen it: it is not a
> resting notice but the immediate result of an action the user just took, in the flow they took it
> in."*

The disclosure clause meets that test exactly — it is a consequence of enrolling, stated in the
enrolment flow, once. It is not a resting notice, it never reappears, and nothing here licenses one.

The reason it is said at all is consistency with how this design handles its other invisible failure.
ADR-0015 §7's whole argument is that when an exposure cannot be detected later, **one sentence at the
moment of choice is the entire defence**, and it warns that removing that line as redundant removes
the only guard. A user cannot subsequently discover that their history rests in plaintext on a
provider's disks by using the application, because — by
[ADR-0015 §5](0015-the-sync-experience.md)'s own design — the application never speaks about sync at
rest, and by [ADR-0013 §3](0013-the-sync-transport.md) the folder is hidden and cannot be browsed to.
The same shape of gap gets the same shape of defence.

**Minimal disclosure still binds** ([ADR-0016 §11](0016-backup-and-restore.md)): the clause states the
fact and the removal route, and never auto-populates or invents an identity to state it about. As
[ADR-0019 §6](0019-naming-the-account-at-enrolment.md) observes, that rule governs *artifacts that
travel* and was never reaching a sentence on the user's own screen — so it is satisfied here without
being stretched.

**Rejected: recording the residual without telling anyone.** Defensible — the consent screen already
names the scope, and a second app-authored warning about a folder categorised non-sensitive is the kind
of sentence that makes users believe something is wrong when nothing is. It loses to the consistency
argument above, but the risk it names is real, which is why the requirement is *one clause at one
moment* rather than a surface.

**Rejected: persisting the disclosure in sync settings**, which is what
[ADR-0019 §7](0019-naming-the-account-at-enrolment.md) chose for the account address — and the contrast
is instructive rather than inconsistent. ADR-0019 persists because **the failure it diagnoses is
discovered months later**, when a second device disagrees, so a message shown once is gone at the
moment of use. This clause has the opposite shape: **the only moment it can change anything is before
the grant exists.** Afterwards the actionable fact is not *"this is disclosed"* but *"here is how to
remove it"* — and [ADR-0015 §10](0015-the-sync-experience.md) already requires sync settings to carry
exactly that, permanently. The durable half is therefore already persistent, and duplicating the rest
would be the resting notice §5 forbids.

### 8. What reopening this would require

Recorded so that a future reader knows what changed rather than re-deriving it. This decision rests on
three things, and it survives until one of them stops being true:

1. **No server, no account, no escrow.** This is fixed by the destination. A server makes key recovery
   possible and makes §3's loss argument evaporate — but it also redraws the destination, so it returns
   as a fresh effort, not as an amendment here.
2. **A secret typeable on the handset is a short numeric one.** [`AGENTS.md`](../../AGENTS.md)
   client-stack rule 8 states the input limitation is winit's and cannot be fixed in this repository. If
   a platform text-input path arrives, §3's *second* half weakens — but its first half, the loss
   asymmetry, does not, and that half alone still refuses.
3. **The material is the user's own study content.** Not credentials — the archive carries none
   ([ADR-0016 §2](0016-backup-and-restore.md)) — and not third-party data. A feature that puts anything
   else into `collection.db` reopens this ADR by making its asset valuation wrong.

## Amendments to accepted ADRs

- **[ADR-0007](0007-the-local-store.md)** — its open item *"Encryption of the store at rest"* is
  **discharged**: the store is not encrypted, by §3 and §4 here. Its owner column no longer points at
  map fog.
- **[ADR-0013 §9](0013-the-sync-transport.md)** — its closing deferral, *"the map's Local encryption /
  device passcode fog covers both uniformly if it ever lands"*, is discharged. It landed, and it says
  no. §9's *theatre* reasoning is not merely upheld but generalised by §4 above, and it now carries a
  consequence §9 did not claim: because the store is never encrypted, the credential's plain file is
  permanent rather than provisional.
- **[ADR-0015 §7](0015-the-sync-experience.md)** — enrolment gains a further closing clause, stating
  what leaves the device and where it is removed (§7 above). **Its existing sentence — *"the first
  device here"* versus the devices it met — is untouched and still the defence against a wrong-account
  enrolment**, which is a different failure with a different remedy. With
  [ADR-0019 §1](0019-naming-the-account-at-enrolment.md)'s *"Connected as …"* this makes three facts at
  one moment; §7 above states the bar a fourth would have to clear.
- **[ADR-0019 §6](0019-naming-the-account-at-enrolment.md)** — unchanged, and brought inside this
  ADR's scope rather than amended. The account address it places in the credential file is now covered
  by §3 and §4's refusals like everything else in that file, and it is named in §1's artifact table so
  the enumeration stays complete. Its reasoning that the address is a property of the grant rather than
  of the collection — and so reaches no export profile and never settles — is untouched and is what
  keeps it out of artifact (iii).
- **[ADR-0019 §7](0019-naming-the-account-at-enrolment.md)** — unchanged. §7 above records why this
  ADR's disclosure clause is *not* persisted in settings while ADR-0019's address is: the two defend
  against failures discovered at opposite times.
- **[ADR-0015 §5](0015-the-sync-experience.md)** — unchanged, and explicitly so. The disclosure clause
  is admitted under the exception ADR-0015 §7 already carves and by the test it already states — the
  immediate result of an action the user just took, in the flow they took it in — not by a new
  distinction drawn here. **Nothing in this ADR licenses ambient speech about sync**, and the count of
  things that may speak at rest stays at two.
- **[ADR-0015 §10](0015-the-sync-experience.md)** — unchanged in substance, **strengthened in
  obligation**. Its requirement that sync settings name the provider's connected-applications route,
  and the name this application appears under, was offered as a courtesy beside a refused control. §5
  above makes it the user's only route to remove plaintext a third party holds indefinitely, so it may
  not be dropped as a nicety. §10's own warning that the menu wording is a third party's UI, *"verified
  at implementation, never pinned in this document"*, applies unchanged.
- **[ADR-0016 §8](0016-backup-and-restore.md)** — its deferral is discharged and its conclusion upheld,
  but **its reasoning is not the reason**. §8 rests on the ASCII input limitation; §3 above rests on the
  loss asymmetry and the offline-guessing arithmetic, either of which alone refuses. §8's own caveat —
  that ADR-0013 §9's theatre argument does not transfer to a travelling artifact — remains correct and
  is answered instead by §4's circularity argument, which does transfer. Its open item *"Whether the
  archive should ever be encrypted"* is discharged.
- **[ADR-0016 §7](0016-backup-and-restore.md)** — its decision to keep Auto Backup on is **upheld and
  given a second justification it did not have**. §7 kept it on to protect against loss; §6 above adds
  that refusing backup where client-side encryption is unavailable would spend that protection to buy
  confidentiality this specification concedes everywhere else. **What §7 did not state, and now must be
  read with, is that the payload is not guaranteed to be unreadable by the provider** — a lock screen
  and Android 9+ are required, and this project's `min_sdk_version = 24` puts part of its own supported
  range outside that (§6).

## Consequences

- **Nothing is implemented as a result of this ADR except one clause of copy.** The decision is a set of
  refusals; its only positive obligation is the disclosure clause of §7 above, which lands in the
  enrolment flow beside the sentence [ADR-0015 §7](0015-the-sync-experience.md) already requires.
- **The map's *Local encryption / device passcode* fog closes**, and with it the last deferral shared by
  three accepted ADRs.
- **A rule enters [`AGENTS.md`](../../AGENTS.md)** — no user-supplied secret — because a decision made
  entirely of refusals is the kind that erodes when an agent meets it without the reasoning.
- **`leitner-store`, `leitner-sync` and `leitner-export` gain no dependency.** No cryptography crate
  enters this workspace, which keeps [ADR-0009](0009-crate-and-workspace-layout.md)'s dependency
  discipline intact and removes a class of platform-conditional code from the two crates that hold a
  platform seam.
- **The desktop's exposure to adversary A is now conceded in writing**, where before it was conceded in
  passing by [ADR-0013 §9](0013-the-sync-transport.md) as a premise for a different argument.
- **The user has one route to remove published data and it is not in this application** (§5). Anyone
  implementing a settings screen who thinks that absence is an oversight should read
  [ADR-0015 §10](0015-the-sync-experience.md) first.
- **Part of this project's own supported Android range cannot get the backup guarantee** (§6), and no
  code change fixes it. Raising `min_sdk_version` to 28 would — that is a reach-versus-confidentiality
  trade this ADR does not make, and it is recorded so the option is visible rather than rediscovered.
- **The residual for adversary D′ has no user-facing statement anywhere** (§6), because a platform
  default has no moment at which to make one. This is the only gap in this ADR that is left open rather
  than argued shut.

## Glossary

- **At rest** — an artifact sitting on storage while the application is not using it, as opposed to in
  transit. All four artifacts in §1 travel over an encrypted channel; that is not what this ADR is about.
- **Theatre** — a protection whose key is reachable by whoever can reach the thing it protects. Coined
  by [ADR-0013 §9](0013-the-sync-transport.md); generalised by §4.

## Open items handed onward

| Item | Owner |
|---|---|
| **Nothing about encryption.** §8 states the three conditions under which this returns; none is a ticket | — |
| Whether the 25 MB Auto Backup quota is measured **before or after compression** — the platform documentation is silent, and the answer moves [ADR-0016 §7](0016-backup-and-restore.md)'s nine-month cutoff estimate by an order of magnitude | [ADR-0016](0016-backup-and-restore.md); surfaced by this ticket's evidence, not owned here |
| **Quota failure is silent** — the whole package is rejected, signalled only by a callback needing a dex this project does not ship and by two log lines, with no documented user notification | [ADR-0016](0016-backup-and-restore.md) |
| Whether the manifest attribute behind the platform's declarative backup refusal is expressible under ADR-0003's Gradle-free packaging — needs a build test, and is only interesting if §6 is ever reopened | Implementation |
