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
`ANDROID_NDK_HOME` and `ANDROID_NDK_ROOT` (for a bare `cargo ndk`; the
Gradle build uses AGP's own NDK) and prepends the tool
directories to `PATH`. **None of these were set on the host before this ticket**, and
nothing writes them to a shell profile — each build must source the script (or an
agent must export them itself).

## What is installed, and where

| Component | Version | Location |
| --- | --- | --- |
| JDK (system OpenJDK, the newest installed) | **27.0.1** | `/usr/lib/jvm/java-27-openjdk` (the `jdk-openjdk` package, not user-installed) |
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

`JAVA_HOME` is the system JDK's directory, set in `scripts/android-env.sh`. The JDK version and the
bytecode target are `cairn.jdk` and `cairn.bytecode` in `android/gradle.properties`, which both
modules read; a JDK bump is those properties plus that one line.

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

The 32-bit targets are installed although the native client packages only `arm64-v8a` and
`x86_64`: they cost nothing, and the earlier client-stack candidates (since dropped,
[ADR-0045](../adr/0045-the-native-android-client.md)) were measured with them.

## Why these versions

- **JDK 27, the newest installed** (it was 17 while the client-stack candidates were being compared).
  Gradle 9.7.1 runs on it and the build compiles with it as the `jvmToolchain`. **The bytecode
  target is 24, not 27:** the Kotlin Gradle plugin AGP 9.4.1 brings (2.2.10) knows JVM targets only up
  to 24 and fails configuration on `jvmTarget` 27 ("Unknown Kotlin JVM target: 27"), and Java's
  `sourceCompatibility` must equal it. So `cairn.bytecode` is 24. Raising it needs a
  Kotlin that knows a later target, which comes with a newer AGP.
- **SDK 34 and 36.** The build uses `compileSdk` 36; platform 34 stays installed from the
  earlier toolchain work, so nothing triggers a download mid-build.
- **NDK 29.0.14206865 (r29 stable), pinned as `ndkVersion`.** It replaced the
  `29.0.13846066` **beta** (`r29-beta3`) this document first recorded: that beta was kept only
  because a client-stack candidate's CLI pinned it and selected the lexicographically highest
  installed NDK, and that candidate is gone ([ADR-0045](../adr/0045-the-native-android-client.md)). r29 stable
  is the release line the Kotlin binding was measured on
  ([research](../research/kotlin-binding/README.md)); AGP 9.4's own default (r28.2) is not taken
  because nothing was measured on it. The Gradle build builds with the directory AGP resolves from
  `ndkVersion`, so Gradle and cargo cannot disagree. The beta may stay installed — nothing selects
  it any more — and can be removed with `sdkmanager --uninstall "ndk;29.0.13846066"`.
- **Gradle 9.7.1 and AGP 9.4.1.** AGP 9.4 needs Gradle 9.6 or later, JDK 17 or later and Build Tools 36,
  and has Kotlin built in. The wrapper in `android/` carries the Gradle version, so no Gradle is
  installed system-wide (an earlier 8.14.3 install existed for a client-stack candidate's CLI, and
  Gradle 8 cannot load an AGP 9 project at all).
- **`compileSdk` 36, and androidx versions that fit it.** The newest Compose, lifecycle and activity
  releases require `compileSdk` 37; the spec pins 36 (as the retired egui build declared), so the project
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
   `~/.android/debug.keystore` — the same key the retired egui build's `cargo-apk` signed with — so
   `adb install -r` over an installed egui build keeps its data.

The debug variant also builds the `panic-probe` cargo feature (an export that panics on purpose,
for the test that a Rust panic arrives as an exception); a release build never carries it. The
release profile has no `panic = "abort"`, and a Rust test fails if one is added.

**Pins:** uniffi `=0.32.2` and JNA 5.19.1 (`@aar`), AGP 9.4.1, Gradle 9.7.1, JDK 27 (toolchain; bytecode target 24), Compose
through its BOM, NDK `29.0.14206865`, cargo-ndk 4.1.2, `compileSdk` and `targetSdk` 36, `minSdk`
24, ABIs `arm64-v8a` and `x86_64`.

**The design module** (`android/design`, a library the app depends on, and the app's only source of
colour, type and spacing). Its tokens in `tokens/*.kt` restate the Rust modules' numbers in `dp`
and `sp`, and `cargo test -p cairn-app kotlin_tokens` fails on any difference, any missing token
and any extra one, so a change to a Rust token needs the Kotlin file edited in the same commit. The
faces live under `src/main/assets/fonts` with their licences beside them in `licenses`; the same
test holds each family's list, in order, byte-equal to what `fonts::definitions()` registers. Look
at it on a handset or emulator with the debug-only specimen:

```sh
adb shell cmd uimode night yes   # or `no`: a stored Light or Dark choice overrides it
adb shell am start -n dev.cairn.app/.SpecimenActivity
```

**Tests on the emulator** (`cairn-test`, started headless so no window opens):

```sh
emulator -avd cairn-test -no-window -no-audio -no-boot-anim -no-snapshot -gpu swiftshader_indirect
cd android && ./gradlew connectedDebugAndroidTest
```

**The development build** ([#237](https://github.com/amin-bf/cairn/issues/237),
[ADR-0051](../adr/0051-settings-on-android.md) §1). `CAIRN_DEVELOPMENT=1` in the environment of the
Gradle build adds the *Development* block (reset and the fixture bench) to Settings: its Kotlin and
strings are `app/src/development`, its tests `app/src/androidTestDevelopment`. Any other value, or
none, adds `app/src/shipping` and `app/src/androidTestShipping` instead, whose block draws nothing and
whose tests assert that none of it is in the APK. The variable is read through Gradle's providers and
is independent of the build type; both builds keep `dev.cairn.app` and the debug key, so one installs
over the other as an upgrade. Run the suite both ways, and finish a handset session on the build made
without it:

```sh
cd android
CAIRN_DEVELOPMENT=1 ./gradlew :app:connectedDebugAndroidTest   # development block and its tests
./gradlew :app:connectedDebugAndroidTest                       # no block, and the test that says so
```

Set `ANDROID_SERIAL` when a handset is also attached, or Gradle installs on both.

**The minified release build** ([#208](https://github.com/amin-bf/cairn/issues/208)). `release` runs
R8 (`isMinifyEnabled`, `isShrinkResources`) with `android/app/proguard-rules.pro`, whose every rule
says where it comes from: JNA's documented set (research §8 of `docs/research/kotlin-binding/`) and a
keep for the whole of `dev.cairn.ffi`, whose direct-mapped native methods are bound to the library's
symbols by name. It is signed with the debug key — a real release key is not part of this build, and
none is in the repository. Without the `dev.cairn.ffi` keep the minified app crashes at launch
(checked by removing it); with it the binding survives.

The proof runs the instrumented `ReleaseBuildTest` — a collection opened under `files/` and
`no_backup/`, a Review sitting to its end, and a Rust error arriving as `CairnException` — against
the `minified` build type: the release configuration plus the keeps the test runner needs in the
app's own dex (`proguard-minified-rules.pro`; the runner shares the app's process and R8 would strip
what only it calls). The other instrumented classes call the app's own classes, which R8 renames, so
they are run against debug only:

```sh
cd android && ./gradlew -Pcairn.testMinified -Pandroid.testInstrumentationRunnerArguments.class=dev.cairn.app.ReleaseBuildTest connectedMinifiedAndroidTest
```

`panic-probe` is a debug-only cargo feature, so its test lives in `androidTestDebug`.

**Sizes** (30.10.2026, minified `release`, `libcairn_ffi.so` built `opt-level = "z"`, LTO, stripped),
beside the research's unminified two-ABI APK of 5,402,986 B (its `classes.dex` 3,439,612 B):

| Item | Bytes |
| --- | --- |
| minified release APK, both ABIs | 6,511,831 |
| `classes.dex` after R8 (1,410,500 B stored; 682,546 B deflated) | 682,546 |
| `libcairn_ffi.so` `arm64-v8a` / `x86_64` | 2,008,000 / 2,087,136 |
| `libjnidispatch.so` `arm64-v8a` / `x86_64` | 165,992 / 116,904 |
| App Bundle (`app-release.aab`, all ABIs) | 7,527,432 |
| delivered to one handset, `arm64-v8a` (derived) | 4,297,031 |
| delivered to one emulator, `x86_64` (derived) | 4,327,743 |

The two per-ABI figures are the APK less the other ABI's three libraries, not a `bundletool`
measurement (it is not installed); an install delivers the base split plus that ABI's split, so the
real figure differs by the split containers' overhead. **The minified APK is larger than the
research's unminified one** because it is not the same app: it carries Compose, the design module
with its bundled faces and the generated bindings, which the spike did not; the two APKs are not
comparable beyond the dex, which R8 took from 3.4 MB to 1.4 MB despite the larger app.

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

This exercises the *other* half of the toolchain — a JDK driving Gradle and AGP (17 at the time), aapt2,
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
   `i686-linux-android` binaries cannot run on it at all, so the 32-bit targets are
   not exercised on our test hardware.
2. **The device runs 4 KB pages**, so it does *not* exercise the 16 KB page-size path at
   runtime. 16 KB alignment is still mandatory for Play uploads — it is satisfied at
   link time (see above) — but do not treat "works on this Pixel" as evidence about
   16 KB-page devices.
3. **The device API (37) is ahead of the build's `compileSdk` (36).** That is fine — an APK
   targeting 36 installs and runs on 37 — but it means the
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

Also: both the emulator (API 36) and the handset (API 37) are far above the declared `minSdk`
24, so neither can reproduce anything that breaks only on older Android (the earlier
client-stack research found one candidate crashing below API 30 for exactly that reason).
Testing `minSdk` 24 needs an API 24–29 system image installed separately.

## Reproducing this setup elsewhere

```sh
# JDK 27: the system's OpenJDK (Arch package `jdk-openjdk`), at /usr/lib/jvm/java-27-openjdk

# Android SDK cmdline-tools -> must land at $ANDROID_HOME/cmdline-tools/latest
curl -L -o cmdline-tools.zip \
  "https://dl.google.com/android/repository/commandlinetools-linux-15859902_latest.zip"
mkdir -p ~/Android/Sdk/cmdline-tools && unzip -q cmdline-tools.zip -d /tmp/ct
mv /tmp/ct/cmdline-tools ~/Android/Sdk/cmdline-tools/latest

export JAVA_HOME=/usr/lib/jvm/java-27-openjdk ANDROID_HOME=~/Android/Sdk
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
