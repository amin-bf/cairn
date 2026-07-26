# Local-first data with an append-only event log

**Research ticket:** #4 (under wayfinder map #1) · **Date of research:** 2026-07-26
**Question:** What are the established approaches to local-first data with an append-only event log, and which are relevant to a review log that must merge across devices?

This is a **research** note. It gathers facts and sharpens trade-offs; it does not pick a design. Every non-obvious claim carries an inline source. Claims I could not source are marked **[inference]**.

Context assumed throughout: Rust, desktop + web (WASM) + Android, **no server of our own**, sync transport deferred but not foreclosed, one user with a handful of devices, tens to low hundreds of review events per day, real clock skew between devices.

---

## Summary of findings

The facts that most constrain the decision:

1. **Only vector clocks / version vectors answer "am I ahead of or behind that other device?"** Scalar clocks (Lamport, HLC, ULID, UUIDv7) give a *total order* but cannot distinguish "behind" from "concurrent". This is stated directly in the distributed-systems literature: "The two-way causality property is provided by vector clocks […] However, the size of vector clocks is O(n) where n is the number of processes" ([Kulkarni, Appleton & Nguyen, ICDCN 2022](https://arxiv.org/pdf/2104.15099)), and Lamport himself notes the converse of the Clock Condition cannot hold ([Lamport 1978](https://lamport.azurewebsites.net/pubs/time-clocks.pdf)). O(n) is *cheap* here: n ≈ 5 devices, not 5,000.

2. **Every production local-first library independently converged on the same primitive:** a per-device monotonic counter, and a version vector `{deviceId → counter}` as the sync handshake. Yjs calls it a *state vector* — "a map of clientId ⇒ clock" ([Yjs INTERNALS.md](https://github.com/yjs/yjs/blob/main/INTERNALS.md)). Loro calls it a *Version Vector* over `OpId = (PeerId, Counter)` ([Loro docs](https://loro.dev/llms-full.txt)). Riak calls it *causal context* ([Riak docs](https://docs.riak.com/riak/kv/latest/learn/concepts/causal-context/index.html)). **This primitive is available without a CRDT library and without a server.** It is the cheapest thing that satisfies standing constraint 3.

3. **Anki — the closest prior art — merges its review log by set union with no conflict handling at all, and merges everything else by last-write-wins on a timestamp.** `merge_revlog` is literally `INSERT OR IGNORE` per entry ([`chunks.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/chunks.rs), [`add.sql`](https://github.com/ankitects/anki/blob/main/rslib/src/storage/revlog/add.sql)). Cards, notes, decks, notetypes and deck config are LWW on `mtime` ([`changes.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/changes.rs)). Deletions are explicit tombstones in a `graves` table — **except review-log deletions, which Anki cannot sync at all**: "Anki can not sync revlog deletions" ([`storage/revlog/mod.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/storage/revlog/mod.rs)).

4. **Anki's ahead/behind test is a scalar `mod` timestamp comparison — and it is exactly the failure mode constraint 3 exists to avoid.** `compared_to_remote` returns `NoChanges` if `remote.modified == local.modified`, `FullSyncRequired` if `remote.schema != local.schema`, else `NormalSyncRequired`, with `local_is_newer: local.modified > remote.modified` ([`meta.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/meta.rs)). It works only because a *server* owns a single authoritative USN counter; the client's own USN is hardcoded to `-1` ([`storage/sync.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/storage/sync.rs)). **With no server, Anki's scheme is unavailable to us.**

5. **Anki refuses to sync if clocks differ by more than 300 seconds**: `if delta.abs() > 300 { … SyncErrorKind::ClockIncorrect }` ([`status.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/status.rs)). A mature SRS app treats clock skew as fatal rather than mergeable. Note it compares against a *server's* clock — a serverless design has no such reference point.

6. **Review events merge commutatively as a set, but the projection over them does not commute.** FSRS's input is "a list of reviews for a card, **in chronological order**", each carrying `delta_t` = days since the previous review ([fsrs-rs `dataset.rs`](https://github.com/open-spaced-repetition/fsrs-rs/blob/main/src/dataset.rs)). So the *log* is order-insensitive but the *replay* is strictly timestamp-ordered. Clock skew therefore corrupts derived intervals even when the merge itself is clean. **This is the sharpest tension in the whole map.**

7. **"Raw grades and timestamps" is not sufficient for replay.** Anki's `reviews_for_fsrs` must filter by *event kind*: it skips cramming entries, discards history prior to a `Reset`, distinguishes manual reschedules from user grades, and honours an `ignore_revlogs_before` cutoff ([`fsrs/params.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/scheduler/fsrs/params.rs)). Its `RevlogReviewKind` enum has six variants: `Learning, Review, Relearning, Filtered, Manual, Rescheduled` ([`revlog/mod.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/revlog/mod.rs)). Constraint 1 needs widening: the log must carry raw grades, timestamps **and event kind**.

8. **Log size is a non-issue at this volume; CRDT history size might not be.** At 200 reviews/day, an Anki-shaped revlog row (38 bytes of fields) plus device id, counter and HLC is ~58 bytes/event → **~4.2 MB/year, ~42 MB/decade** [inference: my arithmetic over the sourced field list and stated volume]. By contrast Automerge "assign[s] a unique ID to every keystroke" and **cannot discard history at all**: "Discarding the history is not currently possible without replacing the document" ([pvh, automerge#799](https://github.com/automerge/automerge/issues/799)).

9. **CRDT actor/peer IDs are deliberately *session* identities, not device identities.** Loro: "Never use user IDs as peer IDs", "Don't assign fixed PeerIDs to users or devices", and a **new** peer ID is generated for each `LoroDoc` instance even when loading the same document ([Loro PeerID Management](https://loro.dev/docs/concepts/peerid_management)). Automerge: "By default automerge will generate a random actor ID for you"; "Actor IDs must not be used in concurrent threads of executiong \[sic\] - all changes by a given actor ID are expected to be sequential" ([`javascript/src/index.ts`](https://github.com/automerge/automerge/blob/main/javascript/src/index.ts)). **Adopting a CRDT does not give you a stable device id — you still need your own.**

10. **Loro is the only one of the three CRDT libraries with a shipped history-truncation story**, and it comes with a hard constraint: "Peers can only sync if they have versions after the shallow snapshot point" ([Loro Shallow Snapshots](https://loro.dev/docs/concepts/shallow_snapshots)). Shallow snapshots are "typically 70-90% smaller than full snapshots" (same source).

11. **wasm32 verification (compiled locally, 2026-07-26, cargo 1.97.0, target `wasm32-unknown-unknown`):** `uuid`, `yrs`, `loro`, `redb`, `rusqlite` (bundled) all `cargo check` clean; `ulid`, `uhlc`, `automerge`, `crdts` need `getrandom`'s `wasm_js` backend enabled and then pass; `fjall` fails outright ("unsupported platform" in `lsm-tree`). Details and caveats in §6.

---

## 1. Event sourcing fundamentals

### Log-plus-replay to derive state

The canonical framing is Kafka's: "If we had infinite log retention, and we logged each change […] we could restore to any point in time by replaying the first N records in the log. This hypothetical complete log is not very practical for systems that update a single record many times as the log will grow without bound even for a stable dataset." ([Kafka design docs](https://github.com/apache/kafka/blob/trunk/docs/design/design.md)). Our situation is the *good* case for a pure log: review events are births, not updates — each review is a new fact about a card, never an overwrite of a previous review. So unbounded-growth-per-key does not apply to the review log.

### Snapshotting

The standard pattern: persist derived state periodically, then on load replay only events after the snapshot. Automerge implements exactly this at the storage layer, and its writeup is the most useful primary description of doing it *without* a transactional store:

> "Occasionally automerge-repo will decide that it's time to 'compact' the document, it will take every change that has been written to storage so far […] and combine them into a single snapshot […] Conveniently the set of changes in the document is uniquely identified by the heads of the document. This means that if we use the tuple `(document ID, <heads of document>)` as the key to the storage we know that even if we overwrite data another process has written it must contain the same changes as the data we are writing […] Each process then needs to keep track of every change it has loaded from storage and then when compacting *only delete those changes*."
> — [Automerge storage under-the-hood](https://github.com/automerge/automerge.github.io/blob/main/content/docs/reference/under-the-hood/storage.md)

That is a concurrency-safe compaction protocol requiring no locks — directly reusable for a snapshot-plus-tail event store, CRDT or not.

### Compaction

Kafka's log compaction is the reference design for bounding a keyed log, and its guarantees are worth quoting because they define what compaction may and may not break:

> 1. "Any consumer that stays caught-up to within the head of the log will see every message that is written."
> 2. "Ordering of messages is always maintained. Compaction will never re-order messages, just remove some."
> 3. "The offset for a message never changes. It is the permanent identifier for a position in the log."
> 4. "Any consumer progressing from the start of the log will see at least the final state of all records in the order they were written. Additionally, all delete markers for deleted records will be seen, provided the consumer reaches the head of the log in a time period less than the topic's `delete.retention.ms` setting (the default is 24 hours)."
>
> — [Kafka design docs](https://github.com/apache/kafka/blob/trunk/docs/design/design.md)

Two transferable lessons:

- **Tombstones must outlive the slowest reader.** Kafka's "delete retention point" and its `delete.retention.ms` default of 24 hours exist because "it is possible for a consumer to miss delete markers if it lags by more than `delete.retention.ms`". For us the "slowest reader" is a device that has been in a drawer for a year — so any tombstone GC horizon must be measured in *device-offline duration*, not hours. This is a direct, concrete cost of supporting deletions.
- **Positions must be stable identifiers.** Kafka never renumbers offsets. Anki likewise never changes a revlog id.

### Projection versioning — what happens when you swap the scheduler

Three distinct approaches in the wild:

**(a) Version the projection, run both in parallel.** Marten (a mature .NET event store) has an explicit `ProjectionVersion`: "When deploying projection changes to production without downtime, you can use projection versioning to run old and new projection versions in parallel." Incrementing `ProjectionVersion` creates separate database tables ([Marten rebuilding projections](https://martendb.io/events/projections/rebuilding.html)). Rebuild itself is "tear down existing rows before replaying events", after which "the entire rebuild write path is insert-only".

**(b) Rebuild in place, in a transaction, over a bounded replay window — what Anki actually does.** Anki's `update_memory_state` re-derives FSRS memory state for matching cards from `revlog`, is documented "Should be called inside a transaction", and takes an `ignore_before: TimestampMillis` that bounds how far back replay goes. If params are `None` (user disabled FSRS) it calls `clear_fsrs_data_for_cards` instead ([`fsrs/memory_state.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/scheduler/fsrs/memory_state.rs)). The user-facing controls confirm this is a deliberate product decision, not an implementation detail:

- "If set, cards reviewed before the provided date will be ignored when optimizing FSRS parameters" — useful when "you imported someone else's scheduling data, or have changed the way you use the answer buttons" ([Anki deck options](https://docs.ankiweb.net/deck-options.html)).
- "Reschedule cards on change" is **off** by default: "future reviews will use the new scheduling, but there will be no immediate change to your workload"; and "This option is not recommended when first switching from SM-2" (same source).

→ **Anki's answer to "what happens to derived state when you swap the scheduler" is: recompute the memory state from the log, but *do not* recompute the due dates unless the user opts in.** That is a decision about user experience, not correctness, and it is the single most directly applicable piece of prior art for constraint 1.

**(c) Upcast events, or copy-and-transform the store.** For *event schema* (not projection) evolution the catalogued tactics are "versioned events, weak schema, upcasting, in-place transformation, and copy-and-transform" ([summary of practice](https://event-driven.io/en/how_to_do_event_versioning/); the canonical treatment is Greg Young, *Versioning in an Event Sourced System*). In a distributed local-first setting, in-place transformation and copy-and-transform are unavailable — you cannot rewrite a log that other devices hold. Ink & Switch's Cambria is the local-first-native answer: "an isolated software layer that translates data between schemas on demand", using bidirectional lenses applied "at read time" rather than write time, precisely so that peers running different app versions can interoperate ([Cambria](https://www.inkandswitch.com/cambria/)). Cambria also states the limit honestly: developers must trade off "consistency, conservation, and predictability" and "there are no perfect options".

**Note on state of the art:** Greg Young's book and the Kafka/Marten patterns date from 2013–2020; nothing I found suggests they have been superseded. Cambria is from 2020 and has not, as far as I can tell, produced a maintained Rust implementation — flagging this as a place where the state of the art may simply not have advanced.

---

## 2. Ordering and causality across devices

### What each mechanism actually buys

| Mechanism | Guarantee | Can it answer "ahead / behind / concurrent"? | Size |
|---|---|---|---|
| **Lamport clock** | `a → b ⇒ C(a) < C(b)`. Total order via `(C, processId)` tiebreak. | **No.** Only "one of these is not after the other". | 8 B (u64) [inference] |
| **HLC** | Same one-way property as Lamport, but the value stays close to physical time and is usable for time-range queries. | **No.** Scalar. | **8 B** — sourced |
| **Vector clock / version vector** | Full partial order: `V₁ ≤ V₂`, `V₂ ≤ V₁`, equal, or **incomparable ⇒ concurrent**. | **Yes.** This is the only option that does. | **O(n)** in devices — sourced |
| **ULID / UUIDv7** | Lexicographically sortable identifier with an embedded ms timestamp. Not a causality mechanism. | **No.** | 16 B |
| **Interval Tree Clocks** | Generalises version vectors; nodes fork/join without global coordination. | **Yes.** | Variable |

### The proofs, from the primary sources

Lamport is explicit that a scalar clock cannot be inverted:

> "Clock Condition. For any events a, b: if a ⟶ b then C(a) < C(b). Note that we cannot expect the converse condition to hold as well, since that would imply that any two concurrent events must occur at the same time."
> — [Lamport 1978, *Time, Clocks, and the Ordering of Events in a Distributed System*](https://lamport.azurewebsites.net/pubs/time-clocks.pdf)

He also frames the total order as a *completion* of the partial order, not a discovery of it: "the relation ⇒ is a way of completing the 'happened before' partial ordering to a total ordering" — and warns that "Unexpected, anomalous behavior can occur if the ordering obtained by this algorithm differs from that perceived by the user."

HLC inherits exactly that one-way property. The formal requirement stated by an HLC co-author is `e hb f ⇒ l.e < l.f` — one direction only ([Demirbas, *Hybrid Logical Clocks*](http://muratbuffalo.blogspot.com/2014/07/hybrid-logical-clocks.html)).

And the sharpest statement, naming the property by name:

> "The two-way causality property is provided by vector clocks […] which keep track of and combine causality information on all processes. However, the size of vector clocks is O(n) where n is the number of processes, which is prohibitively expensive for large distributed systems."
> — [Kulkarni, Appleton & Nguyen, *Achieving Causality with Physical Clocks*, ICDCN 2022](https://arxiv.org/pdf/2104.15099)

**The O(n) objection does not apply to us.** n is a handful of a single user's devices. The stated reason vector clocks are avoided in industry is scale we do not have.

Riak confirms what the partial order buys operationally — vector clocks let a system determine "Whether one object is a direct descendant of the other", "Whether the objects are direct descendants of a common parent", and "Whether the objects are unrelated in recent heritage" ([Riak causal context](https://docs.riak.com/riak/kv/latest/learn/concepts/causal-context/index.html)).

### Concrete sizes and growth

**HLC is 8 bytes, and the layout is fixed by the paper:**

> "NTP uses 64-bit timestamps […] We restrict *l* to track only the most significant 48 bits of *pt* […] 16 bits remain for *c*."
> — [Demirbas](http://muratbuffalo.blogspot.com/2014/07/hybrid-logical-clocks.html)

The ICDCN paper gives the practical split as "the last 16 bits are used to save the value of *l − pt* (typically 12 bits) and *c* (typically 4 bits)" and notes the resolution ceiling: "if 16 bits are no longer available, it implies that the application must not generate two events within 15 μs" ([Kulkarni et al.](https://arxiv.org/pdf/2104.15099)). At a few hundred events/day this ceiling is irrelevant.

**Vector clock growth.** Loro's is the cleanest concrete instantiation: `PeerId` is a `u64` and `Counter` starts at 0 and increments per peer; `VersionVector = { [peerId]: number }` ([Loro version vector docs](https://loro.dev/llms-full.txt)).

My arithmetic [inference], using Anki's actual revlog field list as the payload baseline (`id` i64, `cid` i64, `usn` i32, `ease` u8, `ivl` i32, `lastIvl` i32, `factor` u32, `time` u32, `type` u8 = **38 bytes**) and 200 reviews/day = 73,000 events/year:

| Per-event metadata | Bytes/event | Total/event | MB/year |
|---|---|---|---|
| none (payload only) | 0 | 38 | 2.8 |
| HLC only | 8 | 46 | 3.4 |
| HLC + (u64 peer id, u32 counter) | 20 | 58 | **4.2** |
| + vector clock, 5 devices, u64 id + u32 counter | +60 | 118 | 8.6 |
| + vector clock, 5 devices, UUID id + u64 counter | +120 | 178 | 13.0 |

**The decision-relevant shape of that table: a full vector clock stamped on every event roughly doubles-to-triples the log. A per-event `(deviceId, counter)` costs 12–24 bytes and lets you *reconstruct* any version vector by scanning.** This is precisely what Yjs and Loro do — the vector lives in the *sync handshake*, not in the ops:

> "A state vector defines the known state of each user (a set of tuples `(client, clock)`) […] The client can ask a remote client for missing document updates by sending their state vector (often referred to as *sync step 1*). The remote peer can compute the missing `Item` objects using the `clocks` of the respective clients and compute a minimal update message."
> — [Yjs INTERNALS.md](https://github.com/yjs/yjs/blob/main/INTERNALS.md)

Loro spells out the arithmetic. Given `A = {0:2, 1:3}` and `B = {0:5, 1:3, 2:9}`, "We can easily calculate the Operations the device is missing: All Ops where PeerId == 0 && 2 ≤ Counter < 5, Or PeerId == 2 && Counter < 9" ([Loro version vector docs](https://loro.dev/llms-full.txt)).

**Pruning a version vector is unsafe.** "Classic Version Vectors reveal either scalability problems or loss of accuracy if pruning is used to prevent growth. By pruning version vector context, some update events are lost […] potentially allowing old and obsolete data to creep back from the past" ([Dotted Version Vectors work, Gonçalves/Almeida et al.](https://arxiv.org/pdf/1011.5808)). Dotted Version Vectors bound the vector to the replication degree rather than the client count and "can use as little as 10% of the space consumed by current version vector implementation" — but that optimisation targets a *server-replicated* topology (many clients, few servers), which is not ours. Interval Tree Clocks are the option built for *dynamic* participant sets: they generalise "both Version Vectors and Vector Clocks", require no global IDs, and let "any entity fork a new one" and "the number of participants be reduced by joining arbitrary pairs" ([Almeida, Baquero & Fonte 2008](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf)) — relevant if device *retirement* needs to shrink the vector, which is the one thing plain version vectors cannot do.

### ULID / UUIDv7 are identifiers, not clocks

- **ULID**: 128 bits = "48 bit integer" of "UNIX-time in milliseconds" + "80 bits" of randomness; "26 character string" in Crockford Base32; "Lexicographically sortable!"; within the same millisecond "the `random` component is incremented by 1 bit in the least significant bit position (with carrying)" ([ULID spec](https://github.com/ulid/spec)).
- **UUIDv7**: first 48 bits are a "Unix Epoch timestamp in milliseconds"; monotonicity is **optional**, via one of three documented methods (fixed bit-length counter, monotonic random, sub-millisecond precision) — "implementations can guarantee additional monotonicity via the concepts covered in this section". On clock regression, applications "SHOULD embed sufficient logic to catch these scenarios and correct the problem […] or they should at least report an appropriate error". And crucially for cross-device ordering: "Distributed applications generating UUIDs at a variety of hosts MUST be willing to rely on the random number source at all hosts" ([RFC 9562](https://www.rfc-editor.org/rfc/rfc9562.html)).

Both give a globally-unique, time-sortable event id with no coordination. **Neither tells you whether you are behind another device.** The monotonicity guarantees are *within one generator*, not across devices.

### The 300-second reality check

Anki, which has run this problem in production for two decades, does not try to reconcile skew — it refuses:

```rust
let delta = remote.current_time.0 - local.current_time.0;
if delta.abs() > 300 {
    debug!(delta, "clock off");
    return Err(AnkiError::sync_error("", SyncErrorKind::ClockIncorrect));
}
```

([`status.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/status.rs))

This is a 5-minute tolerance measured against a **server's** clock. A serverless design has no such oracle. HLC partly substitutes: it keeps a scalar close to physical time while preserving `e hb f ⇒ l.e < l.f`, so a device whose wall clock jumps backwards still emits monotonically increasing timestamps. It does **not** fix a device whose clock is wrong by a day — that device will produce `delta_t` values (days since last review) that are simply wrong, and FSRS will consume them as truth.

---

## 3. Device identity

### How local-first systems actually do it

The consistent answer across libraries is **random, generated locally, no coordination** — and, importantly, **per-session rather than per-device**:

- **Yjs**: "Each client is assigned a unique *clientID* property on first insert. This is a random 53-bit integer (53 bits because that fits in the javascript safe integer range)." Item IDs are `ID(clientID, clock)` — "also known as a Lamport Timestamp" — where "the clock counts up from 0 with the first inserted character or item a client makes" ([Yjs INTERNALS.md](https://github.com/yjs/yjs/blob/main/INTERNALS.md)). The Yjs docs add a self-healing behaviour: a document that receives an update carrying its own clientID reassigns itself a new one ([Yjs Y.Doc docs](https://docs.yjs.dev/api/y.doc)).
- **Loro**: `PeerID` is a 64-bit integer; `new LoroDoc()` "Gets a random peer ID"; "New peer ID generated for each `LoroDoc` instance, **even when loading same document**." The explicit "Never" list: "Use user IDs as peer IDs, because an user can have multiple devices / Use fixed IDs / Reuse IDs without proper management / Allow multiple browser tabs or processes to operate on the same reused Peer ID in parallel". If you *do* reuse a PeerID you must "persist the document's local data […] alongside that ID and load it before fetching or applying any remote updates", otherwise you risk "generating a new operation that reuses an existing operation ID, which leads to inconsistent replicas" ([Loro PeerID Management](https://loro.dev/docs/concepts/peerid_management)).
- **Automerge**: `ActorId::random()` is the default ([`ActorId` API](https://docs.rs/automerge/latest/automerge/struct.ActorId.html)), and the library docs state the invariant directly: "By default automerge will generate a random actor ID for you, but most methods for creating a document allow you to set the actor ID […] Actor IDs must not be used in concurrent threads of executiong \[sic\] - all changes by a given actor ID are expected to be sequential." ([`javascript/src/index.ts`](https://github.com/automerge/automerge/blob/main/javascript/src/index.ts), main branch as of 2026-07-26)

**Reading:** these are *not* device identities. They are "one sequential writer" identities. If the map wants a stable device identity in the log (constraint 3), that is a **separate field in the event payload**, orthogonal to any CRDT actor id.

### Platform-provided ids and what breaks them

- **Android `ANDROID_ID` (SSAID)**: since Android 8.0 (API 26) its value is scoped to the combination of app signing key, user, and device — different signing keys on the same device get different values. **It survives uninstall/reinstall** (same signing key) but **changes on factory reset** ([Android best practices for unique identifiers](https://developer.android.com/identity/user-data-ids)). Android's own guidance discourages hardware ids: "Using hardware IDs such as IMEI is discouraged because the user cannot reset them […] In many cases, an app-scoped identifier would suffice."
- **Matrix** delegates: a client "is also free to provide its own `device_id`", and "If the client sets the `device_id`, the server will invalidate any access and refresh tokens previously assigned to that device" ([Matrix client-server spec](https://spec.matrix.org/latest/client-server-api/)). The spec does **not** define a generation algorithm, format, or stability guarantee — I checked and could not find one; the device-id lifecycle is left to implementations.

### The failure modes, and what the sources say

| Event | What happens | Source / status |
|---|---|---|
| **Reinstall** | Random-generated id in app storage is **lost** → new device id, log has a new entry forever. Android SSAID would survive if the signing key is unchanged. | [Android docs](https://developer.android.com/identity/user-data-ids) |
| **Restore from backup onto a *new* device** | The id is copied → **two devices share one id**. This is the dangerous one. | Failure consequence sourced: Loro says reusing a PeerID without also restoring the matching local op state "leads to inconsistent replicas" ([Loro](https://loro.dev/docs/concepts/peerid_management)); Automerge says all changes by an actor ID "are expected to be sequential", which two devices sharing an ID violate. |
| **Restore from backup onto the *same* device** | Correct behaviour — id and counter both restored, sequence continues. Loro's guidance describes exactly this as the *only* safe reuse pattern: persist doc data alongside the ID and load it before applying remote updates. | [Loro](https://loro.dev/docs/concepts/peerid_management) |
| **Device loss** | The device's entry stays in every version vector forever. Plain version vectors cannot retire an entry safely (pruning "allow[s] old and obsolete data to creep back"). ITC is the mechanism designed to shrink participant sets. | [DVV paper](https://arxiv.org/pdf/1011.5808), [ITC paper](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf) |

**On the shared-id failure mode specifically:** the mechanism of harm is concrete and does not require a CRDT. If two devices both believe they are device `D` with counter 41, both emit `(D, 42)` for different reviews. A version-vector-based sync then says "I have everything through `D:42`" and **silently drops the other device's event**. Yjs's mitigation (reassign your own clientID on seeing a duplicate) is the only in-the-wild self-healing behaviour I found ([Yjs Y.Doc docs](https://docs.yjs.dev/api/y.doc)). Anki avoids the whole class of problem by having a server allocate USNs — unavailable to us.

Also worth noting: Yjs increments its clock **only on inserts** — "Deletes are handled in a very different way" and "The clientID's clock is not incremented" on delete ([Yjs INTERNALS.md](https://github.com/yjs/yjs/blob/main/INTERNALS.md)). If we adopt a per-device counter as the sync primitive, *every* logged event must increment it or the "have I got everything from D?" question becomes unanswerable.

---

## 4. CRDTs — where the merge problem actually lives

### The append-only surface vs the mutable surface

This is the crux of area 4, so let me be explicit about which of this app's data is which.

**Genuinely append-only and commutative-as-a-set:**

- Review events (card id, grade, timestamp, event kind, duration). A review is a new fact, never a revision of an earlier one. Anki's implementation is the proof of how little machinery this needs:

  ```rust
  fn merge_revlog(&self, entries: Vec<RevlogEntry>) -> Result<()> {
      for entry in entries { self.storage.add_revlog_entry(&entry, false)?; }
      Ok(())
  }
  ```

  and the underlying SQL is `INSERT OR IGNORE INTO revlog (...)` ([`chunks.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/chunks.rs), [`add.sql`](https://github.com/ankitects/anki/blob/main/rslib/src/storage/revlog/add.sql)). **No conflict resolution at all.** Set union, keyed on the event id.
  - Caveat found in the source: Anki's `RevlogId` is `TimestampMillis::now()` ([`revlog/mod.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/revlog/mod.rs)), so the id *is* a millisecond timestamp. With `uniquify=false` during merge, **two reviews on two devices in the same millisecond silently lose one**. Using a `(deviceId, counter)` or ULID/UUIDv7 event id removes this class of loss entirely.

**Genuinely mutable — this is where the merge problem lives:**

- Card content edits (front/back text, fields)
- Deck renames, deck moves, card→deck reassignment
- Deletions (cards, decks) — and re-creations
- Per-deck settings / scheduler config
- Card→note structural changes (adding a field, removing a template)

Anki's answers for these, from source, are uniformly **LWW on an mtime**, with `<=` so the incoming write wins ties:

```rust
// decks
existing_deck.mtime_secs <= deck.common().mtime
// deck config
existing_conf.mtime_secs <= conf.mtime
// cards and notes: take remote unless we have unsynced local changes AND ours is newer
!existing_card.usn.is_pending_sync(pending_usn) || existing_card.mtime < entry.mtime
```

([`changes.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/changes.rs), [`chunks.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/chunks.rs))

Deletions are explicit tombstones: `apply_graves` removes the object and records it in a graves table with the current USN ([`graves.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/graves.rs)). Structural changes escalate to failure — `merge_notetypes` raises `SyncErrorKind::ResyncRequired` with "notetype schema changed" if field or template counts differ.

**So: LWW-per-field with tombstones is sufficient for a single user's own content edits, and it is what the closest prior art ships.** A CRDT buys character-level merge of concurrent edits to *the same card's text*, which for a single-user app is a rare event with a cheap fallback (keep both, or LWW).

### What a CRDT library costs

| | Automerge 0.10 | yrs 0.27 (Yjs port) | Loro 1.13 |
|---|---|---|---|
| **History retained** | **Always, all of it.** "Automerge documents always carry their history with them"; "we assign a unique ID to every keystroke" | Insert history retained; delete *content* discarded under GC | Retained, or truncated via shallow snapshot |
| **Can history be pruned?** | **No.** "Discarding the history is not currently possible without replacing the document […] because we use a git-like commit hash for each change" | Partially — GC replaces deleted content with a `GC` struct storing only length | **Yes** — `ExportMode::shallow_snapshot(frontiers)` |
| **Tombstones** | Kept. Kleppmann's own measurement: savings from discarding history "are not all that great, which is why history truncation has not been prioritized so far" | Tombstone *structs* kept for ordering; "you can garbage collect tombstones if you don't care about the order of the structs anymore". Concrete size datapoint: B4 trace with 182k inserts / 77k deleted chars → "The deleted set size in a snapshot is only 4.5Kb" | Eg-walker-based core gives "low overhead **without permanent tombstones**" |
| **Rust support** | Rust is the core implementation | Rust is the core (`yrs`); Yjs JS is the reference | Rust is the core |
| **wasm32 (verified 2026-07-26)** | `cargo check` OK **with** `getrandom` `wasm_js` backend | `cargo check` OK out of the box | `cargo check` OK out of the box |
| **Maintained?** | pushed 2026-07-26, 6.4k★, MIT | pushed 2026-07-13, 2.1k★ | pushed 2026-07-22, 6.0k★, MIT |
| **Key gotcha** | Map LWW is "randomly choose one value" (deterministically, but arbitrarily) | clock increments on insert only, not delete | shallow-snapshot peers "cannot import updates from before the shallow start point" |

Sources for the above: [Automerge merge rules](https://github.com/automerge/automerge.github.io/blob/main/content/docs/reference/under-the-hood/merge-rules.md), [Automerge 3.0 blog (2025-07-14)](https://automerge.org/blog/automerge-3/), [automerge#799](https://github.com/automerge/automerge/issues/799), [Yjs INTERNALS.md](https://github.com/yjs/yjs/blob/main/INTERNALS.md), [Yjs Y.Doc docs](https://docs.yjs.dev/api/y.doc), [Loro docs](https://loro.dev/llms-full.txt), [Loro Shallow Snapshots](https://loro.dev/docs/concepts/shallow_snapshots), crates.io + GitHub API queried 2026-07-26.

**Automerge's merge semantics for maps are worth quoting**, because they define what "CRDT" buys you for a deck rename:

> - "If `A` deletes key x and `B` sets x to a new value then set the value of x to the new value `B` set in the merged map"
> - "If both `A` and `B` set the key x to some value then **randomly choose one value**" — where "randomly choose" means "choose one arbitrarily, but in such a way that all nodes agree on the chosen value"
>
> — [Automerge merge rules](https://github.com/automerge/automerge.github.io/blob/main/content/docs/reference/under-the-hood/merge-rules.md)

That is *LWW with a deterministic tiebreak*. For scalar fields (deck name, card front text as a plain string) a CRDT gives you the same outcome as your own LWW, at the cost of unbounded history. The real CRDT win is confined to: (i) collaborative *text* inside a field, (ii) list/tree reordering, (iii) counters. Loro additionally offers a movable tree (`getTree` with `move`) and a movable list, which map onto "deck hierarchy" and "card ordering" if those ever need concurrent restructuring.

### The size argument, made concretely

Automerge 3.0 (2025-07-14) reduced *memory* by >10x — "pasting Moby Dick into an Automerge 2 document consumes 700Mb of memory, in Automerge 3 it only consumes 1.3Mb!" ([Automerge 3.0](https://automerge.org/blog/automerge-3/)). That fixes runtime memory, not on-disk history growth. Ink & Switch's own retrospective is the honest framing:

> "CRDTs accumulate a large change history, which creates performance problems. Our team used PushPin for 'real' documents such as sprint planning. Performance and memory/disk usage quickly became a problem because CRDTs store all history."
> — [Ink & Switch, *Local-first software*](https://www.inkandswitch.com/essay/local-first/)

That essay also names the gap that matters most for our deferred-transport constraint: **"CRDT algorithms provide only for the merging of data, but say nothing about how different users' edits arrive."** A CRDT does not give us sync; it gives us merge. We still have to build or borrow transport either way.

### Evidence bearing on "is a CRDT warranted?"

**Against needing one:**

- The high-volume, sync-critical data (review events) needs *set union*, which is a two-line function (Anki's `merge_revlog`). A CRDT here is machinery for a problem that does not exist.
- The mutable data is single-user, low-contention. Anki ships LWW-on-mtime + tombstones for exactly this and it is not the source of its sync complaints.
- Automerge cannot prune history, and the maintainers explicitly say shallow-clone/cherry-pick are "not yet implemented" and "on the wishlist" as of the automerge#799 thread.
- Constraint 1 says scheduling state is *derived by replay*. Derived state should not be in a CRDT at all — you would be replicating something you can recompute.
- Constraint 2 wants card **content** separable from **review progress**, and portable/publishable decks. A CRDT document conflates them into one opaque binary blob unless you deliberately split into two documents — and Automerge-style documents do not have a clean "export just the content, at a stable identity" story.

**For needing one (or at least Loro):**

- Loro's `versionVector`, `OpId`, shallow snapshots, `ImportStatus` for out-of-order/partial updates, and movable tree/list are all things we would otherwise hand-roll, and it is actively maintained with first-class Rust and verified wasm32 support.
- If card content ever becomes genuinely collaborative (shared/published decks that recipients edit and re-share), hand-rolled LWW will not be enough and retrofitting a CRDT onto an existing log is expensive.
- CRDT libraries *already solve* the "which ops am I missing" problem that constraint 3 demands — using precisely the version-vector primitive from §2.

**My read of the evidence (not a recommendation):** the case for a CRDT library rests entirely on the *mutable content* surface, not the review log. If card content edits are treated as single-writer-at-a-time (which they are, for one user with a handful of devices), Anki's shipped answer — LWW-on-mtime plus tombstones plus explicit escalation to "full sync required" for structural changes — is demonstrably adequate at far larger scale than this app will see. The strongest CRDT-shaped argument is not "we need conflict-free merge", it is "we need `(deviceId, counter)` + version-vector sync anyway, and Loro hands us a tested implementation with history truncation" — but that argument buys a sync engine, not a merge algorithm, and it costs the ability to keep review events as plain, inspectable, self-contained rows.

---

## 5. Prior art

### Anki (the crux)

Anki is worth reading in full because it has run this exact problem, at scale, for two decades, and because its data model is nearly ours: an append-only `revlog` of graded reviews plus mutable cards/notes/decks.

**The sync state machine.** Three outcomes, decided by comparing two scalars:

```rust
let required = if remote.modified == local.modified {
    SyncActionRequired::NoChanges
} else if remote.schema != local.schema {
    SyncActionRequired::FullSyncRequired { upload_ok, download_ok }
} else {
    SyncActionRequired::NormalSyncRequired
};
ClientSyncState { required, local_is_newer: local.modified > remote.modified, … }
```

([`meta.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/meta.rs))

- `mod` = collection change timestamp (ms). `scm` = **schema** change timestamp (ms). `usn` = update sequence number.
- **`scm` mismatch ⇒ full sync.** The collection is not merged; the user picks a winner and the other side's changes are destroyed.
- The offline equivalent is `sync_status_offline()`, which reports `FullSync` if `schema_changed_since_sync()`, `NormalSync` if `collection_changed_since_sync()`, else `NoChanges` ([`status.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/status.rs)).
- The user-visible surface: "The sync button signals the sync type: blue when a normal sync is required, and red when a full sync is required" ([Anki manual — syncing](https://docs.ankiweb.net/syncing.html)).

**USN — and why we cannot have it.** USN is a monotonic counter but it is **server-owned**:

```rust
pub(crate) fn usn(&self, server: bool) -> Result<Usn> {
    if server { /* select usn from col */ } else { Ok(Usn(-1)) }
}
```

Locally-modified rows are marked `usn = -1`; the pending-object query is `usn = ?` when the sentinel is -1, else `usn >= ?` ([`storage/sync.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/storage/sync.rs)). On upload the client rewrites each object's USN to the server's current value (`maybe_update_object_usns`). **There is exactly one counter in the whole system and the server allocates it.** With no server of our own, this design is simply not available — which is, I think, the single most important thing this research says about constraint 3. The reason the map needs device identity + per-device ordering is that Anki's much simpler answer requires a server we do not have.

**What Anki merges vs what it refuses.** From the manual:

> "Reviews and note edits can be merged, so if you review or edit on two different devices before syncing, Anki will preserve your changes from both locations."
> "When the same card is reviewed in different locations […] both reviews will be marked in the revision history, and the card will be kept in the state it was when it was most recently answered."
> Unmergeable: "adding a new field, or removing a card template." "If changes have been made on both ends, only changes on one end can be preserved."
> — [Anki manual — syncing](https://docs.ankiweb.net/syncing.html)

Note the second quote precisely: the *revlog* union-merges (both reviews survive), but the *card's derived state* is last-answered-wins. Anki materialises the projection into the mutable card row and then LWWs it. **Constraint 1 (derive by replay, never store only derived state) is a deliberate divergence from Anki.**

**Additional forced-full-sync triggers found in source, not in the manual:**

- Collection too large: `if meta.collection_bytes > *MAXIMUM_SYNC_PAYLOAD_BYTES_UNCOMPRESSED { info!("collection is too large, forcing one-way sync"); meta.schema = TimestampMillis::now(); }` — the server **fabricates a schema change** to force a one-way sync. The limit is 100 MB × 3 uncompressed ([`meta.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/meta.rs), [`sync/request/mod.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/request/mod.rs)).
- Notetype field/template count mismatch during merge ⇒ `SyncErrorKind::ResyncRequired`.
- Clock skew > 300 s ⇒ `SyncErrorKind::ClockIncorrect` (§2).
- "Check Database" is a known full-sync trigger in the wild ([AnkiDroid#5976](https://github.com/ankidroid/Anki-Android/issues/5976) — community-reported; I did not verify the mechanism in source).

**Review-log deletions are unsyncable.** The strongest single quote for our purposes:

> "Only intended to be used by the undo code, as Anki can not sync revlog deletions."
> — [`rslib/src/storage/revlog/mod.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/storage/revlog/mod.rs)

A mature system with a genuinely append-only review log accepts that it **cannot ever remove an entry** across devices. That is the price of union-merge without tombstones — and it is a price we should be aware we are agreeing to.

**Content/progress separation (bears on constraint 2).** Anki's export has an explicit switch: "If true, scheduling information such as your review history will be exported. If false, Anki will assume that you are sharing the deck with other people, and will remove the entire scheduling information, including marked and leech tags." On re-import, "If some notes in the deck package have previously been imported, Anki will keep the version with the most recent modification time" ([Anki manual — exporting](https://docs.ankiweb.net/exporting.html)). Notes carry a `guid` field distinct from the local `id` ([`NoteEntry` in `chunks.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/chunks.rs)) — a stable content identity separate from local row identity, which is precisely the shape constraint 2 asks for.

### Obsidian Sync

Per-file, transport-level, and **type-dependent**:

- Markdown files: merged using "Google's diff-match-patch algorithm".
- Everything else (including canvases): "last modified wins approach" where "the most recently modified version replaces earlier versions".
- Since 1.9.7 the user chooses between "Automatically merge" (default; may create "duplicate text or formatting issues requiring manual cleanup") and "Create conflict file".
- Caveat: "Conflict resolution settings are device-specific and must be configured on each synced device."
- ([Obsidian Sync troubleshooting](https://obsidian.md/help/sync/troubleshoot))

**Transferable point:** a mature local-first product ships *textual three-way merge for text, LWW for everything else, and a user-visible conflict file as the escape hatch* — no CRDT, no event log.

### Logseq

Logseq's DB version (SQLite-backed) shipped through 2025–early 2026 with "Logseq Sync […] also referred to as RTC (Real Time Collaboration)", in alpha as of early 2026 ([Logseq announcements](https://discuss.logseq.com/t/whats-new-with-logseq-db-april-26-2026/34977), [Logseq DB docs](https://github.com/logseq/docs/blob/master/db-version.md)). **I could not find first-party documentation of Logseq's conflict-resolution or CRDT design** — the technical design does not appear to be published. Treat Logseq as an unresolved data point, not evidence.

### remoteStorage

The remoteStorage spec is the "dumb server, smart client" model — worth knowing because it is the closest thing to a standard for "sync without a server of your own":

- Conditional requests with ETags as version identifiers; `If-Match` yields 412 when the version does not match.
- "The remoteStorage server does not get involved in the conflict resolution — it keeps the canonical current version at all times, but it is up to whichever client discovers a given version conflict to resolve it."
- ([draft-dejong-remotestorage](https://datatracker.ietf.org/doc/draft-dejong-remotestorage/))

**Transferable point:** an append-only log is unusually well suited to a dumb blob store. If each device writes only *its own* append-only segment under a device-scoped key, there are **no cross-device write conflicts at all** — every writer owns its keyspace, and merging is reading everyone's segments. This composes with any commodity storage (S3, Dropbox, WebDAV, git) and requires no server logic. [inference: this is my synthesis of the remoteStorage model with the per-device-log structure, not a claim any of these sources makes.]

### Dexie Cloud (used by Skola, a local-first FSRS flashcard PWA)

Dexie Cloud is **server-authoritative with operation re-execution**: "the expression will be a part of the operation and be re-executed on the server snapshot to ensure consistency", so an offline "mark these done" replays its *where-clause* against current server data rather than blindly LWW-ing rows. For competing property writes, "the operation that was performed latest in time will be the one that overwrites the other". Collaborative text is delegated to Y.js ([Dexie Cloud consistency docs](https://dexie.org/cloud/docs/consistency)).

Skola itself: "Your data lives in your browser using IndexedDB", FSRS for scheduling, optional Dexie Cloud sync ([skola README](https://github.com/h16nning/skola)). **Its README does not describe a review/event log.** So the nearest existing "local-first FSRS app" is *not* event-sourced, and depends on a server for consistency — no useful precedent for our constraints.

### FSRS-adjacent

`fsrs` (fsrs-rs) is BSD-3-Clause, 6.6.1 released 2026-06-09, repo pushed 2026-07-20, 398★ ([crates.io](https://crates.io/crates/fsrs), [repo](https://github.com/open-spaced-repetition/fsrs-rs)). Its data model is the decisive fact for constraint 1:

> `FSRSItem`: "Stores a list of reviews for a card, **in chronological order**."
> `FSRSReview { rating: u32, delta_t: u32 }` where `delta_t` is "The number of days that passed" and "`delta_t` for item first(initial) review must be 0".
> — [fsrs-rs `dataset.rs`](https://github.com/open-spaced-repetition/fsrs-rs/blob/main/src/dataset.rs)

The optimizer consumes review *histories*, so **the full review log is the training set**. This is a strong argument for constraint 1 and against truncating the log: truncate history and you degrade parameter optimisation, which is why Anki exposes "Ignore cards reviewed before" as a *user* decision rather than doing it automatically ([Anki deck options](https://docs.ankiweb.net/deck-options.html)).

---

## 6. Rust crates

All rows queried from the crates.io API and GitHub API on **2026-07-26**. The wasm32 column is my own `cargo check --target wasm32-unknown-unknown` result on cargo 1.97.0, same date — see caveat below the table.

| Crate | Version | Last release | License | wasm32-unknown-unknown | Notes |
|---|---|---|---|---|---|
| `uuid` (v7) | 1.24.0 | 2026-07-15 | MIT OR Apache-2.0 | **OK** (features `v7`, `js`) | RFC 9562 v7. 686M downloads. The safe default for event ids. |
| `ulid` | 3.0.0 | 2026-07-16 | MIT | **OK** with `getrandom` `wasm_js` backend | 26-char sortable string form; per-ms monotonic increment. |
| `rusty_ulid` | 2.0.1 | 2026-07-18 | MIT/Apache-2.0 | not tested | Alternative ULID impl, actively released. |
| `uhlc` | 0.9.0 | 2026-01-12 | EPL-2.0 OR Apache-2.0 | **OK** with `getrandom` `js` (0.2) backend | Production HLC (used by Eclipse Zenoh). 4.8M downloads. **The only maintained HLC crate.** |
| `hlc` | 0.1.1 | **2015-08-21** | MIT | not tested | **Abandoned.** Do not use. |
| `automerge` | 0.10.0 | 2026-06-05 | MIT | **OK** with `getrandom` `wasm_js` backend | Repo pushed 2026-07-26. Cannot prune history. |
| `yrs` | 0.27.3 | 2026-07-13 | MIT (verified: repo `LICENSE` is the MIT text; GitHub API reports NOASSERTION, which is a detection artefact) | **OK** out of the box | Rust Yjs. Repo pushed 2026-07-13. |
| `loro` | 1.13.7 | 2026-07-15 | MIT | **OK** out of the box | Repo pushed 2026-07-22, 6.0k★. Shallow snapshots; movable tree/list; version vectors. |
| `crdts` | 7.3.2 | **2023-08-08** | Apache-2.0 | OK with `getrandom` `wasm_js` backend | **~3 years since last release.** Provides primitive CRDTs (VClock, ORSWOT, LWWReg) rather than a document engine. Useful as a reference for version-vector code; risky as a dependency. |
| `redb` | 4.1.0 | 2026-04-19 | MIT OR Apache-2.0 | **compiles** — but see caveat | Pure-Rust embedded KV, ACID. |
| `rusqlite` | 0.40.1 | 2026-06-06 | MIT | **compiles** (`bundled`) — but see caveat | 85M downloads. What Anki uses. |
| `sqlite-wasm-rs` | 0.5.5 | 2026-05-25 | MIT | purpose-built for wasm | Provides SQLite + a browser VFS (OPFS/IndexedDB) for wasm targets. This, not bare `rusqlite`, is the realistic web path. |
| `fjall` | 3.1.8 | 2026-07-18 | MIT OR Apache-2.0 | **FAILS** — `lsm-tree`: "unsupported platform" | LSM KV store. Actively developed but rules itself out for web. |
| `sled` | 0.34.7 | **2024-10-11** | MIT OR Apache-2.0 | not tested | Long-stalled; 1.0 never landed. |
| `native_db` | 0.8.2 | 2025-07-08 | MIT | not tested | ~1 year since release; small user base (49k downloads). |
| `fsrs` | 6.6.1 | 2026-06-09 | BSD-3-Clause | not tested | The scheduler itself. Consumes chronologically-ordered review histories. |

**Important caveat on the storage crates.** `cargo check` succeeding for `redb` and `rusqlite` on `wasm32-unknown-unknown` proves the code *type-checks and its C compiles*, **not that it works**: `wasm32-unknown-unknown` has no filesystem, so a file-backed store needs a VFS shim (OPFS/IndexedDB) at runtime. That is exactly what `sqlite-wasm-rs` exists to provide. **Do not read "compiles" as "works in the browser" for anything file-backed.** [inference — but a well-founded one; the absence of a filesystem on that target is not controversial.]

**Observation on the `getrandom` pattern.** Four of the crates above (`ulid`, `uhlc`, `automerge`, `crdts`) fail the same way for the same reason: a transitive `getrandom` needs its web backend explicitly enabled (`--cfg getrandom_backend="wasm_js"` plus the `wasm_js`/`js` feature, version-dependent — I hit both `getrandom` 0.3 and 0.4 in one dependency tree). This is a one-line build-config concern, not a per-crate blocker, but it will need handling once in the workspace and it is easy to misdiagnose as "crate X does not support wasm".

---

## Where the map's constraints collide

Flagging these explicitly, as requested.

**A. Constraint 1 (derive by replay) × clock skew (stated hazard) — the sharpest conflict.**
Constraint 1 says the log records raw grades and timestamps so the scheduler stays swappable. But FSRS's input is chronologically-ordered reviews with `delta_t` in days ([fsrs-rs](https://github.com/open-spaced-repetition/fsrs-rs/blob/main/src/dataset.rs)), so **the derived state is a function of the timestamps, and a device with a wrong clock writes wrong facts into a log that is by design immutable and un-deletable.** HLC preserves *causal* monotonicity but cannot correct a wall clock that is a day out. Anki's answer is to refuse to sync at >300 s skew — which requires a server clock we do not have. Options a decision session must weigh: (i) log both the device's wall-clock time *and* an HLC, so a later correction pass can detect and compensate skew; (ii) log a device-local monotonic elapsed time alongside wall time; (iii) accept the corruption and provide a user-facing "ignore reviews before" escape hatch, as Anki does. Note that (iii) is Anki's *actual* shipped mitigation for bad history.

**B. Constraint 1 (never store only derived state) × replay cost × FSRS training needs.**
Snapshotting is the standard fix for replay cost, and Anki materialises derived state into the card row. But Anki also needs the *full* revlog for parameter optimisation, so it cannot truncate. These pull in opposite directions: snapshots let you truncate replay, but the optimiser wants everything. The resolution in Anki is to keep the whole log *and* cache derived state, treating the cache as rebuildable — i.e. snapshot for read performance, never for retention. [inference: the synthesis is mine; the two halves are separately sourced.]

**C. Constraint 1 ("raw grades and timestamps") is under-specified.**
`reviews_for_fsrs` needs event *kinds* to replay correctly — cramming reviews are skipped, history before a `Reset` is discarded, manual reschedules are distinguished from user grades ([`fsrs/params.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/scheduler/fsrs/params.rs)). Anki's `RevlogReviewKind` has six variants. A log of only `(cardId, grade, timestamp)` cannot express "this card was reset" or "this interval was set manually", so replay would silently produce wrong memory states. Constraint 1 should be widened to *raw grades, timestamps, and event kind*.

**D. Constraint 3 (device identity + ordering) × no server.**
Anki gets ahead/behind from a server-allocated USN. Without a server, the only mechanism that answers the question is a version vector over per-device counters (§2). That means: every device must have a *stable* id, and every logged event must increment that device's counter monotonically with no gaps. That in turn makes the reinstall / backup-restore / shared-id failure modes (§3) load-bearing correctness concerns rather than cosmetic ones — and the CRDT libraries explicitly refuse to give you a stable device id, telling you instead to generate a fresh one per session ([Loro](https://loro.dev/docs/concepts/peerid_management)).

**E. Constraint 2 (portable, publishable decks; content separable from progress) × a CRDT document.**
A CRDT document is one opaque binary blob carrying its own history. Constraint 2 wants (i) stable deck identity, (ii) self-contained export, (iii) clean content/progress separation. Anki achieves all three with plain rows: a note `guid` distinct from its local `id`, and an export switch that strips scheduling ([Anki exporting](https://docs.ankiweb.net/exporting.html)). Achieving the same inside a CRDT means two separate documents (content-doc, progress-doc) with a stable cross-reference — doable, but it is a design you have to commit to up front, and Loro's shallow-snapshot constraint ("peers can only sync if they have versions after the shallow snapshot point") means a published content-doc that has been trimmed cannot merge with an old fork of itself.

**F. Constraint 3 (ordering carried in the log) × log size.**
Stamping a full version clock on every event costs 60–120 bytes/event and grows with device count (§2 table). Carrying `(deviceId, counter)` per event costs 12–24 bytes and lets you reconstruct any version vector. This is a mild conflict, easily resolved, but worth naming: **the vector belongs in the sync handshake, not in the events** — which is exactly what Yjs and Loro do.

**G. Append-only × deletion.**
Constraint 1's append-only log has no mechanism for removing a review (mis-graded, imported by accident, from a device with a broken clock). Anki's source says outright that revlog deletions cannot sync. The alternatives all cost something: tombstone events (needs a GC horizon measured in device-offline duration — see the Kafka `delete.retention.ms` lesson), or "ignore before" cutoffs as user-level soft deletes (Anki's choice), or accepting permanence.

---

## Open questions / what this does not settle

1. **What clock-skew mitigation is actually adequate?** I found the failure mode and Anki's refusal-based mitigation, but no source describing a *serverless* solution. Detecting "this device's clock is wrong" without an authority is an open design question, not an answered one.
2. **Is there a maintained Rust version-vector implementation worth depending on?** `crdts` has one (`VClock`) but has not released since 2023-08-08. Loro and yrs have version vectors internally but not as a standalone, reusable primitive. Writing ~100 lines ourselves may be the only option; I did not find a well-maintained standalone crate.
3. **Device retirement.** Version vectors cannot safely shrink; pruning "allow[s] old and obsolete data to creep back". Interval Tree Clocks are designed for exactly this but I found **no maintained Rust ITC crate** — only a Haskell implementation and the 2008 paper. Whether a handful of stale device entries actually matters at this scale is unquantified.
4. **Logseq's design is undocumented.** I could not find first-party technical documentation of Logseq DB/RTC's conflict resolution. If that design matters to the decision, it needs someone to read the source directly.
5. **Cambria has no Rust implementation I could find.** Schema-evolution-by-lens is the right *shape* for local-first projection versioning, but the 2020 Ink & Switch work does not appear to have a maintained library, in Rust or otherwise.
6. **Automerge's "shallow clone"** would materially change the Automerge cost/benefit if it landed. As of the automerge#799 thread it is "on the wishlist" with no date. Worth re-checking before any decision that hinges on it.
7. **Runtime, not compile-time, wasm viability of the storage layer is unverified.** I verified `cargo check`, not that `redb` or `rusqlite` actually function in a browser. That needs a spike.
8. **I did not investigate transport**, per the ticket's scope. But note Ink & Switch's warning that merge and transport are separate problems ("CRDT algorithms provide only for the merging of data, but say nothing about how different users' edits arrive") and my [inference] in §5 that per-device append-only segments in a dumb blob store make transport unusually easy — that inference is untested and is the obvious next research question.
9. **Anki's `mod`-timestamp equality test for "no changes"** (`remote.modified == local.modified`) is fragile in a way I did not chase down: two devices could plausibly have equal `mod` values with different content. I found the code but no discussion of whether this is a known bug.
10. **State-of-the-art dating.** The causality mechanisms are settled theory (1978–2008) and unlikely to have moved. The CRDT libraries were all released within the last 6 weeks of this research, so they are current. The event-sourcing patterns (Kafka compaction, Marten projection versioning, upcasting) are 2013–2024 and I found no evidence of newer consensus. The place I am least confident about currency is **local-first sync engines** — that space moves fast and I sampled only Dexie Cloud, Automerge, Yjs and Loro.

---

## Sources

**Anki (primary — source code and first-party docs)**

- [`rslib/src/sync/collection/meta.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/meta.rs) — `SyncMeta`, `compared_to_remote`, full-sync trigger, size-forced one-way sync
- [`rslib/src/sync/collection/changes.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/changes.rs) — LWW merge of notetypes/decks/deck config/tags; `ResyncRequired`
- [`rslib/src/sync/collection/chunks.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/chunks.rs) — `merge_revlog`, `add_or_update_card_if_newer`, `CardEntry`/`NoteEntry`, `CHUNK_SIZE`, `is_pending_sync`
- [`rslib/src/sync/collection/graves.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/graves.rs) — tombstones for cards/notes/decks
- [`rslib/src/sync/collection/status.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/collection/status.rs) — offline sync status; 300 s clock check
- [`rslib/src/storage/sync.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/storage/sync.rs) — server-owned USN, client `-1` sentinel
- [`rslib/src/revlog/mod.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/revlog/mod.rs) — `RevlogEntry` fields, `RevlogId::new()` = ms timestamp, `RevlogReviewKind`
- [`rslib/src/storage/revlog/mod.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/storage/revlog/mod.rs) + [`add.sql`](https://github.com/ankitects/anki/blob/main/rslib/src/storage/revlog/add.sql) — `INSERT OR IGNORE`; "Anki can not sync revlog deletions"
- [`rslib/src/scheduler/fsrs/memory_state.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/scheduler/fsrs/memory_state.rs) — `update_memory_state`, `ignore_before`, clearing FSRS data
- [`rslib/src/scheduler/fsrs/params.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/scheduler/fsrs/params.rs) — `reviews_for_fsrs` replay filter
- [`rslib/src/sync/request/mod.rs`](https://github.com/ankitects/anki/blob/main/rslib/src/sync/request/mod.rs) — payload size limits
- [Anki manual — Syncing](https://docs.ankiweb.net/syncing.html) · [Deck Options](https://docs.ankiweb.net/deck-options.html) · [Exporting](https://docs.ankiweb.net/exporting.html)
- [AnkiDroid#5976](https://github.com/ankidroid/Anki-Android/issues/5976) — Check Database forcing full sync (community report, unverified)

**Causality and clocks (primary — papers)**

- Lamport, *Time, Clocks, and the Ordering of Events in a Distributed System*, CACM 1978 — [PDF](https://lamport.azurewebsites.net/pubs/time-clocks.pdf)
- Kulkarni, Appleton & Nguyen, *Achieving Causality with Physical Clocks*, ICDCN 2022 — [arXiv PDF](https://arxiv.org/pdf/2104.15099)
- Demirbas (HLC co-author), *Hybrid Logical Clocks*, 2014 — [post](http://muratbuffalo.blogspot.com/2014/07/hybrid-logical-clocks.html); paper: Kulkarni, Demirbas, Madeppa, Avva & Leone, *Logical Physical Clocks*, OPODIS 2014
- Almeida, Baquero & Fonte, *Interval Tree Clocks*, OPODIS 2008 — [PDF](https://gsd.di.uminho.pt/members/cbm/ps/itc2008.pdf)
- Preguiça, Baquero, Almeida, Fonte & Gonçalves, *Dotted Version Vectors*, 2010 — [arXiv PDF](https://arxiv.org/pdf/1011.5808)
- [Riak KV — Causal Context](https://docs.riak.com/riak/kv/latest/learn/concepts/causal-context/index.html)

**Identifiers (specs)**

- [RFC 9562 — UUIDs](https://www.rfc-editor.org/rfc/rfc9562.html) (v7 layout, monotonicity methods, clock regression)
- [ULID spec](https://github.com/ulid/spec)

**CRDT libraries (primary — docs and source)**

- [Yjs INTERNALS.md](https://github.com/yjs/yjs/blob/main/INTERNALS.md) — clientID, state vector, tombstones/GC, snapshot sizes
- [Yjs Y.Doc docs](https://docs.yjs.dev/api/y.doc) — `doc.gc`, clientID reassignment on collision
- [Automerge — Merge Rules](https://github.com/automerge/automerge.github.io/blob/main/content/docs/reference/under-the-hood/merge-rules.md)
- [Automerge — Storage](https://github.com/automerge/automerge.github.io/blob/main/content/docs/reference/under-the-hood/storage.md) — lock-free compaction protocol
- [Automerge 3.0 announcement, 2025-07-14](https://automerge.org/blog/automerge-3/) — 10x memory reduction, Moby Dick numbers
- [automerge#799](https://github.com/automerge/automerge/issues/799) — history cannot be discarded; shallow clone / cherry-pick not implemented
- [Automerge `javascript/src/index.ts`](https://github.com/automerge/automerge/blob/main/javascript/src/index.ts) — actor ID rules · [`ActorId` Rust API](https://docs.rs/automerge/latest/automerge/struct.ActorId.html)
- [Loro consolidated docs (`llms-full.txt`)](https://loro.dev/llms-full.txt) — PeerID u64, OpId, version vectors, export modes, Eg-walker/no permanent tombstones
- [Loro — PeerID Management](https://loro.dev/docs/concepts/peerid_management)
- [Loro — Shallow Snapshots](https://loro.dev/docs/concepts/shallow_snapshots)
- [Loro 1.0 announcement](https://loro.dev/blog/v1.0)

**Event sourcing**

- [Apache Kafka design docs — Log Compaction](https://github.com/apache/kafka/blob/trunk/docs/design/design.md)
- [Marten — Rebuilding Projections](https://martendb.io/events/projections/rebuilding.html) — `ProjectionVersion`, blue-green rebuilds
- [Event versioning tactics summary](https://event-driven.io/en/how_to_do_event_versioning/); canonical text: Greg Young, *Versioning in an Event Sourced System*

**Local-first prior art**

- [Ink & Switch — Local-first software](https://www.inkandswitch.com/essay/local-first/) — seven ideals; CRDT history growth; merge ≠ transport
- [Ink & Switch — Cambria](https://www.inkandswitch.com/cambria/) — schema evolution via bidirectional lenses
- [Obsidian Sync — troubleshooting/conflicts](https://obsidian.md/help/sync/troubleshoot)
- [Logseq DB docs](https://github.com/logseq/docs/blob/master/db-version.md) · [Logseq announcements, 2026-04-26](https://discuss.logseq.com/t/whats-new-with-logseq-db-april-26-2026/34977)
- [remoteStorage — draft-dejong-remotestorage](https://datatracker.ietf.org/doc/draft-dejong-remotestorage/)
- [Dexie Cloud — Consistency](https://dexie.org/cloud/docs/consistency)
- [skola — local-first SRS PWA](https://github.com/h16nning/skola)
- [fsrs-rs `dataset.rs`](https://github.com/open-spaced-repetition/fsrs-rs/blob/main/src/dataset.rs) · [fsrs-rs repo](https://github.com/open-spaced-repetition/fsrs-rs)

**Platform identity**

- [Android — Best practices for unique identifiers](https://developer.android.com/identity/user-data-ids)
- [Matrix client-server API spec](https://spec.matrix.org/latest/client-server-api/)

**Registry / maintenance data**

- crates.io API (`/api/v1/crates/{name}`) and GitHub API, queried 2026-07-26, for every crate and repo in §6
- wasm32 results: local `cargo check --target wasm32-unknown-unknown`, cargo 1.97.0, 2026-07-26
