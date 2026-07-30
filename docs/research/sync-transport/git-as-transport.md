# Git as the sync transport

**Research ticket:** [#33](https://github.com/amin-bf/leitner/issues/33) (under wayfinder map [#1](https://github.com/amin-bf/leitner/issues/1)) · **Date of research:** 2026-07-30
**Question:** Can a git remote carry the review event log between a user's desktop and Android devices, with no server of our own?

This is a **research** note. It gathers facts and sharpens trade-offs; it does not pick a design. Every non-obvious claim carries an inline source; claims I reasoned rather than sourced are marked **[inference]**; claims I could not settle are collected in §8.

Context assumed throughout, from [ADR-0004](../../adr/0004-the-review-event-log.md) and the ticket: Rust, egui/eframe, **desktop and Android only** (the web target was ruled out while resolving [#12](https://github.com/amin-bf/leitner/issues/12), so browser constraints are out of scope). No server of our own, ever. An append-only JSON Lines review log where every row carries `(writer id, sequence number)` and **each device appends only to its own rows**, so two devices never write the same file and merging is set union — there are no cross-device write conflicts. One user, 2–5 devices. Worst case ~200 reviews/day.

Where a claim is load-bearing I tested it with the `git` CLI (2.55.0) or the Rust toolchain (rustc 1.97.0, Android NDK 29.0.13846066) on this machine and reported the command and output. **The workload numbers in §4 come from a synthetic log I generated; they are not measurements of real user data.** The generator is in the appendix.

---

## Summary of findings

Ordered by how much each one constrains the decision.

1. **Nothing here disqualifies git.** The ticket's stated killer — a browser cannot reach a git remote without a proxy — was moot before this research started, because the web target is out of scope. On desktop and Android there was never a proxy problem. Everything below is cost and operational risk, not impossibility.

2. **The single biggest lever is file layout, and it is not the one the ticket assumed.** "One file per writer" and "one file per writer *per month*" differ by **two orders of magnitude in local cost**. Measured over a synthetic decade (126.3 MB of log text, 3 writers, 3,650 daily commits, git 2.55.0):

   | layout | write amplification | `.git` before repack | `.git` after `git gc` | `git gc` time |
   |---|---|---|---|---|
   | one file per writer (5 yr, **half** the decade) | **911.8×** | **9.14 GB** | 109.9 MB | 29.8 s |
   | one file per writer per month (full decade) | **15.7×** | 133.5 MB | **47.5 MB** | **1.0 s** |

   With one growing file, `git add` re-hashes the *entire* file on every commit, so cumulative bytes hashed is quadratic in the number of commits — measured at exactly `(days+1)/2 ×` the log's own size, over three run lengths. A decade means **~225 GB of hashing and ~36 GB of loose objects between repacks** [inference: quadratic extrapolation from three measured points]. Monthly segments bound the amplification at half a month and make the whole decade's history *smaller than the plain text it stores* (47.5 MB for 126.3 MB, ratio 0.376). §4.

3. **A decade of history is nearly free; the current content is the whole cost.** Full clone of the decade repo moved **45.2 MB** on the wire; `--depth 1` moved **36.4 MB**. Ten years of history costs **8.8 MB more than the tip alone**. Shallow clone is not a meaningful optimisation for this shape of data — but `--filter=tree:0` moves only **2.4 MB** for all 3,650 commits, and `--filter=blob:none` **3.4 MB**, which is what makes a cheap "what do you have?" probe possible without cloning. §5.

4. **"Am I behind?" costs two HTTP requests and about 1.3 KB, measured against a real host.** Protocol v2's capability advertisement is **191 bytes**; the `ls-refs` request is **108 bytes**; the reply is **~1,083 bytes for 17 refs** (~55–64 bytes per ref). With one ref per writer that is a few hundred bytes and one kept-alive TLS connection. **The version summary can be carried in the ref *name*** — `refs/heads/w1-seq-0000000134` — so `ls-remote` alone answers the question with no object transfer at all. One caveat found by testing: ref names may not nest, so `refs/heads/w/1` and `refs/heads/w/1/seq/…` cannot coexist. §5.

5. **Git's unit of concurrency control is the ref, not the file — so per-writer *files* do not prevent push conflicts; per-writer *branches* do.** Two devices editing entirely disjoint files on one branch: the second push is rejected `! [rejected] main -> main (fetch first)`. The same two pushes to `refs/heads/w/1` and `refs/heads/w/2` both succeed with no interaction. This is the single most important structural finding for a design where "there are no conflicts to resolve". §7.

6. **`git2` (libgit2) cross-compiles for `aarch64-linux-android` with HTTPS and SSH, verified by building it here** — it produced `libgit2.a`, `libssl.a` (vendored OpenSSL 3.6.3) and `libssh2.a`. It costs three vendored C libraries and an OpenSSL build in CI. `gix` is pure Rust plus `ring`, cross-compiles cleanly, and needs no OpenSSL — **but its SSH transport works by spawning an external `ssh` program**, and Android forbids `execve()` from the app's home directory. So on Android the practical choices are `gix` over **HTTPS only**, or `git2` with the C dependencies. §1.
   Separately: **`gix` 0.86.0 does not compile on rustc 1.97.0 at all** — 16 errors in `gix-hash` 0.26.0 — and this is *not* Android-specific; it fails on the host too. `gix` 0.72.1 compiles everywhere I tried. §1.

7. **`libgit2` is GPLv2 with a linking exception** — "the authors give you unlimited permission to link the compiled version of this library into combinations with other programs, and to distribute those combinations without any restriction coming from the use of this file" ([libgit2 `COPYING`](https://github.com/libgit2/libgit2/blob/main/COPYING)). Not a blocker, but it is the only copyleft licence in either stack. `gix` is MIT OR Apache-2.0 with no copyleft anywhere. §1.

8. **The hosting terms are the sharpest external risk, and they differ.** One host states plainly that private repositories are "explicitly not [for use] as a personal cloud or media storage" ([Codeberg Terms of Use](https://codeberg.org/Codeberg/org/src/branch/main/TermsOfUse.md)) — which is what an append-only personal review log is. Another caps file requests at 5,000/hour explicitly "to prevent CDN-like usage" and hard-blocks pushes past 4 GB ([Bitbucket](https://confluence.atlassian.com/bbkb/what-are-the-repository-and-file-size-limits-1167700604.html)). GitHub's Acceptable Use Policies contain **no clause prohibiting data storage** — only §9 excessive bandwidth and §4 "excessive automated bulk activity" ([AUP](https://docs.github.com/en/site-policy/acceptable-use-policies/github-acceptable-use-policies)) — while its own docs warn "Git is not designed to serve as a backup tool" and recommend repos stay "less than 1 GB" ([large files](https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-large-files-on-github)). At 47.5 MB per decade none of the size limits bite; the *terms* are the exposure, not the bytes. §3.

9. **Unattended operation on Android is the hardest constraint, and it is not specific to git.** While the device is dozing, "network access is suspended", "JobScheduler doesn't run", and sync adapters don't run ([Doze and App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)). A rarely-used app gets network "about once a day". Background work over 10 minutes needs a foreground service with a visible notification, and on Android 15 a `dataSync` foreground service is capped at **6 hours in any 24** before the system throws `RemoteServiceException` ([Android 15 behaviour changes](https://developer.android.com/about/versions/15/behavior-changes-15)). Our transfers are seconds, so the caps are irrelevant; the *notification* and the once-a-day network window are the real design constraints. App-private internal storage is a normal filesystem directory with no permissions required and no other app able to read it ([app-specific storage](https://developer.android.com/training/data-storage/app-specific)) — adequate for a git working directory. §6.

10. **Two recovery paths are worse than expected, both verified by testing.** A killed `git clone` **deletes the target directory entirely** — no partial progress is kept, and git's smart protocol has no resume, so a device on a bad link retries the whole transfer from zero. And a repository with one missing object **is not repaired by re-fetching**: negotiation is ref-based, the remote believes the client already has the object, and `git fsck` still reports the broken link afterwards. Recovery from corruption is re-clone. §7.

---

## 1. Rust git implementations that work on Android

### 1.1 What I built, and how

All builds used the toolchain on this machine: `rustc 1.97.0 (2d8144b78 2026-07-07)`, `cargo-ndk`, NDK `29.0.13846066`, target `aarch64-linux-android`. Each probe was a bare `lib.rs` with a single dependency, so nothing but the crate under test and its tree was compiled.

```
cargo ndk -t arm64-v8a check  --target aarch64-linux-android    # dependency resolution + build scripts
cargo ndk -t arm64-v8a build  --target aarch64-linux-android    # full compile and archive production
```

### 1.2 `git2` / libgit2 — builds, at the cost of three C libraries

`git2` 0.21.0 with `features = ["https", "ssh", "vendored-libgit2", "vendored-openssl"]` **compiles and links for `aarch64-linux-android`**. The archives it produced under `target/aarch64-linux-android/debug/build/`:

```
libgit2-sys-…/out/build/libgit2.a
openssl-sys-…/out/openssl-build/install/lib/libssl.a
libssh2-sys-…/out/build/libssh2.a
```

Resolved versions: `git2` 0.21.0, `libgit2-sys` 0.18.7+**1.9.6**, `libssh2-sys` 0.3.2, `openssl-sys` 0.9.117, `openssl-src` 300.6.1+**3.6.3**.

Three facts about that build worth carrying:

- **Nothing is on by default.** `git2`'s manifest is `default = []`; `https = ["libgit2-sys/https", "openssl-sys", "openssl-probe", "cred"]` and `ssh = ["libgit2-sys/ssh", "cred"]` ([`Cargo.toml`](https://github.com/rust-lang/git2-rs/blob/master/Cargo.toml)). Network support is entirely opt-in.
- **OpenSSL is not optional on Android.** `libgit2-sys`'s build script picks the TLS backend by target: `GIT_WINHTTP` on Windows, `GIT_SECURE_TRANSPORT` on Apple, and otherwise `#define GIT_OPENSSL 1` ([`libgit2-sys/build.rs`](https://github.com/rust-lang/git2-rs/blob/master/libgit2-sys/build.rs)). Android falls in the `else`, so HTTPS means shipping OpenSSL — 3.6.3, compiled from source, in every CI run and for every ABI.
- **libgit2 has an explicit Android carve-out.** The same build script contains `if !target.contains("android") { features.push_str("#define GIT_USE_NSEC 1\n"); }` — nanosecond stat timestamps are disabled on Android. The consequence is coarser index-staleness detection, i.e. git may need to re-hash a file it would otherwise skip [inference: that is what `GIT_USE_NSEC` controls; I did not measure the effect].

### 1.3 `gix` — pure Rust, but SSH is a subprocess

`gix` 0.72.1 with `default-features = false, features = ["blocking-network-client"]` and again with `"blocking-http-transport-reqwest-rust-tls"` added **both check clean for `aarch64-linux-android`**. The only build-script/C crate anywhere in the 262-package lock is `ring` 0.17.14 (pulled in by `rustls` 0.23.43 via `reqwest` 0.12.28). No OpenSSL, no libssh2, no libgit2.

Two caveats, both material:

- **`gix` 0.86.0 (current) does not compile on rustc 1.97.0.** `cargo check` fails with 16 errors in `gix-hash` 0.26.0 (`E0004` non-exhaustive match, `E0308`, `E0665`). I re-ran the same check for the **host** target (`x86_64-unknown-linux-gnu`) and it fails identically, so **this is a crate/toolchain incompatibility, not an Android problem**. Pinning to `=0.72` compiles. Anyone re-checking this should re-test current `gix` before concluding anything about it.
- **`gix`'s SSH transport shells out.** The blocking SSH client "connect[s] to `host` using the ssh program", and its `ProgramKind` enum — "The kind of SSH programs we have built-in support for" — is `Ssh`, `Plink`, `Putty`, `TortoisePlink`, `Simple` ([`gix-transport` ssh module](https://docs.rs/gix-transport/latest/gix_transport/client/blocking_io/ssh/index.html), [`ProgramKind`](https://docs.rs/gix-transport/latest/gix_transport/client/blocking_io/ssh/enum.ProgramKind.html)). Every one of those is an external executable. That collides head-on with Android's sandbox: "Untrusted apps that target Android 10 cannot invoke `execve()` directly on files within the app's home directory", because "Execution of files from the writable app home directory is a W^X violation" and "Apps should load only the binary code that's embedded within an app's APK file" ([Android 10 behaviour changes](https://developer.android.com/about/versions/10/behavior-changes-10)).

  So `gix` + SSH on Android requires either shipping an `ssh` binary inside the APK where it is executable, or writing a custom transport over a Rust SSH client. The latter is available: `russh` 0.62.4 — "Low-level Tokio SSH2 client and server implementation", supporting public-key auth with `ssh-ed25519`, `rsa-sha2-256/512` and ECDSA ([README](https://github.com/Eugeny/russh)) — **checks clean for `aarch64-linux-android`** in the same harness.

### 1.4 Licences (read from the manifests and bundled sources on disk)

| Component | Licence |
|---|---|
| `git2` 0.21.0, `libgit2-sys`, `libssh2-sys` (the Rust wrappers) | `MIT OR Apache-2.0` |
| **bundled libgit2 1.9.6** | **GPLv2 with a linking exception** |
| bundled libssh2 | BSD-3-clause style (Golemon/Stenberg/Josefsson/Microsoft copyrights, "Redistribution and use in source and binary forms, with or without modification, are permitted provided…") |
| bundled OpenSSL 3.6.3 | Apache License 2.0 |
| `openssl-sys` 0.9.117 | `MIT` · `openssl-src` | `MIT/Apache-2.0` |
| `gix` 0.72.1 | `MIT OR Apache-2.0` |
| `russh` 0.62.4 | `Apache-2.0` · `ring` 0.17.14 | `Apache-2.0 AND ISC` · `rustls` 0.23.43 | `Apache-2.0 OR ISC OR MIT` |

The libgit2 exception is the only copyleft in play and it is written to permit exactly this use: "In addition to the permissions in the GNU General Public License, the authors give you unlimited permission to link the compiled version of this library into combinations with other programs, and to distribute those combinations without any restriction coming from the use of this file" ([`COPYING`](https://github.com/libgit2/libgit2/blob/main/COPYING)). The same file insists "the only valid version of the GPL as far as this project is concerned is _this_ particular version of the license (ie v2, not v2.2 or v3.x or whatever)".

---

## 2. Auth without a server

The constraint is that we can register a client identifier but cannot keep a secret and cannot run a redirect endpoint. Three mechanisms clear that bar.

### 2.1 SSH keypair generated on the device

The user-facing cost is stated by the host: "you will need to generate a new private SSH key and add it to the SSH agent. You must also add the public SSH key to your account on GitHub before you use the key to authenticate" ([about SSH](https://docs.github.com/en/authentication/connecting-to-github-with-ssh/about-ssh)). Translated into our setting: **one enrolment per device** — generate a keypair on-device, get the public half onto the host once. Nothing expires, nothing needs refreshing, and the private key never leaves the device.

The friction is the transfer of the public key. On Android that means the user pasting a public key into a web form on the phone, which is bearable, but note the standing constraint that **Android text input in this app is ASCII-only** (`AGENTS.md` rule 8) — a base64 public key is ASCII, so copy-out works; typing anything non-Latin does not. On Android the keypair itself would be generated in-process by a Rust SSH library, not by an `ssh-keygen` binary (§1.3).

### 2.2 A token typed or pasted once

Fine-grained personal access tokens take an expiry "Integer between 1 and 366" days or `none`, and "Infinite lifetimes are allowed but may be blocked by a maximum lifetime policy set by your organization or enterprise owner" ([managing PATs](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens)). Classic tokens can be non-expiring, but "GitHub automatically removes personal access tokens that haven't been used in a year" (same source) — which is precisely the device-in-a-drawer case this app has to survive.

**Decision-relevant shape:** a token with an expiry turns unattended sync into a chore that recurs at most annually and silently breaks sync when it lapses; a non-expiring token is the only variant that is genuinely fire-and-forget, and it is exactly the variant the host discourages and an organisation policy can forbid.

### 2.3 OAuth device flow — the only one that needs no typing and no secret

For the device flow "you must pass your app's client ID… The `client_secret` is not needed", the device shows a short user code, the user enters it at a verification URL in any browser, and the app polls; codes expire after "900 seconds or 15 minutes". It exists "for apps that don't have access to a web browser" ([authorizing OAuth apps](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps)).

This is the mechanism that fits an Android app with no server: the client ID is public by design, there is no redirect URI to host, and enrolment is "type six characters on any device you like". The cost is token lifetime — a GitHub App's "user access token expires after eight hours, and the refresh token expires after six months" ([refreshing user access tokens](https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/refreshing-user-access-tokens)). **A six-month refresh-token horizon is shorter than "device left in a drawer for a year", so device flow alone does not survive the offline case that §7 has to handle.** Classic OAuth App tokens do not expire unless the app opts in — but the linked doc does not state that positively, so I am marking it unverified (§8).

### 2.4 Where the credential lives

**Android.** App-specific internal storage needs no permission, "other apps cannot access files stored within internal storage", "On Android 10 (API level 29) and higher, these locations are encrypted", and files are removed on uninstall ([app-specific storage](https://developer.android.com/training/data-storage/app-specific)). For the secret itself, the keystore is a key container, not a secret store: "Key material never enters the application process… key material remaining non-exportable", bound to the TEE or a StrongBox secure element ([Android Keystore](https://developer.android.com/privacy-and-security/keystore)). The idiomatic arrangement is therefore a keystore-held key that encrypts a token at rest in app-private storage — the keystore cannot hold the token, only protect it.

An SSH private key has the same problem in a sharper form: it must be usable by the SSH library in-process, so it cannot be a non-exportable keystore key unless the SSH implementation delegates signing to the keystore. **Whether `russh` can be driven from an externally-held signer is not something I established** (§8).

**Desktop.** Git's own mechanism is credential helpers — "programs executed by Git to fetch or save credentials from and to long-term storage", invoked with `get`/`store`/`erase`, resolved by prefixing `git credential-` to the configured name ([`gitcredentials`](https://git-scm.com/docs/gitcredentials)). The two built-ins are `cache` (in memory, short-lived) and `store`, which stores credentials **unencrypted on disk indefinitely**. Since we would be embedding a git library rather than driving the CLI, the helper protocol is available to reuse but not automatic; the equivalent choice on desktop is the platform keyring.

---

## 3. Hosting reality

### 3.1 Size and bandwidth, with real numbers

| Host | Per-file | Per-repo | Per-push | Other |
|---|---|---|---|---|
| GitHub | warning >50 MiB, **blocked >100 MiB** | recommended "<1 GB", "less than 5 GB is strongly recommended" | not published | Git LFS free tier 10 GiB storage + 10 GiB bandwidth/month; at $0 budget "Git LFS usage is blocked for the rest of the calendar month" |
| GitLab.com Free | "100 MiB per-file limit… when pushing new files" | **10 GiB per project**; beyond it "the project is set to a read-only state" and "you cannot push changes" | — | — |
| Bitbucket Cloud | 1 GB file upload | warning at 2 GB, **hard block at 4 GB** ("ability to push new commits… will be disabled"); Free workspaces get **1 GB total** | 3.5 GB ("pack exceeds maximum allowed size") | **5,000 file requests/hour "to prevent CDN-like usage"** |
| Codeberg | not published | not published; "unreasonable storage requirements" may trigger warnings or suspension | — | — |

Sources: [GitHub large files](https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-large-files-on-github), [GitHub LFS billing](https://docs.github.com/en/billing/concepts/product-billing/git-lfs), [GitLab push limit](https://docs.gitlab.com/user/free_push_limit/), [GitLab storage quotas](https://docs.gitlab.com/user/storage_usage_quotas/), [Bitbucket limits](https://confluence.atlassian.com/bbkb/what-are-the-repository-and-file-size-limits-1167700604.html), [Codeberg Terms of Use](https://codeberg.org/Codeberg/org/src/branch/main/TermsOfUse.md).

**Against §4's measured 47.5 MB for a synthetic decade, every one of these limits has three orders of magnitude of headroom.** Size is not the risk.

### 3.2 What the terms actually say about using a repo as a data store

This is where the hosts diverge, and it is worth quoting rather than summarising.

**A host that says no, in as many words.** Codeberg scopes the whole service to "projects covered by a licence for free and open source software, free and open source hardware, or free cultural works", and permits private repositories for "really small & personal stuff like your journal, config files, ideas or notes, **but explicitly not as a personal cloud or media storage**" ([Terms of Use](https://codeberg.org/Codeberg/org/src/branch/main/TermsOfUse.md)). A machine-written append-only review log, growing daily and pushed unattended, is closer to the prohibited side of that line than to "notes". Codeberg is a poor default here — not because it would break, but because the terms say not to.

**A host that says nothing, and warns technically instead.** GitHub's Acceptable Use Policies have no clause about storage or non-code data. The nearest are §9 Excessive Bandwidth Use — "If we determine your bandwidth usage to be significantly excessive in relation to other users of similar features, we reserve the right to suspend your Account, throttle your file hosting, or otherwise limit your activity" — and §4, which prohibits "using our servers for any form of excessive automated bulk activity, to place undue burden on our servers through automated means" ([AUP](https://docs.github.com/en/site-policy/acceptable-use-policies/github-acceptable-use-policies)). The Terms of Service add only an API-abuse clause under §H ([ToS](https://docs.github.com/en/site-policy/github-terms/github-terms-of-service)). Its *technical* documentation is discouraging without being prohibitive: "**Git is not designed to serve as a backup tool**" and "Git is not designed to handle large SQL files" ([large files](https://docs.github.com/en/repositories/working-with-files/managing-large-files/about-large-files-on-github)).

  A once-or-twice-daily push of tens of kilobytes from a handful of a single user's devices is hard to construe as "excessive automated bulk activity" [inference — this is my reading of the clause, not a statement by the host].

**A host that enforces against it mechanically.** Bitbucket's 5,000-file-requests-per-hour cap is documented as existing "to prevent CDN-like usage" — a host that has thought about this pattern and priced it.

**A host where storing arbitrary data *is* the product.** A generic SSH storage account sidesteps the terms question entirely, because there is no "this is for code" premise to violate. rsync.net documents running git over its SSH accounts, including `ssh user@rsync.net "git clone --mirror …"` and using credential helpers or tokens for private upstreams ([rsync.net git howto](https://www.rsync.net/resources/howto/git.html)). The catch is the price floor: "1.5 Cents Per GB Per Month" with an "800 GB Minimum Order" ([pricing](https://www.rsync.net/pricing.html)) — roughly $12/month to store 47.5 MB. **That is not a storage cost, it is an entry fee**, and it is the honest answer to "is there a host that unambiguously permits this?": yes, and it is priced for backups, not for a flashcard app.

---

## 4. Behaviour under this workload

### 4.1 What I generated, and the disclaimer

**These numbers come from a synthetic log, not from real user data.** The generator emits JSON Lines rows shaped like the ADR-0004 log, averaging **169.1 bytes**:

```
{"w":1,"s":1,"ts":"2026-01-01T04:53:31.582Z","card":"d8f16adf-cd61-4c38-1027-1e2f414c343c","kind":"review","grade":4,"ivl":3117,"last_ivl":921,"factor":2267,"ms":21951}
```

Card ids are random UUID-shaped hex, so the data has genuine entropy and does not compress unrealistically well. Three writers share 200 rows/day (67/67/66). A decade is **3,650 daily commits and 126.3 MB of text** — about 15% above the ticket's ~110 MB estimate, because JSON key names cost more than the raw field widths.

Harness settings, all disclosed because two of them are not defaults: `gc.auto=0` so that repacks happen at points I control and can time; an explicit `git gc` every 365 commits plus a final one; and `core.looseCompression=1`, which speeds only the loose-object write path and leaves pack compression at its default — without it the single-file runs would not finish in reasonable time, which is itself a finding.

### 4.2 One growing file per writer: the cost is quadratic

Three run lengths, same generator, same settings:

| days | log text | bytes re-hashed by `git add` | amplification | `.git` before final gc | `.git` after gc | ratio to text | ms/commit | final `gc` |
|---|---|---|---|---|---|---|---|---|
| 200 | 6.9 MB | 0.69 GB | **100.3×** | 303.8 MB | 6.3 MB | 0.92 | 138 | 1.3 s |
| 730 | 25.1 MB | 9.18 GB | **365.1×** | 3,036 MB | 45.2 MB | 1.80 | 259 | 10.4 s |
| 1,825 | 63.0 MB | 57.4 GB | **911.8×** | 9,142 MB | 109.9 MB | 1.75 | 375 | 29.8 s |

The amplification figures are `100.3`, `365.1`, `911.8` against `(days+1)/2` = `100.5`, `365.5`, `913` — **the amplification is exactly half the commit count**, because `git add` re-hashes and re-compresses the whole file every time one line is appended. Extrapolating the same law to a decade: **1,825× amplification, ~225 GB hashed, and ~36 GB of loose objects accumulated between repacks** [inference: quadratic extrapolation, not measured — the 5-year run took 744 s and the decade would take roughly four times that].

The intermediate `git gc` times from the 5-year run show the repack cost climbing too: **3.9 s → 10.2 s → 19.3 s → 23.1 s** at years 1–4, and 29.8 s for the final one. On a phone, on battery, that is the number that matters, not the megabytes.

Default git also would not have repacked when I did. With ~9 objects per daily commit (3 blobs, 5 trees, 1 commit) and `gc.auto` defaulting to 6,700 loose objects, the first automatic repack lands around **day 744** [inference: arithmetic over the default and the observed object count] — by which point the 730-day run had **3.0 GB** of loose objects sitting in `.git`.

### 4.3 Monthly segments per writer: the cost is linear and small

Same decade, same rows, one file per writer per calendar month, still one commit per day:

```
layout=monthly days=3650 writers=3
worktree log bytes      : 126277860 (126.3 MB)
bytes re-hashed by add  : 1985011910 (2.0 GB)  amplification x15.7
.git before final gc    : 133467172 (133.5 MB)
.git after  final gc    :  47520964 ( 47.5 MB)  ratio to worktree 0.376
build wall time         : 613.3 s (git add+commit 600.5 s, 165 ms/commit)
intermediate gc runs    : [(365,0.7),(730,0.7),(1095,0.9),(1460,1.5),(1825,1.1),(2190,1.0),(2555,1.1),(2920,0.9),(3285,0.9)]
final git gc            : 1.0 s
```

Every number that mattered above collapses: amplification bounded at half a month (**15.7×**), `.git` never exceeding 133 MB even unrepacked, repacks flat at **~1 second for the whole decade**, and per-commit cost flat at 165 ms instead of climbing past 375 ms. And the entire decade of history — 3,650 commits — occupies **less space than the plain text it stores** (0.376×).

### 4.4 Does an append-only text file delta-compress? Yes, but the default depth cap fights it

A control repository containing only the *final* 730-day content in a single commit, with no history at all, packs to **7.27 MB** for 25.1 MB of text — so git's own compression on this data is **0.29×**. The 730-day repo with full history packs to 45.2 MB, meaning **37.9 MB of history overhead across 730 commits, ≈52 KB per daily commit** for 34.5 KB of new raw text.

That is worse than it needs to be, and the cause is a default. Repacking the same repository with a deeper delta chain:

```
$ git repack -adfq --depth=1000 --window=50
real 0m22,374s   user 2m24,767s
.git: 45,227,334 → 25,500,099 bytes
```

**`pack.depth` defaults to 50**, so with one file revised 730 times git must store a full copy roughly every 50 revisions; lifting the cap recovers 19.7 MB (44%) at the cost of a 22-second repack. This is a real lever, but it only mitigates a layout problem that monthly segments avoid outright.

A separate micro-test settles that thin-pack deltas do work on this data in principle: a single 449 KB file built by 120 daily commits produces a **2,357-byte** thin pack for one day's fetch, against **1,605 bytes** for the same day's new rows piped through `gzip -6`. Delta compression of an append is near-optimal when the pack layout cooperates.

---

## 5. The "am I behind?" handshake

### 5.1 What asking a real remote costs, measured

Against a real public repository over HTTPS with git 2.55.0 (`GIT_TRACE_CURL=1`, and `curl` for the first leg):

| Leg | Bytes | Note |
|---|---|---|
| `GET /info/refs?service=git-upload-pack` with `Git-Protocol: version=2` | **191 down** | capability advertisement only |
| the same GET **without** the v2 header (protocol v0) | **1,441 down** | v0 advertises every ref up front |
| `POST /git-upload-pack` carrying `command=ls-refs` | **108 up** | |
| its response, 17 refs | **983–1,083 down** | ≈55–64 bytes per ref |

Two HTTP requests over one kept-alive TLS connection; ~0.23 s and ~0.20 s from this machine. This is exactly what protocol v2 was designed for: "Reference advertisement will be omitted unless explicitly requested", and `ls-refs` "takes in arguments which can be used to limit the refs sent from the server", including `ref-prefix <prefix>` ([protocol-v2](https://git-scm.com/docs/protocol-v2)).

**One correction to the obvious assumption:** `git ls-remote` does *not* use `ref-prefix`. Tracing the packets with a pattern argument shows the request is byte-identical to the unfiltered one and contains zero `ref-prefix` lines:

```
$ GIT_TRACE_PACKET=1 git -c protocol.version=2 ls-remote <url> 'refs/heads/*' 2>&1 | grep -c ref-prefix
0
=> Send data: 0014command=ls-refs.001aagent=git/2.55.0-Linux0016object-format=sha100010009peel.000csymrefs.000bunborn.0000
```

`ls-remote` asks for everything and filters locally. At 5 writer refs that is irrelevant; it would matter only if the ref namespace grew large.

### 5.2 The summary can live in the ref name — with one hard constraint

Because a ref is just a name pointing at a hash, the `{writer id → highest sequence}` summary can be encoded in the names themselves, making `ls-remote` a complete answer with **zero object transfer**. Verified locally:

```
$ git ls-remote /…/o.git
0a5ee340…  refs/heads/w/1
1ebb903b…  refs/heads/w1-seq-0000000134
1ebb903b…  refs/leitner/w1
```

Two findings from building that:

- **Ref names cannot nest.** Trying to keep both `refs/heads/w/1` and a sequence-bearing child fails at the server:
  ```
  remote: error: cannot lock ref 'refs/heads/w/1/seq/0000000067': 'refs/heads/w/1' exists;
                 cannot create 'refs/heads/w/1/seq/0000000067'
   ! [remote rejected] HEAD -> w/1/seq/0000000067 (refname conflict)
  ```
  So a sequence-in-the-name scheme needs **flat** names (`w1-seq-0000000134`), and advancing the counter is a create-new-then-delete-old pair rather than an update.
- **A non-branch namespace works** on a plain git server: pushing `HEAD:refs/leitner/w1` succeeded and `ls-remote` returns it. Whether the hosted forges accept pushes outside `refs/heads/` and `refs/tags/` I did **not** verify (§8) — and per-writer *branches* avoid the question entirely.

### 5.3 Clone and fetch costs at decade scale

Against the 126.3 MB / 47.5 MB / 32,850-object decade repository, cloned over `file://` (which uses the same pack protocol as the network, so the wire figures transfer; the wall-clock figures do not):

| clone mode | wire | objects | `.git` | wall |
|---|---|---|---|---|
| full | **45.2 MB** | 32,850 | 46.3 MB | 1.44 s |
| `--depth 1` | 36.4 MB | 366 | 36.4 MB | 0.74 s |
| `--depth 30` | 36.4 MB | 624 | 36.5 MB | 0.79 s |
| `--filter=blob:none` | **3.4 MB** | 21,900 | 40.7 MB after checkout faults blobs in | 0.73 s |
| `--filter=tree:0` | **2.4 MB** | 3,650 | 39.2 MB | 0.68 s |

**A decade of history costs 8.8 MB more than the tip alone (45.2 vs 36.4).** Shallow clone buys almost nothing here, because this repository is not deep-history-heavy — it is content-heavy, and the content is all live. The partial-clone filters are the interesting ones: `tree:0` fetches all 3,650 commits for 2.4 MB, which is enough to *read the log's shape* — commit messages, dates, which segments exist — before deciding what to download. Git's own caveat applies: missing objects are "faulted in" on demand and "Dynamic object fetching tends to be slow as objects are fetched one at a time" ([partial-clone](https://git-scm.com/docs/partial-clone)), and the server must opt in via `uploadpack.allowFilter`.

### 5.4 Incremental fetch: what "behind by N days" actually transfers

Measured by building the exact thin pack the server would send (`git pack-objects --stdout --thin --revs --delta-base-offset`):

| device is behind | monthly segments (decade repo) | one file per writer (2-year repo) |
|---|---|---|
| 1 day | 101 KiB | **13 KiB** |
| 7 days | 150 KiB | 91 KiB |
| 30 days | 457 KiB | 389 KiB |
| 90 days | 1,113 KiB | 1,159 KiB |
| 365 days | 4.63 MiB | 12.4 MiB |
| 1,825 days | 22.2 MiB | — |

**The daily-sync cost of the monthly layout depends on where in the month you are.** Stepping one commit at a time across a month boundary:

```
reviews 2035-11-29 : 104268 B      reviews 2035-12-01 :  11360 B
reviews 2035-11-30 : 107552 B      reviews 2035-12-02 :  21420 B
                                   …
                                   reviews 2035-12-29 : 104033 B
```

The cost fits `≈ 8.0 KB + 3.31 KB × day-of-month` and resets on the 1st — mean **≈59 KB/day, ≈21 MB/year of download for 12.6 MB/year of new text**, a wire overhead of about 1.7×. **I could not fully account for that shape** (§8): `--no-reuse-delta` changes nothing for the one-day case, and the micro-test in §4.4 shows thin-pack deltas working near-optimally on an equivalent single file, so it is a property of how `git gc` laid out this pack rather than a limit of the protocol. The practical reading is that **shorter segments make each fetch cheaper**, and daily segment files would make the one-day fetch approach the ~10 KB floor of the data itself [inference].

The single-file layout is *better* on the wire for a daily sync (13 KiB) and *far worse* for a year-behind catch-up (12.4 MiB vs 4.63 MiB) — and, per §4.2, catastrophically worse locally. That is the whole trade-off in two columns.

---

## 6. Unattended operation on Android

### 6.1 The device will not let you sync whenever you like

While the device is idle, the system suspends network access, ignores wake locks, defers `AlarmManager` alarms including `setExact()`, and does not run `JobScheduler` jobs or sync adapters — the last of which "affects WorkManager tasks as well". It emerges only periodically: "the system exits Doze for a brief time to let apps complete their deferred activities. During this *maintenance window*, the system runs all pending syncs, jobs, and alarms, and lets apps access the network", and it "schedules maintenance windows less frequently over time". For an app the user has not opened recently, App Standby defers network to **about once a day**, and the escape hatches `setAndAllowWhileIdle()` / `setExactAndAllowWhileIdle()` "cannot fire alarms more than once per nine minutes, per app" ([Doze and App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)).

**Read against this app's requirements, that is workable.** A review log does not need minute-level freshness; "sync at least daily, and immediately when the user opens the app" is inside what Doze permits without any exemption. What is *not* available is "push my reviews the moment I grade a card while the phone is in my pocket".

### 6.2 Long transfers need a visible notification, and are capped

Work that exceeds WorkManager's ordinary window must be promoted: `setForeground()`/`setForegroundAsync()` mean "WorkManager manages and runs a foreground service on your behalf to execute the `WorkRequest`, while also showing a configurable notification", and "These Workers can run longer than 10 minutes". Targeting Android 14+, "you must specify a foreground service type for all long-running workers" ([long-running workers](https://developer.android.com/develop/background-work/background-tasks/persistent/how-to/long-running)).

The matching type is `dataSync`, requiring `FOREGROUND_SERVICE_DATA_SYNC` and intended for "data upload or download", "backup-and-restore operations", "transfer data between a device and the cloud over a network" ([service types](https://developer.android.com/develop/background-work/services/fgs/service-types)). Its Android 15 limits:

> "Apps targeting Android 15 can run `dataSync` foreground services for a total of **6 hours in a 24-hour period**." After that the system calls `Service.onTimeout()`, the service has "a few seconds to call `Service.stopSelf()`", and otherwise the system throws `android.app.RemoteServiceException: "A foreground service of type dataSync did not stop within its timeout"`. "The 6-hour limit is shared across all an app's `dataSync` foreground services", but "if the user brings the app to the foreground, the timer resets". Separately, `BOOT_COMPLETED` receivers "are not allowed to launch" a `dataSync` foreground service, raising `ForegroundServiceStartNotAllowedException`.
> — [Android 15 behaviour changes](https://developer.android.com/about/versions/15/behavior-changes-15)

**Our transfers are seconds** — §5.4 puts a daily fetch at ~59 KB and a year-behind catch-up at 4.63 MB — so the 6-hour cap is irrelevant and the 10-minute worker window is ample even for a first clone. The binding costs are the **user-visible notification** for anything promoted to the foreground, and the fact that **sync cannot be started from boot**.

The one case that could exceed the window is a *first* clone on a slow link: 45.2 MB full, or 36.4 MB shallow. At a poor 200 kbit/s that is 25–30 minutes — beyond 10 minutes, so a first sync would need the foreground-service path, and per §7.3 it is not resumable.

### 6.3 Does git-on-Android get a real filesystem? Yes

App-specific internal storage "doesn't require any system permissions to read and write", "other apps cannot access files stored within internal storage", is "encrypted" on Android 10+, and is used through ordinary `File` APIs and streams ([app-specific storage](https://developer.android.com/training/data-storage/app-specific)). Scoped-storage restrictions target shared external storage, not this directory. For git's needs — atomic `rename()` for ref updates, `O_EXCL` lock files, ordinary read/write — that is sufficient [inference: the doc establishes it is a normal writable filesystem directory; I did not run a git repository on a handset to confirm every syscall git relies on behaves, and §8 records that gap].

The one capability it does **not** grant is execution: `execve()` on files in the app home directory is refused for apps targeting Android 10+ ([Android 10 behaviour changes](https://developer.android.com/about/versions/10/behavior-changes-10)). So anything in the git stack that works by spawning a helper program — `gix`'s SSH transport (§1.3), git's own credential helpers (§2.4) — has no home there. **Everything must be in-process.**

---

## 7. What goes wrong

### 7.1 Two devices pushing at once — the file layout does not save you

The premise "per-writer files mean git never has to merge anything" is true about *content* and false about *pushes*. Git's compare-and-swap is on the ref. Two clones, each committing a file the other never touches, both pushing `main`:

```
--- A pushes:
   9e7205a..4200cb2  main -> main
--- B pushes (different file, no textual conflict):
 ! [rejected]        main -> main (fetch first)
error: failed to push some refs to '…/origin.git'
hint: Updates were rejected because the remote contains work that you do not
hint: have locally. This is usually caused by another repository pushing to
hint: the same ref.
--- B recovers by rebase, then pushes:
   4200cb2..673daa3  main -> main
```

**Per-writer files on one branch do not remove the conflict; they only guarantee the automatic resolution is trivial.** Every concurrent push still costs a fetch-and-replay round trip, and the recovery has to be implemented and tested because two devices syncing on the same schedule will collide.

**Per-writer branches remove it entirely.** The identical pair of pushes, retargeted:

```
--- A2 -> refs/heads/w/1:   * [new branch]      HEAD -> w/1
--- B2 -> refs/heads/w/2:   * [new branch]      HEAD -> w/2
```

Both succeed; the devices never contend. The cost is that no single ref names "the whole log" — a reader must enumerate refs (which §5.1 shows costs ~55 bytes each) and merge at read time. That is the same shape as ADR-0004's read-time merge, so it is not a new burden.

When a shared ref *is* unavoidable — the mutable surface of deck names, tags and scheduler config — git offers a real compare-and-swap rather than blind force:

```
$ git push --force-with-lease=refs/heads/shared origin HEAD:refs/heads/shared
 ! [rejected]        HEAD -> shared (stale info)
```

The lease is checked against the client's last-known value, so a device that has not seen the newest state cannot clobber it. That is exactly the `ETag` + `If-Match` primitive the ticket asks about, available on any git remote with no server support beyond ordinary `receive-pack`.

### 7.2 A device offline for a year

Nothing structural breaks: the catch-up is a single fetch of **4.63 MiB** (monthly segments, §5.4) and completes in well under a second locally. The failure modes are elsewhere and are all credential-shaped (§2): a fine-grained token past its ≤366-day expiry, a classic token auto-removed after a year of disuse, or a device-flow refresh token past its six-month life. **A year in a drawer is survivable for the data and not survivable for most of the auth options.** SSH keys are the exception — they do not expire.

### 7.3 Interrupted transfers are not resumable, and a killed clone loses everything

Killing `git clone` mid-transfer:

```
$ timeout 0.6 git clone -q file://…/single2y part
exit=124 ; bytes kept in part/:      # the directory is gone
```

Git removes the target directory on failure. A killed `git fetch` into an existing repository is only slightly better: it leaves an orphan temp pack that the retry does not use.

```
timeout=0.5s exit=124 .git bytes=25041771 objects=1   pack dir: tmp_pack_roOm19
timeout=1.0s exit=124 .git bytes=25041771 objects=1   pack dir: tmp_pack_W6i2d7
timeout=2.0s exit=124 .git bytes=25041771 objects=1   pack dir: tmp_pack_3VqgVG
```

Each attempt writes a fresh `tmp_pack_*` and starts over. On a phone that drops off Wi-Fi mid-clone, **the first sync restarts from zero every time**, and each failed attempt leaves ~25 MB of garbage until something prunes it. This is the strongest argument for `--filter=tree:0` or `--depth 1` on the *first* sync (§5.3): a 2.4 MB transfer succeeds on links where a 45 MB one does not.

### 7.4 Corruption is not repaired by refetching

Delete one object from a healthy clone and ask git to fix itself:

```
$ rm C2/.git/objects/75/f31e7f2832a06039604f49cfce0286dcce17f1
$ git fsck
broken link from  commit 97083a87696e794656eb2fee068671f33fafdb81
              to    tree 75f31e7f2832a06039604f49cfce0286dcce17f1
missing tree 75f31e7f2832a06039604f49cfce0286dcce17f1
$ git fetch --all && git fsck
broken link from  commit 97083a87696e794656eb2fee068671f33fafdb81
              to    tree 75f31e7f2832a06039604f49cfce0286dcce17f1
```

**A refetch does not heal it.** Negotiation is driven by refs: the client's ref still points at a commit the server also has, so the server concludes there is nothing to send. Recovery from local corruption means re-cloning — 45.2 MB, or 36.4 MB shallow — which loops back into §7.3's non-resumability. The compensation is that corruption is *detectable*: git object names are content hashes, so `git fsck` gives a definitive integrity answer that a plain file sync cannot.

---

## 8. What I could not establish

Each of these is load-bearing enough that a decision should not silently assume an answer.

1. **Whether the hosted forges accept pushes outside `refs/heads/` and `refs/tags/`.** It works on a plain git server (§5.2); I did not test it against a host, because doing so means writing to a real repository. Per-writer branches avoid needing the answer.
2. **Whether classic OAuth App tokens expire.** The GitHub App document states 8-hour user tokens with 6-month refresh tokens; it does not positively state the OAuth App case, and I did not find a doc that does. This decides whether device-flow enrolment survives §7.2's year-offline device.
3. **Why a one-day fetch in the monthly-segment layout costs the month-to-date rather than the day** (§5.4). The measurement is reproducible and linear; the mechanism is not accounted for, and `--no-reuse-delta` does not change it.
4. **Whether a Rust SSH client can sign with an Android-keystore-held, non-exportable key.** If not, the SSH private key must sit encrypted-at-rest in app-private storage rather than in the TEE (§2.4).
5. **Whether git actually runs correctly against Android app-private storage on a real handset.** The documentation establishes a normal writable filesystem directory; the repo's own rule ("Verify Android on the real handset", `AGENTS.md` rule 9) means this stays open until someone runs a clone and a push on the Pixel.
6. **Whether an `ssh` binary shipped in the APK's native library directory is executable**, which is the only route to `gix` + SSH on Android. The Android 10 rule prohibits `execve()` from the *app home directory*; whether the extracted native-library directory is exempt I did not verify.
7. **Real-network timings.** Every clone and fetch figure in §5 was measured over `file://`. The *wire byte counts* transfer directly (same pack protocol); the wall-clock numbers do not, and no mobile-network measurement was taken.
8. **The decade-scale single-file numbers in §4.2** are extrapolated from three measured points on an exact quadratic, not measured. The 5-year point is real; the 10-year point is arithmetic.

---

## Appendix: reproducing the workload measurements

Environment: git 2.55.0, rustc 1.97.0, NDK 29.0.13846066, Linux, `GIT_CONFIG_NOSYSTEM=1` throughout.

**Row generator** — 169.1 bytes/row average, three writers sharing 200 rows/day:

```python
def rows(w, s0, day, n, rng):
    base = datetime.datetime(2026, 1, 1) + datetime.timedelta(days=day)
    out = []
    for i in range(n):
        t = base + datetime.timedelta(seconds=rng.randint(0, 86399),
                                      milliseconds=rng.randint(0, 999))
        card = "%08x-%04x-4%03x-%04x-%012x" % (rng.getrandbits(32), rng.getrandbits(16),
                 rng.getrandbits(12), rng.getrandbits(16), rng.getrandbits(48))
        ts = t.strftime("%Y-%m-%dT%H:%M:%S.") + ("%03d" % (t.microsecond // 1000))
        out.append('{"w":%d,"s":%d,"ts":"%sZ","card":"%s","kind":"review",'
                   '"grade":%d,"ivl":%d,"last_ivl":%d,"factor":%d,"ms":%d}'
                   % (w, s0 + i, ts, card, rng.randint(1, 4), rng.randint(1, 3650),
                      rng.randint(1, 1200), rng.randint(1300, 3200), rng.randint(600, 30000)))
    return "\n".join(out) + "\n"
```

**Build loop** — per day, append each writer's rows to `log/w<N>/log.jsonl` (single) or `log/w<N>/<YYYY>-<MM>.jsonl` (monthly), then `git add <paths>` and `git commit -q --no-verify`. Repo configured `gc.auto=0` and `core.looseCompression=1`; `git gc -q` every 365 commits, timed; one final `git gc`, timed. Write amplification is the running sum of `os.path.getsize()` on each file at the moment it is staged, divided by the total new content written.

**Exact wire cost of "behind N commits"** — build the thin pack the server would send:

```bash
{ echo HEAD; echo "^$(git rev-parse HEAD~N)"; } \
  | git pack-objects --stdout --thin --revs --delta-base-offset | wc -c
```

**Clone shapes** — `git clone --progress [--depth 1|--depth 30|--filter=blob:none|--filter=tree:0] file://$REPO`, with `uploadpack.allowFilter=true` set on the source; wire bytes read from git's own `Receiving objects: 100% (N/N), X MiB` line.

**Handshake bytes against a live host** —

```bash
curl -s -o /dev/null -w '%{size_download}\n' -H 'Git-Protocol: version=2' \
  "https://<host>/<repo>.git/info/refs?service=git-upload-pack"
GIT_TRACE_CURL=1 git -c protocol.version=2 ls-remote <url> 2>&1 | grep -E 'Send data, |Recv data, '
GIT_TRACE_PACKET=1 git -c protocol.version=2 ls-remote <url> 'refs/heads/*' 2>&1 | grep -c ref-prefix
```

**Android cross-compilation** — a bare `lib.rs` crate per dependency set, then `cargo ndk -t arm64-v8a check --target aarch64-linux-android` (and `build` for `git2`, to force the C archives to be produced and linked). Licences read from `~/.cargo/registry/src/*/<crate>/Cargo.toml` and from the bundled `COPYING`/`LICENSE.txt` inside `libgit2-sys`, `libssh2-sys` and `openssl-src`.
