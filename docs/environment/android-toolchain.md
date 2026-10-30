# Android toolchain

Record of the Android build environment set up for this repo, per
[#7 "Set up the Android toolchain and prove it works"](https://github.com/amin-bf/cairn/issues/7).

**Host**: CachyOS (Arch-based), x86_64, 16 cores. **Set up 27.07.2026; moved to the native client's
pins (AGP 9.4.1, Gradle 9.7.1, NDK r29 stable) on 30.10.2026** under
[#203](https://github.com/amin-bf/cairn/issues/203) — see
[Building the native client](#building-the-native-client).

Everything below was installed **into `$HOME` without root.** Nothing was installed
via `pacman`, so nothing here is shared with the system package manager or will be
touched by a system upgrade.

## Quick start

```sh
source scripts/android-env.sh
```

That sets `JAVA_HOME`, `ANDROID_HOME`, `ANDROID_SDK_ROOT`, `NDK_HOME`,
`ANDROID_NDK_HOME` and `ANDROID_NDK_ROOT` (for `cargo apk` and a bare `cargo ndk`; the
Gradle build uses AGP's own NDK) and prepends the tool
directories to `PATH`. **None of these were set on the host before this ticket**, and
nothing writes them to a shell profile — each build must source the script (or an
agent must export them itself).

## What is installed, and where

| Component | Version | Location |
| --- | --- | --- |
| JDK (Eclipse Temurin) | **17.0.20+8** | `~/.local/share/jdk-17.0.20+8`, symlinked as `~/.local/share/jdk17` |
| Android SDK cmdline-tools | **15859902** | `~/Android/Sdk/cmdline-tools/latest` |
| Android SDK Platform | **android-34**, **android-36**, **android-37.0** | `~/Android/Sdk/platforms` |
| Android build-tools | **34.0.0**, **35.0.0**, **36.1.0**, **37.0.0** | `~/Android/Sdk/build-tools` |
| Android NDK | **29.0.14206865** (`r29`, stable) | `~/Android/Sdk/ndk/29.0.14206865` |
| platform-tools (`adb`, `fastboot`) | **r37.0.0** (`adb` 1.0.41) | `~/Android/Sdk/platform-tools` |
| Emulator | bundled with SDK | `~/Android/Sdk/emulator` |
| System image | `system-images;android-36;google_apis;x86_64` | `~/Android/Sdk/system-images` |
| Gradle | **9.7.1**, through the project's wrapper (`android/gradlew`), fetched on first use | `~/.gradle/wrapper/dists` |
| Android Gradle Plugin | **9.4.1**, with Kotlin built in (no `org.jetbrains.kotlin.android` plugin) | resolved by Gradle |
| `cargo-ndk` | **4.1.2** | `~/.cargo/bin/cargo-ndk` |

Total SDK footprint: **~8.7 GB** under `~/Android/Sdk`.

**Gotcha — API 37 packages are named with a minor version and live on the preview
channel.** `sdkmanager "platforms;android-37"` fails with *"Failed to find package"*.
The real name is **`platforms;android-37.0`** (Google now ships `36.1`, `37.0`, `37.1`),
and it is not on the default channel, so it needs `--channel=3`:

```sh
sdkmanager --channel=3 "platforms;android-37.0" "build-tools;37.0.0"
```

API 37 is installed because **the physical test device runs it** (see below). AGP 8.11.0
accepts `compileSdk = 37` / `targetSdk = 37` without complaint despite the
`android-37.0` directory name.

The symlink `~/.local/share/jdk17` exists so `JAVA_HOME` contains no `+` character —
some build tooling mishandles it in paths — and so a future JDK bump is a symlink
swap rather than an edit to every consumer.

### Rust targets

`rustc 1.97.0`. `aarch64-linux-android` and `wasm32-unknown-unknown` were already
present; this ticket added the remaining three:

```
aarch64-linux-android      # real devices
armv7-linux-androideabi    # 32-bit ARM
i686-linux-android         # 32-bit x86
x86_64-linux-android       # emulator
wasm32-unknown-unknown
x86_64-unknown-linux-gnu
```

Note the client-stacks research: **Dioxus silently dropped 32-bit Android in 0.7.4**
(#5637) while its docs still tell you to add `armv7-linux-androideabi`. The 32-bit
targets are installed anyway — they cost nothing and Tauri may still want them.

## Why these versions

The two candidate stacks in
[#8 "Prototype: pick the client stack"](https://github.com/amin-bf/cairn/issues/8)
have different requirements, and this toolchain must satisfy **both**, since #8 has to
build each one on the same machine.

- **JDK 17.** Dioxus's generated Gradle module sets `jvmTarget = "17"`, so 17 is a hard
  floor; and Tauri **breaks cryptically on JDK 25/26**. 17 is the version both accept.
  (`dx` autodetects JDK 11 on Linux and would fail — hence setting `JAVA_HOME`
  explicitly is mandatory, not optional.)
- **SDK 34 and 36.** Dioxus generates `compileSdk`/`targetSdk` 34; the Tauri CLI uses
  SDK 36. Both platforms are installed so neither stack triggers a download mid-build.
- **NDK 29.0.14206865 (r29 stable), pinned as `ndkVersion`.** It replaced the
  `29.0.13846066` **beta** (`r29-beta3`) this document first recorded: that beta was kept only
  because the Tauri CLI pinned it and selected the lexicographically highest installed NDK, and
  Tauri is no longer a candidate ([ADR-0045](../adr/0045-the-native-android-client.md)). r29 stable
  is the release line the Kotlin binding was measured on
  ([research](../research/kotlin-binding/README.md)); AGP 9.4's own default (r28.2) is not taken
  because nothing was measured on it. The Gradle build builds with the directory AGP resolves from
  `ndkVersion`, so Gradle and cargo cannot disagree. The beta may stay installed — nothing selects
  it any more — and can be removed with `sdkmanager --uninstall "ndk;29.0.13846066"`.
- **Gradle 9.7.1 and AGP 9.4.1.** AGP 9.4 needs Gradle 9.6 or later, JDK 17 and Build Tools 36,
  and has Kotlin built in. The wrapper in `android/` carries the Gradle version, so no Gradle is
  installed system-wide (the 8.14.3 install was the Tauri CLI's, and Gradle 8 cannot load an AGP 9
  project at all).
- **`compileSdk` 36, and androidx versions that fit it.** The newest Compose, lifecycle and activity
  releases require `compileSdk` 37; the spec pins 36 (as the egui build declares), so the project
  pins the Compose BOM `2026.04.01`, lifecycle `2.10.0` and activity-compose `1.12.4`, the last
  that compile against 36. Moving to 37 is a deliberate bump, not a drift.

## Building the native client

The Gradle project is `android/`. **One `assemble` builds everything** — no step by hand:

```sh
source scripts/android-env.sh      # JAVA_HOME and the SDK; the build itself finds the NDK through AGP
cd android && ./gradlew assemble   # debug and release APKs under app/build/outputs/apk/
```

What it runs, in order, joined to AGP through the Variant API
(`addGeneratedSourceDirectory`), so nothing is ordered by a hand-written `dependsOn`:

1. **`cargoNdk<Variant>`** — `cargo ndk -t arm64-v8a -t x86_64 -P 24 build --release --lib -p
   cairn-ffi`, with `ANDROID_NDK_HOME` set to AGP's resolved NDK directory. Its output is the
   `jniLibs` source set.
2. **`uniffiBindgen<Variant>`** — builds `cairn-ffi` for the **host** with the `cli` feature and runs
   `uniffi-bindgen` in library mode, writing Kotlin in `dev.cairn.ffi` into the Kotlin source set.
   The `cli` feature is host-only: the cargo-ndk task never passes it, so it is absent from the
   Android library. Nothing generated is committed (`android/app/build` is git-ignored).
3. AGP compiles, merges and packages both ABIs and signs the debug APK with
   `~/.android/debug.keystore` — the same key `cargo-apk` signs with — so `adb install -r` over
   the egui build keeps its data.

The debug variant also builds the `panic-probe` cargo feature (an export that panics on purpose,
for the test that a Rust panic arrives as an exception); a release build never carries it. The
release profile has no `panic = "abort"`, and a Rust test fails if one is added.

**Pins:** uniffi `=0.32.2` and JNA 5.19.1 (`@aar`), AGP 9.4.1, Gradle 9.7.1, JDK 17, Compose
through its BOM, NDK `29.0.14206865`, cargo-ndk 4.1.2, `compileSdk` and `targetSdk` 36, `minSdk`
24, ABIs `arm64-v8a` and `x86_64`.

**Tests on the emulator** (`cairn-test`, started headless so no window opens):

```sh
emulator -avd cairn-test -no-window -no-audio -no-boot-anim -no-snapshot -gpu swiftshader_indirect
cd android && ./gradlew connectedDebugAndroidTest
```

**The egui build on the emulator.** The upgrade check installs the egui APK first. Two things
differ from the handset: `cargo apk` packages only `aarch64-linux-android` by default, which the
x86_64 emulator cannot run, so build it with `--target x86_64-linux-android`; and the **debug**
egui build crashes on the emulator's Vulkan (wgpu calls a debug-utils entry point the emulated
driver lacks), so use `--release`. `cargo apk build --release` then stops at signing for want of
a release keystore, and its unsigned APK lacks the library: add `lib/x86_64/libcairn_app.so` to it
stored uncompressed (`zip -0`), `zipalign -P 16`, and sign with the debug keystore using
`apksigner`.

## Proof the toolchain works

A throwaway crate (`andsmoke`) was built and run end to end. It depends on
`rusqlite 0.37` with the **`bundled`** feature deliberately — that compiles ~250k lines
of SQLite C through NDK clang, which is the single most demanding thing this app's
storage layer will ask of the toolchain, and is exactly what the client-stacks research
flagged as unverified.

**1. Cross-compilation — both ABIs, C dependency included.**

```sh
cargo ndk -t arm64-v8a -t x86_64 build --release
```

Built clean in **27 s**. Output is genuine Android ELF for both ABIs:

```
ELF 64-bit LSB pie executable, ARM aarch64, ... for Android 21, built by NDK r29-beta3
ELF 64-bit LSB shared object, x86-64,       ... for Android 21, built by NDK r29-beta3
```

**2. Execution on Android.** Pushed the `x86_64` binary to the emulator and ran it
(the `aarch64` build was later run on the real handset — see Device situation):

```
ANDSMOKE_OK sqlite=3.50.2 rows=1 path=/data/local/tmp/smoke.db printf=0.33333333333333330000
ANDSMOKE_OK sqlite=3.50.2 rows=2 ...
```

`rows` incrementing across runs proves the database file is really created, written and
**persisted** on device — not just that the binary starts.

**3. APK packaging.** *(Recorded under the earlier pins; the native client now builds on AGP 9.4.1 and
Gradle 9.7.1 — see above.)* A minimal AGP project (Gradle 8.14.3, AGP 8.11.0, `compileSdk` 36,
`minSdk` 24, Java 17, no Kotlin) packaged the Rust `cdylib` into `jniLibs` and built in
**30 s**. The APK carries `lib/arm64-v8a/libandsmoke.so` and `lib/x86_64/libandsmoke.so`.
Installed and launched on the emulator, `System.loadLibrary("andsmoke")` succeeded:

```
I APKSMOKE: APKSMOKE_OK loaded libandsmoke.so (bundled SQLite) into app process
```

This exercises the *other* half of the toolchain — JDK 17 driving Gradle and AGP, aapt2,
d8, signing, packaging, install — which the native binary test alone does not touch.

### Two research claims settled

- **The missing-compiler-builtins failure did not reproduce.** `sqlx#2299` reports
  `undefined symbol: __extenddftf2 / __lttf2 / __trunctfdf2` when building bundled
  SQLite for Android, with `cargo-ndk --link-builtins` as the documented fix.
  **`--link-builtins` was not needed** on NDK r29 — `llvm-nm -u` shows none of those
  symbols undefined. The smoke test deliberately calls `printf('%.20f', 1.0/3.0)`, the
  128-bit-float path that pulls those builtins in, and it returned correctly on device.
  Treat `--link-builtins` as a fallback to remember, not a required flag.
- **16 KB page alignment works out of the box.** Mandatory for Play uploads since
  2025-11-01. `llvm-readelf -l` reports `LOAD align: 0x4000` on every LOAD segment of
  the aarch64 `.so`, confirming the research's "works with NDK r28+" claim on r29.

## Device situation

Everything above was first proven on an emulator, then **re-proven on real hardware**.

### Physical device — Pixel 8 Pro (verified)

The whole suite was re-run on the human's handset and **passed identically**:

| Property | Value |
| --- | --- |
| Model | Google **Pixel 8 Pro** |
| Android | **17** (API **37**), build `CP2A.260705.006` |
| `ro.product.cpu.abilist` | **`arm64-v8a`** — 64-bit only |
| Kernel page size | **4096** |

- Native `aarch64` binary: `ANDSMOKE_OK sqlite=3.50.2`, `rows` incrementing 1 → 2 → 3
  across runs, so the database is genuinely created, written and **persisted** on the
  handset. Whole open/create/insert/query cycle in **0.05 s real**.
- APK installed, launched, and `System.loadLibrary("andsmoke")` succeeded in the app
  process, with `primaryCpuAbi=arm64-v8a`.
- Rebuilt at `compileSdk`/`targetSdk` **37** and re-installed: also fine
  (`minSdk=24 targetSdk=37` as installed). AGP 8.11.0 is happy with API 37.
- Test artifacts were uninstalled and `/data/local/tmp` cleaned up afterwards.

**Three consequences worth carrying into #8:**

1. **This device is `arm64-v8a` only.** `armv7-linux-androideabi` and
   `i686-linux-android` binaries cannot run on it at all, so Dioxus having dropped
   32-bit Android is a non-issue for our test hardware.
2. **The device runs 4 KB pages**, so it does *not* exercise the 16 KB page-size path at
   runtime. 16 KB alignment is still mandatory for Play uploads — it is satisfied at
   link time (see above) — but do not treat "works on this Pixel" as evidence about
   16 KB-page devices.
3. **The device API (37) is ahead of the stacks' defaults** (Dioxus generates 34, Tauri
   36). That is fine — an APK targeting 36 installs and runs on 37 — but it means the
   handset cannot, by itself, catch anything that only breaks on older Android.

### Getting the device connected

`adb devices` was empty at first because **USB debugging was off**. The diagnostic that
settles this quickly is the USB descriptor rather than `adb`: with debugging off the
Pixel exposes a *single* interface of class `06` (MTP); with it on, a second interface
appears with class `FF`, subclass `42`, protocol `01`.

```sh
lsusb | grep -i google        # 18d1:4ee1 = MTP only
for d in /sys/bus/usb/devices/*/; do
  [ "$(cat $d/idVendor 2>/dev/null)" = "18d1" ] && cat $d*/bInterfaceClass
done
```

**No udev rules were needed after all.** The `android-udev` package is still not
installed and the user is in no `plugdev`/`adbusers` group, but `/dev/bus/usb/...` for the
handset carries an ACL (`crw-rw-r--+`, from `uaccess`) that grants the logged-in user
access. `adb` talks to the device as a normal user. **Nothing in this setup required
root** — the earlier note that a password might be needed turned out not to apply on
this host.

Remaining: after enabling USB debugging you must accept the **"Allow USB debugging?"**
RSA prompt on the phone, or `adb devices` reports the serial as `unauthorized` rather
than `device`.

### Emulator

An AVD named **`cairn-test`** is also set up and working — useful for CI-ish runs and
for the `x86_64` triple, which the handset cannot exercise:

- `system-images;android-36;google_apis;x86_64`, Android 16 / API 36, at
  `~/.android/avd/cairn-test.avd`
- Boots headless in **~15 s** (re-verified at **19 s** after the rename below):
  ```sh
  emulator -avd cairn-test -no-window -no-audio -no-boot-anim -no-snapshot \
           -gpu swiftshader_indirect -accel on
  ```

> **Renamed from `leitner-test`** under [ADR-0028](../adr/0028-the-application-is-named-cairn.md),
> with `avdmanager move avd --name leitner-test --rename cairn-test`. One thing to know if you ever
> repeat this: that command updates `<name>.ini` and `config.ini` but leaves **`hardware-qemu.ini`
> pointing at the old directory**, which no longer exists. It is a generated file and the emulator
> rewrites it on the next launch, so the stale paths clear themselves — but they look alarming in a
> grep, and the way to tell a self-healing artifact from a broken AVD is to boot it, which is why the
> figure above was re-measured rather than assumed.
- **KVM works without a group change** — `/dev/kvm` is `crw-rw-rw-` on this host, so
  hardware acceleration is available even though the user is not in the `kvm` group.

Note `adb` also exists at `/usr/bin/adb` (Arch `android-tools`, v36.0.1) independently of
the SDK copy (r37.0.0). Sourcing `scripts/android-env.sh` puts the **SDK** copy first on
`PATH`. Avoid running both versions against one device — mismatched `adb` clients fight
over the server on port 5037. With both a handset and an emulator attached, set
`ANDROID_SERIAL` to pick one.

### Emulator caveats

The AVD ignored the `pixel_6` device profile: `avdmanager` reported
`Could not load devices from .../devices.xml` and fell back to a default profile. The
AVD works fine, but it is **not** a faithful Pixel (screen size and density are
defaults), so it is a poor witness for a **UI layout** judgement. Now that the Pixel 8
Pro is known-working, prefer the handset for anything about how the app *looks or feels*
and keep the emulator for build/CI checks and the `x86_64` triple.

Also: the research found Dioxus 0.7.x **crashes below API 30** (`tao` calls the
API-30-only `getCurrentWindowMetrics`). Both the emulator (API 36) and the handset
(API 37) are **above** that floor, so **neither can reproduce that bug**. Testing the
declared `minSdk` 24 needs an API 24–29 system image installed separately.

## Reproducing this setup elsewhere

```sh
# JDK 17 (Temurin), no root
curl -L -o jdk17.tar.gz \
  "https://github.com/adoptium/temurin17-binaries/releases/download/jdk-17.0.20%2B8/OpenJDK17U-jdk_x64_linux_hotspot_17.0.20_8.tar.gz"
tar -xzf jdk17.tar.gz -C ~/.local/share/
ln -sfn ~/.local/share/jdk-17.0.20+8 ~/.local/share/jdk17

# Android SDK cmdline-tools -> must land at $ANDROID_HOME/cmdline-tools/latest
curl -L -o cmdline-tools.zip \
  "https://dl.google.com/android/repository/commandlinetools-linux-15859902_latest.zip"
mkdir -p ~/Android/Sdk/cmdline-tools && unzip -q cmdline-tools.zip -d /tmp/ct
mv /tmp/ct/cmdline-tools ~/Android/Sdk/cmdline-tools/latest

export JAVA_HOME=~/.local/share/jdk17 ANDROID_HOME=~/Android/Sdk
export PATH="$JAVA_HOME/bin:$ANDROID_HOME/cmdline-tools/latest/bin:$PATH"
yes | sdkmanager --licenses
sdkmanager "platform-tools" "platforms;android-34" "platforms;android-36" \
           "build-tools;34.0.0" "build-tools;36.1.0" "ndk;29.0.14206865" \
           "emulator" "system-images;android-36;google_apis;x86_64"

# API 37 (the test handset's level) is preview-channel and minor-versioned
sdkmanager --channel=3 "platforms;android-37.0" "build-tools;37.0.0"

rustup target add aarch64-linux-android x86_64-linux-android \
                  armv7-linux-androideabi i686-linux-android
cargo install cargo-ndk --locked

# Gradle comes from the wrapper in android/ (9.7.1); nothing to install.

avdmanager create avd -n cairn-test -k "system-images;android-36;google_apis;x86_64"
```

`cargo-ndk` is only usable as `cargo ndk`; invoking the `cargo-ndk` binary directly
exits non-zero with *"This binary may only be called via `cargo ndk`"*. Do not read that
as a failed install.
