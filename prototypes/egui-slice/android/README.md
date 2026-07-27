# GameActivity experiment — tried, reverted, kept as evidence

This Gradle project builds the slice against **GameActivity** instead of NativeActivity, to try to
get non-Latin (Persian) text input working on Android. **It does not work, and the reason is not
here.**

## What was expected

`android-activity` 0.6.1's NativeActivity backend implements no input method — `set_text_input_state`
and `set_ime_editor_info` are literally `// NOP: Unsupported`. Only key events arrive, which is why
basic ASCII works and Persian does not. Its README points at GameActivity, which has real IME via the
GameTextInput library.

## What actually happens

GameActivity implements IME correctly. **winit never reads it.**

`winit-0.30.13/src/platform_impl/android/mod.rs` handles exactly two input events —
`InputEvent::MotionEvent` and `InputEvent::KeyEvent`. There is no `Ime` handling, no
`text_input_state`, nothing. `set_ime_allowed` only calls `show_soft_input`/`hide_soft_input`, i.e.
it raises the keyboard and then discards everything the keyboard composes.

So the chain is `egui → egui-winit → winit → android-activity`, and the break is at **winit**, two
layers below anything we control. Confirmed empirically: the GameActivity build was installed on a
Pixel 8 Pro and Persian still could not be typed.

## What it cost

| | NativeActivity | GameActivity |
|---|---|---|
| APK | **5.4 MB** | 19 MB |
| `classes.dex` | none | 5.7 MB |
| Build | `cargo apk build` | Gradle project + `androidx.games:games-activity` AAR |
| Extra breakage | — | Kotlin stdlib duplicate-class conflict needing manual excludes |
| Persian input | ❌ | ❌ |

The only thing GameActivity does buy is `accesskit`, which `eframe` refuses alongside
native-activity — and accesskit has no web backend anyway, so it is native-only accessibility.

**Reverted to NativeActivity.** Kept here so nobody spends the day again.
