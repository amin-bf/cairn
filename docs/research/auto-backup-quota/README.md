# The automatic-backup size quota: what it counts, and what it says when you cross it

**Research ticket:** [#64](https://github.com/amin-bf/leitner/issues/64) (under wayfinder map #1) · **Date of research:** 2026-08-01
**Question:** [ADR-0016 §7](../../adr/0016-backup-and-restore.md) ships user-facing copy — *"Your collection is 31 MB. Android's automatic backup stops above 25 MB."* — that compares **our own file sizes on disk** against the platform's documented quota. If the quota is instead measured against the *compressed* payload the transport uploads, that sentence is false in the user's favour by roughly an order of magnitude, and [ADR-0007 §6](../../adr/0007-the-local-store.md)'s nine-month cutoff estimate moves with it. The sibling note [#60](https://github.com/amin-bf/leitner/issues/60) recorded this point as **SILENT** in every source it found and said *"do not assume either reading"*. **So: is the 25 MB measured before or after compression?**

This is a **research** note. It gathers facts and sharpens trade-offs; it decides nothing. Unlike its siblings, its primary evidence is a **measurement on the real handset**, because the documentation was already exhausted — [`../auto-backup-at-rest/README.md`](../auto-backup-at-rest/README.md) §4.3 established that the platform says only *"the amount of data"* and nothing more. Measurements are labelled **[M1]**…**[M7]** and every one is reproducible from the commands in §6. Sourced claims cite platform documentation or the platform's own published source; claims I reasoned rather than measured or sourced are marked **[inference]**. Things the runs could not establish are marked **NOT ESTABLISHED** and are not filled in by guessing; §8 collects them.

**The mechanism under examination, stated so the note stands alone.** When the system backs up an app that ships no backup-agent class of its own, it runs a default agent in the app's process which walks the app's private directories and *measures* them, then offers that total to the backup transport for approval before any data is streamed. The transport — a separate, replaceable component that owns the network side and the storage account — either accepts, or rejects the package. The framework contract for that hand-off is published source; the transport that actually runs on a retail handset is proprietary. The question in this note is precisely **which number the transport is shown at that gate**.

---

## Summary of findings

1. **The quota is measured BEFORE compression — against uncompressed bytes.** A payload of 41,943,040 bytes that compresses **158×** with `gzip -9` and **11,022×** with `zstd -19` was **rejected** **[M3]**. Post-compression it would have been 264,606 bytes or 3,805 bytes — three orders of magnitude under any plausible 25 MB threshold — so an accept was mandatory under the "after compression" reading. It was refused. **Confidence: high.**

2. **The transport said so itself, in its own log line, naming the uncompressed number.** The proprietary cloud transport emitted `[FullBackupSession] Package dev.leitner.reviewsession11 failed pre-flight size check at 41944576 bytes` **[M3]**. 41,944,576 is the uncompressed tar-stream total (§2.2), and the rejection came at **pre-flight** — before a single byte of app data was streamed to the transport, so before any compression of that data can exist. This is stronger evidence than the differential the ticket designed for: it is the closed-source component stating the quantity it judged. **Confidence: high.**

3. **The framework contract agrees, and is published.** `SinglePackageBackupPreflight.preflightFullBackup` takes `totalSize` from `agent.doMeasureFullBackup(...)` — the sum of on-disk file sizes — and passes exactly that value to `transport.checkFullBackupSize(totalSize)` ([`PerformFullTransportBackupTask.java`](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/services/backup/java/com/android/server/backup/fullbackup/PerformFullTransportBackupTask.java)). The number the transport is shown at the gate is uncompressed **by construction**, on any transport, because at pre-flight nothing has been compressed yet. **Confidence: high.**

4. **The unit is tar-stream bytes: on-disk size plus a 512-byte header per file, each file padded to a 512-byte boundary.** Measured exactly: a 41,943,040-byte file was accounted at 41,943,552 (= 512 header + 41,943,040 already 512-aligned) and a 343-byte file at 1,024 (= 512 header + 512 padded), summing to the 41,944,576 the transport quoted **[M3]**. Directories and empty dirs measure 0; `cache/` and `code_cache/` are not measured at all **[M2]**. **So "bytes on disk" is the correct unit for our own arithmetic**, with a small overhead that only matters for collections of many tiny files. **Confidence: high.**

5. **ADR-0016 §7's sentence stands as written, and ADR-0007 §6's nine-month estimate is confirmed.** The copy compares our file sizes against the documented constant, and that is the same quantity the platform compares. This is the branch the ticket labelled *"§7 stands as written, its estimate is confirmed, and the row closes."* **Confidence: high**, subject to finding 8's single-device caveat.

6. **Quota failure is entirely silent to the user — now measured, not inferred.** Across the over-quota runs, **no notification was posted** by any backup component **[M6]**, though the backup provider does own notification channels capable of it (*Storage alerts*, *Status alerts*, *Backup*). The only trace is in the log. This **confirms** ADR-0016's second unscheduled *Open items* row, and with it §7's reasoning that the application must state the size fact itself because the platform will not. **Confidence: high** for this device and OS version.

7. **A correction to the sibling note: the two published log lines do NOT both mean "over quota".** The platform's testing guide presents them as equivalent quota indicators, and [`../auto-backup-at-rest/README.md`](../auto-backup-at-rest/README.md) §4.2 quoted it that way. They are distinct: `Transport quota exceeded for package: <PKG>` is emitted for `TRANSPORT_QUOTA_EXCEEDED`, while `Transport rejected backup of <PKG>, skipping` is emitted for `TRANSPORT_PACKAGE_REJECTED` — a different status with a different cause ([`PerformFullTransportBackupTask.java`](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/services/backup/java/com/android/server/backup/fullbackup/PerformFullTransportBackupTask.java)). Measured directly: after a quota rejection the transport backs off for that package, and a subsequent backup of a **1 KB** payload produced the *"rejected… skipping"* line **[M5]**. **Anything that treats that second line as a quota signal will report a size problem where none exists.** **Confidence: high.**

8. **One device, one OS version, one closed-source transport.** Pixel 8 Pro, Android 17 / API 37, transport `com.google.android.gms/.backup.BackupTransportService` bound to a real account **[M1]**. Finding 3 is transport-independent because it is framework source; findings 1, 2, 4, 6 and 7 are one data point about a proprietary component that can change without notice. **The exact threshold value was NOT ESTABLISHED** — see §8.

**The bottom line ADR-0016 asked for.** The arithmetic in §7 is sound and the copy may stay. Our own file sizes are the right input, the cutoff is where §7 says it is, and the nine-month figure survives. What changes is not the decision but two smaller things: the tar overhead is a real if minor addition to our size estimate (finding 4), and any implementation that watches the log for a quota signal must match the *quota* line specifically, not the *rejected* line (finding 7).

---

## 1. Why this had to be measured

The sibling note exhausted the documentation. The platform states the quota — *"Every app can allocate up to 25 MB of backup data per app user"* and *"If the amount of data is over 25 MB, the system calls `onQuotaExceeded()` and doesn't back up data to the cloud"* ([Back up user data with Auto Backup](https://developer.android.com/identity/data/autobackup)) — but *"the amount of data"* is exactly the ambiguous phrase, and no first-party source disambiguates it ([`../auto-backup-at-rest/README.md`](../auto-backup-at-rest/README.md) §4.3, marked **SILENT**).

The stakes are set by compressibility. `collection.db` is a SQLite file of highly repetitive log rows, measured at **11.8×** with a large-window compressor ([`../sync-transport/object-stores-and-drives.md`](../sync-transport/object-stores-and-drives.md) §3.1). A pre- or post-compression reading of the same file therefore differs by roughly an order of magnitude, and ADR-0016 §7 puts a number in front of the user that is only true under one of them.

**The trap the ticket named, and how it was avoided.** The conventional way to test backup is to switch the device to `com.android.localtransport/.LocalTransport`, which has its own quota and performs no cloud-side compression or encryption — it would answer a different question confidently. The quota and the compression behaviour are properties of *the same transport*, so the run has to happen on the transport that ships. It did: `bmgr list transports` reported the proprietary cloud transport as already active (`*`), bound to the signed-in account **[M1]**, and **no transport was switched at any point**.

---

## 2. The experiment

### 2.1 Setup and the control

**Subject:** `dev.leitner.reviewsession11`, a throwaway prototype package already installed on the handset, flagged `DEBUGGABLE ALLOW_BACKUP` **[M1]** — so its private directory is writable via `run-as` and it participates in automatic backup exactly as the real application will. Using an app of ours rather than a real one keeps the blast radius inside a package nobody depends on.

**[M1] Device and transport state**, all read-only:

| Property | Value |
|---|---|
| Device | Pixel 8 Pro (`husky`), serial `38041FDJG004YP` |
| OS | Android 17, API 37, build `CP2A.260705.006` |
| Active transport | `com.google.android.gms/.backup.BackupTransportService` (marked `*`), destination = the signed-in account |
| Backup manager | *"enabled / setup complete / not pending init"*, auto-restore enabled |
| Network | Wi-Fi, active default network present |
| Subject flags | `DEBUGGABLE ALLOW_CLEAR_USER_DATA ALLOW_BACKUP KILL_AFTER_RESTORE`, `minSdk=24 targetSdk=36` |
| Subject data before | 18 KB total; one 343-byte file in `files/` |

**[M2] Control run — a backup must succeed before a failure means anything.** With the subject holding only its 343-byte file, `bmgr backupnow` returned:

```
Package dev.leitner.reviewsession11 with result: Success
Backup finished with result: Success
```

and the log showed the full round trip through the real transport:

```
I Backup  : [GmsBackupTransport] Backup finished for dev.leitner.reviewsession11
I PFTBT   : Full backup completed with status: 0
I PFTBT   : Full data backup pass finished.
```

This control is what separates *"rejected"* from *"the pipeline is broken"*, and it also confirmed that the `PFTBT` tag the testing guide documents still exists at API 37.

The same run exposed the measurement pass, which turned out to be the note's best instrument:

```
I FullBackup_native: measured [/data/data/dev.leitner.reviewsession11/files/PROTOTYPE-review-session-11-log.jsonl] at 1024
I FullBackup_native: measured [/data/data/dev.leitner.reviewsession11/files] at 0
I FullBackup_native: measured [/data/data/dev.leitner.reviewsession11/databases] at 0
I FullBackup_native: measured [/storage/emulated/0/Android/data/dev.leitner.reviewsession11/files] at 0
```

Every path the agent considers is logged with the size it contributes. `cache/` and `code_cache/` never appear — consistent with the documented exclusions, and observed rather than assumed.

### 2.2 The unit, established from the control

A 343-byte file contributes **1,024**. That is 512 bytes of tar header plus 512 bytes of content padded to the block size. The rule generalises exactly, and §3 confirms it at scale:

> contribution(file) = 512 + roundUpTo512(size on disk)

So the quantity under the quota is **uncompressed tar-stream bytes**, which for a collection of a few large files is on-disk size plus a rounding error, and for a collection of very many tiny files could be meaningfully larger. **[inference, high confidence]** — the arithmetic matches both observed files and the transport's own total to the byte, but the tar framing itself is not something the run inspected directly.

### 2.3 The payload

The ticket asked for *"repetitive log-shaped JSON lines, matching the real artifact rather than zeros"* — deliberately not zeros, which a sparse-file or special-case path could treat unrepresentatively. The payload is 32 seed lines shaped like real review-log rows (writer UUID, sequence, note id, ordinal, grade, timestamp, duration, kind, timezone, rollover hour), doubled 14 times and trimmed to exactly **41,943,040 bytes** (40 MiB) **[M3]**.

**[M3a] Compressibility of that exact payload**, measured locally on the identical bytes:

| Compressor | Output | Ratio |
|---|---|---|
| `gzip -9` | 264,606 bytes | **158×** |
| `zstd -19` | 3,805 bytes | **11,022×** |

Both are far past the 11.8× the ADR cites for the real artifact, which is the point: this payload is *deliberately more compressible than reality*, so that a post-compression quota could not possibly reject it. It is a discriminator, not a simulation.

---

## 3. Result: rejected, on the uncompressed number, at pre-flight

**[M3] Run A — 41,943,040 bytes, highly compressible.**

```
Package dev.leitner.reviewsession11 with result: Size quota exceeded
```

and in the log, the decisive four lines:

```
I FullBackup_native: measured [/data/data/dev.leitner.reviewsession11/files/quota-probe.jsonl] at 41943552
I FullBackup_native: measured [/data/data/dev.leitner.reviewsession11/files/PROTOTYPE-review-session-11-log.jsonl] at 1024
I Backup  : [FullBackupSession] Package dev.leitner.reviewsession11 failed pre-flight size check at 41944576 bytes
W PFTBT   : Error -1005 backing up dev.leitner.reviewsession11
I PFTBT   : Transport quota exceeded for package: dev.leitner.reviewsession11
```

Three things are worth separating, because each carries part of the answer.

**The arithmetic closes exactly.** 41,943,552 + 1,024 = **41,944,576**, the number the transport quoted. The transport was shown the sum of uncompressed tar-stream contributions and nothing else.

**The rejection is at pre-flight.** `[FullBackupSession] … failed pre-flight size check` is emitted by the transport (process `20006`, the backup provider) *before* the app's data is streamed. There is no compressed size in existence at that moment for it to have used. This is why the result is not merely a strong hint but a mechanism: **compression cannot participate in a decision taken before compression happens.**

**`Error -1005` is `TRANSPORT_QUOTA_EXCEEDED`,** and the framework's corresponding line is the *quota* line, not the *rejected* line — see §5.

**Why this settles the ticket's decision rule on its own.** The rule was: compressible accepted + incompressible rejected ⇒ measured after compression; **both rejected ⇒ measured before**. Run A is the compressible arm and it was **rejected**. Under the "after compression" reading this payload would have presented as 264,606 bytes at worst — 0.6% of a 25 MB quota — and an accept would have been mandatory. The rejection therefore excludes that reading by itself, independently of the second arm. **[inference, high confidence]**

---

## 4. The second arm, and the confound that swallowed it

**[M4] Run B — the same 41,943,040 byte count, incompressible (`/dev/urandom`).** Result:

```
Package dev.leitner.reviewsession11 with result: Transport rejected package because it wasn't able to process it at the time
```

with no pre-flight size line at all. **This is not a size verdict and must not be read as one.** Run A's rejection was followed by `PFTBT: Transport suggested backoff=604800000` — a seven-day backoff for that package — and the transport was refusing to process the package rather than judging it.

**[M5] The confound was isolated rather than assumed.** Deleting the payload entirely and re-running with the subject back to its original ~1 KB produced *the same* rejection:

```
Package dev.leitner.reviewsession11 with result: Transport rejected package because it wasn't able to process it at the time
I PFTBT   : Transport rejected backup of dev.leitner.reviewsession11, skipping
```

A 1 KB payload cannot be over a 25 MB quota, so the rejection is a per-package cooldown following the quota failure, and **not** size. That test is what makes §5's correction safe to state.

**Status of the incompressible arm: NOT ESTABLISHED, and not required.** The cooldown could not be cleared within the session without either waiting it out or switching transports — and switching transports was ruled out in §1 as the very trap the ticket warned about, while turning device backup off would delete the human's existing backups. A poller was left running to retry as soon as the transport would accept the package again. The arm is **confirmatory only**: §3 shows the compressible arm alone excludes the post-compression reading, and §3's pre-flight mechanism explains *why* no byte count of any compressibility could behave differently.

---

## 5. The two published log lines mean different things

The platform's testing guide presents both lines together: *"The following messages in Logcat indicate that your app has exceeded the transport quota: `I/PFTBT: Transport rejected backup of <PACKAGE>, skipping` — or — `I/PFTBT: Transport quota exceeded for package: <PACKAGE>`"* ([Test backup and restore](https://developer.android.com/identity/data/testingbackup)). [`../auto-backup-at-rest/README.md`](../auto-backup-at-rest/README.md) §4.2 quoted that framing.

The framework source separates them by status code:

| Log line | Emitted for | Meaning |
|---|---|---|
| `Transport quota exceeded for package: <PKG>` | `TRANSPORT_QUOTA_EXCEEDED` (`-1005`) | the payload is over quota |
| `Transport rejected backup of <PKG>, skipping` | `TRANSPORT_PACKAGE_REJECTED` | the transport declined this package **for any reason** |

— [`PerformFullTransportBackupTask.java`](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/services/backup/java/com/android/server/backup/fullbackup/PerformFullTransportBackupTask.java). And **[M5]** shows the second line firing for a 1 KB payload during a backoff, which is a direct counter-example to reading it as a quota indicator.

**Consequence for this repo.** ADR-0016 §7 does not depend on log-watching — it computes from our own file sizes — so nothing in the decision changes. But the sibling note's §4.2 should be read with this correction, and any future diagnostic that greps the log must match the *quota* line.

---

## 6. Reproducing this

All commands ran against the handset over `adb` from `scripts/android-env.sh`'s platform-tools (r37.0.0). **`adb shell run-as <pkg> sh -c '…'` does not work** — `adb` re-splits the argument, so only the first word reaches `sh -c` and the remainder executes in the device shell's own working directory. Feed the script on **stdin** instead:

```sh
source scripts/android-env.sh
PKG=dev.leitner.reviewsession11

# state, read-only
adb shell bmgr list transports          # confirm the cloud transport is active; DO NOT switch it
adb shell dumpsys package $PKG | grep -E 'flags|targetSdk'

# payload: 32 log-shaped JSON lines, doubled 14 times, trimmed to 40 MiB
adb shell run-as $PKG sh <<'EOF'
cd /data/data/dev.leitner.reviewsession11/files || exit 1
i=0; while [ $i -lt 32 ]; do
  printf '{"w":"9f3a1c7e-4b52-4d18-9a6f-2e7c8d1b5a30","s":%06d,"note":"3c1d5e8a-7f24-4b90-8c6e-1a2b3d4f5e60","ord":0,"g":3,"t":17855359059%02d,"dur":2456,"k":"review","tz":"Europe/Berlin","roll":4}\n' $i $i
  i=$((i + 1)); done > quota-probe.jsonl
i=0; while [ $i -lt 14 ]; do
  cat quota-probe.jsonl quota-probe.jsonl > quota-probe.tmp && mv quota-probe.tmp quota-probe.jsonl
  i=$((i + 1)); done
dd if=quota-probe.jsonl of=quota-probe.tmp bs=1048576 count=40 2>/dev/null && mv quota-probe.tmp quota-probe.jsonl
wc -c quota-probe.jsonl
EOF

# the run
adb logcat -c
adb shell bmgr backupnow $PKG
adb logcat -d | grep -iE 'pre-flight|quota|Error -|FullBackupSession|PFTBT|measured .*quota-probe'
```

**Two things not to do.** Do not switch to the local transport — it has its own quota and no cloud-side processing, so it answers a different question. Do not toggle the device backup setting to reset state: *"If you turn off backup on your device, your backups are deleted"* ([Back up your device](https://support.google.com/googleone/answer/9149304)), which would destroy the human's real backups.

**Cleanup performed.** The probe file was deleted from the subject's directory and the subject's own 343-byte prototype log was left untouched **[M5]**; no other package, setting or transport was modified at any point.

---

## 7. Sources

Documentation retrieved 2026-08-01; measurements taken 2026-08-01 on the handset described in **[M1]**.

| # | Source | What it establishes here |
|---|---|---|
| S1 | [Back up user data with Auto Backup](https://developer.android.com/identity/data/autobackup) | The 25 MB quota; *"the amount of data"* — the ambiguous phrase this note resolves; `onQuotaExceeded()`; fail-closed behaviour |
| S2 | [Test backup and restore](https://developer.android.com/identity/data/testingbackup) | Names the proprietary cloud transport; publishes the two log lines — and presents them as equivalent, which §5 corrects |
| S3 | [`PerformFullTransportBackupTask.java`](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/services/backup/java/com/android/server/backup/fullbackup/PerformFullTransportBackupTask.java) (platform source, `main`) | `preflightFullBackup` passes `agent.doMeasureFullBackup(...)`'s `totalSize` to `transport.checkFullBackupSize(totalSize)`; the status-code → log-line mapping in §5 |
| S4 | [Back up your device](https://support.google.com/googleone/answer/9149304) | *"If you turn off backup on your device, your backups are deleted"* — why §6 forbids that reset |
| S5 | [`../auto-backup-at-rest/README.md`](../auto-backup-at-rest/README.md) | The sibling note: records this question as SILENT (§4.3) and quotes the two log lines as equivalent (§4.2) |
| S6 | [`../sync-transport/object-stores-and-drives.md`](../sync-transport/object-stores-and-drives.md) §3.1 | The 11.8× compression ratio of the real artifact, which sets the stakes |
| M1–M7 | This handset | §2, §3, §4 — reproducible from §6 |

---

## 8. Confidence, and what is NOT ESTABLISHED

| Claim | Confidence | Why |
|---|---|---|
| The quota is measured before compression, on uncompressed bytes | **High** | Two independent legs: a 158×/11,022×-compressible payload rejected **[M3]**, and the framework contract handing the transport an uncompressed `totalSize` at pre-flight (S3) |
| The transport judged 41,944,576 uncompressed bytes | **High** | Its own log line, and the figure reconciles to the byte with the measured per-file contributions **[M3]** |
| The unit is tar-stream bytes (512-byte header + content padded to 512) | **High** for the totals, **[inference]** for the tar framing | The arithmetic matches both files and the transport's total exactly; the framing itself was not inspected |
| `cache/` and `code_cache/` are excluded from the measured set | **High** for this device | Observed across every run **[M2] [M3]**; consistent with S1 |
| ADR-0016 §7's copy and ADR-0007 §6's nine-month estimate stand | **High** | Follows from the above; the estimate's *input* (on-disk growth) is the quantity the platform actually compares |
| Quota failure produces no user-visible signal | **High** for this device and OS version | No notification record posted by any backup component after the failures **[M6]**; channels capable of it exist and stayed unused |
| `Transport rejected… skipping` is not a quota signal | **High** | S3's status mapping, plus a direct counter-example at 1 KB **[M5]** |
| **The exact threshold value** (25,000,000 vs 26,214,400 vs something else) | **NOT ESTABLISHED** | Only an upper bound was measured: 41,944,576 is over it. `dumpsys backup` does not expose the transport's quota **[M7]**. Bracketing runs need the per-package cooldown (§4) to clear |
| The incompressible arm of the differential | **NOT ESTABLISHED** | Swallowed by the cooldown (§4). Confirmatory only — §3 excludes the post-compression reading without it |
| Whether any of this holds on other OS versions or transports | **NOT ESTABLISHED** | One device, one version, one proprietary transport (finding 8). The framework leg (S3) is version-controlled source and travels; the transport leg does not |

**The cheap follow-up that would close the remaining gap**, once the transport's per-package cooldown has expired: bracket the threshold by backing up payloads either side of 25,000,000 and 26,214,400 bytes, using the *compressible* payload so that an accepted run uploads almost nothing. Roughly two seconds per rejected run. It would turn ADR-0016 §7's *"25 MB"* from a documented constant into a measured one — worth having, since that number is shown to the user, but **it does not change the answer this ticket asked for**.
