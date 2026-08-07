# ADR-0024: Identifying a written file

- **Status**: Accepted
- **Date**: 2026-08-01
- **Resolves**: [Decide: how a written file is identified again, when Android keeps neither our media type nor our extension](https://github.com/amin-bf/cairn/issues/72)
- **Map**: [Map: local-first Leitner app spec](https://github.com/amin-bf/cairn/issues/1)
- **Related**: [ADR-0003 §2](0003-client-stack.md) (manifest plus one `.so`),
  [ADR-0008 §2, §10](0008-the-deck-export-format.md) (the artifact that leaves the machine; the
  extension and the `mimetype` member), [ADR-0016 §5, §9](0016-backup-and-restore.md) (the platform
  seam and the launch intent; `.lcoll`), [ADR-0022 §4, §10, §11](0022-the-import-preview-and-export-report.md)
  (the refusal surface; what the application says after an export; the list reads manifests),
  [ADR-0023](0023-sending-a-written-file.md) (sending a written file)
- **Evidence**: [`docs/research/android-file-identity/`](../research/android-file-identity/README.md)

## Context

[ADR-0008 §10](0008-the-deck-export-format.md) rests a distinct extension per profile on its being
*"how the operating system and the user tell a deck file from a whole-collection artifact **before**
opening it"*, and states that *"on Android the intent filter matches a `pathPattern` for the
extension alongside the media type."* [ADR-0016 §5](0016-backup-and-restore.md) admits the Android
**launch intent** as an entry point on the strength of such a filter, and specifies the in-app
**list** as *"query `MediaStore` for our extensions."*

[ADR-0023](0023-sending-a-written-file.md) found, while measuring the outbound send, that
`MediaStore` derives the media type from the extension and discards ours: `.ldeck` is not in
Android's MIME map, so the row settles at `application/octet-stream` whatever we declare. It could
not say what that costs the intent filter, because the filter did not exist — *"nobody has watched a
file manager fail to find us."*

**This ADR watched.** Everything below rests on measurement on the handset — a Pixel 8 Pro, Android
17 / API 37 — in the shipped APK shape, and the answer is worse and better than the reasoning
expected. Worse: the extension cannot reach an intent filter at all, and the in-app list turns out to
be blind to every file the application did not write, which removes the fallback nobody had noticed
was load-bearing. Better: the mangled filename that prompted half the question is **self-inflicted**,
and stops happening when we stop asking for it.

## Decision

### 1. Identity lives in the bytes, not in the name

> **The `mimetype` member at a fixed offset is the sole authority for what a file is. The extension
> is a display string and an enumeration hint, and is never consulted to decide a file's type or
> profile.**

[ADR-0008 §10](0008-the-deck-export-format.md) already stores `mimetype` first and uncompressed
*"because a zip archive's header says nothing about what it contains"*, so the mechanism is not new;
what changes is how much weight it carries. On Android it now carries all of it, for three measured
reasons:

1. **The stored media type is `application/octet-stream` for both profiles.** A `.ldeck` and a
   `.lcoll` are indistinguishable by type on this platform.
2. **The name may not survive the route.** A file arriving through `ACTION_SEND` need not have a
   usable display name at all, and one arriving through a provider URI has a row id where the name
   would be.
3. **The check is free where it matters.** [ADR-0022 §11](0022-the-import-preview-and-export-report.md)
   already describes each listed file *from its own manifest*, so the bytes are open anyway.

**The extension keeps exactly one job**: it is the `LIKE` clause the list queries `MediaStore` with,
because an application cannot sniff a file it cannot enumerate. Enumerating by name and *deciding* by
content is not a contradiction — it is the only split the platform permits.

> **Correction: the extension has a second job it was never given here — it gates *reachability*,
> upstream of the sniff, so "exactly one job" understates what it decides.** Measured on the handset
> (Pixel 8 Pro, API 37) while working [#99](https://github.com/amin-bf/cairn/issues/99): the **same
> deck bytes** written under three names, then typed by `MediaStore`:
>
> | Name | Stored type | Handlers our filters resolve for |
> |---|---|---|
> | `Inbound.ldeck` | `application/octet-stream` | ours among them |
> | `Inbound` (no extension) | `application/octet-stream` | ours among them |
> | `Inbound.txt` | **`text/plain`** | **zero — we are not offered** |
>
> The extension decides the type `MediaStore` stores, which decides whether the broad filter of §2
> fires at all — and that gate sits **upstream of the sniff**. A byte-identical deck named `.txt`
> types as `text/plain`, never reaches the code that would identify it correctly, and **no sniff can
> recover it**: the file is never offered to us in the first place. So a deck under an extension the
> platform recognises is unreachable by any means this application has.
>
> **What does not change.** The sniff remains the sole authority over a file's **profile** — the
> `mimetype` member decides what a file *is*, and this correction is only about what makes a file
> *arrive*. And the reassuring half of the same measurement holds: a **stripped** name still types as
> `application/octet-stream` and still arrives, which is the case reason 2 above (*"the name may not
> survive the route"*) actually cares about. What §1 originally implied and this narrows is the
> extension's authority over **arrival**, never its (absent) authority over profile.

### 2. Reach costs a broad filter, and we pay it

> **The manifest declares `application/octet-stream` for `ACTION_VIEW` and `ACTION_SEND`, alongside
> the precise `application/vnd.leitner.deck+zip` filter.**

The extension-matched filter this specification assumed **cannot work**, and the reason is not the
one that was reasoned. `pathPattern` works perfectly well against `content://` URIs — it fires
whenever the path carries the name. It fails here because **the providers that matter pass a row
id**: `MediaStore`'s own URIs are `content://media/external/downloads/1000057665`, and a real file
manager hands over `content://com.google.android.apps.nbu.files.provider/2/1000057665`. There is no
filename in either, so there is nothing to match.

Watched end to end: with the extension filter installed and verified on the device, one tap on a real
`.ldeck` draws *Open with — Google · Google Wallet · KeePassDX · Sparkasse*, and **Leitner is not in
the list.**

**Without a broad filter there is no inbound path on Android at all.** §3 below establishes that the
list can only ever show files we wrote; the picker stays refused by
[ADR-0016 §5](0016-backup-and-restore.md) because its disqualifying property — delivery through an
activity **result** — is untouched by [ADR-0023](0023-sending-a-written-file.md), which only ever
established that a *send* has no result. So the choice was a broad filter or no inbound route for the
one artifact [ADR-0008 §2](0008-the-deck-export-format.md) calls *"the one artifact that leaves the
machine and arrives with someone who does not have our application"* — on the half of the product
where people actually receive files.

**The cost, stated plainly rather than discovered later: the application appears in the Open-with
list for unrecognised files that are none of its business.** That is not a defect to be fixed but the
price of the entry point, and §1 is what makes it survivable — we read forty bytes and refuse
honestly through [ADR-0022 §4](0022-the-import-preview-and-export-report.md)'s existing surface. This
is not novel practice: a password manager shipping a custom extension declares the same broad filter,
for the same reason, and is in that same sheet.

**`ACTION_SEND` is accepted as well as `ACTION_VIEW`**, so a deck shared straight from a messaging
application arrives without a round trip through `Downloads`.

**The precise filter is kept because it costs nothing and sometimes wins.**
[ADR-0023 §3](0023-sending-a-written-file.md) has our own outbound chooser declare
`application/vnd.leitner.deck+zip`, so a phone-to-phone share matches the precise filter and never
needs the broad one. **The extension-matched filter is not kept**: the broad filter is a strict
superset of it, and it never fires on the routes that matter.

### 3. The list is what we wrote, and it must not pretend otherwise

> **`list` returns files this application wrote. It is not a view of the user's `Downloads` folder,
> and nothing in the interface may imply that it is.**

[ADR-0016 §5](0016-backup-and-restore.md)'s *"no permission at API 29+"* was measured for the **put**,
and does not extend to reading. Measured: a `.ldeck` placed in `Downloads` by another package, in the
same folder, is **invisible** — `Downloads rows visible to us = 0`. With a control file written by
the application first, the same query returns seven rows, every one owned by `dev.leitner.app`. The
query is correct; the folder is not visible. Scoped storage grants an application its own
`MediaStore` rows and nothing else, `READ_MEDIA_*` covers images, video and audio rather than
documents, and the general route to another application's file is the picker that ADR-0016 §5
refused.

This is why §2 is not optional. **A deck someone sends you can never appear in the list**, cannot be
put there, and cannot be sniffed either — sniffing needs bytes we are not permitted to open. The
filter is the only door.

### 4. The Android write declares no media type

> **The `MediaStore` insert sets `_display_name`, and **no** `mime_type`.**

[ADR-0023](0023-sending-a-written-file.md) recorded the collision dedupe as *"it dedupes, and the
suffix lands after the extension"* — `French A1.ldeck (1)`. **That behaviour is conditional on our
own declaration**, which is the half of the finding the outbound measurement could not see. Measured,
same name inserted twice with bytes written each time:

| Declared media type | Second insert stored as |
|---|---|
| *(none)* | `probe72-x (3).ldeck` — extension **kept** |
| `application/octet-stream` — agrees with the name | `probe72-y (3).ldeck` — extension **kept** |
| `application/vnd.leitner.deck+zip` — disagrees | `probe72-z.ldeck (3)` — extension **destroyed** |

Declaring a type that disagrees with the extension is what pushes the suffix past it. So the write
does not need to refuse the platform's name, re-insert under a name of our own, or read the name back
and correct it — **it needs to stop making a claim the platform was never going to keep.**

**Declaring nothing rather than declaring `application/octet-stream`** — the stored result is
identical, so this is a question of which instruction is honest. `application/octet-stream` states a
claim we do not mean, and it would override a future platform that learned to map `.ldeck` to our
real type; declaring nothing lets that improvement through for free.

> **Correction: this section originally also had the insert set `relative_path`, and it never should
> have.** The clause was incidental — nothing above or below argues for it, every measurement here is
> about `mime_type`, and the implementation has never set it. The **collection already decides the
> folder**: inserting into `MediaStore.Downloads.EXTERNAL_CONTENT_URI` lands in `Download/`, verified
> on the handset at API 29 and API 37 ([#98](https://github.com/amin-bf/cairn/issues/98)). So the
> clause bought nothing it was not already getting, and it asked for **one thing more than it looked
> like**: a `relative_path` is how a subfolder gets chosen, and no ADR has ever chosen one. Declaring
> `Download/` explicitly would have been this section's own mistake in miniature — stating a claim we
> do not mean, on a value the platform was going to supply correctly anyway. Anyone wanting exports
> under a subfolder is making a new decision, not implementing this one.

**[ADR-0022 §10](0022-the-import-preview-and-export-report.md)'s "state the name the platform
actually wrote" survives untouched and stays load-bearing.** The dedupe still happens — it simply no
longer eats the extension — and reading the name back remains the only way the application can tell
the user where the file went.

### 5. Facts the implementation must not rediscover

Each is a **silent** failure: it produces no error, or an error at the wrong moment, and two of them
cost this ADR a build cycle each.

1. **A `pathPattern` is ignored unless the filter also declares a host.** Android tests the path list
   only inside the authority test, so with no `android:host` the filter degrades to *"any URI of this
   scheme and type"* — the over-broad filter it was written to avoid. The failure runs both ways: a
   filter meant to be narrow is silently wide, and `dumpsys` reports the pattern faithfully either
   way, so inspection does not reveal it. Adding `host = "*"` yields `Authority: "": -1 WILD` and the
   path tests begin to run.
2. **`cargo-apk` drops the escape.** `path_pattern = ".*\\.ldeck"` reaches the device as
   `GLOB: .*ldeck`. A TOML **literal** string (`'.*\\.ldeck'`) survives. **Verify the emitted
   `AndroidManifest.xml`, never the source**, for anything pattern-shaped.
3. **Android's `PatternMatcher` does not backtrack.** `.*` consumes up to the **first** occurrence of
   the next literal, so `.*ldeck` against `/files/deck.ldeck` stops at the `l` in `files` and never
   recovers. This is why applications in the wild enumerate a pattern once per possible dot count
   rather than writing one.
4. **The dedupe fires on file *creation*, not on the row.** An insert whose bytes are never written
   collides with nothing. A collision test that skips `openOutputStream` silently measures nothing
   and reports success.
5. **A `MediaStore` cursor shows only your own rows**, and an empty cursor is indistinguishable from
   a broken query without a control file written first.
6. **`cargo apk build` needs a JDK on `PATH`.** `apksigner` is a `java` wrapper, and its absence
   surfaces only at the signing step — after a full NDK compile — as
   `apksigner: line 97: exec: java: not found`.
7. **The read grant reaches the *data* URI, never a bare `EXTRA_STREAM` extra — so an `ACTION_SEND`
   fired from a shell measures the harness, not the application.**
   `FLAG_GRANT_READ_URI_PERMISSION` covers the intent's `data` URI and its `ClipData`; a URI sitting
   only in a Parcelable extra is covered by neither. Real senders never meet this, because
   `Activity.startActivity` calls `Intent.migrateExtraStreamToClipData()` and the extra is copied into
   the clip on the way out — **`am start --eu` does not**. The failure is silent twice over: the
   system logs nothing, and every JNI arm in the launch read degrades to `None` by design, so it
   presents as the file simply never arriving. **Distinguish the two by sending a file this
   application owns**, which needs no grant at all — if that arrives and a shell-owned one does not,
   the reader is correct and the harness is the thing that cannot deliver a grant. Measured both ways
   while working [#99](https://github.com/amin-bf/cairn/issues/99).

## Amendments to accepted ADRs

### [ADR-0008 §10](0008-the-deck-export-format.md) — amended in place, and the extension is demoted

§10 is **not superseded**; it is corrected claim by claim, and this is an **Android-only** amendment.

- **"The extension is `.ldeck`; `mimetype` is stored first and uncompressed."** Stands, and is
  **promoted** — §1 above makes it the sole authority rather than a fallback for a mangled name.
- **"A distinct extension per profile … is how the operating system and the user tell a deck file
  from a whole-collection artifact before opening it."** **Half false, and the half is the platform.**
  True of the **user**, who reads the name. False of the **operating system** on Android, which types
  both profiles `application/octet-stream`. It remains true of the desktop.
- **"On Android the intent filter matches a `pathPattern` for the extension alongside the media
  type."** **False, and it is the sentence that misdirected this whole area.** Replaced by §2: the
  providers that matter carry a row id, so no filename reaches a filter, and the pattern would be
  ignored anyway without a host.
- **"Getting this wrong means the file will not open from a file manager or a mail attachment — which
  is the entire distribution channel for a deck."** **The stakes were right and the mechanism was
  wrong.** It does fail; the fix is a broad type filter plus a content sniff, not a better pattern.

**The desktop half of §10 is untouched.** It writes a real path, keeps the extension and the media
type, and `xdg-mime` can map one to the other. The asymmetry this creates is the one
[ADR-0016 §5](0016-backup-and-restore.md) and [ADR-0023 §4](0023-sending-a-written-file.md) already
established, not a new one.

### [ADR-0016 §5](0016-backup-and-restore.md) — the list's scope is narrowed to what we wrote

That section's Android row reads *"query `MediaStore` for our extensions."* True, and **it can only
ever return files this application wrote** — §3 above. The row is amended to say so. Its **put** is
unaffected, and its refusal of the picker is unaffected and now load-bearing in a second way: the
picker's absence is precisely why the broad filter of §2 is the only inbound door.

Its **launch intent** entry point survives, on a different mechanism: matched by media type and
content, never by extension.

### [ADR-0022 §11](0022-the-import-preview-and-export-report.md) — the permissions sentence is reconciled

§11 lists an unreadable file rather than hiding it, because hiding it means *"a user who deliberately
put a file there sees an empty list and concludes the application cannot see the folder — sending
them after a permissions problem that does not exist."*

**There is now a permissions problem that does exist**, and it is not one the user can fix. The
reasoning is preserved where it applies — a file we wrote and cannot parse is still listed, still
marked `unreadable` — but the sentence must no longer imply the list is a view of the folder. A file
another application put in `Downloads` is not listed, cannot be, and the interface must not invite
the user to expect it.

### [ADR-0023](0023-sending-a-written-file.md) — its ADR-0022 §10 amendment is made conditional

That ADR's amendment states *"its Open items row … is answered: it dedupes"*, with the deduped name
`French A1.ldeck (1)`, suffix after the extension. **Correct as measured and incomplete as stated**:
the suffix lands after the extension *only when the insert declares a media type disagreeing with the
name*, which is what §4 above stops doing. Under this ADR the deduped name is
`French A1 (1).ldeck` — the extension survives.

Nothing else in ADR-0023 changes. Its §7 facts stand, its `hand_off` is specified against the URI the
write returned, and §4 above changes what that write declares, not what is handed off.

## Requirements this places on downstream tickets

None. The map's remaining open ticket is a prototype of the authoring screen, which this does not
touch.

## Glossary

- **Sniff** — reading the `mimetype` member at its fixed offset to decide what a file is, without
  parsing the archive.
- **Broad filter** — an intent filter matching `application/octet-stream`, which is every file the
  platform cannot type.
- **Precise filter** — an intent filter matching `application/vnd.leitner.deck+zip`, which only a
  sender that declares our type can match.

## Consequences

- **The application appears in the Open-with sheet for unrecognised files.** Deliberate, priced, and
  the direct cost of having any inbound route at all on Android.
- **`.ldeck` versus `.lcoll` becomes a content question, not a filename question.** The two profiles
  are indistinguishable by name or type on Android, so the code that tells them apart must be the
  same code on both platforms — which removes a divergence rather than adding one.
- **The in-app list is smaller than a user might expect**, and the interface carries that fact rather
  than explaining it away.
- **One fewer thing to build.** The re-insertion under a name of our own that the ticket contemplated
  is not needed; nor is any repair of a mangled extension. The bug was our own declaration.
- **`AGENTS.md` client-stack rule 8 is untouched.** Nothing here types text.
- **The desktop is untouched entirely.** No filter, no sniff-only rule, no change to what is written.

## Open items handed onward

| Item | Owner |
|---|---|
| ~~**Whether an inbound `ACTION_SEND` delivers a readable URI.**~~ The filter is decided here; that the grant arrives is not measured, because completing a share needed a real send from a real account | **Discharged** — see below |
| **The API 24–28 path.** Scoped storage and `MediaStore.Downloads` as measured are API 29+; `min_sdk_version` is 24. Inherited from [ADR-0016 §5](0016-backup-and-restore.md), not created here | Implementation |
| **Whether a recipient can read the bytes** — carried forward unchanged from [ADR-0023](0023-sending-a-written-file.md) | Implementation, under `AGENTS.md` rule 9 |
| Visual treatment of the refusal when an unrecognised file is opened | **Out of scope** — *the visual design pass*, ruled out by [the map](https://github.com/amin-bf/cairn/issues/1) on 2026-07-31 |

### The inbound share, discharged

> **The grant arrives.** A `.ldeck` shared to this application through `ACTION_SEND` was read,
> sniffed and planned — `Arrived: shared (ACTION_SEND)`, `Sniffed: a deck`
> ([#99](https://github.com/amin-bf/cairn/issues/99), Pixel 8 Pro, API 37).

**What blocked it was the sender, not a real account.** This item assumed a share could only be
completed from a messaging application signed into the owner's own accounts. That was too narrow: the
grant is a property of *any* sender that goes through `Activity.startActivity`, because that is what
migrates `EXTRA_STREAM` into the intent's `ClipData` (§5.7). A file manager's own share is therefore
a sufficient sender, and needs no account, no contact and no network.

**It nearly recorded the opposite.** The same share fired from `am start --eu` does *not* arrive, and
with every JNI arm degrading to `None` that is indistinguishable from a broken reader. What separated
them was sending a file this application **owns** — readable with no grant at all — which arrived
through the identical code path. The reader was never in question; the harness simply could not hand
over a grant. That asymmetry is now §5.7 so the next person does not spend the diagnosis again.

**`ACTION_VIEW` and the Open-with sheet are discharged with it.** §2 accepted appearing in the sheet
for unrecognised files as the price of having any inbound route, having watched the
extension-matched filter *fail* to put us there. Under the broad filter we are in that sheet — first,
above the same four applications §2 recorded us missing from.
