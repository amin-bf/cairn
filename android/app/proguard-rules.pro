# Keep rules for the R8-minified release build (#208). R8 renames, removes and inlines what it cannot
# see used, and two things here are used only by name from native code or by reflection.

# --- JNA ---------------------------------------------------------------------------------------
# Source: the JNA FAQ, "How do I use JNA with ProGuard/R8?" (https://github.com/java-native-access/jna/blob/master/www/FrequentlyAskedQuestions.md),
# as quoted in docs/research/kotlin-binding/README.md §8. The .aar ships no consumer rules of its own.
# JNA references java.awt (desktop only) from code that never runs on Android.
-dontwarn java.awt.*
# JNA looks its own classes up by name from libjnidispatch.so, and reads structure fields and
# callback methods by reflection.
-keep class com.sun.jna.* { *; }
-keepclassmembers class * extends com.sun.jna.* { public *; }

# --- The generated bindings --------------------------------------------------------------------
# Source: this project. uniffi's Kotlin (package dev.cairn.ffi, from crates/ffi) declares the
# library's functions as JNA direct-mapped `external` methods, bound to the symbols of
# libcairn_ffi.so by the method's own name, and builds its Structure/Callback/Library subclasses and
# enums by reflection. A renamed or stripped member would surface as UnsatisfiedLinkError, or as an
# exception type the Rust side can no longer construct, so the whole package is kept as written.
-keep class dev.cairn.ffi.** { *; }
