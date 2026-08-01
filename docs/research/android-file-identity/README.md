# Identifying a written file on Android — measured on the handset

Evidence for [#72 "Decide: how a written file is identified again, when Android keeps neither our
media type nor our extension"](https://github.com/amin-bf/leitner/issues/72).

[ADR-0008 §10](../../adr/0008-the-deck-export-format.md) rests a distinct extension per profile on
its being *"how the operating system and the user tell a deck file from a whole-collection artifact
**before** opening it"*, and states that *"on Android the intent filter matches a `pathPattern` for
the extension alongside the media type."* [ADR-0016 §5](../../adr/0016-backup-and-restore.md) admits
the Android **launch intent** as an entry point on the strength of such a filter, and specifies the
in-app **list** as *"query `MediaStore` for our extensions"*.

[`../android-outbound-share/`](../android-outbound-share/README.md) established, while resolving
[#70](https://github.com/amin-bf/leitner/issues/70), that `MediaStore` derives the media type from
the extension and discards ours. It could not establish what that costs the intent filter, because
the filter did not exist: *"this is reasoned from the two measurements above and has not been
observed — nobody has watched a file manager fail to find us."*

**This note is the watching.**

**Measured 2026-08-01** on the handset from [#7](https://github.com/amin-bf/leitner/issues/7):
Google Pixel 8 Pro (`husky`), Android 17 / API 37, `arm64-v8a` only. Harness kept off `main`:
`crates/app/src/probe_identify.rs` and the candidate filters in `crates/app/Cargo.toml`, on the
archival branch
[`prototypes/issue-72`](https://github.com/amin-bf/leitner/tree/prototypes/issue-72), built with the
project's own `cargo apk build` and installed with `adb install -r`. Every APK below contained
exactly `AndroidManifest.xml`, `lib/arm64-v8a/libleitner_app.so` and the signature — **no
`classes.dex`, no `res/`** — so nothing here is bought with the property
[ADR-0003 §2](../../adr/0003-client-stack.md) measured.

## The headline

**An extension-matched intent filter cannot work on this platform, and the cause is not the one that
was reasoned.** It is not that `pathPattern` fails against `content://` URIs — it works fine. It is
that **the providers that matter pass a row id where a filename would be**, so there is no name in
the URI for any pattern to match.

Three further things came out of it, each a silent failure rather than an error: a `pathPattern` is
ignored outright unless the filter also declares a host, the build tool drops the escape in
`.*\.ldeck`, and **an application cannot see `MediaStore` rows it did not write** — which removes the
fallback entry point before anyone had noticed it was load-bearing.

## What was proven, in order

### 1. What a real file manager actually sends

Files by Google, Downloads view, one tap on a real `.ldeck`:

```
ActivityTaskManager: START u0 {act=android.intent.action.VIEW
  dat=content://com.google.android.apps.nbu.files.provider/...
  typ=application/octet-stream cmp=android/com.android.internal.app.ResolverActivity}
```

The logged URI is redacted by `Uri.toSafeString()`; recovered in full from `dumpsys activity
activities` it is:

```
content://com.google.android.apps.nbu.files.provider/2/1000057665
```

**The path is `/2/1000057665`.** The file manager hands over *its own* provider's URI, and the
filename is not in it. `MediaStore`'s own URIs have the same shape
(`content://media/external/downloads/1000057665`), so both routes that matter are nameless.

### 2. Watching the file manager fail to find us

With the extension-matched filter installed — `scheme=content`, `mimeType=application/octet-stream`,
`pathPattern=.*\.ldeck`, verified registered on the device as
`Path: "PatternMatcher{GLOB: .*\.ldeck}"` — the same tap draws:

> **Open with** — Google · Google Wallet · KeePassDX · Sparkasse

**Leitner is not in the list.** This is the observation [#72](https://github.com/amin-bf/leitner/issues/72)
said nobody had made. The four applications offered are ones declaring broad
`application/octet-stream` filters; none of them can open a deck either.

### 3. Intent resolution, asked of the system directly

`adb shell cmd package query-activities`, against the installed filters. This is the platform's own
resolver, not a reimplementation of its rules.

| # | URI | Declared type | `dev.leitner.app` |
|---|---|---|---|
| 1 | `…nbu.files.provider/2/1000057665` — **the real tap** | `application/octet-stream` | **no match** |
| 2 | `media/external/downloads/1000057665` — what our own `hand_off` passes | `application/octet-stream` | **no match** |
| 3 | `media/external/downloads/1000057665` | `application/vnd.leitner.deck+zip` | matches |
| 4 | `com.example.p/files/deck.ldeck` — a path that *does* carry the name | `application/octet-stream` | matches |
| 5 | `…externalstorage.documents/document/primary%3ADownload%2Fprobe72.ldeck` | `application/octet-stream` | matches |
| 6 | `com.example.p/files/deck.ldeck (1)` — the deduped name | `application/octet-stream` | **no match** |
| 7 | an unrelated `.bin` in Downloads | `application/octet-stream` | no match |

Row 3 is the control: the filters are installed and matching works. Rows 4 and 5 are why the
mechanism cannot be called broken — **the pattern fires whenever the path carries the name.** Rows 1
and 2 are the finding. Row 6 records that even where a name *is* present, the collision suffix
defeats the pattern.

### 4. A `pathPattern` is silently ignored unless the filter declares a host

The first round of row 1 and row 2 above returned **matches**, and the reason cost a build cycle.
Android tests the path list only inside the authority test: with no `android:host`, the filter's
authority set is empty, the path list is never consulted, and the filter degrades to *"any URI of
this scheme and type"* — precisely the over-broad filter it was written to avoid.

The failure is silent in both directions. A filter meant to be narrow is secretly wide; and the
device reports the pattern faithfully in `dumpsys` either way, so inspection does not reveal it.
Adding `host = "*"` produced `Authority: "": -1 WILD` and the path tests began to run.

**Prior art in the wild.** A password manager shipping a custom extension — KeePassDX, `.kdbx`,
which is exactly our bind — declares **both** shapes, and its registered filters read:

```
application/x-kdb:   Scheme "file","content"  Authority "" WILD
                     Path "GLOB: .*"          StaticType application/octet-stream
*/*:                 Scheme "file","content"  Authority "" WILD
                     Path "GLOB: .*\.kdbx"  (and ten more variants)
```

The broad `application/octet-stream` filter with path `.*` is what puts it in the Open-with sheet for
every unknown file on the device, and it is what our row 1 sheet shows it winning. **It paid the
price this ADR is deciding whether to pay**, and it also carries the narrow filter for the routes
where a name survives.

### 5. The build tool drops the escape

`path_pattern = ".*\\.ldeck"` in `Cargo.toml` reached the device as
`PatternMatcher{GLOB: .*ldeck}` — the backslash lost between the manifest generator and the APK.
That pattern is *more* permissive and yet fails on paths the intended one matches, because Android's
`PatternMatcher` simple glob does not backtrack: `.*` consumes up to the **first** occurrence of the
next literal, so `.*ldeck` against `/files/deck.ldeck` stops at the `l` in `files` and never
recovers. That non-backtracking behaviour is also why the password manager above enumerates eleven
variants of its own pattern rather than writing one.

A TOML literal string (`'.*\\.ldeck'`) survives to the device as `.*\.ldeck`. **Verify the emitted
`AndroidManifest.xml`, never the source**, for anything pattern-shaped.

### 6. The list can only ever see files we wrote

[ADR-0016 §5](../../adr/0016-backup-and-restore.md)'s *"no permission at API 29+"* was measured for
the **put**. It does not extend to reading. A `.ldeck` placed in `Downloads` by `adb` — owner
`com.android.shell`, sitting in the same folder — queried from inside the application:

```
Downloads rows visible to us = 0
visible: ours=0 foreign=0
no foreign row to open — cannot test the read grant
```

With a control file written by the application first, through exactly the put ADR-0016 §5 specifies:

```
CONTROL wrote our own probe72-ours.ldeck
Downloads rows visible to us = 7
  row name=probe72-ours.ldeck  mime=application/octet-stream  owner=dev.leitner.app
  … all seven owned by dev.leitner.app …
visible: ours=7 foreign=0
```

**The query is correct and the folder is not visible.** Scoped storage grants an application its own
`MediaStore` rows and nothing else; `READ_MEDIA_*` covers images, video and audio, not documents, and
the general route to another application's file is the picker — refused by
[ADR-0016 §5](../../adr/0016-backup-and-restore.md) for needing an activity result.

So *"query `MediaStore` for our extensions"* is true only of files we wrote. A deck someone sends is
not in the list, cannot be put in the list, and no amount of sniffing helps, because sniffing
requires bytes we are not allowed to open.

### 7. The mangled name is self-inflicted

[ADR-0023](../../adr/0023-sending-a-written-file.md) recorded the collision dedupe as *"it dedupes,
and the suffix lands after the extension"* — `French A1.ldeck (1)` — measured while asking for
`application/vnd.leitner.deck+zip` on a `.ldeck` name. **The behaviour is conditional on that
declaration.** The same display name inserted twice, with bytes written each time:

| Declared media type | Second insert stored as |
|---|---|
| *(none)* | `probe72-x (3).ldeck` — extension **kept** |
| `application/octet-stream` — agrees with the name | `probe72-y (3).ldeck` — extension **kept** |
| `application/vnd.leitner.deck+zip` — disagrees | `probe72-z.ldeck (3)` — extension **destroyed** |

Declaring a type that disagrees with the extension is what moves the suffix past it. Declaring
nothing, or declaring the type the platform was going to store anyway, keeps `.ldeck` on the end.

**A trap inside the trap:** the dedupe fires on file **creation**, not on the row. The first run of
this matrix inserted each name twice without writing bytes and observed no collision at all — a
measurement that silently returned the wrong answer. `openOutputStream` and a write are what make
the collision real.

## What this note does **not** establish

- **That an inbound `ACTION_SEND` delivers a readable URI.** Not measured; completing a share into
  the application would have needed a real send from a real account.
- **Anything below API 29.** `MediaStore.Downloads` and the scoped-storage rules measured here are
  API 29+; `min_sdk_version` is 24, and that gap is inherited from
  [ADR-0016 §5](../../adr/0016-backup-and-restore.md) rather than created here.
- **The desktop.** It writes a real path and keeps both the extension and the media type; nothing
  here reaches it.

## Reproducing this

`cargo apk build` invokes `apksigner`, which is a `java` wrapper — **a JDK on `PATH` is required**
and its absence surfaces only at the signing step, after a full NDK compile, as
`apksigner: line 97: exec: java: not found`.

## Cleanup

Every probe row was deleted from `MediaStore` and from `/storage/emulated/0/Download/` after the
run, verified empty by query and by `ls`.
