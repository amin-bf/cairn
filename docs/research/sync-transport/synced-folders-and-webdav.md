# Sync transport, part 1: a folder someone else syncs, and WebDAV

**Research ticket:** [#33](https://github.com/amin-bf/leitner/issues/33) (under wayfinder map [#1](https://github.com/amin-bf/leitner/issues/1)) · **Date of research:** 2026-07-30
**Question:** Can the review log ride on a directory that a sync application the user already runs keeps in step, or on WebDAV against rented storage — on **desktop and Android**, unattended, with no server of ours?

This is a **research** note. It gathers facts and sharpens trade-offs; it decides nothing. Every non-obvious claim carries an inline source. Claims I reasoned rather than sourced are marked **[inference]**. Claims I tested are marked **[tested]** and the command and output are in §5.

Context assumed throughout, from [ADR-0004](../../adr/0004-the-review-event-log.md): an append-only log of JSON Lines rows, each carrying `(writer id, sequence number)`; **every device appends only to its own rows**, so one file or one run of segments per writer, no two devices ever writing the same file, and merge is set union. Rows are relayed byte for byte and never re-encoded (§11). Clients are desktop and Android only — the web target was ruled out while resolving [#12](https://github.com/amin-bf/leitner/issues/12), so browser constraints are out of scope. ~200 reviews/day worst case ≈ 73,000 rows/year; the log is never compacted (ADR-0004 §10). One user, two to five devices.

---

## Summary of findings

The facts that most constrain the decision, disqualifying ones first:

1. **On Android there is exactly one directory that both our app and a third-party sync app can treat as ordinary files, and its API has been deprecated since API 30.** App-private storage is unreachable by anyone else — "Other apps cannot access files stored within internal storage" ([app-specific storage](https://developer.android.com/training/data-storage/app-specific)) — and since Android 11 no app, not even one holding all-files access, can read another app's `Android/data/`: that permission grants "Write access to all internal storage directories **except** `/Android/data/`, `/sdcard/Android`, and most subdirectories of `/sdcard/Android`" ([Manage all files](https://developer.android.com/training/data-storage/manage-all-files)). The exception is `Android/media/<package>`, returned by `Context.getExternalMediaDirs()`, of which Android's own reference says: "These files are scanned and made available to other apps through `MediaStore`… **There is no security enforced with these files.**" — and, in the same paragraph, "**This method was deprecated in API level 30.**" ([`Context`](https://developer.android.com/reference/android/content/Context#getExternalMediaDirs())). Everything else on Android costs a user-picked Storage Access Framework tree — a `content://` URI rather than a path, obtained through an activity result that the Rust Android glue does not expose, so it also costs a hand-written `Activity` subclass in the APK (§1.3).

2. **The peer-to-peer file synchroniser's Android client was discontinued, and the stated cause was the app store.** Syncthing exchanges data directly between devices with no server in between; its Android wrapper was archived on 2024-12-03 with the notice "This app is discontinued. The last release on Github and F-Droid will happen with the December 2024 Syncthing version" ([syncthing/syncthing-android](https://github.com/syncthing/syncthing-android)), the maintainer citing "a combination of Google making Play publishing something between hard and impossible and no active maintenance" ([announcement, 2024-10-20](https://forum.syncthing.net/t/discontinuing-syncthing-android/23002)). The surviving community fork declares `MANAGE_EXTERNAL_STORAGE` and a `specialUse` foreground service in its manifest [tested — see §5.6], and its Play listing returns **HTTP 404**: it ships via F-Droid and GitHub releases only ([Catfriend1/syncthing-android](https://github.com/Catfriend1/syncthing-android)). So this family costs the user a sideload plus an all-files-access grant plus a battery-optimisation exemption, on every Android device.

3. **None of the three big commercial drive clients sync a local folder on Android at all.** The strongest statement is first-party and unambiguous: "**The OneDrive app does not sync files automatically**, but you can upload updated files and edited pictures", and offline-marked files "are **read-only** — you can edit them only when you're online. If you edit a file offline, it saves as a new file, and does not change the original OneDrive file" ([Microsoft](https://support.microsoft.com/en-US/onedrive/use-onedrive-on-android)). Dropbox: "Files and folders in the Dropbox mobile app aren't stored on your phone or tablet, so they aren't available offline by default" ([Dropbox help](https://help.dropbox.com/sync/access-files-offline)), and the synced folder is a computer concept — "If you set your files to online-only, they will still appear in the Dropbox folder **on your computer**" ([Dropbox help](https://help.dropbox.com/sync/sync-overview)). Google's desktop sync client lists only "64-bit Windows 10 and up", "ARM64 Windows 11 and up" and "MacOS Ventura 13.0 or higher"; on Android, offline is a per-file toggle ([Google](https://support.google.com/drive/answer/2375082), [Google](https://support.google.com/drive/answer/2375012?co=GENIE.Platform%3DAndroid)). **These are not transports for us.**

4. **A synced folder cannot answer "am I behind?" at all.** It shows a device its own local copy; there is no remote to interrogate. A directory listing tells you what has already arrived, never what exists elsewhere. The peer-to-peer synchroniser states the delivery model plainly: it "does not upload your data to the cloud but exchanges your data across your machines **as soon as they are online at the same time**" ([FAQ](https://docs.syncthing.net/users/faq.html)) — so two devices that are never awake together never converge, and neither can tell. **[inference]** from that model: the version-summary handshake of ADR-0004 §2 has no counterparty in this family; a device can only observe arrival, never absence.

5. **WebDAV's conditional writes are mandatory to honour and were silently ignored by two of the three servers I tested — in the data-losing direction.** RFC 9110 is a MUST: "the origin server MUST evaluate the If-Match condition per Section 13.2 prior to performing the method", and "An origin server that evaluates an If-Match condition MUST NOT perform the requested method if the condition evaluates to false" (§13.1.1). Nextcloud 34.0.2 obeyed — `If-Match` with a stale tag returned **412** and left the file intact [tested]. `rclone serve webdav` v1.74.4 and `dufs` 0.46.0 both returned **201 Created** and **overwrote the file** for both `If-Match: "0000"` and `If-None-Match: *` [tested]. A client cannot assume the precondition was evaluated.

6. **Appending to a remote file is not portable, and attempting it can destroy the file.** RFC 9110 §14.5 is explicit that partial PUT "support is inconsistent and depends on private agreements with user agents", and — the sharp part — "**Partial PUT is not backwards compatible with the original definition of PUT. It may result in the content being written as a complete replacement for the current representation.**" That is not hypothetical: a `PUT` with `Content-Range: bytes 8-12/13` against both `rclone serve webdav` and `dufs` returned 201 and left the file containing **only the fragment** [tested]; Nextcloud correctly returned **400** and left the file untouched [tested]. The out-of-band extension that does support appending — `PATCH` with `Content-Type: application/x-sabredav-partialupdate` and `X-Update-Range: append` ([sabre/dav](https://sabre.io/dav/http-patch/)) — worked on `dufs` (204), returned **400** on `rclone`, and returned **500** on Nextcloud [tested], despite Nextcloud being built on that same library. **Appending is off the table; segments are forced.**

7. **Segmenting costs real compression, and I measured how much.** Over 73,000 synthetic rows in ADR-0004 §11's exact shape (measured at **151.4 bytes/row raw**, matching the ADR's "roughly 150 bytes"), `zstd -19` gives **12.01×** for one file per writer-year but only **5.02×** for daily 200-row segments — 2.20 MB/year instead of 0.92 MB/year, and 365 files instead of one [tested]. Per decade per writer: ~9.2 MB in one file, ~22 MB in 3,650 daily segments.

8. **Reading only the new tail of a remote file works; that is the one thing that survives.** `Range: bytes=5-` returned **206 Partial Content** with a correct `Content-Range` on all three servers tested. So a large per-writer file can be *read* incrementally even though it cannot be *written* incrementally — a device that knows it holds 8.4 MB of a 9.2 MB file fetches 800 KB, not 9.2 MB. **[inference]**: this makes "few big files, rewritten whole on publish, read by range" a real shape, distinct from "many small segments".

9. **The cheap handshake exists on WebDAV, and its price is one request.** A `PROPFIND` with `Depth: 0` for `getetag` on a collection returned **371 bytes** on Nextcloud, and the value changed both when a file was added and when an existing file was modified [tested] — one round trip answers "has anything changed?". The expensive form is the full listing: `Depth: 1` with named `getetag`+`getcontentlength` over 100 entries cost **28,577 bytes (286 B/entry)**, gzip-compressed on the wire to **2,794 bytes (28 B/entry)**; `allprop` cost **42,485 bytes (425 B/entry)** [tested]. The same listing against `rclone serve webdav` over 1,000 entries cost **245,280 bytes (245 B/entry) with no compression offered even when `Accept-Encoding: gzip` was sent**, and `allprop` cost **787,570 bytes (788 B/entry)** [tested]. RFC 6578's incremental `sync-collection` REPORT is **not** available on the files endpoint: the server answered 415 with "The {DAV:}sync-collection REPORT is not supported on this url." [tested].

10. **The maintained Rust WebDAV client builds clean for `aarch64-linux-android`, and issues the most expensive PROPFIND there is.** `reqwest_dav` 0.3.3 (last published 2026-03-02, 254,184 downloads in 90 days, MIT OR Apache-2.0, [crates.io](https://crates.io/crates/reqwest_dav)) plus `reqwest` with `rustls-tls` passed `cargo ndk -t arm64-v8a check` against NDK 29.0.13846066 on rustc 1.97.0 [tested]. But its `list_raw` hard-codes `<D:allprop/>` — the 425 B/entry form — with no way to name properties through the typed API; `start_request` returns a bare `reqwest::RequestBuilder`, so conditional headers, `Range`, and named-property PROPFIND are all "build it yourself". The TLS stack it pulls in also requires Android-specific bootstrapping: `rustls-platform-verifier` needs `init_hosted()` called "before any networking has a chance to run", taking a `JNIEnv` and an Android `Context` ([docs.rs](https://docs.rs/rustls-platform-verifier/latest/rustls_platform_verifier/)).

11. **Unattended background sync on Android has a hard floor of 15 minutes and a ceiling of 6 hours a day.** The minimum periodic work interval "is 15 minutes (same as the JobScheduler API)" and the actual run time "depends on the constraints that you are using… and on the optimizations performed by the system" ([WorkManager](https://developer.android.com/develop/background-work/background-tasks/persistent/getting-started/define-work)). In Doze, network access is suspended and jobs do not run until a maintenance window, and "over time, the system schedules maintenance windows less frequently" ([Doze and App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)). The escape hatch has its own cap: "The system permits an app's `dataSync` services to run for a total of **6 hours in a 24-hour period**, after which the system calls the running service's `Service.onTimeout(int, int)` method", and failing to stop yields `RemoteServiceException: "A foreground service of type dataSync did not stop within its timeout"` ([Android 15 behaviour changes](https://developer.android.com/about/versions/15/behavior-changes-15)). Asking the user to exempt us from Doze is store-policy-restricted: "**Google Play policies prohibit apps from requesting direct exemption from Power Management features** — Doze and App Standby — in Android 6.0 and above unless the core function of the app is adversely affected" ([Doze docs](https://developer.android.com/training/monitoring-device-state/doze-standby#exemption-cases)). **[inference]:** a spaced-repetition app syncing a few kilobytes has no case for the exemption; 15-minute periodic work on unmetered network is what we get.

12. **Rented WebDAV is cheap and the auth story is settled, but nobody documents conditional requests.** Real prices observed 2026-07-30: 1 TB for **EUR 3.20/month** on a storage box exposing "FTP, FTPS, SFTP, SCP, Samba/CIFS, BorgBackup, Restic, Rclone, rsync via SSH, HTTPS, WebDAV" ([Hetzner](https://www.hetzner.com/storage/storage-box/)); 100 GB for **EUR 2/month** and 10 GB for **EUR 0.50/month**, yearly billing only ([Koofr](https://koofr.eu/pricing/)); 10 GB for **€1,99/month** billed annually on a hosted Nextcloud ([The Good Cloud](https://thegood.cloud/consumers/)); 3 TB for **€4,99/month** annual ([Infomaniak kDrive](https://www.infomaniak.com/en/ksuite/kdrive/prices)). App-specific passwords are the norm and sometimes mandatory: "You will not be able to set up a WebDAV connection, without using an application-specific password" ([Koofr](https://koofr.eu/help/koofr_with_webdav/which-password-to-use-when-connecting-via-webdav/)); "If you use two-factor authentication for your account, device-specific passwords are the only way to configure clients" ([Nextcloud](https://docs.nextcloud.com/server/latest/user_manual/en/session_management.html)). **`If-Match` and `If-None-Match` appear nowhere** in Nextcloud's WebDAV developer manual, which documents six custom upload headers and says only "Any existing file will be overwritten by the request" ([Nextcloud dev manual](https://docs.nextcloud.com/server/latest/developer_manual/client_apis/WebDAV/basic.html)); no other provider in the survey mentions ETag semantics either.

13. **The design does not need conditional writes — the ticket's hypothesis holds.** Every log file is owned by exactly one writer, so no two devices ever `PUT` the same URL; a lost update requires two writers racing on one resource, which cannot occur. **[inference]**, but it follows directly from ADR-0004 §2. Finding 5 therefore prices a hazard we can decline to be exposed to, provided the mutable surface of ADR-0004 §7 is also published per writer rather than as one shared document. The one place conditional requests still earn their keep is the *read* side: `If-None-Match` on GET yields 304 and saves a download, and that is a pure optimisation whose failure mode is a wasted transfer, not corruption.

---

## 1. What Android lets one app share with another

This is the load-bearing question for the whole synced-folder family: **can a third-party sync app sync a directory that our app can also read and write as ordinary files?**

### 1.1 The two places our app can write without asking

Android's storage overview divides the world into app-specific storage, shared storage, and media, and states the visibility rule directly. On app-specific storage: "Other apps cannot access files stored within internal storage", "The system prevents other apps from accessing these locations", and no permission is needed — "Your app doesn't require any system permissions to read and write to files in these directories" ([app-specific storage](https://developer.android.com/training/data-storage/app-specific)). Since Android 10 this is the default posture for external storage too: "Apps that target Android 10 (API level 29) and higher are given scoped access into external storage, or *scoped storage*, by default… Such apps have access only to the app-specific directory on external storage, as well as specific types of media that the app has created" ([data storage overview](https://developer.android.com/training/data-storage)).

So `getFilesDir()` and `getExternalFilesDir()` are both writable by us with plain `std::fs`, and both are **invisible to any sync application**. `getExternalFilesDir()` resolves under `/sdcard/Android/data/<package>`, and Android 11 sealed that directory shut from every direction at once:

- All-files access does not open it — the permission grants "Write access to all internal storage directories **except** `/Android/data/`, `/sdcard/Android`, and most subdirectories of `/sdcard/Android`" ([Manage all files](https://developer.android.com/training/data-storage/manage-all-files)).
- The document picker does not open it — "on Android 11 (API level 30) and higher, you cannot use `ACTION_OPEN_DOCUMENT_TREE` to request individual file selection from: The `Android/data/` directory and all subdirectories. The `Android/obb/` directory and all subdirectories." ([SAF docs](https://developer.android.com/training/data-storage/shared/documents-files); the same restriction is stated for both intent actions on the [Android 11 storage page](https://developer.android.com/about/versions/11/privacy/storage)).

### 1.2 The one directory that is both ours and reachable

`Context.getExternalMediaDirs()` returns `/sdcard/Android/media/<package>`, and it is the sole exception. Android's own reference is unusually candid about why:

> "Returns absolute paths to application-specific directories on all shared/external storage devices where the application can place media files. **These files are scanned and made available to other apps through `MediaStore`.** … **There is no security enforced with these files.** For example, any application holding `Manifest.permission.WRITE_EXTERNAL_STORAGE` can write to these files."
> — [`Context.getExternalMediaDirs()`](https://developer.android.com/reference/android/content/Context#getExternalMediaDirs())

The all-files-access page confirms it counts as shared storage: "Read and write access to all files within shared storage. (Note: The `/sdcard/Android/media` directory is part of shared storage.)" ([Manage all files](https://developer.android.com/training/data-storage/manage-all-files)). And Android 11's restriction list names `Android/data/` and `Android/obb/` — **not** `Android/media/`.

Two independent applications have converged on this directory for exactly this reason. The self-hostable client stores downloads there and tags the location `PUBLIC` — its source comments the resolved path as `/storage/emulated/0/Android/media/com.nextcloud.client/nextcloud/admin@example.cloud/folder/file.txt` ([`FileStorageUtils.java`](https://github.com/nextcloud/android/blob/master/app/src/main/java/com/owncloud/android/utils/FileStorageUtils.java), [`DataStorageProvider.java`](https://github.com/nextcloud/android/blob/master/app/src/main/java/com/owncloud/android/datastorage/DataStorageProvider.java)). The peer-to-peer synchroniser's community documentation reports the same constraint from the other side: under scoped storage its folders live under `Android/media/<its own package>`.

The catch is stated in the same reference paragraph: **"This method was deprecated in API level 30"**, with the migration advice being to insert into a `MediaStore` collection instead — which does not help, because a compressed JSON Lines segment is not media and `READ_MEDIA_*` permissions do not cover it.

### 1.3 The alternative: a user-picked SAF tree

`ACTION_OPEN_DOCUMENT_TREE` "allows users to grant your app access to an entire directory tree", after which "your app can then access any file in the selected directory and any of its sub-directories" ([SAF docs](https://developer.android.com/training/data-storage/shared/documents-files)). Three costs, all documented:

**It is a URI, not a path.** Reads and writes go through `ContentResolver.openInputStream` / `openOutputStream` / `openFileDescriptor(uri, "w")`, and creation through `DocumentsContract.createDocument`. Google's own framing: "The app gains read and write access to a URI that represents the user's chosen location or document." A Rust or C dependency that wants `open(2)` on a filename gets nothing; `openFileDescriptor` yields a `ParcelFileDescriptor` whose raw fd *can* be handed to `std::fs::File::from_raw_fd`, but every open, create, rename and list is a JNI call first. **[inference]**

**Listing is a cursor, and the convenience wrapper is quadratic in round trips.** AOSP's `TreeDocumentFile.listFiles()` queries the children URI for **only** `COLUMN_DOCUMENT_ID`, and every subsequent `getName()`, `length()` and `lastModified()` is its own separate `resolver.query` ([`TreeDocumentFile.java`](https://android.googlesource.com/platform/frameworks/support/+/refs/heads/androidx-main/documentfile/documentfile/src/main/java/androidx/documentfile/provider/TreeDocumentFile.java), [`DocumentsContractApi19.java`](https://android.googlesource.com/platform/frameworks/support/+/refs/heads/androidx-main/documentfile/documentfile/src/main/java/androidx/documentfile/provider/DocumentsContractApi19.java)) [tested — source read, §5.7]. Listing N files with sizes through that wrapper is **1 + 3N** IPC round trips; querying `buildChildDocumentsUriUsingTree` directly with all three columns is **one** cursor. Android warns about the naive path in the same document: "If you iterate through a large number of files within the directory that's accessed using `ACTION_OPEN_DOCUMENT_TREE`, your app's performance might be reduced."

**It survives reboot only if you take it, and not if the folder moves.** "By default, URI permissions last only until device restart"; `takePersistableUriPermission()` fixes that, but "Even after calling `takePersistableUriPermission()`, your app doesn't retain access to the URI if the associated document is moved or deleted. In those cases, you need to ask permission again to regain access to the URI." ([SAF docs](https://developer.android.com/training/data-storage/shared/documents-files)). **[inference]:** a sync application that recreates its folder — restore, re-pair, storage change — silently revokes our access and needs a fresh picker interaction.

Two directories are also excluded outright: "On Android 11 (API level 30) and higher, you cannot use `ACTION_OPEN_DOCUMENT_TREE` to request access to: The root directory of the internal storage volume. The root directory of each SD card volume… The `Download` directory."

**And the picker itself needs plumbing this stack does not have.** Granting a tree means starting an activity for a result and receiving `onActivityResult` — and the Rust Android glue does not surface it. The crate that provides the entry point supports "NativeActivity or GameActivity" and can be extended to other base classes ([rust-mobile/android-activity](https://github.com/rust-mobile/android-activity)); the request that opened that line of work states the gap directly: "I'm writing a Rust library that interacts with Bluetooth on Android where I need to have my own subclass of `Activity` for being able to use `Activity::startIntentSenderForResult` and override `onActivityResult`" ([rust-mobile/ndk#266](https://github.com/rust-mobile/ndk/issues/266)). This lines up with AGENTS.md rule 8, which records that the windowing backend "handles only motion and key events". **[inference]:** a SAF tree therefore costs a hand-written `Activity` subclass in Java or Kotlin inside the APK, on a stack whose whole appeal (ADR-0003) is one crate and one binary per platform. The `getExternalMediaDirs()` route needs no picker and therefore no subclass — which is a large part of why it is the interesting one.

### 1.4 All-files access is not open to us

`MANAGE_EXTERNAL_STORAGE` would restore ordinary file I/O over shared storage, but the store gates it: "To limit broad access to shared storage, the Google Play store has updated its policy to evaluate apps that target Android 11 (API level 30) or higher and request all-files access… This policy is in effect as of May 2021", and "Apps that fail to meet policy requirements or do not submit a Permissions Declaration Form may be removed from Google Play" ([Manage all files](https://developer.android.com/training/data-storage/manage-all-files), [Play policy](https://support.google.com/googleplay/android-developer/answer/10467955)). The eligible list is file managers, backup/restore, anti-virus, document management, on-device search, encryption and device migration. A spaced-repetition app is none of those.

### 1.5 The answer

**Yes, but only in one shape.** Our app writes to its own `getExternalMediaDirs()` path with ordinary file I/O and no permission at all, and a sync application holding all-files access reads and writes it — that combination is documented from both ends. Every other shape costs a SAF tree: a picker interaction per device, a persisted URI permission that can be lost, `content://` plumbing through JNI, and one cursor query rather than `readdir`. And the deprecated status of `getExternalMediaDirs()` means the good shape is the one Android is steering away from.

---

## 2. What the sync applications themselves guarantee

### 2.1 Peer-to-peer, no server — and no Android client from upstream

The peer-to-peer synchroniser is architecturally the best fit on paper: it moves files between devices with no intermediary, which is exactly the "no server of our own" constraint. Its own FAQ states the delivery model and, in doing so, the limitation: it "does not upload your data to the cloud but exchanges your data across your machines **as soon as they are online at the same time**" ([FAQ](https://docs.syncthing.net/users/faq.html)).

Upstream's Android client is gone: repository "archived by the owner on Dec 3, 2024. It is now read-only", with "This app is discontinued. The last release on Github and F-Droid will happen with the December 2024 Syncthing version" ([syncthing/syncthing-android](https://github.com/syncthing/syncthing-android)). The maintainer's stated reason names the app store: "a combination of Google making Play publishing something between hard and impossible and no active maintenance… The app saw no significant development for a long time and without Play releases I do no longer see enough benefit" ([forum announcement, 2024-10-20](https://forum.syncthing.net/t/discontinuing-syncthing-android/23002)).

The community fork is maintained and distributes via "the 'releases' section or F-Droid" ([Catfriend1/syncthing-android](https://github.com/Catfriend1/syncthing-android)); its Play listing returns **HTTP 404** [tested, §5.6]. Its manifest declares `MANAGE_EXTERNAL_STORAGE`, `REQUEST_IGNORE_BATTERY_OPTIMIZATIONS`, `FOREGROUND_SERVICE_SPECIAL_USE` and `android:foregroundServiceType="specialUse"` [tested, §5.6] — i.e. it depends on precisely the two things Google Play restricts. `specialUse` itself is reviewed: developers "specify the `<property>` element within the `<service>` element. These values and corresponding use cases are reviewed when you submit your app in the Google Play Console" ([foreground service types](https://developer.android.com/develop/background-work/services/fgs/service-types)).

### 2.2 A self-hostable server-backed client: two-way, but not for arbitrary folders

The Android client of the self-hostable file-sync server does register a storage provider — `DocumentsStorageProvider` with `android:permission="android.permission.MANAGE_DOCUMENTS"` and a `DOCUMENTS_PROVIDER` intent filter, implementing `queryChildDocuments`, `openDocument`, `createDocument`, `renameDocument`, `deleteDocument` ([AndroidManifest.xml](https://github.com/nextcloud/android/blob/master/app/src/main/AndroidManifest.xml), [`DocumentsStorageProvider.java`](https://github.com/nextcloud/android/blob/master/app/src/main/java/com/owncloud/android/providers/DocumentsStorageProvider.java)). It also declares `MANAGE_EXTERNAL_STORAGE` and `requestLegacyExternalStorage="true"`.

But what it syncs is narrower than "a folder":

- **Auto upload is one-way and media-shaped.** The store description offers "Auto Upload for photos and videos taken by your device" ([full_description.txt](https://github.com/nextcloud/android/blob/master/src/generic/fastlane/metadata/en-US/full_description.txt)), and the trigger watches `MediaStore.Images`, `MediaStore.Video` and `MediaStore.Files` content URIs ([`BackgroundJobManagerImpl.kt`](https://github.com/nextcloud/android/blob/master/app/src/main/java/com/nextcloud/client/jobs/BackgroundJobManagerImpl.kt)).
- **Two-way sync exists, but only for files already marked available offline, and only when the server side moved first.** `OfflineSyncWork.doWork()` calls `checkETagChanged(folderName, …) ?: return` — if the remote collection ETag is unchanged the folder is skipped entirely, before any local file is examined ([`OfflineSyncWork.kt`](https://github.com/nextcloud/android/blob/master/app/src/main/java/com/nextcloud/client/jobs/OfflineSyncWork.kt)). The per-file operation underneath *is* bidirectional and does upload local changes ([`SynchronizeFileOperation.kt`](https://github.com/nextcloud/android/blob/master/app/src/main/java/com/owncloud/android/operations/SynchronizeFileOperation.kt)). **[inference]:** a purely local append by our app, with nothing having changed on the server, is not noticed until some unrelated change bumps that collection's ETag.
- **Cadence is 15 minutes, unmetered only, and off under power saving.** `DEFAULT_PERIODIC_JOB_INTERVAL_MINUTES = 15L`, the offline-sync request sets `NetworkType.UNMETERED`, and the worker no-ops when power saving is on ([`BackgroundJobManagerImpl.kt`](https://github.com/nextcloud/android/blob/master/app/src/main/java/com/nextcloud/client/jobs/BackgroundJobManagerImpl.kt)).

No first-party documentation states this client's scoped-storage position; the evidence above is source code, and a repository search for "scoped storage" returns only lint suppressions.

### 2.3 Command-line sync tooling is not a background service

The general-purpose cloud-storage CLI has Android binaries but disowns them: "See also Android builds… These are built as part of the official release, but haven't been adopted as first class builds yet" ([downloads.md](https://github.com/rclone/rclone/blob/master/docs/content/downloads.md)). Its install guide, FAQ and `mount` documentation contain no Android section at all. Two-way sync is a separate command carrying an explicit warning — "`bisync` is considered an **advanced command**, so use with care… or data loss can result" — and is scheduled externally: "On Linux or Mac, consider setting up a crontab entry… On Windows this can be done using a *Task Scheduler*" ([bisync docs](https://rclone.org/bisync/)). There is no watch or daemon mode.

### 2.4 Change detection and latency

The peer-to-peer synchroniser detects changes two ways: "By regular full scans and by notifications received from the filesystem ('watcher'). By default the watcher is enabled and full scans are done once per hour" ([Understanding Synchronization](https://docs.syncthing.net/users/syncing.html)). The configuration defaults are `fsWatcherEnabled: true`, `fsWatcherDelayS: 10` — "The duration during which changes detected are accumulated, before a scan is scheduled" — and `rescanIntervalS: 3600` ([Configuration](https://docs.syncthing.net/users/config.html)). So **write-to-scan is ~10 s with the watcher, 1 hour without**, plus transfer, plus the requirement that the peer is awake. If the two devices cannot reach each other directly, traffic goes through a relay and "the transfer rate is much lower than a direct connection would allow" ([Relaying](https://docs.syncthing.net/users/relaying.html)).

Against that, WebDAV's latency is whatever polling interval we choose — floored at 15 minutes on Android by WorkManager (§4.6), and immediate on desktop.

### 2.5 Conflict semantics you inherit

Because two devices never write the same file (ADR-0004 §2), content conflicts are not the interesting failure. What remains:

**Partial files are handled well.** "Syncthing never writes directly to a destination file. Instead all changes are made to a temporary copy which is then moved in place over the old version. If an error occurs during the copying or syncing… the temporary file is kept around for up to a day" ([Understanding Synchronization](https://docs.syncthing.net/users/syncing.html)). Temporaries are named `.syncthing.<name>.tmp` on Unix. **[inference]:** a reader therefore never sees a half-written segment, but must ignore `.syncthing.*.tmp` when listing. This dovetails with ADR-0004 §11's rule that "a malformed *final* line is discarded silently", which covers the local crash case rather than the transport case.

**Conflicts still get invented, and they replicate.** If the same path does somehow diverge, "the file with the older modification time will be marked as the conflicting file and thus be renamed" to `<filename>.sync-conflict-<date>-<time>-<modifiedBy>.<ext>`, ties broken by device ID; and those files "are treated as normal files after they are created", so the conflict copy itself syncs everywhere ([Conflicting Changes](https://docs.syncthing.net/users/syncing.html#conflicting-changes)). **[inference]:** with per-writer files this should never fire, but if it does, the reader must either parse or explicitly skip `*.sync-conflict-*` — and since rows are addressed by `(writer, sequence)` and merge by set union, parsing them is harmless and skipping them loses data.

**Deletion propagates by default.** "Versioning… defaults to 'no file versioning', i.e. no old copies of files are kept" ([File Versioning](https://docs.syncthing.net/users/versioning.html)). A deletion anywhere is a deletion everywhere. For a log that is never compacted (ADR-0004 §10), the store has no protection we do not configure ourselves.

**Filename constraints.** The synchroniser defaults `caseSensitiveFS: false`, meaning it runs "extra safety checks for case insensitive filesystems" by default ([Configuration](https://docs.syncthing.net/users/config.html)), and symlinks are unusable on Android — "even though Syncthing may try to synchronise symbolic links on Android, this will not succeed, as the OS does not support them on the user storage" ([FAQ](https://docs.syncthing.net/users/faq.html)). **[inference]:** ADR-0004's writer ids are hex, so names of the form `<writer-hex>-<seq>.jsonl.zst` are lowercase ASCII and dodge every case-folding and reserved-name hazard on Windows, macOS and Android alike. This is a constraint to keep, not a problem to solve.

### 2.6 The handshake

There is none. §Summary-4 above: a synced folder presents the local replica, and no API exposes "what does the remote hold". A device can compute its own `{writer id → highest sequence}` from what it can see, but "am I behind?" is unanswerable in this family — the only signal is that new bytes eventually appear. On Android through a SAF tree, even enumerating the local replica is a `ContentResolver` cursor rather than `readdir`, though a single query over `buildChildDocumentsUriUsingTree` with `COLUMN_DISPLAY_NAME`, `COLUMN_SIZE` and `COLUMN_LAST_MODIFIED` is one round trip, not N (§1.3).

### 2.7 Setup burden, honestly

Per device, before a single row moves, with the peer-to-peer synchroniser: install F-Droid or sideload an APK (not on Play, §2.1); grant all-files access through the special-access settings screen; grant the battery-optimisation exemption its manifest requests; pair the device with each other device by exchanging device IDs; create and share the folder; and on our side, either accept `getExternalMediaDirs()` (deprecated) or run the SAF picker once and persist the URI permission. That is roughly six user actions per Android device and four per desktop, none of which we can perform or verify programmatically. **[inference]**, but each step is sourced above.

---

## 3. WebDAV

### 3.1 The protocol facts

**Listing.** "All DAV-compliant resources MUST support the PROPFIND method"; "A client MUST submit a Depth header with a value of '0', '1', or 'infinity'… Servers MUST support '0' and '1' depth requests… and SHOULD support 'infinity'. In practice, support for infinite-depth requests MAY be disabled, due to the performance and security concerns" — and a server "MAY reject PROPFIND requests on collections with depth header of 'Infinity'" with 403 and a `propfind-finite-depth` precondition (RFC 4918 §9.1, §9.1.1). The response is `207 Multi-Status`, one `<response>` element per member URL, and an empty request body "MUST be treated as if it were an 'allprop' request" — which the RFC itself warns against: "Note that 'allprop' does not return values for all live properties. WebDAV servers increasingly have expensively-calculated or lengthy properties… Instead, WebDAV clients can use propname requests to discover what live properties exist, and request named properties when retrieving values."

**The properties we care about are conditionally required.** `getcontentlength` "MUST be defined on any DAV-compliant resource that returns the Content-Length header in response to a GET" and is computed and therefore protected (§15.4). `getetag` "MUST be defined on any DAV-compliant resource that returns the Etag header" and "MUST be protected because this value is created and controlled by the server" (§15.6). `getlastmodified` likewise (§15.7) — with the RFC's own warning that it is the weaker tool: "since [RFC2616] requires clients to use ETags where provided, a server implementing ETags can count on clients using a much better mechanism than modification dates for offline synchronization or cache control". Note the shape of the requirement: each is a MUST *conditional on the server emitting the corresponding HTTP header*. A server that emits no `ETag` is not required to expose `getetag`.

**PUT.** "A PUT performed on an existing resource replaces the GET response entity of the resource", and "A PUT that would result in the creation of a resource without an appropriately scoped parent collection MUST fail with a 409 (Conflict)" (RFC 4918 §9.7.1) — so a client must `MKCOL` the directory first. PUT to a collection is undefined and "MAY be treated as an error (405 Method Not Allowed)" (§9.7.2).

### 3.2 Conditional writes: available, mandatory to honour, unreliable in practice

**Available and specified.** `If-Match` "makes the request method conditional on the recipient origin server… having a current representation of the target resource that has an entity tag matching a member of the list", and "An origin server MUST use the strong comparison function when comparing entity tags for If-Match… since the client intends this precondition to prevent the method from being applied if there have been any changes" (RFC 9110 §13.1.1). `If-None-Match: *` is the create-if-absent primitive: it "can also be used with a value of `*` to prevent an unsafe request method (e.g., PUT) from inadvertently modifying an existing representation of the target resource when the client believes that the resource does not have a current representation" (§13.1.2).

**Mandatory to honour, when sent.** "When an origin server receives a request that selects a representation and that request includes an If-Match header field, the origin server MUST evaluate the If-Match condition per Section 13.2 prior to performing the method", and "An origin server that evaluates an If-Match condition MUST NOT perform the requested method if the condition evaluates to false. Instead, the origin server MAY indicate that the conditional request failed by responding with a 412 (Precondition Failed) status code." For `If-None-Match` the failure response is not even optional: the server "MUST respond with either a) the 304 (Not Modified) status code if the request method is GET or HEAD or b) the 412 (Precondition Failed) status code for all other request methods" (§13.1.2).

**But the validator that makes it work is only a SHOULD.** "An origin server **SHOULD** send an ETag for any selected representation for which detection of changes can be reasonably and consistently determined" (RFC 9110 §8.8.3), and "A sender MAY send the ETag field in a trailer section". WebDAV adds a preference, not a requirement: "Strong ETags are much more useful for authoring use cases than weak ETags… Note also that weak ETags have certain restrictions in HTTP, e.g., these cannot be used in If-Match headers", and "a WebDAV server **SHOULD NOT** change the ETag (or the Last-Modified time) for a resource that has an unchanged body and location" (RFC 4918 §8.6). **So the chain is: MAY generate → SHOULD send → MUST honour if sent. A server with no ETags is fully conformant and offers no conditional write at all.**

**WebDAV's own `If` header is a weaker instrument than `If-Match`.** It "is intended to have similar functionality to the If-Match header… However, the If header handles any state token as well as ETags", and "If this header is evaluated and all state lists fail, then the request MUST fail with a 412 (Precondition Failed) status" (RFC 4918 §10.4, §10.4.1). But its matching rule is looser: "Matching entity tag: Where the entity tag matches an entity tag associated with the identified resource. **Servers MUST use either the weak or the strong comparison function**" (§10.4.4) — the client cannot tell which it got, where `If-Match` mandates strong comparison.

**Two of three servers ignored it entirely.** [tested, §5.2–5.4] Against `rclone serve webdav` v1.74.4 and `dufs` 0.46.0, both `If-Match: "0000"` on a resource whose real tag was different, and `If-None-Match: *` on a resource that existed, returned **201 Created** and **replaced the file's contents**. Nextcloud 34.0.2 returned **412** for both and left the file untouched, then accepted the same PUT with the correct tag (204). Nothing in the response of the two non-conforming servers distinguishes "precondition passed" from "precondition ignored": both are a 2xx.

**And even where it works, the tag you read may not be the tag you can write with.** A reproduced failure against a Nextcloud instance behind a compressing reverse proxy: a `GET` with gzip returns a weak tag `W/"…"`, a `PUT` with `If-Match: W/"…"` returns 412 forever, and the same PUT with `W/` stripped returns 204 — because "weak ETags… cannot be used in If-Match headers" (RFC 4918 §8.6) and compression at the proxy is enough to weaken them ([koreader#15707](https://github.com/koreader/koreader/issues/15707)). I did **not** reproduce this on my own instance — bare Apache did not compress the file body and the tag stayed strong [tested, §5.5] — so treat it as a hazard that depends on the host's proxy, which no host documents.

### 3.3 Partial and range writes: reading yes, appending no

**Appending via `Content-Range` on PUT is unsafe by specification.** RFC 9110 §14.5 "Partial PUT":

> "Some origin servers support PUT of a partial representation when the user agent sends a Content-Range header field… though such support is inconsistent and depends on private agreements with user agents… An origin server SHOULD respond with a 400 (Bad Request) status code if it receives Content-Range on a PUT for a target resource that does not support partial PUT requests. **Partial PUT is not backwards compatible with the original definition of PUT. It may result in the content being written as a complete replacement for the current representation.**"

That last sentence is the whole finding: the failure mode of guessing wrong is not an error, it is the file being replaced by the fragment. Both lightweight servers did exactly that — 201 Created, file left containing only the appended bytes [tested, §5.4]. Nextcloud returned 400 and preserved the file [tested]. RFC 9110 offers the only safe alternatives in the same section: "targeting a separately identified resource with state that overlaps or extends a portion of the larger resource" — i.e. **segment files** — "or by using a different method that has been specifically defined for partial updates (for example, the PATCH method defined in [RFC5789])".

**The PATCH extension exists but is not a standard and is not portable.** The `Sabre\DAV\PartialUpdate\Plugin` implements RFC 5789 with `Content-Type: application/x-sabredav-partialupdate` and `X-Update-Range` taking `bytes=start-end`, `bytes=start-`, `bytes=-N` or `append`, advertising `sabredav-partialupdate` in the `DAV` response header ([sabre/dav](https://sabre.io/dav/http-patch/)). Tested: `dufs` implements it (**204**, bytes appended); `rclone serve webdav` returns **400**; Nextcloud — which is *built on* that library — returns **500 Internal Server Error** [tested, §5.4]. It is not advertised in any of the three servers' `DAV:` headers.

**Range reads work everywhere I tested.** `Range: bytes=5-` returned `206 Partial Content` with `Content-Range: bytes 5-14/15` on `rclone serve webdav`, on `dufs`, and on Nextcloud [tested]. So a per-writer file can be published as a whole-file `PUT` and consumed incrementally by byte offset. **[inference]:** combined with `getcontentlength` from PROPFIND, a reader knows exactly how many new bytes exist and fetches only those — the append-only property is preserved on the read path even though the write path must rewrite.

### 3.4 What that forces on segment sizing — measured

Since appending is unavailable, each publish either rewrites the writer's whole file or starts a new segment. I generated 73,000 rows in ADR-0004 §11's exact JSON shape (three writer ids, 4,000 note UUIDs, realistic timestamps) and compressed at several segment sizes with `zstd -19` [tested, §5.8]:

| rows/segment | segments/year | compressed total | ratio | bytes/row | avg segment |
|---|---|---|---|---|---|
| 200 (≈ one day) | 365 | 2,199,983 B | 5.02× | 30.1 | 6.0 KB |
| 1,000 | 73 | 2,047,073 B | 5.40× | 28.0 | 28 KB |
| 5,000 | 15 | 1,667,363 B | 6.63× | 22.8 | 111 KB |
| 73,000 (one file/year) | 1 | 920,123 B | 12.01× | 12.6 | 920 KB |

Raw was 11,049,129 B, i.e. **151.4 bytes/row** — ADR-0004 §11's "roughly 150 bytes" is accurate. The ADR's "compresses about ten to one" holds only for large blocks: **daily segmentation cuts the ratio from 12.0× to 5.0×, costing 2.4× the bytes.** Caveat on realism: my generator draws note UUIDs uniformly from 4,000 notes, so the identifier column carries near-maximal entropy; a real deck with skewed review distribution should compress somewhat better at every size, and the *ratio between* sizes is the transferable number, not the absolutes. **[inference]**

Per writer per decade, extrapolating: **~9.2 MB in one rewritten-whole file, ~22 MB in 3,650 daily segments.** Three writers over a decade at daily segmentation is **~11,000 files, ~66 MB** — comfortably inside every quota in §3.6, but see §3.5 for what 11,000 files costs to list. The other side of the ledger: publishing a whole-file rewrite in year ten uploads 9.2 MB to add 6 KB of new rows, on a mobile connection, every time. **[inference]**

### 3.5 The handshake, priced

Measured against Nextcloud 34.0.2 with 100 entries in a collection, and against `rclone serve webdav` with 1,000 [tested, §5.9]:

| Request | Nextcloud 34.0.2 | rclone serve webdav 1.74.4 |
|---|---|---|
| `PROPFIND Depth: 0`, `getetag` on the collection | **371 B**, 1 round trip | n/a — collection `getetag` returned 404 Not Found |
| `PROPFIND Depth: 1`, named `getetag`+`getcontentlength` | 28,577 B / 100 = **286 B/entry** | 245,280 B / 1000 = **245 B/entry** |
| …the same, `Accept-Encoding: gzip` | 2,794 B = **28 B/entry** (10.2× smaller) | **no compression offered** — 245,280 B on the wire |
| `PROPFIND Depth: 1`, `allprop` | 42,485 B = **425 B/entry** | 787,570 B = **788 B/entry** |
| `PROPFIND Depth: infinity` | not attempted | served, 1000 entries |

Three consequences:

- **The cheap handshake is real but server-specific.** One `PROPFIND Depth: 0` for the collection's `getetag` cost 371 bytes and changed both when a file was added and when an existing file was modified [tested]. That answers "has *anything* changed?" in one request. The lightweight Go server exposed no collection ETag at all (`getetag` came back inside a `404 Not Found` propstat for the collection [tested, §5.1]) — so this handshake is a property of the server, not of WebDAV.
- **The full listing scales with segment count, and gzip decides whether that hurts.** 11,000 daily segments after a decade (§3.4) cost **3.1 MB uncompressed / 308 KB gzipped** on the Nextcloud-shaped server, versus **2.7 MB with no compression available** on the Go server. Using `allprop` instead of named properties inflates that by 1.5–3.2×.
- **The incremental listing that would solve this is not available.** RFC 6578's `sync-collection` REPORT, which returns only what changed since a token, was refused: `415 Unsupported Media Type`, "The {DAV:}sync-collection REPORT is not supported on this url." [tested, §5.9]. It is implemented for calendars and contacts, not for files.

**[inference]:** the two extremes price out very differently. Few large files means a 371-byte handshake, a `Depth: 1` listing of a handful of entries, `Range` reads for the tail, and 9.2 MB re-uploaded per publish. Many small segments means a 308 KB listing per handshake at decade scale and 6 KB uploads. Nothing in the protocol forces the choice; the numbers above are what it costs either way.

### 3.6 Who sells it, and what it costs

All prices observed **2026-07-30**; currency and billing period as published.

| Provider | Price | Storage | WebDAV | App-specific credentials |
|---|---|---|---|---|
| [Hetzner Storage Box BX11](https://www.hetzner.com/storage/storage-box/) | **EUR 3.20 / USD 4.00 per month** | 1 TB | included, must be switched on | 100 sub-accounts, each with its own credentials and directory — no token mechanism |
| [Koofr](https://koofr.eu/pricing/) | **EUR 0.50/mo** (10 GB), **EUR 2/mo** (100 GB), EUR 10/mo (1 TB) — "Currently only yearly subscriptions are possible" | 10 GB free tier | included | **mandatory** |
| [The Good Cloud](https://thegood.cloud/consumers/) (hosted Nextcloud) | **€1,99/mo** billed annually (€23,88/yr) | 10 GB | core feature | app passwords |
| [Hetzner Storage Share NX11](https://www.hetzner.com/storage/storage-share/) (managed Nextcloud) | **EUR 4.29 / USD 5.00 per month** | 1 TB | core feature | app passwords |
| [Infomaniak kDrive Solo](https://www.infomaniak.com/en/ksuite/kdrive/prices) | **€4,99/mo** annual (€5,54 monthly) | 3 TB, "cannot be increased" | included on paid tiers, "UNAVAILABLE" on free | none documented |
| [Fastmail](https://www.fastmail.com/help/files/davnftp.html) | bundled with mail | small | `https://webdav.fastmail.com/` | "an app password with WebDAV permissions" |
| [pCloud](https://help.pcloud.com/article/webdav) | paid plans only | — | still offered | **none** — account password, plus email confirmation under 2FA |
| Box | — | — | **removed** — WebDAV reached end of life on 2023-04-28 ([Announcing end of life for Box WebDAV support](https://support.box.com/hc/en-us/articles/360052806073-Announcing-end-of-life-for-Box-WebDAV-support); the article body is script-rendered, so I verified the title and a 200 response, not the body text) | — |

Two further notes. The storage-box vendor lists its protocols as "FTP, FTPS, SFTP, SCP, Samba/CIFS, BorgBackup, Restic, Rclone, rsync via SSH, HTTPS, WebDAV", documents "Up to 10 simultaneous connections per Storage Box account" and "Up to 100 sub-accounts", and warns "The WebDAV protocol does not support the output of the available disk space" ([Hetzner docs](https://docs.hetzner.com/storage/storage-box/)); **what software serves that WebDAV is not stated anywhere first-party**, so its ETag behaviour is unknown until probed. One provider actively discourages the use case: its own FAQ frames WebDAV as for "specific and occasional use cases", disclaims third-party client compatibility, and states it does not support the protocol ([Infomaniak](https://www.infomaniak.com/en/support/faq/2409/connect-to-kdrive-via-webdav)).

The provider list maintained by the self-hostable server's vendor no longer includes any of the hosts named above; it now lists Office EU, NL Hosting, Personal Phoenix, FOSSTech Projects, DKM Ecosystem, IONOS CLOUD, hosting.de, Leviia, Replicant IT and Kazteleport, none with a published consumer price on that page ([providers list](https://nextcloud.com/providers/)).

**No provider in this survey documents ETag or conditional-request behaviour.** That absence was searched for, not stumbled into: the self-hostable server's own WebDAV developer manual mentions ETag once, as a PROPFIND property, enumerates six custom upload headers (`X-OC-MTime`, `X-OC-CTime`, `OC-Checksum`, `X-Hash`, `OC-Total-Length`, `X-NC-WebDAV-AutoMkcol`), and says of PUT only "Any existing file will be overwritten by the request" — `If-Match` and `If-None-Match` appear nowhere in it ([dev manual](https://docs.nextcloud.com/server/latest/developer_manual/client_apis/WebDAV/basic.html)).

### 3.7 Auth from the client's side

What a native app must store is a username and a long-lived secret, sent as HTTP Basic over TLS; app-specific passwords are the norm and in two cases mandatory:

> "Each time you set up a new connection via WebDAV protocol … you need to generate a new application-specific password… **You will not be able to set up a WebDAV connection, without using an application-specific password.**"
> — [Koofr](https://koofr.eu/help/koofr_with_webdav/which-password-to-use-when-connecting-via-webdav/)

> "If you use two-factor authentication for your account, device-specific passwords are the only way to configure clients." / "The server will then deny connections from clients using your login password."
> — [Nextcloud session management](https://docs.nextcloud.com/server/latest/user_manual/en/session_management.html)

> "you should use an application password for login rather than your regular password. In addition to improved security, this increases performance significantly."
> — [Nextcloud WebDAV access](https://docs.nextcloud.com/server/latest/user_manual/en/files/access_webdav.html)

**On expiry:** there is no token refresh flow here — these are passwords, not OAuth tokens. They do not expire on a timer; they stop working when the user revokes the device entry, at which point the server returns 401 and the only remedy is a fresh secret entered by hand. **[inference]**, from the absence of any refresh mechanism in the cited documents. The digest-auth path exists in the client library (`digest_auth` is a direct dependency of `reqwest_dav` [tested, §5.10]) but no surveyed provider documents requiring it.

No provider in the survey publishes a per-request rate limit. One publishes connection ceilings instead — "Up to 10 simultaneous connections per Storage Box account" ([Hetzner docs](https://docs.hetzner.com/storage/storage-box/)) — and one publishes a daily bandwidth figure of 1000 GB/day/user, framed as advisory ([Infomaniak](https://www.infomaniak.com/en/support/faq/2387/manage-kdrive-storage)). A login lockout returning HTTP 429 after repeated wrong passwords was reported in one provider's help centre but **I could not re-locate the page to verify it — treat as unsourced**. **[inference]:** whether or not that specific lockout exists, an unattended client that retries a rejected credential in a loop is the obvious way to get an account blocked, so a 401 must stop the loop rather than back off.

### 3.8 Rust client support

| Crate | Latest | Published | 90-day downloads | Licence | Verdict |
|---|---|---|---|---|---|
| [`reqwest_dav`](https://crates.io/crates/reqwest_dav) | 0.3.3 | 2026-03-02 | 254,184 | MIT OR Apache-2.0 | the only maintained option |
| [`rustydav`](https://crates.io/crates/rustydav) | 0.1.3 | 2021-10-09 | 1,688 | **GPL-3.0** | unmaintained, licence unusable for a shipped binary |
| [`webdavc`](https://crates.io/crates/webdavc) | 0.1.1 | 2022-12-31 | 12 | **GPL-3.0** | dead |
| [`hyperdav`](https://crates.io/crates/hyperdav) | 0.2.0 | 2018-08-24 | 18 | MIT | dead |
| [`dav-server`](https://crates.io/crates/dav-server) | 0.11.0 | 2026-02-21 | 120,938 | Apache-2.0 | **server**, not a client |
| [`opendal`](https://crates.io/crates/opendal) | 0.57.0 | 2026-06-01 | 3,253,921 | Apache-2.0 | multi-backend abstraction with a WebDAV service; heavier |

Registry data queried from the crates.io API on 2026-07-30.

**Cross-compilation [tested, §5.10]:** `reqwest_dav` 0.3.3 with `reqwest` 0.13 (`default-features = false`, `rustls-tls`) passed `cargo ndk -t arm64-v8a check` against NDK 29.0.13846066 on rustc 1.97.0, pulling in `aws-lc-rs`, `rustls` 0.23, `rustls-platform-verifier` and `jni` 0.22.4. A plain `cargo check --target aarch64-linux-android` **fails** without the NDK on `PATH` — `error occurred in cc-rs: failed to find tool "aarch64-linux-android-clang"` — because the TLS backend compiles C.

**What the crate does and does not give you** (source read at `reqwest_dav-0.3.3/src/lib.rs`):

- `list_raw(path, depth)` hard-codes the request body to `<D:propfind xmlns:D="DAV:"><D:allprop/></D:propfind>` — the 425 B/entry form of §3.5, with no parameter for naming properties.
- Typed methods are `get`, `put`, `delete`, `mkcol`, `mv`, `cp`, `list`, `unzip`. **No conditional-header parameter, no `Range`, no `PATCH`.**
- `start_request(method, path)` returns a `reqwest::RequestBuilder` with auth already applied, so `If-Match`, `Range` and a named-property PROPFIND are all reachable — as hand-written requests.

**[inference]:** this is close enough to "a plain HTTP client job" that the crate's value is the PROPFIND XML parsing (via `serde-xml-rs` 0.6) and the digest-auth handling, not the WebDAV abstraction.

**Android runtime gotcha:** the TLS verifier this pulls in is not self-sufficient on Android. "In order for the crate to call into the JVM, it needs handles from Android" — `rustls_platform_verifier::android::init_hosted()` must be called with a `JNIEnv` and a `Context` "before any networking has a chance to run" ([docs.rs](https://docs.rs/rustls-platform-verifier/latest/rustls_platform_verifier/)). Under `android-activity` that means reaching the JVM through `ndk-context` at startup. Note that `ndk-context` 0.1.1 has not been published since 2022-04-19 despite 13.1 M downloads in 90 days [crates.io, 2026-07-30].

### 3.9 Unattended on Android

**Periodic work has a 15-minute floor and no timing guarantee.** "The minimum repeat interval that can be defined is 15 minutes (same as the JobScheduler API)"; "The interval period is defined as the minimum time between repetitions. The exact time that the worker is going to be executed depends on the constraints that you are using in your WorkRequest object and on the optimizations performed by the system"; and with constraints, a run "could be delayed, or even skipped if the conditions are not met within the run interval" ([WorkManager](https://developer.android.com/develop/background-work/background-tasks/persistent/getting-started/define-work)).

**Doze suspends the network outright.** While in Doze the system suspends network access, ignores wake locks, defers standard alarms, and does not run sync adapters or JobScheduler jobs (which includes WorkManager); pending work runs in maintenance windows, and "over time, the system schedules maintenance windows less frequently". Under App Standby, an idle app gets network access "about once a day" ([Doze and App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby)). The exemptions are foreground services and high-priority push messages — and push requires a server, which we do not have.

**The foreground-service escape hatch is capped and store-reviewed.** A network sync is the `dataSync` type, requiring `FOREGROUND_SERVICE_DATA_SYNC` ([service types](https://developer.android.com/develop/background-work/services/fgs/service-types)), and on Android 15: "The system permits an app's `dataSync` services to run for a total of 6 hours in a 24-hour period, after which the system calls the running service's `Service.onTimeout(int, int)` method… If the service does not call `Service.stopSelf()`, the system throws an internal exception… `RemoteServiceException: "A foreground service of type dataSync did not stop within its timeout"`". Also: "Apps that target Android 15 or higher are not allowed to launch a data sync foreground service from a `BOOT_COMPLETED` broadcast receiver" — so there is no sync-on-boot. Google's stated alternatives are expedited WorkManager work (under ~10 minutes), a user-initiated data transfer job (`JobScheduler.setUserInitiated`, requiring a visible progress notification), or `shortService` (~3 minutes) ([data transfer options](https://developer.android.com/develop/background-work/background-tasks/data-transfer-options)).

**Asking the user to opt out of Doze is not open to us.** "Google Play policies prohibit apps from requesting direct exemption from Power Management features — Doze and App Standby — in Android 6.0 and above unless the core function of the app is adversely affected", with acceptable cases limited to messaging/calling, safety, task automation and peripheral-companion apps ([Doze docs](https://developer.android.com/training/monitoring-device-state/doze-standby#exemption-cases)).

**[inference]:** the realistic unattended envelope on Android is a 15-minute periodic job with a network constraint, degrading to roughly once a day for a phone left untouched, with a foreground service reserved for a sync the user explicitly started. That is adequate for a spaced-repetition log at 200 rows/day, but it means "unattended" is measured in hours, not seconds.

---

## 4. The two families side by side

| | Folder synced by another app | WebDAV against rented storage |
|---|---|---|
| Server of ours | none | none |
| Android: can our app use ordinary file I/O? | only in `getExternalMediaDirs()` (deprecated at API 30); otherwise SAF `content://` URIs | n/a — it is HTTP |
| Android: is the peer software installable from the store? | no for the peer-to-peer synchroniser (Play listing 404, upstream archived 2024-12-03); the drive clients do not sync folders at all | n/a |
| "Am I behind?" | **unanswerable** — only the local replica is visible | 1 request, 371 B (collection `getetag`) where the server exposes one; otherwise a full listing at 28–788 B/entry |
| Append to remote | n/a — local append, then it propagates | **impossible portably**; `Content-Range` PUT may silently truncate |
| Read only new bytes | n/a | yes — `Range` → 206 on all three servers tested |
| Conditional write | n/a | mandated by RFC 9110 §13.1.1; **silently ignored by 2 of 3 servers tested** |
| Write→visible latency | ~10 s watcher + transfer, **but both devices must be awake together**; 1 h if the watcher is off | our polling interval; ≥15 min on Android, immediate on desktop |
| Partial-file exposure | none — temp file then atomic rename | none for whole-file PUT [inference] |
| Deletion | propagates by default (versioning defaults to off) | ours alone to issue |
| Cost to the user | free software, ≈6 setup actions per Android device | EUR 0.50–4.29/month, one account, one app password per device |
| Devices must overlap in time | **yes** | no |

---

## 5. What I tested, and how to reproduce it

Environment: Linux 7.1.5 x86_64, cargo/rustc 1.97.0, `rclone` v1.74.4, `dufs` 0.46.0, Nextcloud 34.0.2 in Docker (`nextcloud:latest`, SQLite), NDK 29.0.13846066, `zstd` from PATH. All servers were local; no third-party account was touched.

**5.1 PROPFIND shape.** `rclone serve webdav /tmp/davroot --etag-hash MD5`; `PROPFIND Depth: 1` with named props returned `207` and, per file, `<D:getcontentlength>18</D:getcontentlength>`, `<D:getlastmodified>`, `<D:getetag>"1946340162867fa47344419ace58597b"</D:getetag>`. For the **collection itself**, `getcontentlength` and `getetag` came back inside a `HTTP/1.1 404 Not Found` propstat — no collection ETag.

**5.2 Conditional PUT, lightweight Go server.** Against a file created via PUT whose real tag was `"e06d0d302a7ccf3af6bc8199ee2c1d3c"`:
```
PUT If-Match: "0000000000000000"  -> 201   file content became "CLOBBER"
PUT If-None-Match: *  (existing)  -> 201   file content became "CLOBBER"
```

**5.3 Conditional PUT, Nextcloud 34.0.2** (`/remote.php/dav/files/admin/`, Basic auth):
```
PUT If-Match: "0000"              -> 412   content unchanged ("row1")
PUT If-None-Match: * (existing)   -> 412   content unchanged
PUT If-Match: <correct etag>      -> 204   content updated
OPTIONS -> DAV: 1, 3, extended-mkcol, access-control, …, 2
```

**5.4 Partial write, all three servers.** `dufs` — `PUT Content-Range: bytes 8-12/13` → **201**, file replaced by the 5-byte fragment; `PATCH` with `X-Update-Range: append` → **204**, bytes appended. `rclone serve webdav` — `PUT Content-Range: bytes 18-35/36` → **201**, 18-byte file containing only the appended row; `PATCH … append` → **400**. Nextcloud — `PUT Content-Range` → **400**, content preserved; `PATCH … append` → **500 Internal Server Error**. `Range: bytes=5-` GET → **206** with `Content-Range: bytes 5-14/15` on all three.

**5.5 Weak-ETag hazard, not reproduced here.** `GET` and `HEAD` with and without `Accept-Encoding: gzip` against bare Nextcloud+Apache all returned the same strong tag `"89b93fd1784f2c3d577a36f25579ad02"` — no `W/` prefix, no `Content-Encoding`. The failure reported in [koreader#15707](https://github.com/koreader/koreader/issues/15707) requires a compressing layer in front; my instance had none.

**5.6 Peer-to-peer synchroniser fork, distribution and permissions.** `curl` of `play.google.com/store/apps/details?id=com.github.catfriend1.syncthingfork` → **HTTP 404**. Its `app/src/main/AndroidManifest.xml` declares `WRITE_EXTERNAL_STORAGE`, `RECEIVE_BOOT_COMPLETED`, `REQUEST_IGNORE_BATTERY_OPTIMIZATIONS`, `FOREGROUND_SERVICE`, `FOREGROUND_SERVICE_SPECIAL_USE`, `MANAGE_EXTERNAL_STORAGE`, and a service with `android:foregroundServiceType="specialUse"`.

**5.7 SAF listing cost, from AOSP source.** `TreeDocumentFile.listFiles()` issues one `resolver.query(childrenUri, new String[] { COLUMN_DOCUMENT_ID }, …)`; `getName()`, `lastModified()` and `length()` each route to `queryForString`/`queryForLong`, which issue `resolver.query(self, new String[]{column}, …)` — one query per field per file.

**5.8 Segment compression.** `python3` generator producing 73,000 rows in ADR-0004 §11's shape, piped to `zstd -19`; results in §3.4. Raw 11,049,129 B = 151.4 B/row.

**5.9 Handshake cost.** 100 files created over WebDAV on Nextcloud, 1,000 on the Go server; `curl -w '%{size_download}'` on each PROPFIND form; numbers in §3.5. Collection `getetag` changed from `"6a6b53270b404"` → `"6a6b5333a17ee"` on adding a file and → `"6a6b5333e96fd"` on modifying an existing one. `REPORT` with a `sync-collection` body → `415`, `Sabre\DAV\Exception\ReportNotSupported`.

**5.10 Android cross-compile.** `cargo ndk -t arm64-v8a --manifest-path /tmp/davcheck/Cargo.toml check` with `ANDROID_NDK_HOME=…/ndk/29.0.13846066` → `Finished dev profile … in 7.34s`, having checked `reqwest_dav v0.3.3`, `reqwest v0.13.4`, `rustls v0.23.43`, `rustls-platform-verifier v0.7.0`, `aws-lc-rs v1.17.3`, `jni v0.22.4`. Without the NDK: `error occurred in cc-rs: failed to find tool "aarch64-linux-android-clang"`.

---

## Confidence

**High** — Android's storage visibility rules and the `Android/data` / `Android/media` split (first-party reference documentation, corroborated by two independent applications' source); the peer-to-peer Android client's discontinuation and the fork's distribution channel (archived repo, maintainer's own announcement, HTTP 404 from the store); the commercial drive clients not syncing folders on Android (explicit first-party statements, strongest from Microsoft); RFC text on PROPFIND, PUT, `If-Match`/`If-None-Match`, partial PUT, and the `If` header; the Android background-execution limits and their numbers; the measured PROPFIND sizes, compression ratios and cross-compile result — all reproduced locally with commands recorded above.

**Medium** — that conditional writes are broadly unreliable across hosted providers. I tested three server implementations and two ignored preconditions, but those two are lightweight file servers, not what a paid host runs; the one production-grade server behaved correctly. The right reading is "not safe to assume", not "usually broken". Also medium: the prices in §3.6, which I did not verify against a checkout flow and which carry no first-party VAT statement in the storage-box case; and the claim that a SAF tree can be granted over `Android/media/<pkg>` — the exclusion list names only `Android/data` and `Android/obb`, but I could not test on a device and OEM behaviour varies.

**Low / untested** — the weak-ETag-under-compression failure (§3.2, §5.5): reported and reproduced by a third party, not by me. SAF throughput for real file I/O through a content URI on a real handset: I read the API surface and the AOSP listing code but ran nothing on the Pixel; per AGENTS.md rule 9, this needs the real handset before anything is designed on it. What software serves the storage-box WebDAV endpoint, and therefore its ETag semantics: no first-party statement exists.

---

## Sources

**Android platform (first-party documentation and AOSP source)**

- [Data and file storage overview](https://developer.android.com/training/data-storage) — storage categories, scoped storage at API 29, cross-app visibility table
- [App-specific storage](https://developer.android.com/training/data-storage/app-specific) — "Other apps cannot access files stored within internal storage"; uninstall behaviour
- [Access documents and other files (SAF)](https://developer.android.com/training/data-storage/shared/documents-files) — `ACTION_OPEN_DOCUMENT_TREE`, `takePersistableUriPermission`, excluded directories, the large-directory performance caution
- [Manage all files on a storage device](https://developer.android.com/training/data-storage/manage-all-files) — what `MANAGE_EXTERNAL_STORAGE` grants and denies; the May 2021 Play policy
- [Google Play — Permissions and APIs that Access Sensitive Information](https://support.google.com/googleplay/android-developer/answer/10467955) — all-files-access eligibility and removal from the store
- [Storage updates in Android 11](https://developer.android.com/about/versions/11/privacy/storage) — direct file paths for media; `Android/data` and `Android/obb` exclusion
- [Access media files from shared storage](https://developer.android.com/training/data-storage/shared/media) — direct file paths only for own/attributed files; "random reads and writes… up to twice as slow"
- [`Context.getExternalMediaDirs()`](https://developer.android.com/reference/android/content/Context#getExternalMediaDirs()) — "no security enforced"; deprecated at API 30
- [`TreeDocumentFile.java`](https://android.googlesource.com/platform/frameworks/support/+/refs/heads/androidx-main/documentfile/documentfile/src/main/java/androidx/documentfile/provider/TreeDocumentFile.java) · [`DocumentsContractApi19.java`](https://android.googlesource.com/platform/frameworks/support/+/refs/heads/androidx-main/documentfile/documentfile/src/main/java/androidx/documentfile/provider/DocumentsContractApi19.java) — one query per field per file
- [Doze and App Standby](https://developer.android.com/training/monitoring-device-state/doze-standby) — network suspension, maintenance windows, the Play prohibition on requesting exemption
- [Define your work requests (WorkManager)](https://developer.android.com/develop/background-work/background-tasks/persistent/getting-started/define-work) — 15-minute floor; no timing guarantee
- [Foreground service types](https://developer.android.com/develop/background-work/services/fgs/service-types) — `dataSync`, `specialUse` and its Play Console review
- [Behavior changes: Android 15](https://developer.android.com/about/versions/15/behavior-changes-15) — 6-hour `dataSync` cap, `onTimeout`, no FGS from `BOOT_COMPLETED`
- [Alternatives to data transfer foreground services](https://developer.android.com/develop/background-work/background-tasks/data-transfer-options)

**Peer-to-peer file synchronisation (first-party)**

- [syncthing/syncthing-android](https://github.com/syncthing/syncthing-android) — archived 2024-12-03, discontinuation notice
- [Discontinuing syncthing-android](https://forum.syncthing.net/t/discontinuing-syncthing-android/23002) — maintainer's stated reasons, 2024-10-20
- [Catfriend1/syncthing-android](https://github.com/Catfriend1/syncthing-android) — fork, distribution channels, `AndroidManifest.xml`
- [Understanding Synchronization](https://docs.syncthing.net/users/syncing.html) — watcher vs hourly scan, temp-file-then-rename, conflict naming and tie-break
- [FAQ](https://docs.syncthing.net/users/faq.html) — "online at the same time"; Android symlink limitation
- [Configuration](https://docs.syncthing.net/users/config.html) — `fsWatcherDelayS: 10`, `rescanIntervalS: 3600`, `caseSensitiveFS: false`
- [File Versioning](https://docs.syncthing.net/users/versioning.html) — defaults to none
- [Relaying](https://docs.syncthing.net/users/relaying.html) — relayed transfer rate

**Server-backed sync clients (first-party source and help centres)**

- [nextcloud/android `AndroidManifest.xml`](https://github.com/nextcloud/android/blob/master/app/src/main/AndroidManifest.xml) · [`DocumentsStorageProvider.java`](https://github.com/nextcloud/android/blob/master/app/src/main/java/com/owncloud/android/providers/DocumentsStorageProvider.java) · [`FileStorageUtils.java`](https://github.com/nextcloud/android/blob/master/app/src/main/java/com/owncloud/android/utils/FileStorageUtils.java) · [`DataStorageProvider.java`](https://github.com/nextcloud/android/blob/master/app/src/main/java/com/owncloud/android/datastorage/DataStorageProvider.java) · [`OfflineSyncWork.kt`](https://github.com/nextcloud/android/blob/master/app/src/main/java/com/nextcloud/client/jobs/OfflineSyncWork.kt) · [`SynchronizeFileOperation.kt`](https://github.com/nextcloud/android/blob/master/app/src/main/java/com/owncloud/android/operations/SynchronizeFileOperation.kt) · [`BackgroundJobManagerImpl.kt`](https://github.com/nextcloud/android/blob/master/app/src/main/java/com/nextcloud/client/jobs/BackgroundJobManagerImpl.kt)
- [Dropbox — Access files offline](https://help.dropbox.com/sync/access-files-offline) · [Sync overview](https://help.dropbox.com/sync/sync-overview) · [Export files](https://help.dropbox.com/installs/export-files-mobile)
- [Google — Drive for desktop system requirements](https://support.google.com/drive/answer/2375082) · [Use Drive offline](https://support.google.com/drive/answer/2375012?co=GENIE.Platform%3DAndroid)
- [Microsoft — Use OneDrive on Android](https://support.microsoft.com/en-US/onedrive/use-onedrive-on-android) · [Automatically save photos and videos](https://support.microsoft.com/en-us/office/automatically-save-photos-and-videos-with-onedrive-for-android-66605e54-48b8-4f55-bcff-34159702e344)
- [rclone downloads.md](https://github.com/rclone/rclone/blob/master/docs/content/downloads.md) · [bisync](https://rclone.org/bisync/) · [README](https://github.com/rclone/rclone/blob/master/README.md)

**Standards**

- [RFC 4918 — HTTP Extensions for WebDAV](https://www.rfc-editor.org/rfc/rfc4918.html) — §8.6 ETag, §9.1 PROPFIND, §9.7 PUT, §10.4 the `If` header, §15.4/15.6/15.7 live properties
- [RFC 9110 — HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html) — §8.8 validators, §9.3.4 PUT, §13.1.1/13.1.2 preconditions, §14.5 Partial PUT
- [RFC 5789 — PATCH Method for HTTP](https://www.rfc-editor.org/rfc/rfc5789.html) · [sabre/dav HTTP PATCH support](https://sabre.io/dav/http-patch/) — the `X-Update-Range: append` extension
- RFC 6578 (`sync-collection` REPORT) — tested absent on the files endpoint, §5.9

**Hosted storage (first-party pricing and docs, observed 2026-07-30)**

- [Hetzner Storage Box](https://www.hetzner.com/storage/storage-box/) · [Storage Box docs](https://docs.hetzner.com/storage/storage-box/) · [Storage Share](https://www.hetzner.com/storage/storage-share/)
- [Koofr pricing](https://koofr.eu/pricing/) · [Koofr WebDAV help](https://koofr.eu/help/koofr_with_webdav/which-password-to-use-when-connecting-via-webdav/)
- [The Good Cloud](https://thegood.cloud/consumers/) · [Nextcloud providers list](https://nextcloud.com/providers/)
- [Infomaniak kDrive pricing](https://www.infomaniak.com/en/ksuite/kdrive/prices) · [kDrive WebDAV FAQ](https://www.infomaniak.com/en/support/faq/2409/connect-to-kdrive-via-webdav)
- [Nextcloud — Accessing files via WebDAV](https://docs.nextcloud.com/server/latest/user_manual/en/files/access_webdav.html) · [Session management](https://docs.nextcloud.com/server/latest/user_manual/en/session_management.html) · [WebDAV developer manual](https://docs.nextcloud.com/server/latest/developer_manual/client_apis/WebDAV/basic.html)
- [pCloud WebDAV](https://help.pcloud.com/article/webdav) · [Fastmail WebDAV](https://www.fastmail.com/help/files/davnftp.html) · [Box WebDAV end of life](https://support.box.com/hc/en-us/articles/360043696414)

**Rust ecosystem**

- crates.io API (`/api/v1/crates/{name}`), queried 2026-07-30, for `reqwest_dav`, `rustydav`, `webdavc`, `hyperdav`, `dav-server`, `opendal`, `jni`, `ndk-context`, `android-activity`
- [`reqwest_dav`](https://github.com/niuhuan/reqwest_dav) — source read at 0.3.3 (`src/lib.rs`)
- [`rustls-platform-verifier`](https://docs.rs/rustls-platform-verifier/latest/rustls_platform_verifier/) — Android `init_hosted()` requirement
- [rust-mobile/ndk#266](https://github.com/rust-mobile/ndk/issues/266) — needing an `Activity` subclass for `onActivityResult`, which the SAF picker requires

**Third-party reports (not primary; flagged in place)**

- [koreader#15707](https://github.com/koreader/koreader/issues/15707) — weak ETag under gzip producing an infinite 412 loop against WebDAV
