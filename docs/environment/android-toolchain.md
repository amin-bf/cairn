# Android toolchain

Record of the Android build environment set up for this repo, per
[#7 "Set up the Android toolchain and prove it works"](https://github.com/amin-bf/leitner/issues/7).

**Host**: CachyOS (Arch-based), x86_64, 16 cores. **Set up 2026-07-27.**

Everything below was installed **into `$HOME` without root.** Nothing was installed
via `pacman`, so nothing here is shared with the system package manager or will be
touched by a system upgrade.

## Quick start

```sh
source scripts/android-env.sh
```

That sets `JAVA_HOME`, `ANDROID_HOME`, `ANDROID_SDK_ROOT`, `NDK_HOME`,
`ANDROID_NDK_HOME`, `ANDROID_NDK_ROOT`, `GRADLE_HOME` and prepends the tool
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
| Android NDK | **29.0.13846066** (`r29-beta3`, clang 21.0.0) | `~/Android/Sdk/ndk/29.0.13846066` |
| platform-tools (`adb`, `fastboot`) | **r37.0.0** (`adb` 1.0.41) | `~/Android/Sdk/platform-tools` |
| Emulator | bundled with SDK | `~/Android/Sdk/emulator` |
| System image | `system-images;android-36;google_apis;x86_64` | `~/Android/Sdk/system-images` |
| Gradle | **8.14.3** | `~/.local/share/gradle-8.14.3` |
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
[#8 "Prototype: pick the client stack"](https://github.com/amin-bf/leitner/issues/8)
have different requirements, and this toolchain must satisfy **both**, since #8 has to
build each one on the same machine.

- **JDK 17.** Dioxus's generated Gradle module sets `jvmTarget = "17"`, so 17 is a hard
  floor; and Tauri **breaks cryptically on JDK 25/26**. 17 is the version both accept.
  (`dx` autodetects JDK 11 on Linux and would fail — hence setting `JAVA_HOME`
  explicitly is mandatory, not optional.)
- **SDK 34 and 36.** Dioxus generates `compileSdk`/`targetSdk` 34; the Tauri CLI uses
  SDK 36. Both platforms are installed so neither stack triggers a download mid-build.
- **NDK 29.0.13846066 — and *only* this one.** This is the Tauri CLI's pinned NDK. The
  CLI actually selects the **lexicographically highest installed NDK**, not its pin, so
  installing any newer NDK (30.x is available) would silently displace it. Keeping
  exactly one NDK makes the pin and the selection agree.
- **Gradle 8.14.3.** The version the Tauri CLI uses. Dioxus generates Gradle **9.1.0** +
  AGP 8.7.0 projects; those ship their own wrapper, which will download that
  distribution on first use, so the two do not conflict.

### Caveat: the pinned NDK is a beta

`source.properties` reports `Pkg.Revision = 29.0.13846066-beta3`,
`Pkg.ReleaseName = r29-beta3`. **Tauri's pinned NDK is a beta release**, not a stable
one. It works (see below), but it is worth knowing before debugging anything strange in
native code, and worth revisiting if #8 picks Tauri.

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

**3. APK packaging.** A minimal AGP project (Gradle 8.14.3, AGP 8.11.0, `compileSdk` 36,
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

An AVD named **`leitner-test`** is also set up and working — useful for CI-ish runs and
for the `x86_64` triple, which the handset cannot exercise:

- `system-images;android-36;google_apis;x86_64`, Android 16 / API 36, at
  `~/.android/avd/leitner-test.avd`
- Boots headless in **~15 s**:
  ```sh
  emulator -avd leitner-test -no-window -no-audio -no-boot-anim -no-snapshot \
           -gpu swiftshader_indirect -accel on
  ```
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
           "build-tools;34.0.0" "build-tools;36.1.0" "ndk;29.0.13846066" \
           "emulator" "system-images;android-36;google_apis;x86_64"

# API 37 (the test handset's level) is preview-channel and minor-versioned
sdkmanager --channel=3 "platforms;android-37.0" "build-tools;37.0.0"

rustup target add aarch64-linux-android x86_64-linux-android \
                  armv7-linux-androideabi i686-linux-android
cargo install cargo-ndk --locked

# Gradle 8.14.3
curl -L -o gradle.zip "https://services.gradle.org/distributions/gradle-8.14.3-bin.zip"
unzip -q gradle.zip -d ~/.local/share/

avdmanager create avd -n leitner-test -k "system-images;android-36;google_apis;x86_64"
```

`cargo-ndk` is only usable as `cargo ndk`; invoking the `cargo-ndk` binary directly
exits non-zero with *"This binary may only be called via `cargo ndk`"*. Do not read that
as a failed install.
