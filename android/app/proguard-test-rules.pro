# The instrumented-test APK is minified too when the tests run against the release build. The
# androidx.test libraries mention these compile-time-only annotations without shipping them.
-dontwarn com.google.errorprone.annotations.CanIgnoreReturnValue
-dontwarn com.google.errorprone.annotations.MustBeClosed

# The runner finds test classes and their @Test methods by reflection; without this R8 removes them
# as unused and the run reports zero tests.
-keep class dev.cairn.app.** { *; }
