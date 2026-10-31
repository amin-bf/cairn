# ADR-0045: The native Android client — Kotlin and Compose over the shared Rust core

- **Status**: Accepted
- **Date**: 2026-10-29
- **Decided by**: the repository owner, while charting the map *Cairn as a Native Android App in
  Kotlin*; §1 earlier, in conversation, as recorded in [#121](https://github.com/amin-bf/cairn/issues/121)'s
  *Out of scope*
- **Supersedes**: [ADR-0003](0003-client-stack.md) **on Android only**;
  [ADR-0016 §5](0016-backup-and-restore.md)'s *no file picker* and *no text field for a filename*
  **on Android only**; by §8, [ADR-0026](0026-the-per-tap-keyboard-re-pop.md) whole and
  [ADR-0025](0025-the-authoring-screen-under-a-soft-keyboard.md) except its §4
- **Related**: [ADR-0006 §5](0006-the-review-session-experience.md) and
  [ADR-0038 §6](0038-the-mark-and-the-icon-rule.md) (native clients are being built; the gesture
  belongs to the platform), [ADR-0021 §7](0021-note-ordering-saving-and-the-note-list.md) and
  [#186](https://github.com/amin-bf/cairn/issues/186) (settle on leaving the foreground),
  [ADR-0020](0020-protection-at-rest.md) (no secret, on its own grounds)

## Context

egui's Android build cannot take composed text: winit's Android backend has no IME path, so Persian
cannot be typed on the handset at all (`AGENTS.md` client-stack rule 8), and every keyboard rule since
(ADR-0025, ADR-0026, the vendored patch) is a workaround for a backend that reports neither text nor
insets. #186 found typing broken on `main` on the handset and was closed rather than fixed, because
the egui Android build was already going to be replaced. The design pass (#121) wrote every design
decision so that it would transfer to a native renderer; this ADR is the renderer arriving.

## Decision

1. **Android is a native app in Kotlin, with Jetpack Compose for the UI. The desktop stays egui.**
   Whether the desktop ever leaves egui is a separate effort.
2. **The Kotlin app is a UI over the shared Rust core, not a rewrite.** `cairn-core`, `cairn-store`
   and `cairn-export` are called through generated bindings (uniffi is the expected tool; the binding
   itself is the map's research ticket). There is one implementation of scheduling, replay, the log,
   card identity and the import rules. A Kotlin rewrite would make two implementations that must agree
   byte for byte on log rows, `CardRef` encoding, interval fuzz and the `.cdeck` format, with nothing
   to check that they do.
3. **It lives in this repository**, as a Gradle project beside the crates that builds the Rust core
   for Android itself. The core and its mobile consumer change together.
4. **It takes over the package id `dev.cairn.app`** and installs as an upgrade over the egui build,
   reading the same `getFilesDir()` collection and the same `getNoBackupFilesDir()` writer marker.
5. **The egui Android build is retired once the Kotlin app reaches parity.** With it go the
   `cargo-apk` packaging, the vendored `egui-winit` patch, the window's inset seam and the ASCII-only
   input rule. Until then both exist, and nothing new is invested in the egui Android build.
   *Done on 31.10.2026: see §8.*
6. **On Android the system file picker is used, and the user may type a filename.** ADR-0016 §5
   refused both for reasons that were properties of the egui stack: `NativeActivity` cannot receive an
   activity result, and text input was ASCII-only. A Kotlin activity has neither limit. The desktop
   keeps ADR-0016 §5 as written. **The passphrase refusal stands**: ADR-0020 §1 refuses any secret on
   two grounds independent of the input method.
7. **Where Android has an established practice, the app follows it without a decision of its own.**
   Lifecycle, threading, permissions, storage access, theming plumbing, testing and packaging are done
   the way the platform's own documentation recommends. A decision is raised only where Cairn's own
   ADRs bind the answer, or where the platform offers no settled practice.
8. **The egui Android build is retired (the repository owner, 31.10.2026), and an upgrade over an
   installed egui app needs nothing of its own.** The Kotlin app reached parity with the file
   surface slice, judged on the handset the same day. Two decisions, both the owner's:
   - **The retirement happens now, and the desktop stays egui.** §5's list goes, with everything that
     existed only for that build: the `cargo-apk` packaging and the `cdylib`, the vendored
     `egui-winit` and its scripts, the window's inset seam with its reserved band and guards, the
     ASCII-only input rule and the editor's line saying the phone types Latin text only, the
     launch intent, the folded editor and the stacked grades a thumb got, and `cairn-app`'s Android
     arm. `cairn-store::platform` and `cairn-export::platform` keep their three arms, and the Android
     arm of each refuses: the Kotlin app passes the directories in and moves the files itself
     (ADR-0046 §5, ADR-0047 §6).
   - **No step is built for the upgrade over an installed egui app.** §4 already makes it an upgrade:
     the same package id, the same debug key, a higher version code, and the same `getFilesDir()`
     collection and `getNoBackupFilesDir()` marker. Nothing else is migrated, checked or announced.

   **What is kept, though the egui Android build used it**: `cairn-desktop` as its own crate (the
   scripts, captures and docs run `-p cairn-desktop`), `crates/app/res` (the Kotlin build ships the
   launcher icon from it, and the desktop's icon face is generated from its drawable), and the egui
   Settings fixture bench (the desktop reaches it too).

   **Backup and restore are not lost by this.** The egui build never had them on Android;
   [ADR-0053](0053-backup-and-restore-on-android.md) builds them in the Kotlin app as the next slice.

What the Kotlin app inherits unchanged: the domain and store rules through the shared core; the design
system's values and rules (ADR-0031 to ADR-0044, stated in logical pixels and in terms of *touch*
rather than of a renderer); the shipped faces, Cairn Icons included (ADR-0038 §1); and the platform
facts the egui build measured (storage locations and Auto Backup, the intent-filter rules of ADR-0024,
and the frozen background of client-stack rule 10). The `MediaStore` facts of ADR-0023 and ADR-0024
were the egui build's write path; the Kotlin app writes through the save dialog (ADR-0047 §3), and
only their lesson on the declared type carries over (`AGENTS.md` deck-export rule 12).

## Amendments to accepted ADRs

§8 retires what these describe on Android; each now holds for the desktop, or as history.

- **[ADR-0003 §5 §6](0003-client-stack.md)** — `cargo-apk`, the per-target `eframe` split, the
  hand-written JNI and the Android text-input choice were the egui Android build's. The split of
  `cairn-desktop` out of `cairn-app` stays, for the reason above.
- **[ADR-0007 §6](0007-the-local-store.md)** — the Android data directory is passed in by the
  Kotlin app; the hand-written JNI is gone. The locations are unchanged.
- **[ADR-0009 §1](0009-crate-and-workspace-layout.md)** — the two boundaries `cargo-apk` and the
  `cdylib` forced stand by choice now.
- **[ADR-0015 §9](0015-the-sync-experience.md)** — the advance statement of the Android text-input
  limitation has nothing to state: the Kotlin app takes composed text.
- **[ADR-0016 §5 §8](0016-backup-and-restore.md)** — §5's seam keeps its Android arms, which refuse;
  §8's ASCII-input argument is gone, and its refusal stands on ADR-0020 §1.
- **[ADR-0024 §5](0024-identifying-a-written-file.md)** — the `cargo-apk` facts (the dropped escape,
  the JDK on `PATH`) describe a packager no longer used.
- **[ADR-0025](0025-the-authoring-screen-under-a-soft-keyboard.md)** and
  **[ADR-0026](0026-the-per-tap-keyboard-re-pop.md)** — retired whole: the inset seam, the reserved
  band, the guards and the vendored patch. ADR-0025 §4's warning above the fields stays.
- **[ADR-0035 §3](0035-the-vertical-anchor.md)** — the egui app no longer runs under a thumb, so it
  draws only the pointer's row; the Kotlin app's grades are its own Review slice's.
- **[ADR-0047](0047-the-file-surface-on-android.md)** — its Consequences' re-wording of the
  `MediaStore` rules is done: `AGENTS.md` deck-export rules 11–15 and backup rules 5, 6 and 8.

## Out of scope here

Sync — the drive backend and sign-in are built on no platform yet — and iOS are separate efforts.
