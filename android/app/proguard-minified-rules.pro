# Added to the `minified` build type only (the release configuration plus what the test runner needs
# from the app's own dex, since app and runner share one process). Never part of the shipped release.
-dontwarn com.google.errorprone.annotations.CanIgnoreReturnValue
-dontwarn com.google.errorprone.annotations.MustBeClosed
-keep class androidx.tracing.** { *; }
# The runner calls into the stdlib and coroutines that the app alone would have shrunk.
-keep class kotlin.** { *; }
-keep class kotlinx.coroutines.** { *; }
