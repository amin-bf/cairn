# ADR-0053: Backup and restore on Android — built in this map, and restore is reached by opening the archive

- **Status**: Accepted
- **Date**: 2026-10-31
- **Resolves**: [Native Android: What Backup and Restore Are on the Handset](https://github.com/amin-bf/cairn/issues/246)
- **Map**: [Map: Cairn as a Native Android App in Kotlin](https://github.com/amin-bf/cairn/issues/193)
- **Decided by**: the repository owner, 31.10.2026, accepting the agent's recommendations on §1, §2
  and §3. §4 is the agent's, under [ADR-0045 §7](0045-the-native-android-client.md): each point
  follows the platform's practice or an earlier ADR, so none was put to him
- **Amends, on Android only**: [ADR-0041, *Consequences*](0041-the-file-surface.md) (restore has no
  surface), [ADR-0052 §3](0052-the-file-surface-on-android-is-import-only.md) (an archive meets the
  refusal until restore is built)
- **Related**: [ADR-0016](0016-backup-and-restore.md), [ADR-0047](0047-the-file-surface-on-android.md),
  [ADR-0041 §3](0041-the-file-surface.md), [ADR-0022 §5 §6 §12](0022-the-import-preview-and-export-report.md),
  [ADR-0023](0023-sending-a-written-file.md), [ADR-0046 §5](0046-the-line-between-kotlin-and-rust.md),
  [ADR-0051](0051-settings-on-android.md)

## Context

**No renderer has ever shipped backup or restore.** ADR-0041's *Consequences* record it: *"Restore
has no surface anywhere… The application also never writes an archive; only the bench does."*
ADR-0042 drew no backup action on Settings. The domain half exists: the archive is built
(`build_collection`), an opened archive is previewed (`restore_preview`, its refusals, the one line
and the merge sentence), and the store merges log lines (`Collection::ingest`). On Android, an
archive opened today meets the deck preview's refusal (ADR-0052 §3).

Deck export left this map ([ADR-0052 §1](0052-the-file-surface-on-android-is-import-only.md)) on the
same fact: no renderer had it, so parity did not need it. Backup and restore differs in two ways.
ADR-0016 specifies it end to end, where deck export's screen was never drawn. And the map's
destination names it, with the owner asking for full parity.

## Decision

### 1. Backup and restore is built in this map, on Android first

> **The backup slice builds the backup action, the save, the report and its hand-off, the restore
> preview and the restore itself, on Android. The desktop keeps its refusal of an archive until a
> desktop effort builds the same.**

**Rejected: let it leave the map as deck export did.** It would leave the Kotlin app with no way to
write or read the one artifact ADR-0016 exists for, and Android's Auto Backup (ADR-0016 §7) as the
only floor.

### 2. Restore is reached by opening the archive, with no action of its own

> ***Open a file…*, *Open with* and share already reach one preview, and the bytes decide what it
> shows (ADR-0022 §6, ADR-0047 §2). An archive opened by any of them shows the restore preview.
> The words of Settings' *Files* block say that a backup is opened there too.**

**Rejected: a *Restore from a backup…* action** opening the same picker and the same preview. It is
easier to find, but it is a second door to the same room, and a restore is rare enough that the
words beside the one door carry it.

### 3. A finished restore lands on the note list, unfiltered

> **When the restore commits, the screen is the note list with no filter**, as an import lands on
> the note list (ADR-0041 §3). An import of one deck is filtered to it; a restore spans every deck,
> so it lands as an import of several does, unfiltered. Nothing is reported after it commits (ADR-0022 §5).

### 4. Settled by the platform's practice or an earlier ADR, without a question

- **The backup action** is a row in Settings' *Files* block, beside *Open a file…* (ADR-0016 §6,
  ADR-0047's amendment to ADR-0041 §5). Its subtitle is ADR-0016 §6's nudge, *"Last backup 3 March.
  1,240 reviews since."* or *"No backup yet. 4,200 reviews across 812 notes."*, with ADR-0016 §7's
  size line beneath once it is true.
- **"Last backup" is device-local** and never settles: ADR-0016 §6 says backup writes nothing to the
  log and nothing that settles. It is recorded only after the provider's write succeeds, so a
  cancelled or failed save leaves the nudge as it was.
- **Saving** opens the system save dialog (`ACTION_CREATE_DOCUMENT`, `application/octet-stream`)
  with the suggested name `Cairn backup YYYY-MM-DD.ccoll` (ADR-0047 §3). The creation instant is
  taken once and serves both the name and the manifest (ADR-0016 §11). The archive is built in Rust
  off the main thread, and its bytes cross (ADR-0046 §5). Cancelling writes nothing and says nothing.
- **The report** names the document as the provider named it, with the counts, and no location line
  (ADR-0047 §4). It is shown in the backup row itself, as Settings on Android keeps reset's question
  in its row (ADR-0051 §6), with a *Share* hand-off beside it that never fires by itself (ADR-0023).
  ADR-0016 §6's *"not a backup until the user moves it off the device"* is no line of the report: the
  user has just chosen the place, and the hand-off is the help ADR-0023 §2 gives the off-device half.
- **ADR-0051 §1's share-sheet check** is made against a real archive in this slice's handset
  judgement, as ADR-0052 §3 moved it here.
- **The restore preview** is one line and the merge sentence (ADR-0016 §12, ADR-0022 §12), under the
  same gate as the import preview. An archive from another collection is refused in its place,
  naming the way out; on Android the way out drops *"then enrol"*, since no sync is built
  (ADR-0051 §3 §4). An empty collection adopts the archive's identity (ADR-0016 §10). Back or
  *Cancel* dismisses it and restores nothing.
- **The restore is one transaction**: the log lines are merged in, the mutable surface is merged by
  its stamps, and an empty collection adopts the identity (ADR-0016 §4 §10). It is started from the
  foreground, runs off the main thread, and nothing persists until it completes. An open editor is
  settled first and a sitting is left alone, as with an import (ADR-0041 §3, ADR-0022 §6).
- **The read ceiling** (32 MiB, set by the file-surface spec) is revisited by the backup spec, since
  an archive may be larger than any deck file.

## Amendments to accepted ADRs

Both on Android only.

- **[ADR-0041, *Consequences*](0041-the-file-surface.md)**: restore has a surface on Android, the
  restore preview reached by opening an archive (§2), and the application writes an archive (§1).
  The desktop reads the bullet as written.
- **[ADR-0052 §3](0052-the-file-surface-on-android-is-import-only.md)**: the refusal *"This is a
  collection archive, not a deck file."* holds on Android only until the backup slice lands; from
  then on an archive shows the restore preview.

## Consequences

- **Domain work the slice needs**, none of it a decision: the mutable surface merged by stamp on
  restore, the restore applied in one transaction, "last backup" recorded in the store's local
  table, the nudge's counts, and the collection's size for ADR-0016 §7's line.
- `collection_filename()` still returns `collection.ccoll`. The slice moves it to ADR-0047 §3's
  dated name.
- The map's *"Not yet specified"* item for backup and restore is now specified; the spec round
  collects ADR-0016 §4 §6 §7 §10–§12, ADR-0022 §5 §12, ADR-0023, ADR-0047 §2–§4 and this ADR.
