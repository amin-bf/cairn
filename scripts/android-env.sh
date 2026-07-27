# Android toolchain environment for leitner-app.
#
# Source this before any Android build:  source scripts/android-env.sh
#
# Everything below lives under $HOME and was installed without root.
# See docs/environment/android-toolchain.md for what was installed and why
# these exact versions.

export JAVA_HOME="$HOME/.local/share/jdk17"
export ANDROID_HOME="$HOME/Android/Sdk"
export ANDROID_SDK_ROOT="$ANDROID_HOME"

# Exactly one NDK is installed on purpose: the Tauri CLI selects the
# lexicographically highest installed NDK rather than its own pin, so a second
# (newer) NDK would silently displace the pinned one.
export NDK_HOME="$ANDROID_HOME/ndk/29.0.13846066"
export ANDROID_NDK_HOME="$NDK_HOME"
export ANDROID_NDK_ROOT="$NDK_HOME"

export GRADLE_HOME="$HOME/.local/share/gradle-8.14.3"

export PATH="$JAVA_HOME/bin:$GRADLE_HOME/bin:$ANDROID_HOME/cmdline-tools/latest/bin:$ANDROID_HOME/platform-tools:$ANDROID_HOME/emulator:$PATH"
