# Sending a file out of the app on Android — measured on the handset

Evidence for [#70 "Decide: whether the app helps send an exported deck file"](https://github.com/amin-bf/cairn/issues/70).

[ADR-0016 §5](../../adr/0016-backup-and-restore.md) removed the file **picker** because
`ACTION_CREATE_DOCUMENT` and `ACTION_OPEN_DOCUMENT` deliver through an activity **result**, which
`android.app.NativeActivity` does not forward to native code — catching one needs a Java subclass,
a dex, and a build system that [ADR-0003 §2](../../adr/0003-client-stack.md)'s *"manifest plus one
`.so`"* does not ship. That same section then admitted the Android **launch** intent, because *"a
launch intent is readable from the activity with no result callback and no dex."*
[ADR-0022](../../adr/0022-the-import-preview-and-export-report.md)'s handoff to #70 observes that a
**send** is in the second category and adds: *"That is an argument for looking, not a verification;
`AGENTS.md` rule 9 applies."*

This note is the looking.

**Measured 2026-08-01** on the handset from [#7](https://github.com/amin-bf/cairn/issues/7):
Google Pixel 8 Pro (`husky`), Android 17 / API 37, `arm64-v8a` only. Harness kept off `main`:
`crates/app/src/probe_share.rs` on the archival branch
[`prototypes/issue-70`](https://github.com/amin-bf/cairn/tree/prototypes/issue-70), built with the
project's own `cargo apk build` and installed with `adb install -r`. The APK it produced contained
exactly:

```
AndroidManifest.xml
lib/arm64-v8a/libleitner_app.so
META-INF/…            (signature)
```

**No `classes.dex`. No `res/`.** Every result below was obtained in that shape, so nothing here is
bought with the property ADR-0003 §2 measured and ADR-0009 and ADR-0014 §3 lean on.

## The headline

**A send works, and the argument that removed the picker genuinely does not reach it.**
`startActivity(Intent.createChooser(ACTION_SEND, …))` returned without throwing, and
`com.android.intentresolver/.ChooserActivityLauncher` became the top resumed activity — the system
share sheet, fully populated with real targets. Confirmed on two separate runs.

Four things came with it that a spec has to say out loud, because each is a silent failure rather
than an error: the context is not the Activity, the media type does not survive, a name collision
renames the file *past* its extension, and we cannot enumerate handlers even though the chooser can.

## What was proven, in order

### 1. `startActivity` works from `NativeActivity`, with no dex

```
LEITNER_PROBE: startActivity(createChooser(..)) OK — no dex, no res/
topResumedActivity=ActivityRecord{… com.android.intentresolver/.ChooserActivityLauncher …}
```

The screenshot beside this note (`share-sheet.png`) shows the sheet the handset actually drew:
WhatsApp, Slack, Gmail, Messenger, Quick Share, and five direct-share contact targets.

### 2. The context is `android.app.Application`, not the Activity — so `FLAG_ACTIVITY_NEW_TASK` is mandatory

The first attempt **threw**, and the exception is the useful part:

```
android.util.AndroidRuntimeException: Calling startActivity() from outside of an Activity context
requires the FLAG_ACTIVITY_NEW_TASK flag. Is this really what you want?
    at android.app.ContextImpl.startActivity(ContextImpl.java:1211)
    at android.content.ContextWrapper.startActivity(ContextWrapper.java:438)
```

The stack goes through `ContextWrapper`, not `Activity`. Asked directly:

```
LEITNER_PROBE: context class = android.app.Application ; instanceof Activity = false
```

So the handle `ndk_context::android_context().context()` returns — the same handle
`leitner-store::platform::android` already uses for `getFilesDir()` — is the **Application**
context. `Activity.startActivity` would not require the flag; `ContextImpl.startActivity` does.

**And the flag has to go on the chooser, not on the intent being chosen.** `Intent.createChooser`
returns a *fresh* `Intent`; it inherits neither `FLAG_ACTIVITY_NEW_TASK` nor
`FLAG_GRANT_READ_URI_PERMISSION` from the intent it wraps. Setting them only on the inner intent is
exactly the first failure above, and forgetting the grant on the outer one is the same mistake
pointed at the recipient's read access instead — which would fail *after* the user picks an app.

### 3. ADR-0016 §5's Android put is verified — the open item is discharged

That section's Android row (*"insert into `MediaStore` `Downloads` via `ContentResolver`, write to
the returned URI"*) carried *"It is not verified on the handset — see Open items, and `AGENTS.md`
rule 9."* It works:

```
LEITNER_PROBE: insert -> content://media/external/downloads/1000057657
LEITNER_PROBE: wrote 20 bytes to the MediaStore URI
LEITNER_PROBE: read back, available=20 bytes
```

No permission was requested and none was needed, at API 37, as §5 predicted.

### 4. `MediaStore` derives the media type from the **extension** and discards ours

Five inserts, each asking for a name and a type, then read back twice — once by the application
immediately, once through `adb` after the media scan:

| | Asked name | Asked type | App reads back at once | Stored, after the scan |
|---|---|---|---|---|
| A | `probe-a.ldeck` | `application/vnd.leitner.deck+zip` | as asked | **`application/octet-stream`** |
| B | `probe-b.zip` | `application/vnd.leitner.deck+zip` | as asked | **`application/zip`** |
| C | `probe-c.ldeck` | *(none)* | `application/octet-stream` | `application/octet-stream` |
| D | `probe-d.ldeck` | `application/zip` | **name became `probe-d.ldeck.zip`** | `application/zip` |
| E | `probe-a.ldeck` | `application/vnd.leitner.deck+zip` | **name became `probe-a.ldeck (1)`** | `application/octet-stream` |

Two findings, and both bear on an accepted ADR:

- **The declared type does not survive.** `.ldeck` is not in Android's MIME map, so the row settles
  at `application/octet-stream` no matter what we wrote. Row B is the control: an extension Android
  *does* know overrides our type just the same. **The read-back an application performs immediately
  after `insert` returns the value it asked for**, so a check made at write time sees a type that is
  no longer there minutes later — the failure is invisible from inside the app.
- **Declaring a type the name disagrees with renames the file.** Row D asked for `probe-d.ldeck` and
  got `probe-d.ldeck.zip`. So the name and the type must agree, and for `.ldeck` the only
  self-consistent declaration is `application/octet-stream`.

**What this costs [ADR-0008 §10](../../adr/0008-the-deck-export-format.md)**: its claim that a
distinct extension is *"how the operating system and the user tell a deck file from a whole-collection
artifact **before** opening it"* holds for the **extension** — which is what the intent filter matches
on, so reopening still works — and **fails for the media type**, which is the other half of what an
operating system uses. On Android a `.ldeck` and a `.lcoll` carry the same `application/octet-stream`.
The `mimetype` member at a fixed byte offset, which §10 put there precisely because *"a zip archive's
header says nothing about what it contains"*, is what remains to tell them apart, and it now carries
more weight on this platform than §10 assumed.

### 5. A name collision dedupes — and the suffix lands *after* the extension

Row E asked for a name already taken and got **`probe-a.ldeck (1)`** — verified on disk as
`/storage/emulated/0/Download/probe-a.ldeck (1)`. Not `probe-a (1).ldeck`.

[ADR-0022 §10](../../adr/0022-the-import-preview-and-export-report.md) declined to specify collision
behaviour, saying only that *"whether a colliding display name overwrites, dedupes to
`French A1 (1).ldeck`, or fails differs by platform and API level"*, and specified reading back the
written name instead. **That decision is vindicated and the guess in it was wrong in an instructive
way**: it dedupes, and the deduped name **no longer ends in `.ldeck`**. A second export of the same
deck therefore produces a file the extension-matched launch-intent filter of
[ADR-0008 §10](../../adr/0008-the-deck-export-format.md) will not offer to open, and which a
recipient's operating system cannot type-identify either. Reading the name back is the only way the
application can even know this happened.

### 6. Package visibility filters *our* enumeration, not the chooser

```
LEITNER_PROBE: queryIntentActivities(application/vnd.leitner.deck+zip) visible to us = 1
```

against a sheet showing WhatsApp, Slack, Gmail, Messenger and Quick Share for that same intent.
`AppsFilter` logged the reason — the APK declares no `<queries>` element, so Android 11+ package
visibility hides nearly everything from us:

```
AppsFilter: interaction: dev.leitner.app -> com.google.android.gm/… BLOCKED
AppsFilter: interaction: dev.leitner.app -> com.google.android.apps.docs/… BLOCKED
AppsFilter: interaction: dev.leitner.app -> com.android.chrome/… BLOCKED
```

So **the application cannot count, name or preview the targets**, and any design that wanted to
would need a `<queries>` element in the manifest. The chooser is a system component and is not
filtered, so nothing about the share itself needs it. A custom `vnd.` type is not a problem for the
chooser: it drew the full sheet for `application/vnd.leitner.deck+zip`.

### 7. The recipient resolves the right filename; the chooser's own preview line does not

The sheet's file row read `1000057660` — the `MediaStore` row id — in both runs. Queried the way a
receiving application does, the URI resolves correctly:

```
$ adb shell content query --uri content://media/external/downloads/1000057660 \
      --projection _display_name:_size:mime_type
Row: 0 _display_name=probe-a.ldeck, _size=17, mime_type=application/octet-stream
```

So this is a preview quirk for a URI whose type is `application/octet-stream`, not a naming failure —
the recipient gets `probe-a.ldeck`. Recorded because the sheet is what a user sees, and a bare number
where a filename belongs looks like our bug.

## What this note does **not** establish

- **That a recipient can read the bytes.** The read grant was set correctly on the chooser intent,
  and the metadata query above succeeds, but completing a share into a real application would have
  sent a file to a real contact from the owner's own accounts. Not done deliberately.
- **Anything below API 37.** `MediaStore.Downloads` and the permission-free insert are API 29+;
  `min_sdk_version` is 24. The API 24–28 path is unmeasured and would need a different mechanism.
- **The desktop half.** There is no share sheet on the desktop; that is a design judgement rather
  than a platform fact and #70 settles it.

## Cleanup

All five probe rows were deleted from `MediaStore` and from
`/storage/emulated/0/Download/` after the run.
