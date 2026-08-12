# The Review slice on a handset-shaped screen

[#125](https://github.com/amin-bf/cairn/issues/125), the first of the map's two deliberately scarce
Android checkpoints, in the order it was worked: the **mechanical half** first, then
[the sitting](#the-sitting). Seven captures of the shipped app — `main` at
`c59a070c`, the Review slice complete through the frame ([ADR-0031](../../adr/0031-the-page-frame.md)),
the scale ([ADR-0032](../../adr/0032-the-type-scale-and-the-rhythm.md)), the card
([ADR-0033](../../adr/0033-the-card.md)) and the controls
([ADR-0034](../../adr/0034-the-controls.md)) — running on Android for the first time in this map.

**These are not the checkpoint.** #125 exists for what only a hand and an eye can settle: the palette
at low brightness in a dim room, legibility of the smallest text at arm's length, whether the desktop
rhythm reads cramped or hollow, and whether a control is comfortable under a thumb. None of that is
in `pixel8pro/`. What is there is everything that had to happen *before* that sitting, plus the
findings that turned out to be answerable without a thumb.

**The sitting has since happened**, on the physical device, and it is written up in
[The sitting](#the-sitting) at the foot of this file with two captures in `handset/`. It found the
thing the emulator could not: the slice is arranged for a pointer even though it is *sized* for a
thumb. Read that section before this one if you want the checkpoint's result rather than its
preparation.

## What produced these

An emulated device configured to the handset's geometry — 1344×2992 at 480dpi, which the platform
reports as `sw448dp w448dp h997dp` at a scale factor of 3.0, the same numbers the physical device
reports. It ran **headless** (`-no-window`) for the same reason the desktop harness runs in a nested
compositor: a screen that costs the operator their focus to look at gets looked at less.

```sh
avdmanager create avd -n cairn-p8p -k "system-images;android-36;google_apis;x86_64" -d pixel_8_pro
emulator -avd cairn-p8p -no-window -no-audio -no-boot-anim -no-snapshot -gpu swiftshader_indirect
cargo apk build -p cairn-app --lib --release --target x86_64-linux-android
adb install -r target/release/apk/cairn.apk
adb shell screencap -p /sdcard/c.png && adb pull /sdcard/c.png
```

Two things about that build line are worth writing down, because both cost time to discover.

**`--target` overrides `build_targets`.** `crates/app/Cargo.toml` names `aarch64-linux-android` only,
and an emulator is x86_64. `cargo apk build --target` takes the override without editing the
manifest — but it writes both architectures to the **same** `target/release/apk/cairn.apk`, so the
second build silently replaces the first. Copy each APK out before building the other.

**Release signing is still deliberately unconfigured** ([ADR-0009 §10](../../adr/0009-crate-and-workspace-layout.md)),
so `cargo apk build --release` compiles fully and stops at signing. The debug profile signs itself
and needs nothing — but it produces a **270 MB** APK against the release build's 6.4 MB, unstripped
and unoptimised, which is the wrong thing to judge a design on. The APK handed to the handset was
built by adding `[package.metadata.android.signing.release]` pointing at the local debug keystore
for the length of one build and reverting it immediately. ADR-0009's reason for leaving it out is
that the key is *"a developer's local debug key, an absolute path to one machine"* — which is an
argument against committing it, not against using it, so nothing here contradicts that ADR.

## What the emulator can and cannot answer

It matches the handset on **geometry, density, API level and the layout the platform hands the
window**. It cannot answer anything about **light** or **touch** — a mouse says nothing about a
thumb, and a host monitor says nothing about an OLED panel at low brightness. The split this
directory assumes is the one #125 was re-scoped to: emulator for the mechanical half and for
rendering questions, handset for the judgement.

## What it found

### Persian renders on Android, in all three families

`06-settings-persian-specimen` is the capture worth the whole exercise. The font specimen draws the
same Persian sentence in the proportional, monospace and bold families, and **all three are correct**
— right-to-left, joined, with the full stop at the far left. Client-stack rule 7 is why this needed
looking at rather than assuming: within a family the *first* matching face wins, more than one
shipped face carries the Arabic script, and a glyph existing is not the same as the right face
drawing it.

This is the first photograph of the arrangement [ADR-0015 §9](../../adr/0015-what-the-app-promises.md)
promises. The editor states it in words in `05-notes-editor` — *"This device types Latin text only —
author other scripts on the desktop and they sync here"* — and the specimen is the other half of that
sentence: what cannot be **typed** here is nonetheless **drawn** here, correctly. The two halves had
never been seen together on the platform they describe.

### The nav row can end up under the status bar

`07-nav-under-the-status-bar` is the defect. The application reserves a top band for exactly this
(`lib.rs`, *"reserving the top is what stops the first line of text being drawn under the status
bar"*), and on a **cold launch it works** — `01-review-start` is a cold launch with no input at all
and the band is correct.

It was **activity re-creation** that lost it. A second launch intent onto a live instance recreated
the activity, `bands.top` came back zero, the nav row painted over the clock, and it stayed wrong
until an input event arrived. Rotation, a theme change and returning from the background under
memory pressure all recreate an activity the same way, so the reproduction is not exotic even though
the route used here was.

> **Corrected by the sitting, and the correction is the interesting half.** Rotation does *not*
> recreate this activity: the manifest declares `configChanges=0x4a0` —
> `orientation | screenSize | keyboardHidden` — so the activity handles all three itself. Of the
> three routes named above only a theme change (`uiMode`, outside that mask) would do it. On the
> physical device the band was correct across four driven routes, **including a genuine re-creation**
> captured with no input event at all. See [The sitting](#the-sitting).

The value is read live every frame through the seam, and the seam is documented to degrade to *"no
insets"* rather than fail loudly when the JNI read does not succeed — `getRootWindowInsets` is
*"null until the view is attached"*. A silent zero is therefore indistinguishable from a genuine
zero, which is the property that let this reach a capture at all. **Recorded, not acted on**: #125 is
a checkpoint and the platform freeze holds.

### The seam reads `systemBars()`, which excludes the display cutout

Measured from the window manager on this device: the status bar inset is **84px**, the display cutout
is **151px**, and the window is `EDGE_TO_EDGE_ENFORCED` because the application targets SDK 36.
`platform::android::read_insets` asks for `WindowInsets.Type.systemBars()`, and that family does not
include `displayCutout()` — so on any geometry where the cutout is the taller of the two, the top
band is short by the difference and content lands under the camera. On this device the word
*Settings* sits inside the cutout rectangle.

Whether that is visible on the physical handset depends on whether its status bar is taller than its
cutout, which is the ordinary case and would hide the gap entirely. **This is a handset question**,
listed below.

> **Answered on the device: there is no gap.** The physical Pixel 8 Pro reports a status bar of
> **151px** against a cutout of **151px** — identical, so `systemBars()` and `displayCutout()`
> coincide and the seam is invisible. The nav row's top edge sits exactly at y=151. The emulator's
> 67px shortfall came from *its* status bar being 84px against the same cutout; real hardware sizes
> the status bar to swallow its own cutout. The defect is a latent portability risk on some other
> geometry, not something an eye can find here.

### The page is much taller than the slice was arranged for

`03-review-revealed` is the Review slice as decided, on 997dp of height instead of the 860px window
every capture in `docs/design/controlled-2026-08-12/` was taken at. The card, the grades and *Edit
note* occupy the top half and the bottom half is empty page. Nothing is wrong — the frame is one
column, centred, margin 28, exactly as ADR-0031 fixes it — but the amount of unspent page is a
density question, and density is on the handset's list.

`04-notes-list` sharpens an entry the map already carries under *Not yet specified*: the note rows
are left-packed chips that spend none of their column, and at 448dp with the actions beside them the
row reads as three equal buttons rather than a note with two actions.

### One native crash, once, not reproduced

A SIGSEGV — null pointer, in `art::JNI<false>::GetObjectClass`, called from `libcairn_app.so` on the
`android_main` thread — was collected as a tombstone during the first session, at the moment of the
first tap. It did not reproduce across a further launch-and-tap cycle, and the release build carries
no symbols, so the frame below the JNI entry could not be named. The JNI surface the application has
at all is the inset seam and the file-picking function; the seam is called every frame and had been
called for two hundred seconds without incident before the crash.

That is the whole of what is known. It is recorded here rather than diagnosed because a crash seen
once without symbols is a lead, not a finding, and building a symbol-carrying Android profile is
work this checkpoint was not opened to do.

## The sitting

Everything above is preparation. This is the checkpoint: the same build in a hand, on the physical
Pixel 8 Pro, screen brightness pinned low in an already-dim room, judged one question at a time.
`handset/01-review-revealed-low-brightness.png` is the screen all four judgements were made against.

**The black band at the top of both captures is a redaction, not a defect.** These come off real
hardware rather than an emulator, so the status bar holds system chrome that is nothing to do with
the application — see *Landing work → Rules that are easy to break silently*. It is painted over
**in place** rather than cropped, at the exact inset height — 151px in portrait, 84px in landscape —
so every coordinate this section measures is still true of the image as committed.

### The palette holds at low brightness

The card still reads as **a well cut into the page** — the edge survives. This is the answer worth
the trip. ADR-0033 separates the card from the page by **1.121:1**, and that figure was chosen
against a desktop monitor; an OLED panel at low brightness crushes the bottom of the ramp far harder
than a monitor does, which made it the likeliest thing in the slice not to survive. It survived.

#116's question is discharged with no change to the palette. That is the third independent result in
this map pointing the same way — #124 found all five arrangement variants read better with ADR-0030
unchanged, #131 found the distance from the baseline lives in the frame rather than the colour, and
the expected supersession still has support from nowhere.

### The 12px tier is legible at arm's length

The box badge and the interval preview are both readable at real reading distance without leaning in.

That discharges something the map was waiting on. ADR-0030 §3's **7:1 contrast floor** was reasoned
from a 9px small tier that ADR-0032 raised to 12, and the map has been carrying the floor as a number
that "kept its number and lost its argument". The 12px tier is now demonstrated comfortable at
distance on the smallest screen the design targets, so nothing holds 7:1 up from the legibility side.
Whoever reopens the palette has room to move.

### The slice is arranged for a pointer, and this is the finding

The screen was judged to **look** good — the empty lower half reads as calm, deliberate, not
unfinished — and to be a **stretch one-handed**. Those are not two results. They are the same fact
seen twice: it looks calm because everything is up top, and it is a stretch for exactly that reason.

Measured off `handset/01-review-revealed-low-brightness.png`: the control cluster ends at **y=1880 of
2992**. The card, the grades and *Edit note* occupy the top 63% of the page, and the bottom ~1100px
— the part a thumb owns — is empty.

What is out of reach, precisely: **the card's reveal tap**, and **Forgot**, the full-width bar
sitting highest in the cluster. *Barely* and *Easy*, at the two horizontal extremes of the segmented
row, flip between comfortable and a stretch depending on which hand holds the phone — a second and
independent axis. **Nothing is undersized**: the centre segment is comfortable in either hand and
nothing was mis-hit. No target needs to grow. This is placement alone.

It matters more than a layout nit because it is the first time this map's own rule — **hit targets
and density follow touch, not the pointer** — has met an actual thumb. The rule was honoured in
*sizing*: a 36px control is still 36px, and the sitting confirms that was right. The **arrangement**
was laid out for a pointer anyway, and nothing in the rule as written catches that. Nor could the
desktop have caught it: at the 860px window every capture in `docs/design/controlled-2026-08-12/` was
taken at, there is no leftover height for the content to sit above.

Raised as [#141](https://github.com/amin-bf/cairn/issues/141), which frames it as the question
ADR-0031 left unasked — it decided what the leftover **width** does, and never asked what the
leftover **height** does, because at 860px there wasn't any.

### The card must not absorb the slack

Established while scoping #141, and recorded here because it is a fact about the code rather than a
judgement. `surface::REVIEW_HEIGHT` is a **constant 300 logical px**: the card does not scale with
the page at all today, and at a scale factor of 3.0 it draws ~900 device px to hold two words.
Growing it to fill a 997dp page is the failure `surface.rs`'s own module header already argues
against when it refuses that height for the note list, where it "would make a four-card note 1,200px
of mostly nothing". ADR-0033 makes the card a well cut into the page, and a well that is mostly empty
stops reading as an object sized to its contents. The slack belongs to the page, not the card.

### Landscape, which the emulator set has no capture of

`handset/02-review-landscape.png`. At 2992px of width the frame holds at **measure 640, centred**,
with the leftover width doing nothing — ADR-0031 exactly as fixed, now photographed at more than four
times the measure. The display cutout moves to the left edge at 151px and comes nowhere near content.

### The two platform questions were struck rather than answered

`systemBars()` and the nav row band are both properties of the inset seam in the winit/`NativeActivity`
stack, and the map rules that stack out of scope — *"Android moves to a native Kotlin UI"*. No work
was ever going to be built on either answer, so listing them as handset questions was a scope error
the map had already foreclosed. Both were measured before that was noticed and both are recorded
inline above, where they correct the emulator's account: **there is no cutout gap on this device**,
and **rotation cannot lose the nav row** because the activity is not re-created by it.

The four judgements above are the ones that survive the migration, because none of them is a fact
about a renderer. They are facts about an eye and a thumb, and they transfer to a Kotlin client
unchanged.
