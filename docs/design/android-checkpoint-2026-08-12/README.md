# The Review slice on a handset-shaped screen

The **mechanical half** of [#125](https://github.com/amin-bf/cairn/issues/125), the first of the
map's two deliberately scarce Android checkpoints. Seven captures of the shipped app — `main` at
`c59a070c`, the Review slice complete through the frame ([ADR-0031](../../adr/0031-the-page-frame.md)),
the scale ([ADR-0032](../../adr/0032-the-type-scale-and-the-rhythm.md)), the card
([ADR-0033](../../adr/0033-the-card.md)) and the controls
([ADR-0034](../../adr/0034-the-controls.md)) — running on Android for the first time in this map.

**These are not the checkpoint.** #125 exists for what only a hand and an eye can settle: the palette
at low brightness in a dim room, legibility of the smallest text at arm's length, whether the desktop
rhythm reads cramped or hollow, and whether a control is comfortable under a thumb. None of that is
in this directory. What is here is everything that had to happen *before* that sitting, plus the
findings that turned out to be answerable without a thumb.

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

## What the handset still has to answer

Everything #125 was actually opened for, none of which is above:

1. **The palette at low brightness, in a dim room** — what #116 was opened for.
2. **Legibility of the smallest text at arm's length** — the 12px small tier, raised from 9 by
   ADR-0032, which is also the tier the box badge now draws in.
3. **Density** — whether the desktop rhythm reads cramped or hollow in the hand, and what the empty
   lower half of `03-review-revealed` should be doing.
4. **Touch targets and one-handed reach** — the three segmented passes in particular, which are the
   narrowest controls the slice has, and the reveal tap on the card face.
5. **Whether the cutout gap above is visible on this device**, or hidden by a taller status bar.
6. **Whether the nav row is ever seen under the status bar in ordinary use** — rotate the device and
   return to the app from the background.
