# Sync transport: key-value stores and personal cloud drives, reached directly from the client

**Research ticket:** [#33](https://github.com/amin-bf/leitner/issues/33) (under wayfinder map #1) · **Date of research:** 2026-07-30
**Question:** Can a rented object store, or a consumer cloud drive reached through its own API, carry the review log between a user's devices with **no server of our own** — and what does each actually cost in bytes, requests, money, credentials and setup steps?

This is a **research** note. It gathers facts and sharpens trade-offs; it decides nothing. Every non-obvious claim carries an inline primary source. Claims I reasoned rather than sourced are marked **[inference]**. Claims I measured are marked **[measured]** and the command is in §10.

**Scope.** Clients are **desktop and Android only** — the web target was ruled out while resolving [#12](https://github.com/amin-bf/leitner/issues/12), so browser and cross-origin constraints are out of scope here. One user, 2–5 devices, no sharing between users. Two data shapes, per [ADR-0004](../../adr/0004-the-review-event-log.md): an append-only review log where every row carries `(writer id, sequence number)` and **each device appends only to its own rows**, plus a small mutable surface (deck names, tags, scheduler config, deletion flags) that settles per key by a counter that jumps above any counter it sees.

Sibling notes under this ticket cover the other candidate families. This one covers **(A) object storage the user rents** and **(B) personal cloud drives via their own APIs**.

---

## Summary of findings

Disqualifying or near-disqualifying first, then the numbers.

1. **Conditional writes are not needed for the log, and are mildly harmful there.** The mechanism the ticket names — compare the stored entity tag before overwriting, `412` if it moved — exists and is well specified: "PUT and DELETE requests MAY have an 'If-Match' request header, and MUST fail with a 412 response code if that does not match the document's current version" ([remoteStorage protocol draft-22, §6](https://datatracker.ietf.org/doc/html/draft-dejong-remotestorage-22)). But it solves *lost updates to a shared key*, and under a per-writer keyspace **no key ever has two writers**. What a plain unconditional `PUT` already guarantees is enough: "Any read (GET or LIST request) that is initiated following the receipt of a successful PUT response will return the data written by the PUT request", including "A process writes a new object to Amazon S3 and immediately lists keys within its bucket. The new object appears in the list." ([Amazon S3 data consistency model](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html)). Worse, a conditional `If-None-Match: *` turns the *harmless* case — a client retrying after an ambiguous timeout and re-uploading byte-identical content — into a spurious `412`, because "If multiple conditional writes or copies occur for the same object name, the first write operation to finish succeeds. Amazon S3 then fails subsequent writes with a `412 Precondition Failed` response" ([conditional writes](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html)). **This widens the candidate set exactly as the ticket suspected.** Details and the two places a race *does* survive in §1.

2. **Storage cost is not a discriminator; the user's uplink is.** At the ticket's worst case the monthly *bill* for two devices syncing six times a day is **$0.0040 on one metered store, $0.0032 on another, and $0.0000 on a third** [inference: my arithmetic over published prices, §3]. What differs by four orders of magnitude is bytes pushed: republishing a whole 10 MB per-writer log on every sync is **1,800 MiB per device per month** at six syncs a day and **7,200 MiB** at twenty-four, against **0.2 MiB** for segment objects. On a metered mobile connection that is the entire decision.

3. **A finding for ADR-0004 rather than for transport: its "compresses about ten to one" claim holds for `zstd` and fails for `gzip`.** A decade of rows in the ADR's own interchange form measures 111,295,393 B raw — reproducing ADR-0004 §10's "around 110 MB raw" to within 1% — compressing to **9,462,142 B with `zstd -19` (11.76×)** but only **27,907,906 B with `gzip -9` (3.99×)** [measured, §3.1]. Every row repeats a 36-character UUID and the same nine keys, and `gzip`'s 32 KiB window cannot reach far enough back to exploit it. **The compressor choice changes the decade-scale per-writer object by 3×.**

4. **Segment objects break "listing is the handshake" within a decade, and the fix is a key-name convention.** A listing page returns at most 1,000 keys — "By default, the action returns up to 1,000 key names. The response might contain fewer keys but will never contain more" ([ListObjectsV2](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListObjectsV2.html)). Two devices × six syncs/day × ten years is **43,800 live objects**, so a naïve full listing costs **44 round trips and ~14 MB** [inference, §2]. Issuing one listing *per writer prefix* with `start-after` set to the highest sequence that device already holds collapses this to **W requests of ~254 bytes each** [measured envelope, §2].

5. **Object metadata alone answers "am I behind?" — no `GET` needed.** A listing entry carries `Key`, `LastModified`, `ETag`, `Size` and `StorageClass` ([ListObjectsV2](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListObjectsV2.html), and [measured] against a live public bucket in §10). If the sequence number is zero-padded into the key name, the listing *is* the version summary. A five-writer snapshot listing measures **~1.8 KB in one round trip** [inference over the measured 254-byte envelope and 287-byte-plus-key-length per-entry cost].

6. **The credential problem is real and the three metered stores differ sharply.** Only one of them lets a user mint a key restricted to a *name prefix*, with an expiry: `namePrefix` "limits access to file names that begin with a specific prefix", `bucketIds` means "the new key can only access the specified buckets", and `validDurationInSeconds` "must be less than 1000 days (in seconds)" ([Backblaze b2_create_key](https://www.backblaze.com/apidocs/b2-create-key), [application keys](https://www.backblaze.com/docs/cloud-storage-application-keys)). Another supports bucket scoping but its token documentation describes no prefix scoping and no TTL — "you can scope your token to a set of buckets" and account tokens "remain valid until manually revoked" ([Cloudflare R2 API tokens](https://developers.cloudflare.com/r2/api/tokens/)). The third can express prefix restrictions but only through hand-written policy JSON with an explicit `Deny` on `s3:prefix` ([S3 bucket policy examples](https://docs.aws.amazon.com/AmazonS3/latest/userguide/amazon-s3-policy-keys.html)) — not something a user does unaided. **A leaked key on a lost phone costs, in the best case, write access to that device's own prefix until the expiry; in the worst case, full read/write/delete over the whole bucket, indefinitely.**

7. **One rented store is disqualified by its billing floors, not its API.** A 1 TB monthly minimum, a 90-day minimum storage duration and a 4 KB minimum billable object size ([Wasabi minimum storage duration policy](https://docs.wasabi.com/docs/how-does-wasabis-minimum-storage-duration-policy-work), [monthly minimum storage charge](https://docs.wasabi.com/docs/how-does-wasabis-monthly-minimum-storage-charge-work), [minimum object size](https://knowledgebase.wasabi.com/hc/en-us/articles/115001684511-What-are-the-minimum-and-maximum-object-sizes-that-can-be-stored-in-Wasabi)) mean a user storing 20 MB of review log pays for 1 TB. Segment objects of 389–1,190 bytes would each be billed as 4 KB — a **3–11× storage-billing inflation** on top of that [inference].

8. **The Rust story is clean on Android, but every candidate drags in a C toolchain.** Four crates `cargo check` clean for `aarch64-linux-android` [measured, §5] — but only once the NDK's `clang` is exported, because the default cryptographic backend compiles C: `error occurred in cc-rs: failed to find tool "aarch64-linux-android-clang"` from `aws-lc-sys v0.43.0`. The repo already installs that NDK for `cargo apk`, so this is a build-environment fact, not a blocker.

9. **The drive-API disqualifier is not OAuth itself — a public client with PKCE is explicitly supported everywhere — it is what surrounds it.** Native apps are public clients: "Native apps are classified as public clients…they MUST be registered with the authorization server as such" and "it is NOT RECOMMENDED for authorization servers to require client authentication of public native apps clients using a shared secret" ([RFC 8252 §8.4, §8.5](https://datatracker.ietf.org/doc/html/rfc8252)). The providers agree: the client secret is marked "(Optional)" for installed apps and "is not applicable to requests from clients registered as Android, iOS, or Chrome applications" ([Google, OAuth for iOS & desktop](https://developers.google.com/identity/protocols/oauth2/native-app)); "Public clients, which include native applications and single page apps, must not use secrets or certificates when redeeming an authorization code" ([Microsoft identity platform auth code flow](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow)). **The costs are elsewhere:** one provider freezes new user links two weeks after the 50th user unless a review is passed ([Dropbox developer guide](https://www.dropbox.com/developers/reference/developer-guide)); another expires refresh tokens after **7 days** while the project sits in "Testing" publishing status, and caps live refresh tokens at **100 per Google Account per client ID** ([Google OAuth 2.0](https://developers.google.com/identity/protocols/oauth2)).

10. **The one genuinely good news on drives: the app-private folder scope is *non-sensitive*, so verification is not mandatory.** "Before you can access the application data folder, you must request access to the `https://www.googleapis.com/auth/drive.appdata` **non-sensitive** scope" ([Drive application data folder](https://developers.google.com/workspace/drive/api/guides/appdata)), and "If your app utilizes only *non-sensitive* scopes, it is not mandatory for your app to complete the app verification process" ([Google OAuth verification](https://support.google.com/cloud/answer/13463073)). No app review, no verification-time endpoint (which would have been a server).

11. **Change detection on drives is cheaper than listing an object store, and one of them has a free push channel.** A long-poll endpoint blocks up to 480 s, needs **no authentication**, runs on a separate host, and works for app-folder apps: `timeout UInt64(min_value=30, max_value=480) = 30`, `host = "notify"`, `auth = "noauth"`, `allow_app_folder_app = true` ([Dropbox API spec, `files.stone`](https://github.com/dropbox/dropbox-api-spec/blob/master/files.stone)) [measured — I read the spec file directly, §10].

12. **Unattended background sync on Android is capped by the platform regardless of transport, and in two app-standby buckets it is impossible.** Doze "Suspends network access", "Doesn't let `JobScheduler` run" and "Defers standard `AlarmManager` alarms […] to the next maintenance window" ([Doze and App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)). Worse, in the **Rare** and **Restricted** app-standby buckets **network is listed as "Disabled"** ([power management restrictions](https://developer.android.com/topic/performance/power/power-details)) — precisely the buckets a device that has been in a drawer lands in. The deferrable-work API's floor is **15 minutes** between periodic runs ([define work](https://developer.android.com/develop/background-work/background-tasks/persistent/getting-started/define-work)), and escaping Doze with a foreground service now costs a hard cap: "The system permits an app's `dataSync` services to run for a total of 6 hours in a 24-hour period" and such a service **cannot be started from boot** ([Android 15 behaviour changes](https://developer.android.com/about/versions/15/behavior-changes-15)).

---

## Part A — Object storage the user rents

### 1. Conditional writes: what exists, and whether we need any

#### 1.1 The model the ticket names

The cleanest primary statement of the entity-tag-plus-precondition model is the remoteStorage protocol, which makes it normative rather than advisory:

> "All successful GET, HEAD, PUT and DELETE requests MUST return an 'ETag' header with, in the case of GET and HEAD the current version, in the case of PUT, the new version, and in case of DELETE, the version that was deleted."
>
> "PUT and DELETE requests MAY have an 'If-Match' request header, and MUST fail with a 412 response code if that does not match the document's current version."
>
> "A PUT request MAY have an 'If-None-Match: *' header, in which case it MUST fail with a 412 response code if the document already exists."
>
> — [remoteStorage protocol, draft-dejong-remotestorage-22, §6](https://datatracker.ietf.org/doc/html/draft-dejong-remotestorage-22)

The same draft makes folder listings carry versions, so a client can diff a directory without downloading it: a folder GET returns per-item descriptions each containing "a string-valued 'ETag' field…representing the document's current version" (same source, §4). That is the shape of a cheap "am I behind?" handshake, and it is worth noting the protocol got there in 2016 without inventing anything — it is plain HTTP conditional requests, [RFC 7232](https://datatracker.ietf.org/doc/html/rfc7232).

#### 1.2 What object stores actually support, and when it arrived

| Store | `If-None-Match: *` on write | `If-Match: <etag>` on write | Arrived | Source |
|---|---|---|---|---|
| Amazon S3 | Yes, on `PutObject`, `CompleteMultipartUpload`, `CopyObject` | Yes, same three | `If-None-Match` **2024-08-20**, `If-Match` **2024-11-25** | [conditional writes](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html); [announcement, Aug 2024](https://aws.amazon.com/about-aws/whats-new/2024/08/amazon-s3-conditional-writes/); [announcement, Nov 2024](https://aws.amazon.com/about-aws/whats-new/2024/11/amazon-s3-functionality-conditional-writes/) |
| Cloudflare R2 | Yes | Yes (also `If-Modified-Since`, `If-Unmodified-Since`) | not dated in the doc | [R2 S3 API compatibility](https://developers.cloudflare.com/r2/api/s3/api/) |
| Backblaze B2 (S3-compatible) | **Not documented** | **Not documented** | — | [B2 S3-compatible API](https://www.backblaze.com/docs/cloud-storage-s3-compatible-api) lists unsupported features as "ACLs, IAM Roles, Object Tagging, Website Configuration, Browser-based uploads to pre-signed URLs using `POST`" and says nothing about conditional write headers |

The exact semantics, which matter if we ever *do* want them:

> "Conditional writes with the `If-None-Match` header evaluate against existing objects in a bucket. If there's no existing object with the same key name in the bucket, the write operation succeeds, resulting in a `200 OK` response. If there's an existing object, the write operation fails, resulting in a `412 Precondition Failed` response."
>
> "If multiple conditional writes or copies occur for the same object name, the first write operation to finish succeeds. Amazon S3 then fails subsequent writes with a `412 Precondition Failed` response."
>
> "You can also receive a `409 Conflict` response in the case of concurrent requests if a delete request to an object succeeds before a conditional write operation on that object completes."
>
> — [How to prevent object overwrites with conditional writes](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-writes.html)

Two operational constraints from the same page: `If-Match` needs both `s3:PutObject` **and** `s3:GetObject` permission ("This enables the caller to check the ETag"), and "To use conditional writes, you must use AWS Signature Version 4 to sign the request."

**Confidence on B2:** *low-to-medium* that conditional writes are unsupported. Backblaze's own documentation does not mention them either way, and third-party comparisons claim support. **This must be tested against a real bucket before it is load-bearing** — it is exactly the kind of claim that silently changes.

#### 1.3 The sharper question: is a conditional write needed at all?

**For the review log, no — and it is actively counterproductive.**

The argument, stated as a chain so it can be attacked:

1. Every log row carries `(writer id, sequence number)` and each device appends only to its own rows, gap-free (ADR-0004). So a layout of `log/<writer id>/<sequence>` — whether one object per writer or one per segment — has the property that **each key has exactly one possible author**.
2. A plain `PUT` to a key nobody else writes cannot lose an update. There is no second writer to lose it to.
3. The store already guarantees the reader sees it: "Amazon S3 provides strong read-after-write consistency for PUT and DELETE requests of objects in your Amazon S3 bucket in all AWS Regions. This behavior applies to both writes to new objects as well as PUT requests that overwrite existing objects and DELETE requests" ([data consistency model](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html)).
4. Listing is covered too, which is what makes the handshake in §2 sound: "A process writes a new object to Amazon S3 and immediately lists keys within its bucket. The new object appears in the list" (same source). **List-after-write is strongly consistent, not eventually consistent** — this is the 2020 change that makes the whole design viable, and it is why prior-generation object-store sync designs needed conditional writes and ours does not [inference].
5. The one behaviour that *would* bite a shared key is documented and does not apply to us: "Amazon S3 does not support object locking for concurrent writers. If two PUT requests are simultaneously made to the same key, the request with the latest timestamp wins. If this is an issue, you must build an object-locking mechanism into your application" (same source).
6. Adding `If-None-Match: *` on top makes retries unsafe rather than safe. The realistic failure is *not* two devices racing; it is one device losing its connection after the store committed the write but before the response arrived, then retrying with identical bytes. Unconditionally that retry is idempotent and free. Conditionally it returns `412`, which the client cannot distinguish from "someone else got there first" without an extra `HEAD` **[inference]**.

**Where the races actually are.** Three, none of them in the log:

- **The mutable surface, only if it is a single shared object.** If deck names, tags and scheduler config live in one `config.json` that every device overwrites, that *is* a shared key and it *does* lose updates — the "latest timestamp wins" rule above applies literally. **But ADR-0004 §7 already settles values by a per-key counter that jumps above any counter it sees, which is a merge function, not an overwrite.** Sharding the mutable surface per writer (`state/<writer id>.json`, merged at read time by that counter rule) removes the shared key and therefore removes the need for a conditional write. The trade is one small `GET` per writer instead of one, and slightly more read-side code. **This is the real decision the mutable-store bullet in the ticket is pointing at, and it is a design choice, not a constraint imposed by the store** [inference].
- **Compaction and roll-up.** Deleting segments after merging them into a larger object is the one place where a stale actor can destroy data another actor still needs. The lock-free discipline for this is already recorded in the sibling note: keep track of every change loaded from storage and "when compacting *only delete those changes*" ([Automerge storage under-the-hood](https://github.com/automerge/automerge.github.io/blob/main/content/docs/reference/under-the-hood/storage.md), quoted in [`../local-first-event-log/README.md`](../local-first-event-log/README.md) §1). A conditional delete would also work here — "Conditional deletes evaluate if your object exists or is unchanged before deleting it. You can perform conditional deletes using the `DeleteObject` or `DeleteObjects` APIs" ([conditional requests](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-requests.html)) — but the read-set discipline needs nothing from the store.
- **Torn publication, which is a liveness bug not a correctness bug.** A reader can see writer A's new segment before A's head pointer, or the pointer before the segment. Writing data before pointers, and treating a dangling pointer as "not yet there, retry", makes this benign. If the sequence number lives in the key name (§2) there is no separate pointer at all and the window closes entirely **[inference]**.

**Consequence for the candidate set.** Any store that offers unconditional `PUT`, `GET`, `DELETE` and a prefix listing is sufficient. Conditional-write support becomes a *nice-to-have for compaction*, not an entry requirement — which is exactly the widening the ticket asked us to test rather than assume.

### 2. The handshake: what a listing costs

#### 2.1 The API's own limits

- **Page size:** "By default, the action returns up to 1,000 key names. The response might contain fewer keys but will never contain more." Pagination is by opaque `NextContinuationToken`, which "is obfuscated and is not a real key".
- **Ordering:** "For general purpose buckets, `ListObjectsV2` returns objects in lexicographical order based on their key names." (Directory buckets do **not** — worth knowing if anyone reaches for the low-latency bucket type.)
- **Prefix:** "Limits the response to keys that begin with the specified prefix."
- **Delimiter:** rolls keys up into `CommonPrefixes`, and "All of the keys that roll up into a common prefix count as a single return when calculating the number of returns."
- **`start-after`:** "StartAfter is where you want Amazon S3 to start listing from. Amazon S3 starts listing after this specified key."
- **Fields per entry:** `Key`, `LastModified`, `ETag`, `Size`, `StorageClass` (plus checksum fields in practice); `Owner` is omitted unless `fetch-owner=true`.

All from [ListObjectsV2](https://docs.aws.amazon.com/AmazonS3/latest/API/API_ListObjectsV2.html).

The equivalent on the native B2 API: `maxFileCount` defaults to 100 with a maximum of 10,000, but "the maximum number of files returned per transaction is 1000", entries carry `fileName`, `contentLength`, `contentSha1`, `uploadTimestamp` and `fileId`, and `prefix`, `delimiter` and `startFileName` all exist ([b2_list_file_names](https://www.backblaze.com/apidocs/b2-list-file-names)).

#### 2.2 Measured wire cost

Against a live public bucket (§10 has the commands):

| Request | Response bytes |
|---|---|
| `?list-type=2&prefix=zzzznonexistent&max-keys=5` (empty result) | **254** |
| `?list-type=2&max-keys=1` | 865 |
| `?list-type=2&max-keys=2` | 1,260 |
| `?list-type=2&max-keys=5` | 2,445 |

Per-entry cost is `(2445 − 865) / 4 = 395` bytes for a **108-character** key, so the fixed XML per entry is **287 bytes plus the key length** [measured + inference]. The 254-byte envelope grows only by the prefix string; the 865-byte single-entry response also carries a ~228-byte continuation token that a complete listing does not.

Applying that to a realistic key, `log/3f9a2b1c/0000000000000731.zst` (33 characters):

| Layout | Live objects | Requests | Bytes on the wire |
|---|---|---|---|
| One object per writer, sequence in the key, 5 writers | 5 | **1** | 254 + 5 × 320 = **~1.85 KB** |
| Segments, 2 devices × 6 syncs/day × 10 years, listed whole | 43,800 | **44** | ~14.1 MB |
| Segments, same, one listing per writer prefix with `start-after` = highest sequence held | 43,800 | **W** (2) | **~254 B each** when up to date |

[inference: arithmetic over the measured per-entry cost.] The third row is the one that matters: **the version summary the device already holds is exactly the `start-after` key**, so "am I behind?" is `KeyCount == 0`.

#### 2.3 Can metadata alone answer it — and can the key name *be* the handshake?

Yes to both.

- The listing returns `ETag` and `Size` without any `GET`. Verified live: `HEAD` on a public object returns `ETag: "7fb98fdd354afbd9f6ac81fe9a511279"`, the same value that appeared in the listing [measured].
- Conditional reads work as specified and cost nothing when nothing changed: `GET` with `If-None-Match` on the current tag returned **`304`, 0 bytes downloaded**; `GET` with a wrong `If-Match` returned **`412`, 348 bytes** [measured]. So even a content fetch can be made free-when-unchanged. There is no charge for the conditionality itself: "There is no additional charge for conditional reads, conditional writes or conditional deletes. You are only charged existing rates for the applicable requests, including for failed requests" ([conditional requests](https://docs.aws.amazon.com/AmazonS3/latest/userguide/conditional-requests.html)).
- **Encoding the sequence number in the key name makes the listing the handshake.** Keys sort lexicographically (sourced above), so a zero-padded fixed-width sequence sorts numerically. `prefix=log/&delimiter=/` returns one `CommonPrefixes` entry per writer for discovery; `prefix=log/<writer>/&start-after=<highest held>` returns exactly the rows we lack. No `GET`, no separate manifest object, no pointer that can be torn.

**Caveat.** Under the *snapshot* shape this requires publishing to a **new** key each time (the sequence changes) and deleting the old one, which is two operations and a transient window with two objects for that writer. Under the *segment* shape each key is written once and never rewritten, which is strictly simpler. [inference]

### 3. Appending: nobody supports it, so segments versus snapshots

**No candidate supports true append.** Object stores replace whole objects; the S3 documentation's own framing is that "Updates to a single key are atomic" and that a PUT to an existing key overwrites it ([data consistency model](https://docs.aws.amazon.com/AmazonS3/latest/userguide/Welcome.html)). Multipart upload is not an append either — parts must be assembled into one `CompleteMultipartUpload`, and "Uploading to the same part number replaces the previous part" ([R2 S3 API](https://developers.cloudflare.com/r2/api/s3/api/)). So the choice is segments or republish.

#### 3.1 Measured payload sizes

Synthetic rows in **exactly** the interchange form ADR-0004 §11 pins — `{"k","w","s","n","o","g","t","d","ms"}` with a 16-hex writer id and an RFC 9562 canonical note UUID — as JSON Lines [measured, §10]:

| Rows | Raw | zstd-19 | gzip-9 | zstd ratio |
|---|---|---|---|---|
| 730,000 (a decade at 200/day) | **111,295,393 B (106.1 MiB)** | **9,462,142 B (9.02 MiB)** | 27,907,906 B | **11.76×** |
| 73,000 (one year) | 11,056,472 B | 1,036,927 B | 2,791,194 B | 10.66× |
| 200 (one day) | 29,816 B | 6,330 B | 7,875 B | 4.71× |
| 33 (one sync at 6/day) | 4,899 B | **1,190 B** | 1,352 B | 4.12× |
| 8 (one sync at 24/day) | 1,181 B | **389 B** | 402 B | 3.04× |

Three things this settles:

- **Rows are ~152 bytes, not 38–58.** The brief for this ticket quoted 38–58 bytes/row, which is the *packed* figure ADR-0004 §11 explicitly contrasts against ("roughly 150 bytes against 56"). The 150-byte figure is the one that matters for transport, because §11 also requires that "A row is relayed byte for byte and never re-encoded" — **the interchange form is what crosses the wire, so transport must be costed against 152 bytes.** At 152.5 B/row the decade total is 111.3 MB raw, which reproduces ADR-0004 §10's "around 110 MB raw" to within 1%.
- **ADR-0004's "compresses about ten to one" is true for `zstd`, and false for `gzip`.** At decade scale `zstd -19` gives **11.76×** but `gzip -9` gives only **3.99×** — 9.0 MiB against 26.6 MiB for the same bytes. The cause is window size: every row repeats a 36-character UUID and the same nine keys, and `gzip`'s 32 KiB window cannot reach back far enough to exploit it. **The compressor choice changes a decade-scale per-writer object by 3×**, which is not a detail if snapshots are on the table [inference]. ADR-0004 §10's "15 MB compressed" sits between the two and is conservative for `zstd`.
- **Small segments compress badly** — 3.04× at eight rows against 11.76× at a decade — so per-sync overhead is worse than the annual ratio suggests, though still trivial in absolute terms (389 bytes).

For the cost table below I use the ticket's round **10 MB per-writer snapshot**, which the measurement supports as the worst case: 9.02 MiB if a single writer produced the whole decade at 200/day.

#### 3.2 Published prices, from the providers

Amazon S3, US East (N. Virginia), read out of AWS's own machine-readable price list rather than the marketing page [measured, §10]:

- Storage, S3 Standard: **`$0.023` per GB-month** for the first 50 TB (`TimedStorage-ByteHrs`, General Purpose).
- `PUT`, `COPY`, `POST`, `LIST`: **`$0.005` per 1,000** (`Requests-Tier1`).
- `GET` and all others: **`$0.0004` per 1,000** (`Requests-Tier2`, listed as `$0.004 per 10,000`).
- Data transfer out to the internet: **`$0.090` per GB** for the first 10 TB/month "beyond the global free tier", then `$0.085`, `$0.070`, `$0.050`. The [pricing page](https://aws.amazon.com/s3/pricing/) puts that free tier at "the first 100GB per month, aggregated across all AWS Services and Regions".
- **Minimums:** S3 Standard states none. "S3 Standard-IA and S3 One Zone-IA storage have a minimum billable object size of 128 KB" and "are charged for a minimum storage duration of 30 days" (same page) — so the cheaper tiers are *not* usable for small segment objects.

Cloudflare R2 ([pricing](https://developers.cloudflare.com/r2/pricing/)):

- Storage: **`$0.015` / GB-month** (Standard).
- **Class A `$4.50` / million** — includes `PutObject`, `ListObjects`, `CopyObject`, `CreateMultipartUpload`, `CompleteMultipartUpload`, `UploadPart`.
- **Class B `$0.36` / million** — includes `GetObject`, `HeadObject`.
- **Free operations:** `DeleteObject`, `DeleteBucket`, `AbortMultipartUpload`.
- **Egress: free.**
- **Free tier:** 10 GB-month storage, 1 million Class A, 10 million Class B per month.
- Minimum storage duration of 30 days applies only to Infrequent Access; Standard has none.

Backblaze B2 ([pricing](https://www.backblaze.com/cloud-storage/pricing), [transaction pricing](https://www.backblaze.com/cloud-storage/transaction-pricing)):

- Storage: **`$6.95` per TB/month** (= `$0.00679` / GB-month), and "First 10GB storage is always free".
- Egress: **`$0.01`/GB** beyond "up to 3x of average monthly data stored", which is free.
- **Class A (uploads: `b2_upload_file` / `PutObject`), Class B (downloads: `GetObject`, `HeadObject`) and Class C (listing: `b2_list_file_names` / `ListObjectsV2`) are all listed as free** for pay-as-you-go. Only Class D (event notifications) is charged, at "$0.004 per 10,000" with the first 2,500/day free.
- No minimum object size or minimum storage duration is stated.

Wasabi ([pricing](https://wasabi.com/pricing)): "Starting at $7.99 TB/month", "No fees for egress or API requests" — but see finding 7: a **1 TB monthly minimum**, a **90-day minimum storage duration** and a **4 KB minimum billable object size**. **Disqualified for this workload on billing floors alone.**

#### 3.3 The actual monthly numbers

Steady state at the ticket's worst case: 10 MB compressed per-writer log, `W` devices, `N` syncs/day, 30 days, one publish and one listing per device per sync [inference: my arithmetic over the prices above; AWS's 100 GB/month egress free tier ignored, which only makes the S3 column pessimistic].

| Devices | Syncs/day | Shape | Uploaded/month | Class-A-equivalent ops | S3 | R2 | B2 | Live objects after 10 yrs |
|---|---|---|---|---|---|---|---|---|
| 2 | 6 | snapshot | **3,600 MiB** | 720 | $0.0040 | $0.0032 | $0.0000 | 2 |
| 2 | 6 | segment | **0.4 MiB** | 720 | $0.0040 | $0.0032 | $0.0000 | 43,800 |
| 2 | 24 | snapshot | **14,400 MiB** | 2,880 | $0.0148 | $0.0130 | $0.0000 | 2 |
| 2 | 24 | segment | **0.5 MiB** | 2,880 | $0.0148 | $0.0130 | $0.0000 | 175,200 |
| 5 | 6 | snapshot | **9,000 MiB** | 1,800 | $0.0101 | $0.0081 | $0.0000 | 5 |
| 5 | 6 | segment | **1.0 MiB** | 1,800 | $0.0101 | $0.0081 | $0.0000 | 109,500 |
| 5 | 24 | snapshot | **36,000 MiB** | 7,200 | $0.0371 | $0.0324 | $0.0000 | 5 |
| 5 | 24 | segment | **1.3 MiB** | 7,200 | $0.0371 | $0.0324 | $0.0000 | 438,000 |

**Per-device upload per month: 1,800 MiB (snapshot, 6 syncs/day), 7,200 MiB (snapshot, 24 syncs/day), 0.2–0.3 MiB (segments, either).**

Three things fall out:

1. **The bill does not distinguish the shapes at all**, because ingress is free everywhere and 10 MB of storage is inside every free tier. Snapshot and segment rows are identical to the cent. Anyone arguing the shapes on cost is arguing about a rounding error.
2. **The uplink distinguishes them by a factor of ~9,000.** Republishing costs the *user's* bandwidth, battery and time — 1.8–7.2 GB per device per month, much of it plausibly cellular. That is the whole case against snapshots, and it is not visible in any provider's pricing page.
3. **Segments trade that for object-count growth** that breaks naïve listing (finding 4) and would be catastrophic under a 4 KB minimum billable object size (finding 7, on the store where that applies: 438,000 objects × 4 KB = 1.7 GB billed for ~50 MB of data [inference]).

**A third shape worth naming, since neither of the ticket's two is obviously right: segments plus periodic roll-up.** Write a segment per sync; once a writer accumulates, say, 512 segments, merge them into one object and delete the merged ones under the read-set discipline from §1.3. That bounds live objects to roughly `W × 512` while keeping per-sync upload at segment size. Cost: one 10 MB republish per writer per ~85 days at six syncs/day, i.e. **~0.12 MiB/day amortised** against 60 MiB/day for republish-every-sync [inference].

### 4. The credential problem

With no server, the client holds long-lived storage credentials. There is no way around this: every one of these stores authenticates with a static key pair signed per request (AWS Signature Version 4 or the provider's equivalent). What differs is how narrowly the key can be cut and how long it lives.

| | Scope to one bucket | Scope to a key prefix | Expiry | User can create it unaided |
|---|---|---|---|---|
| Backblaze B2 | Yes (`bucketIds`) | **Yes (`namePrefix`)** | **Yes, `validDurationInSeconds` < 1000 days** | Yes — the console exposes bucket, capability and prefix as form fields |
| Cloudflare R2 | Yes | **Not documented** | **Not documented** | Yes — but only at bucket granularity |
| Amazon S3 | Yes | Yes, via IAM policy JSON with `s3:prefix` and an explicit `Deny` | Only via STS (which needs something to call it) | **Realistically no** |

Sources: [b2_create_key](https://www.backblaze.com/apidocs/b2-create-key) ("When provided, the new key can only access the specified buckets"; `namePrefix` "limits access to file names that begin with a specific prefix"; `validDurationInSeconds` "must be less than 1000 days (in seconds)"), [B2 application keys](https://www.backblaze.com/docs/cloud-storage-application-keys) (capabilities are "Read and Write - Read Only - Write Only"), [R2 API tokens](https://developers.cloudflare.com/r2/api/tokens/) ("you can scope your token to a set of buckets"; account tokens "remain valid until manually revoked"), [S3 bucket policy examples](https://docs.aws.amazon.com/AmazonS3/latest/userguide/amazon-s3-policy-keys.html).

The S3 prefix pattern, for the record, is not one statement but two — an `Allow` on `s3:ListBucket` conditioned on `"s3:prefix": "projects"` **plus** an explicit `Deny` for any other prefix, because "explicit `Deny` statements always override `Allow` statements" (same source). Anyone who writes only the `Allow` has an ineffective policy, since a broader grant elsewhere silently wins. That is a footgun, not a user-facing feature.

**What a leaked key on a lost phone costs.** All three key types are bearer credentials with no device binding and no proof-of-possession.

- Best case (prefix-scoped, write-only, expiring): the attacker can write objects under that one device's prefix until the expiry. They cannot read the other devices' rows, cannot delete, and the damage self-heals when the key expires.
- Realistic case (bucket-scoped, read-write, no expiry): the attacker reads the user's entire review history — which cards they study, when, and how badly — and can delete all of it. Review logs are not neutral data; study history discloses what someone is learning and when they are awake.
- All three providers show the secret exactly once. "Your master app key is shown only when you generate it, and it is not shown again" and "This is the only time it will be returned, so you need to keep it" ([application keys](https://www.backblaze.com/docs/cloud-storage-application-keys), [b2_create_key](https://www.backblaze.com/apidocs/b2-create-key)). So recovery from a lost phone means *rotating* the key on every remaining device, by hand.
- **There is no revocation channel we control.** Revocation happens in the provider's console, by the user, if they think of it.

**Setup burden, counted honestly.** Steps between "user installs the app" and "sync works", for the most favourable of the three:

1. Create an account with the storage provider (email, password, payment card — even on a free tier).
2. Create a bucket, choosing a globally-or-account-unique name and a region.
3. Open the application-keys page, create a key, choose the bucket, choose read-and-write, type a name prefix, optionally set a duration.
4. Copy a key ID and a secret shown once.
5. Type or paste both into the app — **on each of 2–5 devices**, including the phone.

That is five steps and an account signup before the first review syncs, repeated in part per device. On Android step 5 means entering a ~25-character key ID and a ~31-character secret into a text field; both are ASCII, so the repo's ASCII-only Android IME limitation (`AGENTS.md` rule 8) does not block it, but it is still a miserable first-run experience and a strong argument for a device-to-device transfer of the credential (QR code, or a paired-device handoff) rather than retyping [inference].

**This is the finding most likely to disqualify the whole family.** Not any single technical gap — the API surface is genuinely sufficient — but that the design asks a language-learner to become an object-storage administrator, and hands them a bearer secret whose loss is unrecoverable without manual rotation.

### 5. Rust client support on Android

All four candidates `cargo check` clean for `aarch64-linux-android` with `rustc 1.97.0` on 2026-07-30 [measured, §10]:

| Crate | Version checked | Licence | Repo last release | Notes |
|---|---|---|---|---|
| `object_store` | 0.14.1, feature `aws` | MIT/Apache-2.0 | 2026-07-15 | Apache Arrow project; 75.6M downloads |
| `aws-sdk-s3` + `aws-config` | 1.140.0 / 1.10.1 | Apache-2.0 | 2026-07-24 | AWS's own SDK; 74.2M downloads |
| `rust-s3` | 0.36.0 (`default-features = false`, `tokio-rustls-tls`) | MIT | 0.37.2, 2026-05-04 | community-maintained, single maintainer |
| `opendal` | 0.57.0, features `services-s3`, `services-gdrive`, `services-dropbox`, `services-onedrive` | Apache-2.0 | 2026-07-27 | Apache project; **also covers all three consumer drives in Part B behind one interface** |

Metadata from the crates.io API [measured, §10].

**The one real friction: the cryptographic backend needs a C toolchain.** With no `CC` exported, the `object_store` check fails:

```
Compiling aws-lc-sys v0.43.0
error: failed to run custom build command for `aws-lc-sys v0.43.0`
  cargo:warning=Compiler family detection failed due to error: ToolNotFound:
    failed to find tool "aarch64-linux-android-clang"
  error occurred in cc-rs: failed to find tool "aarch64-linux-android-clang"
```

`cargo tree -i aws-lc-sys` shows it arriving by two independent routes — `object_store` depends on `aws-lc-rs` directly (request signing), and `rustls 0.23` pulls it as the default cryptographic provider through `reqwest`/`hyper-rustls`/`rustls-platform-verifier`. Exporting `CC_aarch64_linux_android` to the NDK's `aarch64-linux-android24-clang` makes all four check clean in 7–17 s. The repo already pins exactly one NDK for `cargo apk` (`scripts/android-env.sh`, NDK 29.0.13846066), so this is satisfied by the existing toolchain.

The alternative backend, `ring`, is not obviously better: it also builds C and assembly, and its last release was **2025-03-11**, sixteen months before this note, against `aws-lc-rs`'s 2026-07-17 [measured from crates.io]. Licences: `aws-lc-rs` is `ISC AND (Apache-2.0 OR ISC)`, `ring` is `Apache-2.0 AND ISC` — both permissive, neither copyleft.

**No OpenSSL anywhere** in the checked configurations, which matters because a Rust `openssl-sys` build for Android is the classic source of NDK pain. Every candidate defaults to rustls.

---

## Part B — Personal cloud drives via their own APIs

### 6. App-scoped folders

| Provider | Name | What the app sees | What the user sees | Quota | Source |
|---|---|---|---|---|---|
| Google Drive | application data folder (`appDataFolder` space) | "a special hidden folder that your app can use to store application-specific data"; "Only the application that created the data in the `appDataFolder` can access it" | "its contents are hidden from the user and from other Google Drive apps"; the folder "is deleted when a user uninstalls your app from their My Drive. Users can also delete your app's data folder manually" | Counts as "other storage", which "often includes device backups and hidden data from apps connected to your Drive"; user removes it via Drive settings → **Manage apps** | [appdata guide](https://developers.google.com/workspace/drive/api/guides/appdata), [about files](https://developers.google.com/workspace/drive/api/guides/about-files), [Drive storage help](https://support.google.com/drive/answer/6374270) |
| Dropbox | App folder | "A dedicated folder named after your app is created within the Apps folder of a user's Dropbox"; the app "gets read and write access to this folder only" | **Fully visible.** "users can contribute by moving files into it" | user's Dropbox quota [inference] | [developer guide](https://www.dropbox.com/developers/reference/developer-guide) |
| OneDrive | App Root special folder (`approot`) | "The application's personal folder. Usually in `/Apps/{Application Name}`"; "Special folders are automatically created the first time an application attempts to write to one, if it doesn't already exist. If a user deletes one, it is recreated when written to again." | Visible at that path | user's OneDrive quota [inference] | [get special folder](https://learn.microsoft.com/en-us/onedrive/developer/rest-api/api/drive_get_specialfolder) |

Two consequences worth flagging for the decision:

- **The hidden folder is a backup hazard, not a backup.** Content the user cannot see is content the user cannot copy out, and the app's own uninstall deletes it. The two visible app folders are the opposite: the user can inspect the JSON Lines, copy them, and — inconveniently — also move or delete them mid-sync. A design that tolerates a file vanishing is required either way [inference].
- **Google's folder has operations that simply fail**: apps "cannot share files within this folder, move files between storage locations, or trash files", which returns `notSupportedForAppDataFolderFiles` ([appdata guide](https://developers.google.com/workspace/drive/api/guides/appdata)). Deletion must be permanent rather than to trash — fine for segment roll-up, but it removes the safety net.

### 7. OAuth for a native app with no server

#### 7.1 What the standard requires

[RFC 8252, *OAuth 2.0 for Native Apps*](https://datatracker.ietf.org/doc/html/rfc8252) is unusually prescriptive, and every requirement it imposes is satisfiable without a server:

- §5: "native apps MUST use an external user-agent to perform OAuth authorization requests", and §8.12: "This best current practice requires that native apps MUST NOT use embedded user-agents to perform authorization requests." The rationale is that an embedded webview lets the host app "access the user's full authentication credential, not just the OAuth authorization grant."
- §6: "Public native app clients MUST implement the Proof Key for Code Exchange (PKCE) extension to OAuth, and authorization servers MUST support PKCE for such clients." §8.1: "An app that intercepted the authorization code would not be in possession of this secret, rendering the code useless."
- §7: three redirect options — private-use URI schemes (§7.1), claimed `https` URIs (§7.2), loopback interface redirection (§7.3).
- §8.4: "Native apps are classified as public clients…they MUST be registered with the authorization server as such." §8.5: "Secrets that are statically included as part of an app distributed to multiple users should not be treated as confidential secrets…it is NOT RECOMMENDED for authorization servers to require client authentication of public native apps clients using a shared secret."

**None of this needs a server.** A loopback redirect means a listener on `127.0.0.1:<random port>` inside our own process; an external user-agent means launching the system browser. Both are ordinary desktop and Android capabilities.

#### 7.2 Is a client secret required, and can a public client be registered?

**No, and yes, on all three.**

- Google marks the client secret "(Optional)" in the installed-app token exchange, and states "The `client_secret` is not applicable to requests from clients registered as Android, iOS, or Chrome applications." For desktop, the redirect is the loopback address: "http://127.0.0.1:port or http://[::1]:port". PKCE is supported: "Google supports the Proof Key for Code Exchange (PKCE) protocol to make the installed app flow more secure" — though Google labels it "Recommended" rather than "Required", which is weaker than RFC 8252's MUST. Two removals to note: "Custom URI schemes are no longer supported due to the risk of app impersonation" (for the iOS/desktop client types), and "The manual copy/paste option, also referred to as an out of band (OOB) redirect method, is no longer supported." ([OAuth for iOS & desktop](https://developers.google.com/identity/protocols/oauth2/native-app))
- Microsoft is blunt: "Public clients, which include native applications and single page apps, must not use secrets or certificates when redeeming an authorization code", and the client secret is "required for confidential web apps" only, with the note "Don't use the application secret in a native app or single page app because a `client_secret` can't be reliably stored on devices or web pages." Recommended native redirect values are `https://login.microsoftonline.com/common/oauth2/nativeclient` for embedded browsers or **`http://localhost` for apps that use system browsers**. PKCE is "recommended for all application types, both public and confidential clients". ([auth code flow](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow))
- Dropbox recommends PKCE precisely for this case: "PKCE is an open extension to OAuth 2.0, and solves this problem using dynamic codes instead of the static client_secret", and directs "desktop apps, mobile apps, single-page applications, and open source applications" to "the OAuth code flow with PKCE, with refresh tokens". Refresh tokens require `token_access_type=offline` on the authorization URL. ([OAuth guide](https://developers.dropbox.com/oauth-guide))

#### 7.3 App review, and what the scopes cost

This is where the family gets expensive, and the three diverge completely.

- **Google: no review needed for the folder we want.** The application-data-folder scope is explicitly categorised **non-sensitive** ([appdata guide](https://developers.google.com/workspace/drive/api/guides/appdata)), and "Apps that request access to scopes categorized as *sensitive* or *restricted* must complete Google's OAuth app verification" while "If your app utilizes only *non-sensitive* scopes, it is not mandatory for your app to complete the app verification process" ([verification requirements](https://support.google.com/cloud/answer/13463073)). **Crucially, nothing here requires the developer to run a verification-time endpoint** — no domain ownership proof, no hosted privacy policy check, no callback the reviewer hits. That would have been a server. *Confidence: medium-high.* Google's verification policy has moved before, and brand verification is still required if the consent screen is to show a logo (same source).
- **Dropbox: a review gate at 50 users.** Apps start in development status and can link only the creator's account; then "you will have two weeks to apply for and receive production status approval before your app's ability to link additional Dropbox users will be frozen", triggered at 50 linked users. Review checks "that your app doesn't request an unnecessarily broad permission based on the functionality provided" ([developer guide](https://www.dropbox.com/developers/reference/developer-guide)). App-folder access is the least-privileged option, which helps, but **the ceiling is hard: the 51st user cannot connect until a human at Dropbox approves.**
- **Microsoft:** the app-folder permission `Files.ReadWrite.AppFolder` is listed as the **least privileged** delegated permission for personal Microsoft accounts on the special-folder API ([get special folder](https://learn.microsoft.com/en-us/onedrive/developer/rest-api/api/drive_get_specialfolder)). I did not find a consumer-account review gate equivalent to the other two. *Confidence: low* — absence of evidence, and I did not exhaust Microsoft's publisher-verification documentation.

#### 7.4 Refresh tokens: the quiet disqualifier for unattended sync

Unattended sync means the token must survive months of not being used. It does not, uniformly.

Google states the failure conditions outright ([OAuth 2.0](https://developers.google.com/identity/protocols/oauth2)):

- "The refresh token has not been used for six months" — **a device in a drawer for seven months needs the user to sign in again.** This is exactly the offline-device case the whole design is meant to survive.
- "There is currently a limit of 100 refresh tokens per Google Account per OAuth 2.0 client ID." With 2–5 devices this is not binding, but each re-authorisation consumes one and the oldest is invalidated when the limit is hit.
- "A Google Cloud Platform project with an OAuth consent screen configured for an external user type and a publishing status of 'Testing' is issued a refresh token expiring in **7 days**, unless the only OAuth scopes requested are a subset of name, email address, and user profile." **A project left in Testing forces re-login weekly.** Since the scope we need is non-sensitive and therefore needs no verification, moving to production status should be available — but this is the single most likely thing to be got wrong.
- Also: "The user has revoked your app's access", "The user account has exceeded a maximum number of granted (live) refresh tokens".

Microsoft: "Refresh tokens for web apps and native apps don't have specified lifetimes. Typically, the lifetimes of refresh tokens are relatively long", with rolling replacement — "Refresh tokens aren't revoked when used to acquire new access tokens. You're expected to discard the old refresh token." Access tokens come back with `"expires_in": 3599`, i.e. **one hour**. The 24-hour cap applies only to `spa`-typed redirect URIs, which a native app does not use. ([auth code flow](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-auth-code-flow))

Dropbox: refresh tokens are described as "long-lived" with no stated expiry, and access-token lifetime is deliberately not published — "The exact expiry time of a token is returned by the token endpoint" ([OAuth guide](https://developers.dropbox.com/oauth-guide)). *Confidence: low* on any specific number here; the honest statement is "read `expires_in` and do not hard-code."

### 8. Change detection

All three offer a cursor, so "am I behind?" is one call rather than a listing. This is materially better than the object-store handshake in §2.

| Provider | Mechanism | Cost of "nothing changed" | Push? | Source |
|---|---|---|---|---|
| Google Drive | `changes.getStartPageToken` once, then `changes.list` with `pageToken`; the last page returns `newStartPageToken` to store. "If the `nextPageToken` is listed, it can be used to gather the next page of changes. If it's not listed, the client application should store the `newStartPageToken`." A `spaces` parameter selects the storage space, so it can be scoped to the application data folder. | one `changes.list` (100 quota units) | no client-side push without a webhook endpoint (which is a server) | [manage changes](https://developers.google.com/workspace/drive/api/guides/manage-changes) |
| Dropbox | `list_folder` → cursor → `list_folder/continue`; or `list_folder/get_latest_cursor`, "A way to quickly get a cursor for the folder's state. Unlike `list_folder`, `list_folder/get_latest_cursor` doesn't return any entries." Cursors are "long-lived, but may expire if unused for an extend time", and expiry surfaces as a `reset` error meaning "Call `list_folder` to obtain a new cursor." | one `list_folder/continue`, or **zero while long-polling** | **Yes.** `list_folder/longpoll` blocks up to 480 s, "plus up to 90 seconds of random jitter added to avoid the thundering herd problem", on `host = "notify"` with **`auth = "noauth"`** and `allow_app_folder_app = true`. Its result carries an optional `backoff` telling the client how long to wait. | [`files.stone`](https://github.com/dropbox/dropbox-api-spec/blob/master/files.stone) [measured — read directly, §10], [detecting changes](https://developers.dropbox.com/detecting-changes-guide) |
| OneDrive | `delta` returning `@odata.nextLink` then `@odata.deltaLink`; `?token=latest` returns "empty response with latest delta token" for establishing a baseline without enumerating. Deletions arrive with a `deleted` facet. | one `delta` call with the stored `deltaLink` | no | [driveItem: delta](https://learn.microsoft.com/en-us/onedrive/developer/rest-api/api/driveitem_delta) |

Three details that will bite an implementation:

- **Cursors expire and the recovery is a full resync.** OneDrive: "There may be cases when the service can't provide a list of changes for a given token…In these cases the service returns an `HTTP 410 Gone` error…and a `Location` header containing a new nextLink that starts a fresh delta enumeration from scratch", with `resyncChangesApplyDifferences` / `resyncChangesUploadDifferences` telling the client which way to reconcile. Dropbox has the same failure as a `reset` error. **A device offline long enough loses its cursor**, which is the same drawer case as the refresh-token expiry — and for us it is cheap to recover from, because the log is a set and re-listing is idempotent [inference].
- **The delta feed is a state feed, not a change feed.** "The delta feed shows the latest state for each item, not each change. If an item were renamed twice, it would only show up once, with its latest name", "The same item may appear more than once in a delta feed…You should use the last occurrence you see", and "**When using delta you should always track items by id**" (same source). Since our objects are immutable once written this is mostly moot, but it forbids treating file names as stable identity.
- **The long-poll endpoint is unauthenticated.** That is a genuine architectural gift for a serverless client: a device can hold a cheap blocking connection open with no credential in flight, and it works for app-folder apps. It is also the only push-shaped mechanism in this entire note; everything else is polling.

### 9. Rate limits, quotas, and unattended operation on Android

#### 9.1 Published limits

- **Google Drive**: "Per minute per project: 1,000,000 quota units", "Per minute per user per project: 325,000 quota units", "Per day per project: 400,000,000 quota units", with read operations costing 5 units, list operations 100, downloads 200 ([usage limits](https://developers.google.com/workspace/drive/api/guides/limits)). At 325,000 units/minute per user and 100 units per `changes.list`, a single user could poll **3,250 times a minute** before throttling — irrelevantly far above anything we would do [inference]. Also: "Google Workspace users can only upload 750 GB per day", "The maximum file size that users can upload is 5 TB".
- **Dropbox**: **no published numbers.** "While Dropbox does not publish exact rate limits, these limits are *not* designed to inhibit normal applications." The contract is behavioural: "the API call will return an HTTP 429 error, returning the reason of too_many_requests", "Rate limited responses always include a Retry-After header", limits "apply per user who has linked your app", and — the trap — "Rate limited requests themselves *also* count towards rate limits, and thus rapid retry loops without pause or respecting this header will be counter-productive." ([performance guide](https://developers.dropbox.com/dbx-performance-guide))
- **Microsoft Graph**: the general throttling page does not carry Files/OneDrive numbers; it redirects to the SharePoint guidance ([throttling limits](https://learn.microsoft.com/en-us/graph/throttling-limits)). *Confidence: low* — I did not pin a number, and anyone relying on a specific OneDrive rate should chase it in the SharePoint documentation.

**Assessment:** rate limits do not discriminate between these three for a single user syncing a few times an hour. The interesting number is the *absence* of one at Dropbox, which means back-off must be implemented against `Retry-After` rather than against a budget.

#### 9.2 Unattended on Android — the platform, not the provider, is the constraint

This applies identically to Part A and Part B; it is a property of the phone.

**Doze** ([Doze and App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)) applies, verbatim:

> - "Suspends network access."
> - "Ignores wake locks."
> - "Defers standard `AlarmManager` alarms, including `setExact()` and `setWindow()`, to the next maintenance window."
> - "Doesn't perform Wi-Fi scans."
> - "Doesn't let sync adapters run."
> - "Doesn't let `JobScheduler` run."

Relief comes in bursts: "Periodically, the system exits Doze for a brief time to let apps complete their deferred activities. During this *maintenance window*, the system runs all pending syncs, jobs, and alarms, and lets apps access the network." And they get rarer: "Over time, the system schedules maintenance windows less frequently."

**App Standby buckets are the harder limit**, and the table is worth reproducing because two rows say *Disabled* under Network ([power management restrictions](https://developer.android.com/topic/performance/power/power-details)):

| Bucket | Regular jobs | Network |
|---|---|---|
| Active | up to 20 min in a rolling 60 min | No restrictions |
| Working set | up to 10 min in a rolling 4 h | No restrictions |
| Frequent | up to 10 min in a rolling 12 h | No restrictions |
| Rare | up to 10 min in a rolling 24 h | **Disabled** |
| Restricted | once per day for up to 10 min | **Disabled** |

Also: "App Standby defers background network activity for apps with no recent user activity" and "If the device is idle for long periods of time, the system allows idle apps network access about once a day" ([Doze and App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)).

**So a phone that has not been opened in weeks — the exact case the sync design exists to handle — is in Rare or Restricted, where background network is off.** The realistic guarantee is: *sync happens when the user opens the app, and opportunistically before that.* Any design that promises "your phone quietly catches up in the background" is overpromising [inference].

**Does a network sync need a foreground service?** Android's own answer is no, and it discourages one:

> "In most cases, your best option for running background tasks is to use WorkManager."
> "If a task is not critical and can be deferred, you should use WorkManager instead of a foreground service, as foreground services can potentially put a heavy load on the device."
> "If a background work task takes longer than 10 minutes to complete, it's highly likely to be interrupted."
>
> — [background tasks overview](https://developer.android.com/develop/background-work/background-tasks)

The deferrable-work API's floor is coarse: "The minimum repeat interval that can be defined is 15 minutes (same as the JobScheduler API)", with constraints available for `NetworkType`, `BatteryNotLow`, `RequiresCharging`, `DeviceIdle` and `StorageNotLow` ([define work](https://developer.android.com/develop/background-work/background-tasks/persistent/getting-started/define-work)). **Fifteen minutes is finer than we need**, since the whole point is offline-first reconciliation.

If a foreground service is used anyway, the cost is now explicit ([Android 15 behaviour changes](https://developer.android.com/about/versions/15/behavior-changes-15)):

> "The system permits an app's `dataSync` services to run for a total of 6 hours in a 24-hour period, after which the system calls the running service's `Service.onTimeout(int, int)` method"
> "Fatal Exception: android.app.RemoteServiceException: 'A foreground service of type dataSync did not stop within its timeout: [component name]'"
> "`BOOT_COMPLETED` receivers are *not* allowed to launch the following types of foreground services: `dataSync`, `camera`, `mediaPlayback`, `phoneCall`, `mediaProjection`, `microphone`"
> — and if one tries, "the system throws `ForegroundServiceStartNotAllowedException`".

The `dataSync` type itself is exactly our use case ("Data upload or download… Transfer data between a device and the cloud over a network") and needs `FOREGROUND_SERVICE_DATA_SYNC` in the manifest with `android:foregroundServiceType="dataSync"`, with **no runtime prerequisites** ([foreground service types](https://developer.android.com/develop/background-work/services/fgs/service-types)). But Android 15 pushes away from it, and **it cannot be started at boot**, so it cannot be the mechanism that catches a device up after a restart [inference].

#### 9.3 Provider SDKs assume Java/Kotlin — what that means for a pure-Rust client

Every one of these providers ships an Android SDK written for the JVM. That has three concrete consequences for an egui/eframe binary:

1. **We do not use their SDKs.** All three APIs are plain HTTPS+JSON (or XML for object storage), reachable from `reqwest`. §5 shows one crate that already implements the S3, Google Drive, Dropbox and OneDrive protocols in Rust behind a single interface and checks clean for `aarch64-linux-android`. So the SDKs' language is a non-issue for the *data path*.
2. **The auth path is the part that touches the platform.** RFC 8252 requires an external user-agent, which on Android means launching a browser — a Custom Tab, or at minimum an `ACTION_VIEW` intent — and receiving the redirect back. Both are Java API calls, so the pure-Rust client needs a JNI hop, which the repo already does for filesystem paths (`AGENTS.md`, storage table). The custom-scheme redirect must also be declared in the manifest as an intent filter. **This is the single largest piece of Android-specific plumbing the drive family imposes, and the object-store family does not impose it at all** — a key and secret need no browser [inference].
3. A silver lining worth recording against `AGENTS.md` rule 8 (Android text input is ASCII-only, and cannot be fixed): **an OAuth flow types the password into the system browser, not into our app**, so it sidesteps the IME limitation entirely. Pasting a storage secret does too, but *typing* one does not. Neither family is blocked, but OAuth is the one that never asks our text field to handle anything [inference].

---

## 10. Method: commands run and outputs

Everything marked [measured] came from one of these. Run 2026-07-30 on Linux, `rustc 1.97.0`, `cargo 1.97.0`.

**Listing wire cost, against a live anonymously-listable bucket:**

```
$ curl -s -o /dev/null -w "%{size_download}\n" \
    "https://noaa-goes16.s3.amazonaws.com/?list-type=2&prefix=zzzznonexistent&max-keys=5"
254
$ curl -s -o /dev/null -w "%{size_download}\n" "https://noaa-goes16.s3.amazonaws.com/?list-type=2&max-keys=1"
865
$ curl -s -o /dev/null -w "%{size_download}\n" "https://noaa-goes16.s3.amazonaws.com/?list-type=2&max-keys=2"
1260
$ curl -s -o /dev/null -w "%{size_download}\n" "https://noaa-goes16.s3.amazonaws.com/?list-type=2&max-keys=5"
2445
```

The empty listing in full, which is the whole envelope:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Name>noaa-goes16</Name><Prefix>zzzznonexistent</Prefix><KeyCount>0</KeyCount><MaxKeys>5</MaxKeys><IsTruncated>false</IsTruncated></ListBucketResult>
```

A populated entry, showing the metadata available without a `GET`:

```xml
<Contents><Key>ABI-L1b-RadC-Reproc/2017/351/00/RP_ABI-L1b-RadC-M3C01_G16_s20173510002228_e20173510004599_c20250632343546.nc</Key><LastModified>2025-05-20T18:07:12.000Z</LastModified><ETag>&quot;7fb98fdd354afbd9f6ac81fe9a511279&quot;</ETag><ChecksumAlgorithm>CRC64NVME</ChecksumAlgorithm><ChecksumType>FULL_OBJECT</ChecksumType><Size>2159575</Size><StorageClass>STANDARD</StorageClass></Contents>
```

**Conditional reads, live:**

```
$ curl -s -o /dev/null -w "status=%{http_code} bytes=%{size_download}\n" \
    -H 'If-None-Match: "7fb98fdd354afbd9f6ac81fe9a511279"' "https://noaa-goes16.s3.amazonaws.com/<key>"
status=304 bytes=0
$ curl -s -o /dev/null -w "status=%{http_code} bytes=%{size_download}\n" \
    -H 'If-Match: "deadbeefdeadbeefdeadbeefdeadbeef"' -r 0-0 "https://noaa-goes16.s3.amazonaws.com/<key>"
status=412 bytes=348
```

**Conditional *writes* were not tested** — that needs an authenticated bucket on each provider. Everything in §1.2 is from documentation, and the B2 row is the one to verify before relying on it.

**Prices, from AWS's own unauthenticated price list rather than the marketing page:**

```
$ curl -s --compressed -o /tmp/s3us.json \
    "https://pricing.us-east-1.amazonaws.com/offers/v1.0/aws/AmazonS3/current/us-east-1/index.json"
$ python3 -c "..."   # filter products by usagetype, join to terms.OnDemand.priceDimensions
Requests-Tier2 | GET and all other requests | $0.004 per 10,000 | 0.0000004 per Requests
Requests-Tier1 | PUT/COPY/POST or LIST      | $0.005 per 1,000  | 0.0000050 per Requests
TimedStorage-ByteHrs | General Purpose | $0.023 per GB - first 50 TB / month | 0.023 per GB-Mo
```

```
$ curl -s --compressed -o /tmp/dt.json \
    "https://pricing.us-east-1.amazonaws.com/offers/v1.0/aws/AWSDataTransfer/current/index.json"
$0.090 per GB - first 10 TB / month data transfer out beyond the global free tier
$0.085 per GB - next 40 TB   |  $0.070 per GB - next 100 TB  |  $0.050 per GB - over 150 TB
```

**Payload sizes** — rows in ADR-0004 §11's exact interchange form (16-hex writer id, RFC 9562 canonical note UUID, nine keys) as JSON Lines, `zstd -19` and `gzip -9`:

```
 730000 rows | raw  111295393 B (152.5 B/row) | zstd-19 9462142 B | gzip-9 27907906 B | zstd ratio 11.76
  73000 rows | raw   11056472 B (151.5 B/row) | zstd-19 1036927 B | gzip-9  2791194 B | zstd ratio 10.66
    200 rows | raw      29816 B (149.1 B/row) | zstd-19    6330 B | gzip-9     7875 B | zstd ratio  4.71
     33 rows | raw       4899 B (148.5 B/row) | zstd-19    1190 B | gzip-9     1352 B | zstd ratio  4.12
      8 rows | raw       1181 B (147.6 B/row) | zstd-19     389 B | gzip-9      402 B | zstd ratio  3.04
```

The decade row reproduces ADR-0004 §10's "around 110 MB raw" independently, which is a useful cross-check that the synthetic rows are the right shape.

**Rust builds for Android**, each in its own scratch crate to avoid feature unification:

```
$ CC_aarch64_linux_android=$NDK/.../aarch64-linux-android24-clang \
  AR_aarch64_linux_android=$NDK/.../llvm-ar \
  cargo check --target aarch64-linux-android
object_store 0.14.1 (feature "aws")                              Finished in 10.75s
aws-sdk-s3 1.140.0 + aws-config 1.10.1                           Finished in 17.22s
rust-s3 0.36.0 (default-features=false, "tokio-rustls-tls")      Finished in  6.52s
opendal 0.57.0 (services-s3, -gdrive, -dropbox, -onedrive)       Finished in 16.58s
```

Without `CC_aarch64_linux_android` the first of these fails in `aws-lc-sys v0.43.0` with `failed to find tool "aarch64-linux-android-clang"`. `cargo tree -i aws-lc-sys` shows it arriving both directly from `object_store` and transitively through `rustls 0.23`.

**Crate metadata** from the crates.io API (licence, newest version, last update) is quoted in §5.

**Dropbox long-poll parameters** were read from the provider's own interface-definition file rather than the rendered docs, which are client-rendered and not fetchable:

```
$ curl -s https://raw.githubusercontent.com/dropbox/dropbox-api-spec/master/files.stone
struct ListFolderLongpollArg
    timeout UInt64(min_value=30, max_value=480) = 30
route list_folder/longpoll (...)
    attrs
        host = "notify"
        auth = "noauth"
        allow_app_folder_app = true
        scope = "files.metadata.read"
```

---

## 11. Confidence and what to re-check

| Claim | Confidence | Why |
|---|---|---|
| Conditional writes unnecessary for a per-writer keyspace | **High** | Follows from quoted strong read-after-write and list-after-write guarantees plus the single-author property; the reasoning chain is in §1.3 and is attackable step by step |
| Listing byte costs and the 1,000-key page limit | **High** | Measured live and stated in the API reference |
| Published prices and the cost table | **High** for the prices (read from AWS's own price-list JSON and the providers' pricing pages), **medium** for my arithmetic — marked [inference], and prices move |
| B2 does not support conditional writes | **Low-medium** | Absence in their documentation, contradicted by third-party comparisons. **Test against a real bucket before relying on it either way.** |
| R2 tokens cannot be prefix-scoped or given a TTL | **Medium** | Absence in the token documentation. Absence of evidence, not evidence of absence |
| Google's application-data-folder scope is non-sensitive and needs no verification | **Medium-high** | Stated directly in two Google sources, but verification policy has changed before |
| Google refresh tokens die after six months unused, 7 days in Testing status | **High** | Stated verbatim by Google |
| Dropbox freezes new links after 50 users pending review | **High** | Stated verbatim |
| Microsoft has no equivalent consumer review gate | **Low** | I did not exhaust publisher-verification documentation |
| Microsoft Graph OneDrive rate limits | **Not established** | The Graph page redirects elsewhere; no number pinned |
| Android network is disabled in Rare and Restricted standby buckets | **High** | Stated in Android's own power-management table |
| All four Rust crates build for `aarch64-linux-android` | **High** | Measured today, `cargo check` only — not `cargo build`, so linking is unverified |

**Not covered here, by design:** synced folders, WebDAV and a git remote (sibling notes under this ticket); conflict UX and the plane case (out of scope per the ticket); the web target (ruled out in [#12](https://github.com/amin-bf/leitner/issues/12)).

**The two things a decision session should test before committing**, because they are cheap to test and expensive to be wrong about: (1) whether conditional writes work on the chosen store, with a real key and two racing clients; (2) how long a real Android handset, left alone for a fortnight, actually goes between successful background syncs.
