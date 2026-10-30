plugins {
    id("com.android.application") version "9.4.1" apply false
    id("com.android.library") version "9.4.1" apply false
    // AGP 9 has Kotlin built in, so there is no `org.jetbrains.kotlin.android` plugin. The Compose
    // compiler is still a plugin, and it must be the version of the Kotlin AGP 9.4.1 builds with.
    id("org.jetbrains.kotlin.plugin.compose") version "2.2.10" apply false
}
