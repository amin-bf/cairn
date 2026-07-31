# Sync

Publishing this device's rows to storage nobody here owns, and reading back what other devices
published. The mechanism only — *when* sync runs and how divergence is shown to the user are settled
by [ADR-0015](../../../docs/adr/0015-the-sync-experience.md) and live in `ui`. Two of its answers
reach back into this crate's assumptions: **there is no background sync on either platform** (§2), so
nothing here is ever entered from a scheduler; and **the app never rests in a "behind" state** (§4),
because the handshake that discovers it and the fetch that fixes it are one operation.

Depends on `log` for row identity and the version summary, and on `content` transitively. Does not
depend on `replay` or `ui`: nothing here knows what a card is.

**Bound by** [ADR-0013](../../../docs/adr/0013-the-sync-transport.md), whose glossary this file
supersedes; also by [ADR-0004 §2](../../../docs/adr/0004-the-review-event-log.md) (row identity and
the version summary), [ADR-0004 §7](../../../docs/adr/0004-the-review-event-log.md) (the mutable
surface and its stamps), [ADR-0004 §10 and §11](../../../docs/adr/0004-the-review-event-log.md) (the
log is never compacted; the interchange form is relayed byte for byte) and
[ADR-0007](../../../docs/adr/0007-the-local-store.md) (the local copy is authoritative).

## The two rules that fail silently

**The `log` and `state` roll-ups are opposite.** `…/log/` merges **losslessly** — every row survives,
because ADR-0004 §10 says the log is never compacted. `…/state/` merges **lossily by design** —
only the winning stamp per key is kept, because that is what settling means. Applying the state rule
to the log destroys review history and nothing downstream can detect it.

**Write the merged object before deleting anything, and delete only what it covers.** Deletion in the
application data folder is permanent — there is no trash — so ordering is the only protection. A
reader that fetches a key deleted since it listed gets `404`; the correct response is to list again,
not to attempt recovery.

## Language

**Remote**:
The storage the devices share. A **rendezvous point, never a system of record** — deleting it costs
one republish and no data, because every device holds the whole log locally.
_Avoid_: Server (there is none), backup (it is not one, and #37 owns that), the cloud.

**Namespace**:
The keys one writer owns. **One writer, one namespace**, for the collection's lifetime — the
invariant the whole transport rests on, because it is what makes every key single-author and so
removes any need for compare-and-swap.

**Object**:
One immutable blob at one key. Written once and never modified; only roll-up deletes. Objects carry
compressed interchange lines and nothing else.
_Avoid_: File, blob, chunk.

**Segment**:
The object one publish writes — exactly the rows produced since the last publish. The smallest
object; every larger one is a roll-up of these.

**Roll-up**:
Merging `K` adjacent objects into one covering their union, then deleting the merged ones. Bounds
live objects so a **cold start** stays cheap. Triggered by count, never by a clock.
_Avoid_: Compaction — which in this repository means the log-trimming that ADR-0004 §10 forbids, and
the two must never be confused.

**Cold start**:
A device fetching everything: newly enrolled, or recovering from an expired change cursor. The
operation that decides how objects are sized, because its cost is one request per object.

**Publish**:
Writing this device's new rows as a segment. Never rewrites, never touches another writer's
namespace.

**State stream**:
The per-writer change stream of stamped assignments to the mutable surface. Rolls up to *the latest
value this writer assigned to each key it touched*, which is exactly a per-writer snapshot — so
snapshot and change stream are the same thing paid for at different times.

**Enrolment**:
Granting one device access to the remote, once. Uses the device flow: a short code entered on
whatever device the user likes, so no credential is ever typed into this application.
_Avoid_: Login, sign-in, pairing — there is no account of ours and no device-to-device step.

**Grant**:
What enrolment obtains and what revocation removes. **Revocation is all-or-nothing**: every device
holds a token issued against one client id, so revoking for a lost device logs out all of them.
