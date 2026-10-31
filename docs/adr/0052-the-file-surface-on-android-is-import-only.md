# ADR-0052: The file surface on Android is import only — deck export leaves the map

- **Status**: Accepted
- **Date**: 2026-10-31
- **Resolves**: [Native Android: What the File Surface Holds on the Handset](https://github.com/amin-bf/cairn/issues/239)
- **Map**: [Map: Cairn as a Native Android App in Kotlin](https://github.com/amin-bf/cairn/issues/193)
- **Decided by**: the repository owner, 31.10.2026, accepting the agent's recommendation on §1. §2
  and §3 are the agent's, under [ADR-0045 §7](0045-the-native-android-client.md): each follows
  the platform's practice or an earlier ADR, so none was put to him
- **Amends, on Android only**: [ADR-0047 §3 §4](0047-the-file-surface-on-android.md) (export
  through the save dialog — its deck half), [ADR-0051 §1](0051-settings-on-android.md) (where the
  share sheet is checked)
- **Related**: [ADR-0022 §5 §6 §9](0022-the-import-preview-and-export-report.md),
  [ADR-0041 §1–§3](0041-the-file-surface.md), [ADR-0024 §2](0024-identifying-a-written-file.md),
  [ADR-0046 §5](0046-the-line-between-kotlin-and-rust.md), [ADR-0016](0016-backup-and-restore.md)

## Context

The map's destination is **parity** with the egui Android app, so that the egui build can retire.
The egui build's file surface is the inbound half only: the file list on Settings, the *Open with*
and share intent filters, and the import preview with its gate, landing on the note list filtered
to the deck. **It has no export screen.** ADR-0022 §9 says what one would state, but the design
pass ruled its drawing out of scope (ADR-0042, *Consequences*), so no renderer draws one. Export
reached the egui handset only through the hand-off specimen, which ADR-0051 §1 dropped.

ADR-0047 §3 §4 and ADR-0051 §1 were nonetheless written as if deck export came with the
file-surface slice. Building it there would mean designing a screen first on Android, which is
beyond parity.

## Decision

### 1. The file-surface slice brings import, and deck export leaves this map

> **On Android the file surface is *Open a file…* (ADR-0047 §1), the *Open with* and share
> filters, and the import preview with its gate and landing. Deck export is not built by this map.
> A later effort draws ADR-0022 §9's export screen for both renderers.**

Export is a new screen on every renderer, not a part of the egui app the Kotlin app has to match.
Charting it here would add design questions without bringing the egui build's retirement any closer.
ADR-0047 §3's rules for a deck file's name and ADR-0047 §4's report still hold. They bind whichever
effort builds deck export.

**Rejected: deck export as a later slice of this map**, after backup and restore. That would put a
screen nobody has designed between the map and its destination.

**Rejected: import and export together in this slice.** It is the same new screen, only earlier.

### 2. The import side carries over as specified, with Android's practice where the ADRs are silent

> **Nothing inbound needed a decision.** ADR-0047 settles the picker, ADR-0046 §5 settles who reads
> the bytes, and ADR-0022 and ADR-0041 settle the preview, its gate and its landing. The rest is the
> platform's practice.

- **The intent filters move to the Kotlin manifest as the egui build declares them**
  (`VIEW` and `SEND`, ADR-0024 §2). The package id is the same (ADR-0045 §4), so *Open with* keeps
  offering the app after the upgrade.
- **Back on the preview is the gate's dismissal**, *Cancel* or the lone *Close*. On Android, Back
  leaves a screen without acting, and ADR-0041 §3's rule that the gate is the only way out holds,
  because Back is the gate's own way out. Nothing is imported by Back.
- **A file that arrives while the app is running** (a new intent to the running activity) takes the
  screen the way a cold start does, with an open editor settled first (ADR-0041 §3, ADR-0021 §7).
- **The document is read off the main thread** through `ContentResolver`, once, and its bytes cross
  to Rust (ADR-0046 §5). No permission is asked for (ADR-0047, *Consequences*).

### 3. What this leaves to backup and restore

> **The save dialog, the export report and the hand-off (ADR-0047 §3 §4, ADR-0023) are built first
> by the backup slice, for the collection archive. ADR-0051 §1's share-sheet check is made against
> that archive. Until restore is built, an archive opened on Android meets the preview's refusal,
> *"This is a collection archive, not a deck file."*, as it does on the desktop (ADR-0041,
> *Consequences*).**

## Amendments to accepted ADRs

Both on Android only.

- **[ADR-0047 §3 §4](0047-the-file-surface-on-android.md)**: the save dialog, the report and the
  hand-off serve the archive in this map. A deck file uses them when a later effort builds deck
  export (§1). ADR-0047's amendment to ADR-0041 §5 reads *the Open a file… action and the backup
  action* until then.
- **[ADR-0051 §1](0051-settings-on-android.md)**: the share sheet is checked against a real archive
  export in the backup slice, not against a deck export in the file-surface slice (§3).

## Consequences

- The file-surface spec round collects ADR-0047 §1 §2 (the picker, any type, the bytes decide),
  ADR-0022 §4–§7 and ADR-0041 §1–§3 (the preview, its gate, the landing), ADR-0024 §2 (the filters),
  ADR-0046 §5 (the bytes cross), ADR-0051 §5 (the *Files* block is now drawn) and §2 here.
- The empty note list's *"or import one"* (ADR-0051 §4) becomes true with this slice.
- The desktop has no export screen either, so neither renderer can write a deck file for a person
  to share. That is recorded, not solved: it belongs to the effort that designs export.
