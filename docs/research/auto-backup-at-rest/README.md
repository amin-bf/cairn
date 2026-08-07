# The platform's automatic app-data backup: what the provider holds, and when

**Research ticket:** [#60](https://github.com/amin-bf/cairn/issues/60) (under wayfinder map #1) · **Date of research:** 2026-07-31
**Question:** [ADR-0016 §7](../../adr/0016-backup-and-restore.md) leaves Android's automatic app-data backup switched on, so a user who never enables sync still has `collection.db` uploaded to the same company that [ADR-0013 §3](../../adr/0013-the-sync-transport.md) publishes the log to. [#58](https://github.com/amin-bf/cairn/issues/58) treats that path as *covered by the platform*, on the belief that the payload is encrypted on the device under a key the receiving company cannot obtain. **Is that belief true, for whom, and where does it fail?**

This is a **research** note. It gathers facts and sharpens trade-offs; it decides nothing. Every non-obvious claim carries an inline primary source — platform documentation, the platform's own published source, or a first-party security paper — and where the wording matters the source is quoted rather than paraphrased. Claims I reasoned rather than sourced are marked **[inference]**. Places where the documentation says nothing are marked **SILENT** and are not filled in by guessing; §8 collects them.

**The mechanism under examination, stated so the note stands alone.** Android runs a system service that periodically copies an app's private files off the device and uploads them to storage held by the operating-system vendor, under the account signed in on the phone. No app code is involved and no user action enrols: an app that targets API level 23 or higher participates by default, and switching it off is a manifest attribute (`android:allowBackup="false"`). The upload is described as *"backs up a user's data from apps that target and run on Android 6.0 (API level 23) or higher. Android preserves app data by uploading it to the user's Google Drive, where it's protected by the user's Google Account credentials. The backup is end-to-end encrypted on devices running Android 9 or higher using the device's PIN, pattern, or password"* ([Back up user data with Auto Backup](https://developer.android.com/identity/data/autobackup)). That single sentence contains both halves of the answer below: an unconditional upload, and a conditional guarantee.

---

## Summary of findings

1. **There are two encryption layers, and only the outer one is unconditional.** The vendor's own security guidance separates them explicitly: *"The Standard Android Backup system always encrypts backup data in transit and at rest. This encryption is applied regardless of the Android version in use and of whether your device has a lock screen. Starting from Android 9, if the device has a lock screen set, then the backup data is not only encrypted, but encrypted with a key not known to Google (the lock screen secret protects the encryption key, thus enabling end-to-end encryption)"* ([Security recommendations for backups](https://developer.android.com/privacy-and-security/risks/backup-best-practices), last updated 2024-10-25). So *"encrypted at rest"* is always true and answers a different question than the one #58 needs. **Confidence: high.**

2. **When the conditional layer does apply, the key protection is a hardware-enforced guessing limit on the lock-screen secret, not a passphrase the user chose.** *"The backup is encrypted with a randomly generated key that is further encrypted with a hash of the user's lockscreen PIN, pattern, or password. This encrypted key is securely shared with a cohort of secure enclaves located across Google's data centers… With this secure enclave, there is a limited number of incorrect attempts strictly enforced by the custom firmware. By design, this means that no one (including Google) can access a user's backed-up application data without specifically knowing their PIN, pattern, or password"* ([Android Security Paper 2024](https://services.google.com/fh/files/misc/android-security-paper-2024.pdf), *Backup encryption*, p. 27). The enclave is a custom chip whose firmware *"maintain[s] a strictly incrementing per-Vault counter of failed attempts"* and cannot be patched without wiping its keys ([Google Cloud Key Vault Service whitepaper](https://developer.android.com/about/versions/pie/security/ckv-whitepaper), version date 2018-03-06). **Confidence: high** that this is what is claimed; **medium** that it holds in practice — §1.3, the component doing the encrypting is not open source.

3. **With no lock screen, the payload is uploaded anyway and the receiving company holds a key to it.** This is the sourced consequence of finding 1, not an inference: encryption *"is applied regardless… of whether your device has a lock screen"*, and only *"if the device has a lock screen set"* is it *"encrypted with a key not known to Google"* (same source). The user-facing help says the same from the other side: *"Your backups are uploaded to Google and encrypted with your Google Account password. For some data, your phone's screen lock PIN, pattern, or password is also used to encrypt your data"* ([Back up your device](https://support.google.com/googleone/answer/9149304)). A swipe or trusted-context unlock does not count: *"To help protect your backed-up data, use a PIN, pattern, or password screen lock, instead of a swipe or Smart Lock"* ([Back up or restore data on your Android device](https://support.google.com/android/answer/2819582)). **Confidence: high.**

4. **The platform models client-side encryption as a per-operation flag an app may test — which is itself the strongest structural evidence that it is not a universal guarantee.** *"Transport flag indicating that the transport has client-side encryption enabled. i.e., the user's backup has been encrypted with a key known only to the device, and not to the remote storage solution. Even if an attacker had root access to the remote storage provider they should not be able to decrypt the user's backup data"* — `FLAG_CLIENT_SIDE_ENCRYPTION_ENABLED`, *"Added in API level 28"*, [reference](https://developer.android.com/reference/android/app/backup/BackupAgent) and verbatim in the platform source ([`BackupAgent.java`](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/core/java/android/app/backup/BackupAgent.java), lines 168–177). An app can also *refuse* to be backed up without it, declaratively — §2.2. **Confidence: high.**

5. **This project's floor is below the version where the guarantee begins, so a supported device can be structurally incapable of it.** `min_sdk_version = 24` (`crates/app/Cargo.toml`) admits Android 7.0 through 8.1 handsets, where the client-side-encryption flag does not exist — it arrived at API level 28 (finding 4) — while automatic backup itself runs from API level 23. **On an API 24–27 device there is no configuration, and no user action, that makes the payload unreadable to the receiving company.** The only levers are excluding the data or switching backup off. **Confidence: high.**

6. **Being on Android 9 or higher is not sufficient either: the enrolment must have happened there.** *"If you've upgraded your development devices to Android 9, you need to disable and then re-enable data backup after upgrading. This is because Android only encrypts backups with a client-side secret after informing users in Settings or the setup wizard"* ([Auto Backup](https://developer.android.com/identity/data/autobackup)). A phone that turned backup on under Android 8 and later upgraded therefore keeps a backup the vendor can read, silently, with a modern OS version on the box. **Confidence: medium-high** — stated for development devices, and the documentation does not say whether production upgrades are re-prompted (**SILENT**).

7. **The declarative fix exists, and on this project's target it is currently unreachable through the toolchain.** An app can require the guarantee — `requireFlags="clientSideEncryption"` for devices on Android 11 or lower, renamed to `disableIfNoEncryptionCapabilities="true"` inside `<cloud-backup>` for apps targeting Android 12 or higher ([Auto Backup](https://developer.android.com/identity/data/autobackup); [Security recommendations](https://developer.android.com/privacy-and-security/risks/backup-best-practices): *"we recommend requiring end-to-end encryption which means allowing backups only on Android 9 or higher and only when the lock screen is set"*). The newer form needs the `android:dataExtractionRules` manifest attribute, and the manifest generator behind this repo's Gradle-free packaging exposes `allow_backup` and `full_backup_content` but **has no field for it** ([`android_manifest::Application`](https://docs.rs/android-manifest/latest/android_manifest/struct.Application.html), 58 fields, checked 2026-07-31). With `target_sdk_version = 36`, the older file governs only *"devices running Android 11 or lower"*. **Confidence: medium** — the struct is definitive for that crate, but I did not attempt a build. §3.3.

8. **Exceeding the 25 MB quota fails closed and silently: no upload, no user-visible signal, and the only documented channels are a Java callback we cannot host and a log line.** *"If the amount of data is over 25 MB, the system calls `onQuotaExceeded()` and doesn't back up data to the cloud. The system periodically checks whether the amount of data later falls under the 25 MB threshold and continues Auto Backup when it does"* ([Auto Backup](https://developer.android.com/identity/data/autobackup)). The observable trace is two log lines — *"`I/PFTBT: Transport rejected backup of <PACKAGE>, skipping`"* or *"`I/PFTBT: Transport quota exceeded for package: <PACKAGE>`"* ([Test backup and restore](https://developer.android.com/identity/data/testingbackup)). No notification to the user is documented (**SILENT**). Interaction with encryption: **none is documented** — the quota is enforced by the same transport that reports the encryption flag, but no source states whether the 25 MB is measured before or after compression and encryption (**SILENT**). **Confidence: high** on the behaviour, **not established** on the interaction.

9. **Deletion is coarser than the equivalent control for the sync folder, and there is no documented per-app path.** One first-party page gives both routes side by side: *"Find & delete unneeded device backups: Go to the Backups section of your storage"* versus *"Delete hidden app data: Go to Google Drive settings. Click Manage apps"* ([Clean up & troubleshoot your Google storage](https://support.google.com/drive/answer/6374270)). The second is per application — the removal path already recorded at [`../sync-transport/object-stores-and-drives.md`](../sync-transport/object-stores-and-drives.md) §6. The first is per *device backup*, an all-apps unit. The other documented routes are equally coarse or automatic: *"If you turn off backup on your device, your backups are deleted"*, *"To delete your backup, you can also use your Google Account dashboard"*, and *"If you don't use your device for 57 days, the data you backed up (except photos or videos) is also erased"* ([Back up your device](https://support.google.com/googleone/answer/9149304)). **Confidence: high** for what exists; **the absence of a per-app control is absence of evidence** — see §5.3.

10. **Uninstalling the app does not remove the backup — the opposite of the sync folder's behaviour.** The vendor's own regression script uninstalls the package and reinstalls it *precisely* to prove the cloud copy comes back: `pm uninstall --user 0 "$1"` followed by `adb install-multiple`, with the instruction *"validate that it works correctly, with all data retained"* ([Test backup and restore](https://developer.android.com/identity/data/testingbackup)). The hidden sync folder is documented as the reverse — it *"is deleted when a user uninstalls your app"* ([application data folder](https://developers.google.com/workspace/drive/api/guides/appdata), quoted in the sibling note §6). **So the more private-looking of the two surfaces is the one that survives uninstall.** **Confidence: high** [inference on the direction of the test's intent, high on the commands].

**The bottom line #58 asked for.** For a user on Android 9 or higher who set a PIN, pattern or password *and* enabled backup on that version, the receiving company holds ciphertext it states it cannot decrypt. For a user with no lock screen — or on any handset between this project's API 24 floor and API 27 — it holds the payload under keys it manages, which for threat-modelling purposes is plaintext: readable on request, by subpoena, or by an insider, with encryption-at-rest protecting only against someone who steals the disks. **Both populations get there without an enrolment moment, because nobody opted in.**

---

## 1. Is the payload encrypted under a key the provider cannot access? (Q1)

### 1.1 Two layers, and the sentence that separates them

The claim that matters is not *"is it encrypted"* — it always is — but *"who holds the key"*. The vendor's security-recommendations page is the only source found that states both in one breath, which is why it is quoted here in full rather than summarised:

> "The Standard Android Backup system always encrypts backup data in transit and at rest. This encryption is applied regardless of the Android version in use and of whether your device has a lock screen. Starting from Android 9, if the device has a lock screen set, then the backup data is not only encrypted, but encrypted with a key not known to Google (the lock screen secret protects the encryption key, thus enabling end-to-end encryption)."
>
> — [Security recommendations for backups](https://developer.android.com/privacy-and-security/risks/backup-best-practices)

Read as a decision table:

| Device state | Uploaded? | Encrypted at rest? | Key holder |
|---|---|---|---|
| Android 9+, lock screen set, backup enabled on 9+ | yes | yes | the device, via the user's lock-screen secret |
| Android 9+, no lock screen | yes | yes | the storage operator |
| Android 6–8.1, any lock screen | yes | yes | the storage operator |
| Backup switched off, or app opts out | **no** | — | — |

The rest of §1 is about the second column of the first row; §2 and §3 are about the rows under it.

### 1.2 What protects the key, when there is one

The design has a name — a cloud key vault — and a published construction. Three sources agree, at increasing detail.

**The user-visible claim** ([Android Security Paper 2024](https://services.google.com/fh/files/misc/android-security-paper-2024.pdf), §*Backup encryption*, p. 27):

> "Devices that run Android 9 and higher support end-to-end encrypted backup, a capability whereby the backup data is encrypted on the device using a device and user specific key. The backup server has no ability to decrypt the backup archive.
>
> The backup is encrypted with a randomly generated key that is further encrypted with a hash of the user's lockscreen PIN, pattern, or password. This encrypted key is securely shared with a cohort of secure enclaves located across Google's data centers. None of the data shared with the secure enclave is known to Google, and the device verifies the identity of the secure enclave by checking its root of trust.
>
> With this secure enclave, there is a limited number of incorrect attempts strictly enforced by the custom firmware. By design, this means that no one (including Google) can access a user's backed-up application data without specifically knowing their PIN, pattern, or password."

**Why a four-digit PIN is claimed to be enough.** The whole scheme exists because the secret a human will actually remember has too little entropy to resist offline guessing, so the guessing is made online and finite:

> "One approach to bridge the gap between the requirements for cryptographic secrets and human memorable secrets is to use a Cloud Key Vault (CKV) service to store a high entropy 'recovery key', protected by a low entropy human memorable secret. The CKV service will release the recovery key only to a party that proves knowledge of the correct human memorable secret. Brute force attacks against the human memorable secret can be thwarted by the CKV service, which will enforce an absolute limit on the number of failed attempts to prove knowledge of the secret."
>
> "For the first time, the Cloud Key Vault enables lock screen protection for Android backups stored in the Cloud as well. This means that Google's servers have no ability to access or restore the contents of the encrypted backups – only a device with the user's LSKF can decrypt the backups."
>
> — [Google Cloud Key Vault Service](https://developer.android.com/about/versions/pie/security/ckv-whitepaper), version date 2018-03-06

**The enforcement is hardware, and deliberately un-patchable.** The counter lives inside a custom chip, and updating the firmware destroys the keys rather than preserving them across a change of logic:

> "they can generate and securely share a key pair with other members of their Cohort such that the firmware logic protects the private key from leaking outside of the Titan chips in the Cohort. They can also perform the Vault Opening operation, and maintain a strictly incrementing per-Vault counter of failed attempts (where the counter is backed by state stored inside the Titan chip)."
>
> "we also alter the Titan boot loader to ensure that the chip's stored data (such as the private key for the Cohort) is completely wiped before any update is applied. The downside of this protection is that we cannot patch bugs in the firmware without experiencing some data loss"
>
> — same whitepaper

A second, software rate limit sits in front of it, aimed at a different attacker: *"the CKV service will enforce an increasing time delay after each subsequent failed Vault Opening request"*, to stop an account hijacker burning the hardware counter and locking the real user out (same source).

The 2018 announcement adds the one operational number that the design turns on — that the chip authorises *every* access, so the limit cannot be side-stepped by copying the ciphertext:

> "Starting in Android Pie, devices can take advantage of a new capability where backed-up application data can only be decrypted by a key that is randomly generated at the client. This decryption key is encrypted using the user's lockscreen PIN/pattern/passcode, which isn't known by Google. Then, this passcode-protected key material is encrypted to a Titan security chip on our datacenter floor. The Titan chip is configured to only release the backup decryption key when presented with a correct claim derived from the user's passcode. Because the Titan chip must authorize every access to the decryption key, it can permanently block access after too many incorrect attempts at guessing the user's passcode… By design, this means that no one (including Google) can access a user's backed-up application data without specifically knowing their passcode."
>
> — [Google and Android have your back by protecting your backups](https://security.googleblog.com/2018/10/google-and-android-have-your-back-by.html), 2018-10-12

### 1.3 What this evidence can and cannot establish

Three honest limits, because #58 must record residuals rather than reassurances:

- **The component that performs the encryption is not open source.** The active cloud transport is named in the vendor's own test script as `com.google.android.gms/.backup.BackupTransportService`, and the documentation describes it as *"the active cloud backup transport on most devices, part of Google Mobile Services"* ([Test backup and restore](https://developer.android.com/identity/data/testingbackup)). What *is* in the open platform source is only the interface: the flag by which that transport tells an app whether it encrypted client-side ([`BackupAgent.java`](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/core/java/android/app/backup/BackupAgent.java)). **So the guarantee is attestable from the vendor's statements and from a commissioned audit, not from reading the code** [inference].
- **The audit is stated but its report is not quoted here.** *"the Android Security & Privacy team hired global cyber security and risk mitigation expert NCC Group to complete a security audit… While there were some issues discovered during this audit, engineers corrected them quickly"* (security blog, above). I did not retrieve the report itself; it is third-party and outside this note's source bar, and its findings are 2018-era.
- **The whitepaper is explicitly unfinished.** *"This document is still a work-in-progress, and details of the implementation are still being finalized"*, and the promised detail never arrived in it: *"The detailed protocol specification is still in progress"*. Eight years on, the protocol section is still a placeholder. **Nothing newer supersedes it**; the 2024 security paper's four paragraphs (§1.2) are the current published depth.

**Confidence:** *high* that the mechanism and its claims are as stated; *medium* that the property holds against a determined adversary at the provider, because the claim rests on unauditable firmware plus one eight-year-old external review.

---

## 2. What happens with no lock screen (Q2)

### 2.1 The payload still leaves the device

Nothing about a missing lock screen suppresses the upload. The condition attaches only to *which key* is used — finding 1's quote is unambiguous on both halves, and the consumer-facing help states the default in the vendor's own words:

> "Your backups are uploaded to Google and encrypted with your Google Account password. For some data, your phone's screen lock PIN, pattern, or password is also used to encrypt your data so it can be backed up safely."
>
> — [Back up your device](https://support.google.com/googleone/answer/9149304)

*"Encrypted with your Google Account password"* describes an account-managed key — the operator can produce the data on a password reset, which is the definition of holding the key. The Android-side help says the same in the softer register that makes the boundary visible: *"Your backups are uploaded to Google. **Some** of your data is end to end encrypted with your device's screen lock PIN, pattern, or password"* (emphasis added, [Back up or restore data](https://support.google.com/android/answer/2819582)).

**What counts as a lock screen is narrower than "the phone asks me something".** The same page's guidance is explicit that convenience unlocks do not qualify: *"To help protect your backed-up data, use a PIN, pattern, or password screen lock, instead of a swipe or Smart Lock."* A swipe-to-unlock phone, or one relying on a trusted place or paired device, is in the no-lock-screen row of §1.1's table. **Confidence: medium-high** — this is phrased as a recommendation rather than a specification, but it is consistent with the mechanism (there is no secret to hash in either case).

### 2.2 An app can refuse to be backed up without the guarantee

The platform anticipates exactly this population and offers a declarative opt-out — which is, incidentally, further confirmation that the unguaranteed case is real and common enough to design for.

For an app targeting Android 12 or higher, on devices running Android 12 or higher:

> "Your app can set the `disableIfNoEncryptionCapabilities` flag in the `<cloud-backup>` section to make sure the backup happens only if it can be encrypted, such as when the user has a lock screen. Setting this constraint stops backups from being sent to the cloud if the user's device cannot support encryption, but because D2D transfers aren't sent to the server, they continue to operate even on devices that don't support encryption."
>
> — [Back up user data with Auto Backup](https://developer.android.com/identity/data/autobackup)

For devices running Android 11 or lower, the older per-rule form:

> "`clientSideEncryption`: the user's backup is encrypted with a client-side secret. This form of encryption is enabled on devices running Android 9 or higher as long as the user has enabled backup in Android 9 or higher and has set a screen lock (PIN, pattern, or password) for their device."
>
> — same source, describing `requireFlags="clientSideEncryption"` on an `<include>` element

And the recommendation that ties them together: *"If you can't exclude sensitive data from your backup, then we recommend requiring end-to-end encryption which means allowing backups only on Android 9 or higher and only when the lock screen is set. You can achieve this by using the `requireFlags="clientSideEncryption"` flag, which needs to be renamed to `disableIfNoEncryptionCapabilities` and set to true starting from Android 12"* ([Security recommendations for backups](https://developer.android.com/privacy-and-security/risks/backup-best-practices)).

**The trade this poses is exact, and it is a decision rather than a finding:** requiring the guarantee converts *"a user with no lock screen gets a readable backup"* into *"a user with no lock screen gets no backup"*, on a platform path they never opted into and will never be told about. §3.3 covers whether this repo's packaging can even express the newer flag.

### 2.3 Where the documentation is SILENT on this case

- **Whether the user is told.** No source found states that a device without a lock screen surfaces any indication that its backups are not end-to-end encrypted, in Settings, at setup, or at restore.
- **What happens to an existing end-to-end-encrypted backup when the lock screen is removed.** Whether the next backup is re-encrypted under the account key, whether the old one is deleted, and whether an old dataset can still be restored — none of it is documented.
- **Whether a biometric-only configuration is possible at all** (Android requires a knowledge factor to enrol biometrics, so probably not — but that is inference, not a source).

---

## 3. Version differences, and this project's floor (Q3)

### 3.1 The two version lines, against `min_sdk_version = 24`

| Property | Introduced | Source |
|---|---|---|
| Automatic app-data backup runs at all | **Android 6.0, API 23** (app must target and run on it) | [Auto Backup](https://developer.android.com/identity/data/autobackup); [Test backup and restore](https://developer.android.com/identity/data/testingbackup) repeats it as a test prerequisite |
| Client-side encryption of that backup | **Android 9, API 28** | [Android Security Paper 2024](https://services.google.com/fh/files/misc/android-security-paper-2024.pdf) p. 27; `FLAG_CLIENT_SIDE_ENCRYPTION_ENABLED` *"Added in API level 28"* ([reference](https://developer.android.com/reference/android/app/backup/BackupAgent)) |
| Config surface moves to `dataExtractionRules` | **Android 12, API 31** (for apps targeting it) | [Auto Backup](https://developer.android.com/identity/data/autobackup) |

**This project's floor is `min_sdk_version = 24`, with `target_sdk_version = 36`** (`crates/app/Cargo.toml`). Stated against that floor:

- **Every device we support participates in the backup.** 24 ≥ 23, and participation is automatic. There is no supported handset on which the upload does not happen by default.
- **API levels 24, 25, 26 and 27 — Android 7.0, 7.1, 8.0, 8.1 — cannot have the guarantee.** The mechanism that would provide it did not exist. For those devices the storage operator holds a usable key to `collection.db`, and *no* choice available to the user or to us changes that except excluding the data or disabling backup entirely.
- **The four-version gap is the whole finding for Q3.** Raising the floor to 28 would close it; that is a client-stack decision ([ADR-0003](../../adr/0003-client-stack.md)), not a backup one, and it is recorded here only as the lever that exists.

**SILENT:** no source found states what share of live devices sits below API 28, and this note deliberately does not import a distribution dashboard figure — those move, and the ADR should not depend on one.

### 3.2 The upgrade trap: version alone is not sufficient

An Android 9-or-higher handset does not imply an end-to-end-encrypted backup. The enrolment must have happened on a version that offered it:

> "If you've upgraded your development devices to Android 9, you need to disable and then re-enable data backup after upgrading. This is because Android only encrypts backups with a client-side secret after informing users in Settings or the setup wizard."
>
> — [Back up user data with Auto Backup](https://developer.android.com/identity/data/autobackup)

The stated reason is *consent*, not capability: the platform will not silently begin using a client-side secret without having told the user about it in a settings screen or setup flow. A long-lived phone that had backup on before its upgrade is therefore in the unguaranteed row while presenting as a modern device. **Confidence: medium-high.** The instruction is addressed to developers about development devices; whether production upgrades re-prompt through the setup wizard is **SILENT**, and it is the single most consequential silence in this note, because it decides whether the exposed population is "no-lock-screen and old handsets" or "those plus anyone who has carried the same phone across a major upgrade".

### 3.3 Whether the mitigation is reachable from this repo's packaging

[ADR-0003](../../adr/0003-client-stack.md) ships the Android build with no Gradle project — a manifest plus a native library, generated from Cargo metadata. That constrains which manifest attributes can be emitted at all.

- The manifest model behind that packaging exposes `allow_backup` and `full_backup_content: Option<Resource<XmlResource>>` — and, across all 58 fields, **no `data_extraction_rules`** ([`android_manifest::Application`](https://docs.rs/android-manifest/latest/android_manifest/struct.Application.html), read 2026-07-31).
- Arbitrary XML resources *can* be shipped: the packaging tool takes a `resources = "path/to/resources_folder"` key, noting *"If not specified, resources will not be included in the APK"* ([cargo-apk README](https://github.com/rust-mobile/cargo-apk/blob/main/cargo-apk/README.md)). So `res/xml/backup_rules.xml` is deliverable; the attribute that points the system at the Android-12+ form is not.
- And the older form does not cover modern devices for an app in our position: the `full-backup-content` syntax is documented as *"the configuration file that controls backup for devices running Android 11 or lower"*, with the newer syntax controlling *"devices running Android 12 or higher"* ([Auto Backup](https://developer.android.com/identity/data/autobackup)).

**Consequence, marked as inference:** with `target_sdk_version = 36`, shipping `requireFlags="clientSideEncryption"` today would constrain only Android 8.1-and-below devices — a set that overlaps the API 24–27 population from §3.1 that cannot satisfy the flag anyway — while every Android 12+ handset would keep backing up unconditionally. **Requiring the guarantee on the devices where it is achievable needs either a manifest attribute the current generator cannot emit, or `android:allowBackup="false"`, which is the opposite decision.** **Confidence: medium** — the field list is definitive for that crate at that version, but I did not attempt a build, and a passthrough mechanism may exist that the README does not document. **Verify before this becomes load-bearing.**

---

## 4. Does the 25 MB quota interact with any of it? (Q4)

### 4.1 What the quota is

> "Every app can allocate up to 25 MB of backup data per app user. There's no charge for storing backup data."
>
> "Backup data is stored in a private folder in the user's Google Drive account, limited to 25 MB per app. The saved data does not count toward the user's personal Google Drive quota. Only the most recent backup is stored. When a backup is made, any previous backup is deleted. The backup data can't be read by the user or other apps on the device."
>
> — [Back up user data with Auto Backup](https://developer.android.com/identity/data/autobackup)

### 4.2 What crossing it does — and how quietly

> "**Caution:** If the amount of data is over 25 MB, the system calls `onQuotaExceeded()` and doesn't back up data to the cloud. The system periodically checks whether the amount of data later falls under the 25 MB threshold and continues Auto Backup when it does."
>
> — same source

**It fails closed, and it retries forever.** Two properties worth separating:

1. **No partial upload and no truncation.** The transport rejects the package's backup; it does not upload the first 25 MB. So crossing the quota is the one documented condition under which the payload stops reaching the storage operator at all — the exposure this note is about *ends* at exactly the moment the backup guarantee does [inference].
2. **The threshold is not a one-way door.** Data falling back under 25 MB resumes backups with no user action.

**The signal is developer-only.** Three channels exist and no more:

- `onQuotaExceeded()`, a callback on a `BackupAgent` — which is a Java class, requiring a dex this build does not produce. [ADR-0016 §7](../../adr/0016-backup-and-restore.md) already reaches this conclusion independently (*"`onQuotaExceeded()` needs a backup agent class, which needs a dex; we do not need it"*), and this note confirms the premise rather than changing it.
- The system log, whose exact lines are published: *"The following messages in Logcat indicate that your app has exceeded the transport quota: `I/PFTBT: Transport rejected backup of <PACKAGE>, skipping` — or — `I/PFTBT: Transport quota exceeded for package: <PACKAGE>`"* ([Test backup and restore](https://developer.android.com/identity/data/testingbackup)).
- Nothing else. **No user-visible notification, banner or settings indication is documented anywhere** (**SILENT**). This is the sourced basis for ADR-0016 §7's decision to state the size fact in the app itself: the platform will not say it.

**A stale backup is left in place, and the user cannot tell it is stale.** *"When a backup is made, any previous backup is deleted"* — a backup that is *not* made deletes nothing, so the last successful upload persists indefinitely under the same 57-day inactivity rule as any other (§5). Nothing documents an expiry or a marking for a dataset whose app has since outgrown the quota [inference, medium confidence].

### 4.3 The interaction the ticket asks about: none is documented

> **Later note — this section's open question was settled by measurement on 2026-08-01, and one claim in §4.2 was corrected.** The quota is measured **before** compression, against uncompressed tar-stream bytes: the transport is handed an uncompressed total at pre-flight, before any app data is streamed. And the two log lines §4.2 quotes as equivalent quota indicators are **not** equivalent — *"Transport rejected backup of … , skipping"* fires for a generic package rejection and was observed on a 1 KB payload. This section is left as written, being the point-in-time record of what the documentation alone establishes; the resolution and its evidence are in [`../auto-backup-quota/README.md`](../auto-backup-quota/README.md) ([#64](https://github.com/amin-bf/cairn/issues/64)).

The quota and the encryption state are both properties of the same transport — it is the transport that rejects an over-quota package (§4.2) and the transport whose flags report client-side encryption (§1.3). But:

- **No source states whether the 25 MB is measured before or after compression and encryption.** The documentation says only *"the amount of data"* (**SILENT**). This matters: `collection.db` is a SQLite file of highly repetitive rows, and the sibling note measured 11.8× compression on that shape with a large-window compressor ([`../sync-transport/object-stores-and-drives.md`](../sync-transport/object-stores-and-drives.md) §3.1) — so a pre- or post-compression reading of the same file differs by an order of magnitude and moves ADR-0016's nine-month cutoff correspondingly. **Do not assume either reading.**
- **No source states any coupling in the other direction** — nothing suggests an unencrypted backup gets a different quota, or that requiring encryption changes it.
- The one *behavioural* similarity is worth naming because it makes both mechanisms safe by the same token: **both fail closed.** Over quota → nothing is uploaded. Encryption required but unavailable → *"stops backups from being sent to the cloud"* (§2.2). Neither degrades into uploading something weaker.

---

## 5. Can the user delete the backup, and from where? (Q5)

### 5.1 The documented routes

| Route | Granularity | Wording |
|---|---|---|
| Storage cleanup page → *Backups* | **one device's whole backup** | *"Find & delete unneeded device backups: Go to the Backups section of your storage. Scroll to Backups."* ([Clean up & troubleshoot your Google storage](https://support.google.com/drive/answer/6374270)) |
| Turn the setting off on the phone | **everything that device backed up** | *"If you turn off backup on your device, your backups are deleted."* / *"Turn off Android Google Backup… Your backup data is erased, except what you backup to Google Photos."* ([Back up your device](https://support.google.com/googleone/answer/9149304)) |
| Account dashboard | account-wide | *"To delete your backup, you can also use your Google Account dashboard."* (same source) |
| Doing nothing for 57 days | that device's backup | *"If you don't use your device for 57 days, the data you backed up (except photos or videos) is also erased."* (same source); *"Your backup remains as long as you use your device. If you last used your device more than 57 days ago, the backup is no longer available."* |
| Automatic dataset expiry | old device-setup lifetimes | *"Backups from each device-setup-lifetime are stored in separate datasets… Obsolete datasets are automatically deleted after a period of inactivity."* ([Auto Backup](https://developer.android.com/identity/data/autobackup)) |

Two constraints on where the user must be standing: *"To make, manage, or delete backups, you need to use a mobile device. On your computer, you can check what data and apps are included in your backup file"* (Google One help), and the developer documentation's own — dated — pointer that the per-app *listing* exists but only as a listing: *"Users can see a list of apps that have been backed up in the Google Drive Android app. On an Android-powered device, users can find this list in the Drive app's navigation drawer under Settings > Backup and reset."*

### 5.2 The comparison with the sync folder's removal path

The same first-party page carries both instructions, one line apart, which makes the contrast unusually clean:

> "Clear other storage. This category often includes device backups and hidden data from apps connected to your Drive.
> **Find & delete unneeded device backups:** Go to the Backups section of your storage. Scroll to Backups.
> **Delete hidden app data:** Go to Google Drive settings. Click Manage apps."
>
> — [Clean up & troubleshoot your Google storage](https://support.google.com/drive/answer/6374270)

| | Published sync objects (application data folder) | Automatic app-data backup |
|---|---|---|
| Removal granularity | **per application** — *Manage apps* lists our app by name | **per device backup** — all apps together |
| Reached by | Drive settings → *Manage apps* ([`../sync-transport/object-stores-and-drives.md`](../sync-transport/object-stores-and-drives.md) §6) | storage page → *Backups*, or turning the phone's backup setting off |
| Survives uninstalling the app | **no** — *"deleted when a user uninstalls your app"* ([appdata guide](https://developers.google.com/workspace/drive/api/guides/appdata)) | **yes** — the vendor's own test uninstalls and reinstalls to prove restore works (§ finding 10) |
| Expires by itself | not documented | **yes** — 57 days of device inactivity |
| Entered by | an explicit enrolment the user performed | nothing; it is on by default |

**The asymmetry is the finding.** The surface the user chose is individually removable and self-cleaning on uninstall; the surface they never chose is removable only in a unit that includes every other app on the phone, and outlives the app itself. For [#58](https://github.com/amin-bf/cairn/issues/58) this means the disclosure sentence written for sync cannot be reused: it would point at a control that, for this path, does not exist at the same granularity.

### 5.3 Where the documentation is SILENT

- **Per-app deletion of the automatic backup.** No first-party source found describes deleting one app's payload while keeping the rest of the device backup. The developer documentation describes a per-app *view*; every deletion instruction found is per device or per account. **This is absence of evidence, not evidence of absence** — a settings screen may offer it without being documented, and it is cheap to check on a handset before an ADR states it.
- **Whether deletion is immediate or eventual**, and whether a deleted backup is unrecoverable.
- **Whether an app can delete its own backup programmatically.** The system tool `bmgr` exists for testing and its documentation warns *"bmgr restore does not work for encrypted backups"* ([bmgr](https://developer.android.com/tools/bmgr)), but no supported app-facing API for deleting one's own cloud payload was found.
- **Whether the payload counts against the user's account storage.** The developer documentation says it does not (*"The saved data does not count toward the user's personal Google Drive quota"*), while the consumer storage-cleanup page lists device backups under storage the user is being helped to reclaim. **The two sources are in tension and this note does not resolve it.**

---

## 6. One consequence specific to this repo

[ADR-0013 §9](../../adr/0013-the-sync-transport.md) deliberately places the sync refresh token *inside* the backup set, so that a replaced phone arrives already authorised — the exact opposite of the writer marker, which is excluded because restoring it would manufacture a duplicate writer. That decision is sound on its own terms, and this note does not reopen it. But it does add one line to the exposure statement: **on a device in the unguaranteed rows of §1.1, what the storage operator can read includes not only the review history but a bearer credential to the app's private folder in that same operator's storage.** The blast radius is bounded — the scope reaches *"our app's own hidden folder and nothing else"* (ADR-0013 §9) — and the holder of the credential is the party who already holds the data it unlocks, so this is a coherence point for the ADR's prose rather than a new hazard [inference]. It would become a genuine escalation if the published-log storage and the backup storage were ever different companies.

---

## 7. Sources

All retrieved 2026-07-31. Every row is first-party: platform documentation, platform source, a first-party security paper, or first-party product help.

| # | Source | What it establishes here |
|---|---|---|
| S1 | [Back up user data with Auto Backup](https://developer.android.com/identity/data/autobackup) | Participation from API 23; upload destination; *"end-to-end encrypted on devices running Android 9 or higher"*; the 25 MB quota and `onQuotaExceeded()`; `requireFlags="clientSideEncryption"` and `disableIfNoEncryptionCapabilities`; the Android 11-or-lower vs Android 12-or-higher config split; the post-upgrade re-enable instruction; dataset lifetimes |
| S2 | [Security recommendations for backups](https://developer.android.com/privacy-and-security/risks/backup-best-practices) (updated 2024-10-25) | **The decisive sentence**: always encrypted in transit and at rest regardless of version or lock screen; *"key not known to Google"* only from Android 9 with a lock screen; the recommendation to require end-to-end encryption |
| S3 | [Android Security Paper 2024](https://services.google.com/fh/files/misc/android-security-paper-2024.pdf), §*Backup encryption*, p. 27 | Random key wrapped by a hash of the lock-screen secret; enclave cohort; attempt limit in firmware; *"no one (including Google) can access"* |
| S4 | [Google Cloud Key Vault Service whitepaper](https://developer.android.com/about/versions/pie/security/ckv-whitepaper), 2018-03-06 | Why a low-entropy secret suffices; per-vault failed-attempt counter in a custom chip; firmware updates wipe keys; software rate limiting; Android 9 as the first client platform; protocol section still unfinished |
| S5 | [Google and Android have your back by protecting your backups](https://security.googleblog.com/2018/10/google-and-android-have-your-back-by.html), 2018-10-12 | Chip authorises *every* access; permanent block after too many wrong guesses; external audit commissioned |
| S6 | [`BackupAgent`](https://developer.android.com/reference/android/app/backup/BackupAgent) and [`BackupAgent.java`](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/core/java/android/app/backup/BackupAgent.java) | `FLAG_CLIENT_SIDE_ENCRYPTION_ENABLED`, added at API 28, defined verbatim in platform source — client-side encryption is a *per-operation, testable* property |
| S7 | [Test backup and restore](https://developer.android.com/identity/data/testingbackup) | Names the proprietary cloud transport; publishes the over-quota log lines; the uninstall/reinstall restore script |
| S8 | [Back up or restore data on your Android device](https://support.google.com/android/answer/2819582) | *"Some of your data is end to end encrypted"*; *"use a PIN, pattern, or password screen lock, instead of a swipe or Smart Lock"* |
| S9 | [Back up your device](https://support.google.com/googleone/answer/9149304) (computer and Android variants) | *"encrypted with your Google Account password"*; deletion by turning backup off; account dashboard; the 57-day inactivity erasure; management requires a mobile device |
| S10 | [Clean up & troubleshoot your Google storage](https://support.google.com/drive/answer/6374270) | Both removal paths side by side: device backups vs *Manage apps* for hidden app data |
| S11 | [`bmgr`](https://developer.android.com/tools/bmgr) | *"bmgr restore does not work for encrypted backups"* |
| S12 | [`android_manifest::Application`](https://docs.rs/android-manifest/latest/android_manifest/struct.Application.html) and [cargo-apk README](https://github.com/rust-mobile/cargo-apk/blob/main/cargo-apk/README.md) | `allow_backup` and `full_backup_content` exist; no `data_extraction_rules` field; `resources` folder support |
| S13 | [Drive application data folder](https://developers.google.com/workspace/drive/api/guides/appdata) | The comparison case: per-app hidden folder, deleted on uninstall (also cited in [`../sync-transport/object-stores-and-drives.md`](../sync-transport/object-stores-and-drives.md) §6) |

---

## 8. Confidence, and what is SILENT

| Claim | Confidence | Why |
|---|---|---|
| Encrypted at rest unconditionally; key held by the operator unless Android 9+ *and* a lock screen | **High** | S2 states both halves in one paragraph; S8 and S9 agree from the consumer side |
| Key protection is a lock-screen-derived wrap plus a hardware-enforced guess limit | **High** as a description of the design | S3, S4, S5 agree in detail |
| The operator genuinely cannot decrypt in the guaranteed case | **Medium** | The claim is repeated across three first-party sources, but the encrypting component is proprietary (S7) and the only external review is 2018 (S5). Not independently verifiable |
| No lock screen → operator-readable payload, still uploaded | **High** | Direct consequence of S2's wording, corroborated by S9 |
| Swipe / trusted-context unlock does not qualify | **Medium-high** | S8 phrases it as a recommendation, not a specification |
| API 24–27 devices cannot have client-side encryption at all | **High** | The flag is *"Added in API level 28"* (S6); the capability is dated to Android 9 in S3 and S4 |
| A pre-Android-9 enrolment carried across an upgrade stays unguaranteed | **Medium-high** | S1 states it for development devices; production upgrade behaviour is SILENT |
| Over-quota fails closed, with no user-visible signal | **High** | S1 for the behaviour, S7 for the only observable trace |
| Whether the 25 MB is measured pre- or post-compression | **Not established** | SILENT in every source found. Changes ADR-0016's cutoff by an order of magnitude |
| No per-app deletion path for the automatic backup | **Medium** | Absence of evidence across S1, S9, S10 — cheap to falsify on a handset, and worth doing before an ADR asserts it |
| Backup survives uninstall | **High** | S7's own test procedure depends on it |
| The Android-12+ encryption requirement is unreachable from this repo's packaging | **Medium** | S12's field list is definitive for that crate version; no build was attempted |

**Documentation is SILENT on, and the ADR should say so rather than infer:** whether a user without a lock screen is ever told their backup is not end-to-end encrypted; what happens to an existing encrypted backup when a lock screen is removed; whether production upgrades re-prompt so that backups become client-side encrypted; whether the quota is measured before or after compression; whether one app's backup can be deleted alone; whether the payload counts against account storage (two first-party sources disagree); and how long deletion takes to propagate.

**The three cheap checks that would retire the biggest residuals**, all on a real handset and none requiring code: (1) remove the lock screen and observe whether anything is said, anywhere; (2) find out whether a per-app deletion control exists in the current storage settings; (3) build with `full_backup_content` set and confirm what the generated manifest actually contains.
