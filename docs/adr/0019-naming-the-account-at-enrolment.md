# ADR-0019: Naming the account at enrolment

- **Status**: Accepted
- **Date**: 2026-07-31
- **Resolves**: [Decide: whether the enrolment screen names the account](https://github.com/amin-bf/leitner/issues/59)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/leitner/issues/1)
- **Related**: [ADR-0013 §3 §8 §9 §10](0013-the-sync-transport.md) (the non-sensitive scope, the
  device flow's scope allowlist, the credential file, the per-(account, application) folder),
  [ADR-0015 §1 §4 §7 §12](0015-the-sync-experience.md) (what may speak about sync, the resting
  surface, enrolment, the settings screen),
  [ADR-0016 §10 §13](0016-backup-and-restore.md) (the identity check, and reachability-not-identity),
  [ADR-0004 §7](0004-the-review-event-log.md) (the mutable surface, and what this value is kept off),
  [ADR-0010](0010-leeches.md) (*detect and surface, never intervene*)

## Context

Two accepted ADRs record this item and both say it is owned by neither of them —
[ADR-0015](0015-the-sync-experience.md) *Open items*: **"Unowned — needs a decision before enrolment
ships"**; [ADR-0016](0016-backup-and-restore.md) *Open items*: **"Not scheduled"**. It is the only
decision either ADR left standing with no owner.

The setup is settled and not reopened here. [ADR-0013 §10](0013-the-sync-transport.md) scopes the
application data folder per (account, application), so a device pointed at the **wrong account** gets
an empty folder — indistinguishable from being the first device to enrol.
[ADR-0015 §7](0015-the-sync-experience.md) makes enrolment end by stating what it found, and records
that this one sentence is the whole defence and the weakest point in that ADR.
[ADR-0016 §13](0016-backup-and-restore.md) then closed off the alternative structurally: in a
wrong-account enrolment **every collection id agrees**, because all the devices hold the same
collection and merely cannot see each other, so **the failure is reachability, not identity** and no
identity check could ever have caught it.

**What this ADR does not inherit is the framing all three documents put on what remains.** The
ticket, ADR-0015's *Open items* row and ADR-0016's *Open items* row each describe naming the account
as *"the only thing that would detect the case"* and *"the only lever left"*. §2 finds that false, and
the correction is what makes the decision defensible rather than merely sympathetic.

## Decision

### 1. The enrolment screen and sync settings name the account

> **Enrolment states the account it connected as, and sync settings keeps it. It appears on the
> enrolment screen and in sync settings, and nowhere else.**

[ADR-0015 §7](0015-the-sync-experience.md)'s closing sentence gains a clause:

> *"Connected as `you@example.com`. This is the first device here."*
>
> *"Connected as `you@example.com`. Found 2 other devices: Laptop, Pixel."*

And [ADR-0015 §12](0015-the-sync-experience.md)'s settings screen gains a row, beside the device list
that already sits there.

The **"and nowhere else"** clause is [ADR-0015 §3](0015-the-sync-experience.md)'s rule reused
verbatim, for the same reason it gave: this is the kind of string later copied into a tooltip, a
header and an about box by someone who was not in this conversation.

### 2. The value is diagnosis, not detection — and three documents claim otherwise

**ADR-0015 §7's sentence is a real lever, not a null one.** Walk the cases it actually covers:

| case | what §7 says | caught? |
|---|---|---|
| Second device, wrong account | *"This is the first device here"* | **Yes** — said to a user who knows they enrolled another device |
| Re-enrolment after [ADR-0013 §3](0013-the-sync-transport.md)'s 7-month refresh-token death, wrong account | *"This is the first device here"* | **Yes** — it demonstrably worked before |
| Any account, folder non-empty | *"Found 2 other devices"* | **Not applicable** — the account is provably right, because the peers are visible |

So §7 **detects**, at zero scope cost, in the cases that matter. Describing the account name as the
only thing that would detect the case is wrong in all three documents that say it, and an ADR built on
that premise would be arguing for a redundancy.

**What §7 cannot do is diagnose.** The user has to translate *"this is the first device here"* into
*"therefore I signed in on the wrong account"*, and that inference is not available to most people.
Every competing hypothesis is more intuitive: the folder was cleared, the other device was reset, the
application forgot, sync is broken.

**This is where it stops being imperfect and becomes actively harmful.** Each of those wrong
hypotheses routes to a **repair action** — sync again, disconnect and reconnect, reinstall, restore
from an archive — and **not one of them can fix an account mismatch**. Every attempt fails. Every
failure raises the user's confidence that the collection is actually gone. A misdiagnosis here does
not merely delay the fix; it walks the user toward believing they have lost their data, on a system
whose central promise ([ADR-0016 §2](0016-backup-and-restore.md)) is that they cannot.

Naming the account collapses the diagnosis into something **read rather than inferred**: an address
the user does not recognise, shown next to the sentence that puzzled them.

> **The account name is bought for diagnosis. Detection was already paid for.**

### 3. The application can never compare accounts; only a person can

[ADR-0016 §13](0016-backup-and-restore.md)'s reachability argument generalises further than that ADR
claimed. It concluded that no *collection identity* check can catch the case. The stronger statement
is that **no check of any kind can**, including one on the account address itself:

> In a wrong-account enrolment the device sees an empty folder. There is no peer, no namespace and no
> published byte to compare against. **The only comparand that exists is the user's own memory of the
> previous enrolment.**

**Rejected, and structurally void rather than merely expensive: publishing the address so devices can
cross-check.** Writing *"this device connected as X"* into the application data folder cannot help,
because the device that needs to read it is looking at a **different folder**. It is unreadable
exactly when it is needed. This is the tempting design and it fails on both axes at once — void
benefit, and a real disclosure (§6).

Two consequences follow, and both bound what the feature may become:

- **The account name can never be an error, a block or a warning. It is a statement.** That places it
  in *detect and surface, never intervene* — [ADR-0010](0010-leeches.md), then
  [ADR-0014 §2](0014-when-parameter-optimisation-runs.md), then
  [ADR-0015 §1](0015-the-sync-experience.md), and now the fourth time on this map.
- **There is no wrong account in the absolute — only an account that differs from the one the other
  device used.** A first device enrolled on an odd account is harmless; nothing is broken until a
  second device disagrees. What is being protected is **consistency across enrolments**, not
  correctness of one. That is why human recognition is not a weak substitute for a check here: it is
  the only thing that can span two enrolments months apart on two devices.

### 4. The scope is `openid email`, and it is `email` rather than `profile`

Both source ADRs write *"the `email` or `profile` scope"* as though the two were interchangeable.
**They are not, and the choice is not neutral.**

- **`email`** yields the address — the entire diagnostic.
- **`profile`** yields display name and picture. **Neither has any diagnostic value**, and both are
  ambient identity of exactly the kind [ADR-0016](0016-backup-and-restore.md)'s minimal-disclosure
  rule exists to keep out.

> **The requested scope set is `openid email drive.appdata`. `profile` is declined.**

`openid` is not optional padding: the provider's OpenID Connect specification requires the scope
parameter to begin with `openid` before `email` or `profile` may be included
([OpenID Connect](https://developers.google.com/identity/openid-connect/openid-connect)). All three
are on the limited-input-device flow's published allowlist, which
[ADR-0013 §8](0013-the-sync-transport.md) already verified — that allowlist is exactly `email`,
`openid`, `profile`, `drive.appdata`, `drive.file` and two video scopes
([OAuth 2.0 for TV and Limited-Input Device Applications](https://developers.google.com/identity/protocols/oauth2/limited-input-device)).

**It costs one request, not zero, and that was worth checking.** The limited-input-device flow's
documented token response carries `access_token`, `expires_in`, `refresh_token`, `scope` and
`token_type` — **no `id_token`**. So the address does not arrive with the grant; it is fetched with a
single `GET` to the UserInfo endpoint (`https://openidconnect.googleapis.com/v1/userinfo`) using the
access token, once, at enrolment. That is one request on a screen that is already doing network, and
it needs no platform capability, so [ADR-0013 §8](0013-the-sync-transport.md)'s prize — a flow with no
platform surface, no `#[cfg(target_os)]`, no third function in a platform seam
([ADR-0009 §4](0009-crate-and-workspace-layout.md)) — is untouched.

### 5. The property ADR-0013 §3 chose the scope for survives, and this was checked rather than assumed

[ADR-0013 §3](0013-the-sync-transport.md) chose this backend substantially because the scope is
**non-sensitive**, and it was explicit that the value is not the saved paperwork: *"No verification
means **no verification-time endpoint** — no domain-ownership proof, no hosted policy a reviewer
fetches, no callback. Every one of those would have been a server, which the destination forbids."*

**Adding scopes is exactly the move that could have destroyed that, which is why the ticket required
it be established rather than assumed.** It does not:

> *"If your app utilizes only **non-sensitive** scopes, it is not mandatory for your app to complete
> the app verification process."*
> — [OAuth app verification](https://support.google.com/cloud/answer/13463073)

`openid`, `email` and `profile` are non-sensitive. The provider privileges that exact subset
elsewhere, too: it is the one scope set whose presence exempts a project in *Testing* publishing
status from the 7-day refresh-token expiry that
[ADR-0013 §3](0013-the-sync-transport.md) records as a console trap
([Using OAuth 2.0 to Access Google APIs](https://developers.google.com/identity/protocols/oauth2)).

> **No verification, no verification-time endpoint, no server. ADR-0013 §3's load-bearing property is
> confirmed rather than spent.** Its ceiling row — *none, because the scope is non-sensitive* — still
> reads true with the wider set.

**Honest addition to [ADR-0013 §8](0013-the-sync-transport.md)'s recorded phishing shape.** That
section bounds the blast radius of a stolen device code at *"our own application data folder and
nothing else"*. With `email` granted, the radius grows by the victim's address — **a fact an attacker
running that attack already had**, since the attack requires talking to the person. Recorded because
the section is an honesty commitment, not because it changes the assessment.

### 6. The address lives with the credential, so it reaches no artifact

**The address is a property of the grant, not of the collection.** It describes which account *this
device* holds a token for. Another device on the same account derives its own from its own grant and
never needs to be told. It fails [ADR-0004 §7](0004-the-review-event-log.md)'s membership test one
level further out than [ADR-0010 §5](0010-leeches.md)'s suspension or
[ADR-0011 §7](0011-new-card-rate-and-daily-limits.md)'s new-card rate did — those were at least
collection state that did not belong in the log. This is not collection state at all.

> **It is stored beside the credential, in [ADR-0013 §9](0013-the-sync-transport.md)'s plain file in
> application-private storage. Never on [ADR-0004 §7](0004-the-review-event-log.md)'s mutable
> surface. Disconnect deletes it with the grant, because it is part of the grant.**

**The export question then dissolves rather than needing a policy**, which is this map's recurring
shape — [ADR-0008](0008-the-deck-export-format.md) disposed of ADR-0004 §11's writer-id disclosure the
same way. [ADR-0016 §4](0016-backup-and-restore.md)'s `collection` profile is *everything that
settles, plus the log, **minus device identity and credentials***, so a value living with the
credential is excluded **without a clause naming it**, in both profiles.

**The trap this avoids is worth naming, because the mutable surface is where it would naturally have
gone.** There, the address would be carried into every `.lcoll` (that surface is *"everything that
settles"*) **and** published to the remote ([ADR-0013 §7](0013-the-sync-transport.md)) — for the
benefit §3 already showed to be void.

**And the distinction that resolves the apparent conflict with minimal disclosure:**
[ADR-0016 §11](0016-backup-and-restore.md)'s rule — *"no author name, no device label, no ambient
identity ever auto-populated"* — is about **artifacts that travel**. It governs *auto-populating a
field in a file that goes somewhere*. **A user's own address on their own settings screen is not a
disclosure surface.** The rule is untouched; it was never reaching this.

### 7. It is not a third speaker, because it states a fact and makes no claim

[ADR-0015 §1](0015-the-sync-experience.md)'s rule — exactly two things may speak about sync, a dead
grant and [ADR-0004 §8](0004-the-review-event-log.md)'s clock-skew warning — is the rule that ADR
itself predicted would erode first. It is not eroded here, and the test is
[ADR-0015 §4](0015-the-sync-experience.md)'s own title: **the resting surface states a fact, never a
claim.**

Everything §1 forbids is a **claim about sync state the application cannot back** — a checkmark,
*"in sync"*, *"up to date"*, a persistent status indicator. *"Connected as X"* makes **no claim about
sync state whatsoever**. It is the same category as the standing configuration facts
[ADR-0015 §12](0015-the-sync-experience.md) already holds: the device list with its per-writer *"last
published ⟨when⟩"*, the disconnect control, the revocation explanation, the desktop-authoring
statement.

**Rejected: showing it once at enrolment and not persisting it** — which is how the ticket and both
*Open items* rows framed the safe version, and which would have wasted the purchase. **The failure
this feature exists to diagnose is discovered months later**, when a second device disagrees. A
message shown once at enrolment is gone at exactly the moment of discovery, and would defend only
against catching the mistake in the act — the case §2 shows ADR-0015 §7 already covers for free.

It also destroys the **only comparison that exists anywhere in this design**. §3 established that the
application can never compare accounts. A *person* can: open settings on the phone and on the laptop,
read two different addresses. That comparison lives or dies on the address being in settings.

### 8. Decided now because it is free now

Adding a scope after enrolment ships is not a code change — it is a **re-consent for every device
already enrolled**, each one walking the device flow again, including devices in drawers whose owners
will read it as a fault. Nothing is implemented and no grant exists, so the wider scope set costs
exactly one line in a constant today.

This is [ADR-0017 §7](0017-card-slots.md)'s argument on a different surface, and the second time this
map has taken a decision early on the ground that its cost is not constant over time.

## Amendments to accepted ADRs

- **[ADR-0015 §7](0015-the-sync-experience.md)** — *"**Only `drive.appdata` is requested** — not
  `email` — so the consent screen asks for exactly one thing"* is **reversed**. The requested set is
  `openid email drive.appdata`; the consent screen asks for two things and the screen still states the
  scope in plain words. Its closing sentence gains the *"Connected as …"* clause (§1). Its claim that
  naming the account is *"the one thing that **would detect** it"* is **corrected to diagnose** (§2).
- **[ADR-0015 §12](0015-the-sync-experience.md)** — the settings screen gains a row: **the connected
  account address**, sourced from §1 above.
- **[ADR-0015](0015-the-sync-experience.md) *Open items*** — the row *"Whether to name the account on
  the enrolment screen … Unowned — needs a decision before enrolment ships"* is **discharged** by this
  ADR. Its §16 item 2 note, which ends *"the live trade it leaves … is in Open items"*, is likewise
  discharged.
- **[ADR-0016 §13](0016-backup-and-restore.md)** — its reachability finding is **widened**: it rules
  out not only a collection-identity check but **any** application-side check, including one on the
  account address (§3). Its conclusion that ADR-0015 §7's sentence stands is unchanged; what changes is
  that the sentence is no longer the *whole* defence, having gained a diagnostic beside it.
- **[ADR-0016](0016-backup-and-restore.md) *Open items*** — the row *"Whether the enrolment screen
  should name the account it connected as … Not scheduled"* is **discharged**.
- **[ADR-0013 §8](0013-the-sync-transport.md)** — the scope set this project requests is `openid email
  drive.appdata` rather than `drive.appdata` alone; all three were already verified as on the flow's
  allowlist there. Its phishing-shape blast radius gains the victim's address (§5).
- **[ADR-0013 §9](0013-the-sync-transport.md)** — the credential file gains one field, the account
  address, deleted with the grant on disconnect (§6).

## Consequences

- **A fourth *detect and surface, never intervene*.** The account name can never block, warn or error,
  because §3 makes any check structurally impossible. Anything that later turns it into a warning is a
  defect, and it will look like an improvement.
- **The consent screen asks for two things instead of one**, at the friction point ADR-0015 §7 names as
  the hardest moment in the feature. Accepted: the screen states the scope in plain words, and *"see
  your email address"* is a claim a user can evaluate.
- **The address is cached at enrolment and never maintained**, so an account that later changes its
  address shows a stale one. This is correct rather than merely tolerable — the diagnostic answers
  *"what did I enrol as"*, and the stale value is the true answer to that question.
- **Brand verification is unaffected and is not a new cost.** The lighter *brand-verification* process
  is what an application completes to display branding on the consent screen; it is independent of
  scope sensitivity and applied equally before this decision.
- **One more thing lives in the credential file**, which is outside both export profiles and — per
  [ADR-0013 §9](0013-the-sync-transport.md) — deliberately **inside** the Android backup set, the
  opposite side of the line from [ADR-0007 §5](0007-the-local-store.md)'s writer marker. That is the
  right side here and improves the diagnostic rather than merely riding along: a restored phone arrives
  already authorised *and* already stating what it enrolled as, and **replacing a device is one of the
  likelier occasions for a wrong-account re-enrolment**.
- **`leitner-sync` gains one request and one field.** No new crate, no new dependency, no platform
  capability, no `#[cfg]`.

## Open items handed onward

| Item | Owner |
|---|---|
| Exact copy for the *"Connected as …"* line on both surfaces | Implementation; §1 fixes where it appears, not its wording |
| Visual treatment of the account row in sync settings | Out of scope — *the visual design pass*, which [ADR-0006 §10](0006-the-review-session-experience.md) opened and ADR-0010, ADR-0015, ADR-0017 and [ADR-0018](0018-the-card-pane-ordering.md) have joined |
