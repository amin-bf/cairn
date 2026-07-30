# Sync transport over storage we do not own

**Research ticket:** [#33](https://github.com/amin-bf/leitner/issues/33) (under wayfinder map
[#1](https://github.com/amin-bf/leitner/issues/1)) · **Date of research:** 2026-07-30

**Question:** with no server of our own, what can carry the review event log between a user's desktop
and Android devices?

This is a **research** note. It gathers facts and sharpens trade-offs; it decides nothing. The
decision is a later ticket's job.

Three slices were investigated in parallel, each against primary sources, each testing load-bearing
claims rather than asserting them. This README is the synthesis — **what the three slices say
together that none of them says alone**. Each slice's own note carries its sources, its measurement
commands, and its confidence levels:

| Slice | What it covers |
| --- | --- |
| [`git-as-transport.md`](./git-as-transport.md) | A git remote as the store: Rust git libraries on Android, hosting terms and limits, pack behaviour measured over a synthetic decade, `ls-remote` as the handshake |
| [`synced-folders-and-webdav.md`](./synced-folders-and-webdav.md) | A folder another application keeps in step, and WebDAV against rented storage: Android's scoped-storage limits, conditional writes and appends tested against three servers, measured handshake and segment costs |
| [`object-stores-and-drives.md`](./object-stores-and-drives.md) | A rented object store, and a personal cloud drive through its own API: conditional-write support and necessity, segment-versus-snapshot pricing, OAuth with no server, change-detection cursors |

**Scope note.** This ticket's body made the web target load-bearing — it argued the deciding finding
would be that a browser cannot reach a git remote without a proxy, and a proxy is a server we do not
have. That question was moot before the reading started: the map records the web target as **out of
scope**, ruled out while resolving [Decide: the local
store](https://github.com/amin-bf/leitner/issues/12). The budget went to Android depth instead, which
is now half the client surface and the harder half.

---

## The seven findings that matter

### 1. Conditional writes are not needed — and declining to need them is worth more than having them

The ticket asked whether anything needs compare-and-swap at all, given each writer owns its own
keyspace. **It does not**, and two slices established this independently, arriving at the same
proviso.

A conditional write solves *lost updates to a shared key*. Under a per-writer keyspace no key ever
has two writers, so the failure it prevents cannot occur. What an unconditional write already
guarantees is sufficient — including that a newly written object appears in a subsequent listing, the
property the whole design leans on. Worse, a conditional write makes one realistic case *worse*: a
client retrying after an ambiguous timeout, re-uploading byte-identical content, is rejected rather
than succeeding harmlessly.

The proviso, stated the same way in both slices: this holds **provided ADR-0004 §7's mutable surface
is also published per writer** rather than as one shared document. If the mutable surface becomes a
single shared object, its writers race and the conclusion reverses. That makes "how the mutable store
moves" — the item ADR-0004 explicitly handed onward — a question about *concurrency control*, not
just about bytes.

And this is not merely a saving. Conditional writes were tested against three WebDAV servers and
**two of them silently ignored the precondition, in the data-losing direction** — returning `201
Created` and overwriting the file for both a stale `If-Match` and an `If-None-Match: *`, despite the
governing specification making evaluation a MUST. A client cannot tell from a `2xx` that the
condition was ever evaluated. A design that does not need conditional writes is not just cheaper; it
is immune to a silent-corruption hazard that a design depending on them would have to detect and work
around per server.

### 2. Segment granularity is the real dial, and three independent measurements bound it from both sides

The ticket framed this as a binary — per-writer segments merged at read time, or whole-snapshot
republish. **It is neither: it is a continuous dial, and the three slices measured three different
penalties that pull in opposite directions.**

Republishing a whole per-writer log on every sync is ruled out by **uplink, not by money**. The
monthly bill is between $0.0000 and $0.0040 either way — ingress is free everywhere and the data sits
inside every free tier — but the bytes a device pushes are **1,800 MiB per month** at six syncs a day
against **0.2 MiB** for segments. That is a mobile-data bill appearing on no pricing page, and it is a
factor of roughly nine thousand.

But the other end of the dial has its own penalties, and they are not obvious:

- **Compression collapses.** The same rows compress **12.01×** as one file per writer-year and only
  **5.02×** as daily 200-row segments — 2.20 MB/year instead of 0.92 MB/year — because the
  compressor's window cannot reach across file boundaries to the repeated writer identifiers.
- **Listing stops being the handshake.** Two devices syncing six times a day for ten years is 43,800
  objects, against a 1,000-key listing page cap: 44 round trips and ~14 MB for a naïve full listing.
- **Local cost explodes on a content-addressed store.** Measured over a synthetic decade, one growing
  file per writer gives **911.8× write amplification** and a 9.14 GB unrepacked repository at the
  five-year mark, because the whole file is re-hashed on every commit — cumulative bytes hashed is
  quadratic in commit count, confirmed at exactly `(days+1)/2` over three run lengths.

**Time-bucketed segments satisfy all three constraints, and one slice measured the optimum
independently.** One file per writer *per month* bounds write amplification at **15.7×** and makes a
decade of history *smaller than the text it stores* — 47.5 MB of repository for 126.3 MB of log, with
flat one-second maintenance, against 29.8 s and climbing for the single-file layout. It also puts
object counts in the hundreds rather than the tens of thousands, and gives the compressor a large
enough window to work in.

The residual cost is that the *current* bucket is still republished on each sync. That is bounded by
one month of rows rather than the whole log — on the order of 10 MB per device per month at six syncs
a day, averaged over a filling bucket [inference: arithmetic over the measured row size and
compression ratios]. Closed buckets are immutable forever, which is what makes them cache, compress
and pack well. **The decision ticket picks a point on this dial; the research establishes that both
ends are penalised and the interior is not.**

### 3. A synced folder is the one family with a structural disqualification, and the deepest part is not about Android

Three separate problems, in increasing order of severity.

**It cannot answer "am I behind?" even in principle.** A folder shows a device its own local copy;
there is no remote to interrogate. A directory listing reports what has *arrived*, never what exists
elsewhere. The peer-to-peer model states its delivery guarantee plainly — data moves between machines
"as soon as they are online at the same time" — so two devices never awake together never converge,
and **neither can tell that this is happening**. Standing constraint 3 and ADR-0004 §2 require a
version-summary handshake; in this family that handshake has no counterparty. This is a missing
capability, not a cost.

**On Android there is exactly one directory both our app and a third-party sync application can treat
as ordinary files, and its API was deprecated at API 30** — with its own reference stating "there is
no security enforced with these files" in the same paragraph as the deprecation notice. App-private
storage is unreachable by any other app, and since Android 11 nothing reaches another app's data
directory, not even all-files access. Everything else costs a user-picked Storage Access Framework
tree — `content://` URIs rather than paths, obtained through an activity result the Rust Android glue
does not surface, so it also costs a hand-written `Activity` subclass in the APK. **This is the same
`NativeActivity` limitation recorded as rule 8 in `AGENTS.md`**, the one that makes Android text input
ASCII-only — the same missing seam, surfacing in a second place.

**The sync applications themselves have retreated from Android.** The peer-to-peer client was
archived in December 2024, its maintainer citing store publishing difficulty; the surviving fork is
sideload-only and declares exactly the permissions that make store distribution hard. And the three
large commercial drive clients do no local-folder sync on Android at all — one states outright that
its app "does not sync files automatically", another that its files "aren't stored on your phone or
tablet", and the third ships a desktop-only sync client.

### 4. Android caps unattended sync identically for every candidate, so it discriminates between none of them

All three slices hit the same wall independently. While dozing, network access is suspended and
deferred work does not run; in the **Rare** and **Restricted** app-standby buckets network is listed
as **Disabled** outright — precisely where a device that has been in a drawer lands. The periodic-work
floor is 15 minutes. Escaping via a foreground service costs a visible notification, is capped at six
hours in any 24, and cannot be started from boot. Asking the user for a power-management exemption is
store-policy-restricted unless the app's core function is adversely affected, which a
few-kilobyte sync cannot claim.

Our transfers take seconds, so the caps never bite. **What binds is the once-a-day network window for
an idle app and the visible notification** — and since this is a platform property rather than a
transport property, it changes what the app can *promise*, not which store it should use. The honest
promise is: sync happens when the user opens the app, and opportunistically before that.

### 5. Every remote-addressable candidate has a cheap handshake, and in three of four the version summary can live in a name

The ticket asked whether answering "am I behind?" means fetching everything. It does not, anywhere
except a synced folder.

| Store | Cheapest handshake | Cost |
| --- | --- | --- |
| Git remote | `ls-remote` — protocol v2 capability advertisement plus `ls-refs` | 2 HTTP requests, ~1.3 KB for 17 refs, **zero object transfer** |
| Object store | One listing per writer prefix, `start-after` = highest sequence held | W requests, ~254 B envelope; a 5-writer listing ~1.8 KB in one round trip |
| WebDAV | `PROPFIND Depth: 0` for the collection entity tag | **371 B, one round trip** — but a change *detector*, not a summary |
| Personal drive | A delta/change cursor; one provider offers an unauthenticated long-poll push channel (30–480 s) | Cheaper than an object-store listing |
| Synced folder | — | **Not possible** (finding 3) |

**The version summary can be encoded in the store's own naming.** In a git remote it fits in the ref
*name*, so `ls-remote` alone answers the question with no objects moved — with one caveat found by
testing: ref names cannot nest, so the scheme must be flat. In an object store, a zero-padded sequence
number in the key means the listing *is* the summary, with entity tag and size in the metadata so no
fetch is needed. WebDAV is the weakest: the cheap form tells you only that *something* changed, the
full listing costs 28 B/entry gzipped on a compressing server but 245 B/entry on one that offers no
compression even when asked, and the standardised incremental-sync report is not available on the
files endpoint at all.

### 6. Where the families actually separate is credentials, setup and terms — not bytes and not money

Money is a non-discriminator: $0.000–0.004/month for object storage, EUR 0.50–3.20/month for rented
WebDAV, free for git hosting. One rented store is disqualified purely on **billing floors** — a 1 TB
monthly minimum, a 90-day minimum duration and a 4 KB minimum billable object, which inflates segment
storage 3–11× on top of charging for a terabyte to hold twenty megabytes.

What differs materially:

- **A git remote** uses per-device keys or tokens, individually revocable — a well-understood story.
  Its exposure is **the hosts' terms**: one states plainly that private repositories are "explicitly
  not [for use] as a personal cloud or media storage", which is what an append-only personal review
  log is; another caps file requests explicitly to prevent CDN-like usage; a third has no storage
  clause in its acceptable-use policy at all, while its own documentation warns that git is not
  designed as a backup tool. At 47.5 MB per decade no *size* limit binds — the terms are the
  exposure, not the bytes. The one host that unambiguously permits it has an 800 GB minimum order.
- **A rented object store** has the worst credential story. Only one provider lets a user mint a key
  scoped to a name prefix with an expiry from a form; another offers bucket scoping with no prefix
  scoping and no expiry; the third can express it only through hand-written policy JSON. Setup is an
  account signup plus roughly five steps, repeated per device, with the secret shown once — so a lost
  phone means manual rotation everywhere.
- **Rented WebDAV** has the cheapest and most conventional story: app-specific passwords are the norm
  and are mandatory under two-factor authentication, so they are per-device and individually
  revocable. No provider surveyed documents entity-tag or conditional-request behaviour — which
  finding 1 makes a non-issue.
- **A personal cloud drive** clears the bar everyone expects it to fail: a public OAuth client with
  PKCE and a loopback redirect is explicitly supported, no client secret is required, and the
  app-private folder scope is **non-sensitive**, so no app verification and **no verification-time
  endpoint** — which would have been a server. The costs are elsewhere and they are real: refresh
  tokens die after six months unused (exactly the drawer-device case) and after seven days while the
  project remains in testing status, are capped at 100 per account per client, and one provider
  freezes new user links two weeks after the fiftieth user pending review.

### 7. A structural correction to this ticket's own premise: per-writer *files* do not prevent push conflicts

The ticket asserted that because each device appends only to its own rows, "there are no cross-device
write conflicts to resolve at all". For content-addressed transport this is **true of file content
and false of publishing**, because git's compare-and-swap is on the **ref**, not the file. Two devices
committing entirely disjoint per-writer files to one branch: the second push is rejected. The same two
pushes to per-writer **branches** both succeed with no interaction.

The premise is recoverable, but it costs a design commitment — per-writer refs — that the ticket did
not know it was making. The analogous commitment in the other families is per-writer key prefixes,
which finding 1 already shows is load-bearing for a different reason. **"One writer, one namespace" is
the invariant the whole transport design rests on**, whichever store wins.

---

## Two knock-ons for ADR-0004

Findings that bear on the map's own text rather than on transport.

1. **The interchange row size and decade projection are confirmed.** Two slices independently
   generated rows in [ADR-0004](../../adr/0004-the-review-event-log.md) §11's exact shape and measured
   **151.4 B** and **152.5 B** per row against the ADR's "roughly 150 bytes", reproducing §10's "around
   110 MB raw" per decade to within 1%.

2. **§10's "compresses about ten to one" is conditional on two things §11 does not fix — the
   compressor's window and the segment size.** A decade of rows compresses **11.76×** with `zstd -19`
   and **12.01×** as one file per writer-year, but only **3.99×** with `gzip -9`, because gzip's 32 KiB
   window cannot reach back to the repeated writer identifiers — and only **5.02×** as daily segments
   even with a large-window compressor, because the window cannot span files. So §10's "15 MB
   compressed per decade" holds for a large-window compressor over large segments, and becomes roughly
   27 MB under gzip or roughly 22 MB under daily segmentation. **The interchange form should name the
   compressor, or the size figure should carry the condition** — and note that this couples §10 to a
   transport decision the ADR deliberately deferred.

---

## What this does not settle

- **Which store to use.** That is the decision ticket's job, and this note deliberately leaves it
  open. What it establishes is that four families remain live (git remote, rented object store,
  rented WebDAV, personal cloud drive) and one is structurally disqualified (a folder another
  application syncs).
- **Where on the segment dial to land**, and how the ADR-0004 §7 mutable surface moves — snapshot or
  change stream — which finding 1 shows is also a concurrency-control question.
- **Whether git runs on the real handset.** Nothing here was verified on the Pixel 8 Pro, and
  `AGENTS.md` rule 9 requires that before anything is designed on it. The same applies to Storage
  Access Framework throughput.
- **Eight open items in the git slice** (§8 there), notably whether the hosted forges accept pushes to
  non-branch refs — which finding 7 makes load-bearing — and whether classic token types expire, which
  decides whether a year-offline device can still authenticate.
- **Conditional-write support on one object store** was not verifiable from its documentation and
  needs testing against a real bucket.

The decade-scale figures in the git slice are extrapolated from three measured points and labelled as
such; all workload numbers throughout come from synthetic logs generated to ADR-0004 §11's shape, not
from real user data.
