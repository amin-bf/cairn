# Android toolchain environment for cairn-app.
#
# Source this before any Android build:  source scripts/android-env.sh
#
# Everything below lives under $HOME and was installed without root.
# See docs/environment/android-toolchain.md for what was installed and why
# these exact versions.

export JAVA_HOME="$HOME/.local/share/jdk17"
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"

# NDK r29 stable, the release line the binding was measured on. The Gradle project pins the same
# version as `ndkVersion` and builds with the directory AGP resolves from it; these variables serve
# the egui build (`cargo apk`) and a bare `cargo ndk`.
export NDK_HOME="$ANDROID_HOME/ndk/29.0.14206865"
export ANDROID_NDK_HOME="$NDK_HOME"
export ANDROID_NDK_ROOT="$NDK_HOME"

# No Gradle on PATH: the native client builds through its own wrapper (`android/gradlew`, 9.7.1).
export PATH="$JAVA_HOME/bin:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$PATH"
