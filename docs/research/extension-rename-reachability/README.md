# The renamed extensions: reachability and dedupe, measured on the handset

Evidence for [ADR-0028 §3](../../adr/0028-the-application-is-named-cairn.md)'s open item: whether
`.cdeck` and `.ccoll` keep the property `.ldeck` and `.lcoll` had, that
[ADR-0024 §1](../../adr/0024-identifying-a-written-file.md) proved a file's **reachability** depends
on.

**Measured 2026-08-08** on the handset: Pixel 8 Pro (`husky`), Android 17 / API 37, build
`CP2A.260705.006`, against the shipped APK shape — `dev.cairn.app`, a manifest plus one
`lib/arm64-v8a/libcairn_app.so`, no `classes.dex`.

## Why this had to be measured rather than reasoned

ADR-0024 established that the extension does a job **upstream of everything else**: `MediaStore`
derives the stored media type from the filename, and only a type of `application/octet-stream` causes
the broad intent filter to fire. A byte-identical deck under an extension the platform *does*
recognise types as something else, is never offered to the application, and **no sniff can recover
it** — the bytes never arrive.

Whether a given extension is absent from the platform's media-type map is a **fact about a third
party's table**. Nothing in this repository can derive it, and the failure mode is silent: the file
simply never appears, with nothing in `logcat` either. So renaming the extension is not a string
change until this has been run.

## Method

One fixture, **identical bytes under every name** — a zip whose first member is `mimetype`, stored
and uncompressed, carrying `application/vnd.cairn.deck+zip`, so the type sits at byte offset 38
(ADR-0008 §10):

```
sha256  bdb891d7475c0ce1d79e428a24c68983891e28360814646a08a2216bc8a7f22f
```

Pushed to `Downloads` under five names, scanned into `MediaStore`, then two reads:

```sh
adb shell content query --uri content://media/external/file \
  --projection _display_name:mime_type --where "_display_name LIKE 'Inbound%'"
adb shell pm query-activities -a android.intent.action.VIEW -t <stored type>
adb shell pm query-activities -a android.intent.action.SEND -t <stored type>
```

`.ldeck` is carried as a **positive control** and `.txt` as the **negative** one, so the table shows
the new extensions behaving like the old rather than merely behaving acceptably.

## Result

| Name | Stored type | `dev.cairn.app` resolves (VIEW / SEND) |
|---|---|---|
| `Inbound.cdeck` | `application/octet-stream` | **yes** |
| `Inbound.ccoll` | `application/octet-stream` | **yes** |
| `Inbound.ldeck` *(control)* | `application/octet-stream` | yes |
| `Inbound` *(no extension)* | `application/octet-stream` | yes |
| `Inbound.txt` *(control)* | **`text/plain`** | **no — not offered** |

The **precise** filter was checked separately and resolves for
`application/vnd.cairn.deck+zip`, confirming the manifest half of
[ADR-0028 §3a](../../adr/0028-the-application-is-named-cairn.md).

Verified against the **emitted** `AndroidManifest.xml` rather than the source (AGENTS.md
deck-export rule 15): `package="dev.cairn.app"`, the broad `application/octet-stream` filter, the
precise `application/vnd.cairn.deck+zip` filter, and **no `pathPattern`**.

## Part 2 — the dedupe rule, re-run through the application's own seam

The reachability table above was measured with files pushed over `adb`. That is fine for what it
tests, because `MediaStore` types a file from its **name** and the pusher is irrelevant. It is *not*
fine for the dedupe rule, which is a property of the **insert the application performs** — and
AGENTS.md client-stack rule 9 records the general form of that trap: a shell-fired intent measures
the harness rather than the application. So this half was driven through the shipped
`Write a deck file` control on the device, exercising `platform::put` exactly as an export does.

Three presses, same requested name, `Downloads` empty beforehand:

| Press | Reported by the application | `MediaStore` stored type | Size |
|---|---|---|---|
| 1 | `Asked for "Specimen.cdeck" — written as "Specimen.cdeck"` | `application/octet-stream` | 844 B |
| 2 | `… written as "Specimen (1).cdeck"` | `application/octet-stream` | 844 B |
| 3 | `… written as "Specimen (2).cdeck"` | `application/octet-stream` | 844 B |

Three things hold, and each is a rule elsewhere in the repository:

- **The suffix lands before the extension**, so the written file still ends `.cdeck` and stays
  recognised. This is what declaring no `mime_type` buys ([ADR-0024 §4](../../adr/0024-identifying-a-written-file.md)):
  a declared type disagreeing with the name is the only thing that produces `Specimen.cdeck (1)`.
- **`MediaStore` dedupes rather than overwriting or failing**, so the second and third writes are new
  rows, not a replaced one.
- **The application reports the name it was given back, never the one it asked for**
  ([ADR-0022 §10](../../adr/0022-the-import-preview-and-export-report.md)). With no file picker the
  user chose neither name nor location, so the report is the only way they can find the file.

**The API 29 arm was not re-run** — no such device was to hand — and the figure there stays as
originally measured under `.ldeck`. The dedupe rule belongs to `MediaStore` rather than to the
extension, and the extension is precisely what this re-run varied, so the older figure is not
invalidated by the rename.

## What this establishes, and what it does not

**Establishes**: `.cdeck` and `.ccoll` are absent from the platform's media-type map at API 37, type
as `application/octet-stream`, and reach our filters — reproducing every column ADR-0024 measured for
`.ldeck`. ADR-0028 §3's open item is discharged, and the rename costs no reachability.

**Does not establish** anything about API 24–36. ADR-0024's own figures were taken at API 37 and API
29, and this run is API 37 only; the media-type map is a platform table that can differ by release.
Nothing here claims below 37 — the same window ADR-0023 and ADR-0024 each left open, unchanged by
this measurement.

**Does not re-measure the sniff or the outbound share.** Those are unchanged by a rename of the
extension and remain covered by their own runs — the `mimetype` member is still the sole authority
over a file's profile. The **dedupe** rule *was* re-measured, in part 2, because it runs through the
application's own write rather than the platform's name-typing.

**A second application was installed during this run.** `dev.leitner.app`, the pre-rename build, was
still on the device and also resolves for `application/octet-stream`. It appears as an extra row in
the resolver output and does not affect the result, which is whether `dev.cairn.app` is present.
Since the package id changed (ADR-0028 §5), the two are separate applications with separate data.
