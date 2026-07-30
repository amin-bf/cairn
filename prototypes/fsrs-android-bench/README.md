# fsrs-android-bench

Throwaway harness for [#20](https://github.com/amin-bf/leitner/issues/20): does
`fsrs::compute_parameters` link, load and run on the real handset, and at what cost?
Findings live in [`docs/research/fsrs-on-device/`](../../docs/research/fsrs-on-device/README.md);
this directory is the evidence, not code to build on.

## Layout

| Path | What it is |
|---|---|
| `core/` | The measurement itself: synthetic corpus generation, timing, RSS and thread sampling. Built as `rlib` **and** `cdylib` — the `.so` the APK loads. |
| `cli/` | A binary, pushed to the device and run under `adb shell`. Separate crate from the `cdylib` on purpose: one crate with both makes `cargo-apk` exit non-zero (`AGENTS.md`, client-stack rule 5). |
| `android/` | Minimal AGP project that loads the `.so` and runs the same measurement inside a real app process, which is scheduled differently from a shell. |

## Running it

```sh
source scripts/android-env.sh      # from the repo root — mandatory, nothing is on PATH otherwise

# desktop reference
cargo run --release -p fsrsbench-cli -- 730000

# handset, under adb shell
cargo ndk -t arm64-v8a build --release
adb push target/aarch64-linux-android/release/fsrsbench /data/local/tmp/fsrsbench
adb shell chmod 755 /data/local/tmp/fsrsbench
adb shell "/data/local/tmp/fsrsbench 5000,20000,73000,250000,730000 0.05 20 73000"

# handset, inside an app process
cargo ndk -t arm64-v8a -o android/app/src/main/jniLibs build --release
(cd android && "$GRADLE_HOME/bin/gradle" --no-daemon assembleDebug)
adb install -r android/app/build/outputs/apk/debug/app-debug.apk
adb logcat -c
adb shell am start -n dev.leitner.fsrsbench/.MainActivity \
  --es spec "5000,73000,730000" --es tag foreground --el delay_ms 0
adb logcat -d -s FSRSBENCH
```

Arguments are `sizes cards_per_item seed eval_below`. `delay_ms` on the activity exists to start a
run and then press HOME before the work begins — that is how the backgrounded case was measured.

The release APK is unsigned; `assembleDebug` is what installs. The native library is a `--release`
build either way, which is the only thing the measurement depends on.
