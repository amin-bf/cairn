// The native Android client (ADR-0045): Kotlin and Jetpack Compose over the shared Rust core. It
// lives beside `crates/` and builds the Rust core itself — see `buildSrc` and
// docs/environment/android-toolchain.md.
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "cairn-android"

include(":app")
include(":design")
