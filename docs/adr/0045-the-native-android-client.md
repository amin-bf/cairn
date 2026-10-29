# ADR-0045: The native Android client — Kotlin and Compose over the shared Rust core

- **Status**: Accepted
- **Date**: 2026-10-29
- **Decided by**: the repository owner, while charting the map *Cairn as a Native Android App in
  Kotlin*; §1 earlier, in conversation, as recorded in [#121](https://github.com/amin-bf/cairn/issues/121)'s
  *Out of scope*
- **Supersedes**: [ADR-0003](0003-client-stack.md) **on Android only**;
  [ADR-0016 §5](0016-backup-and-restore.md)'s *no file picker* and *no text field for a filename*
  **on Android only**
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
6. **On Android the system file picker is used, and the user may type a filename.** ADR-0016 §5
   refused both for reasons that were properties of the egui stack: `NativeActivity` cannot receive an
   activity result, and text input was ASCII-only. A Kotlin activity has neither limit. The desktop
   keeps ADR-0016 §5 as written. **The passphrase refusal stands**: ADR-0020 §1 refuses any secret on
   two grounds independent of the input method.
7. **Where Android has an established practice, the app follows it without a decision of its own.**
   Lifecycle, threading, permissions, storage access, theming plumbing, testing and packaging are done
   the way the platform's own documentation recommends. A decision is raised only where Cairn's own
   ADRs bind the answer, or where the platform offers no settled practice.

What the Kotlin app inherits unchanged: the domain and store rules through the shared core; the design
system's values and rules (ADR-0031 to ADR-0044, stated in logical pixels and in terms of *touch*
rather than of a renderer); the shipped faces, Cairn Icons included (ADR-0038 §1); and the platform
facts the egui build measured (storage locations and Auto Backup, the MediaStore and intent-filter
rules of ADR-0023 and ADR-0024, and the frozen background of client-stack rule 10).

## Out of scope here

Sync — the drive backend and sign-in are built on no platform yet — and iOS are separate efforts.
